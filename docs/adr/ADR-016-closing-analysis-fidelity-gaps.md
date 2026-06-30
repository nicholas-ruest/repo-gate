# ADR-016 — Closing Analysis-Fidelity Gaps Between the MVP and ADR Design Intent

**Status:** Accepted

---

## Context

The RepoGate MVP (milestones P01–P17) was delivered on main. During the build, four deliberate simplifications diverged from the design intent recorded in ADR-006, ADR-007, and ADR-010, plus a cross-cutting observability shortcut that affects ADR-009 and ADR-013. These were pragmatic choices made to reach a working end-to-end pipeline, not accidental omissions. This ADR names each gap precisely, documents the accepted interim state, and records the decided remediation sequence so the work can be planned and tracked.

None of the remediations described below are yet implemented at the time this ADR is accepted. Code references below are accurate to the post-MVP main branch.

**Observed gaps in order of analytical impact:**

**Gap 1 — Schema enforcement not wired at runtime (ADR-007).**
`write_schema<T>` exists in `crates/repogate-core/src/claude_schemas.rs` and correctly derives JSON Schema from `schemars::JsonSchema`-annotated types. However, every phase invocation constructs `ClaudeInvocation` with `schema_path: None`:
- `crates/repogate-orchestrator/src/pipeline/arch_mapping.rs` line 59
- `crates/repogate-orchestrator/src/pipeline/feature_discovery.rs` line 145 (both the repomix path and the per-module fan-out)
- `crates/repogate-orchestrator/src/pipeline/synthesis.rs` line 37
- `crates/repogate-orchestrator/src/pipeline/risk_analysis.rs` line 62

Consequently, the `--json-schema` flag is never passed to the `claude` CLI. Output structure is enforced only by the prompt text, not by the model-level schema constraint mandated in ADR-007. On parse failure, phases fall back silently: `arch_mapping.rs` falls back to heuristic candidates (`unwrap_or(candidates)`); `feature_discovery.rs` silently skips the unparseable module (`continue`); `risk_analysis.rs` returns `RiskAnalysisOutput::default()` (`unwrap_or_default()`); `synthesis.rs` returns `None` for the boundary description. Additionally, `RiskAnalysisOutput` (defined locally in `risk_analysis.rs`) does not implement `schemars::JsonSchema` and is not registered in any `write_phase_schema` call, so its schema cannot be enforced even after schema_path is wired.

**Gap 2 — 8-dimension commercial score is not genuinely differentiated (ADR-010).**
`crates/repogate-core/src/claude_schemas.rs` defines `ModuleAssessment` with `commercial_value_estimate: Option<f32>` and `estimated_tier: Option<String>`. There is no `CommercialScore` field in `ModuleAssessment`. In `pipeline/runner.rs`, `build_scoring_inputs` reads `commercial_value_estimate` from the stored assessment and feeds it to `uniform_commercial(estimate)`, which seeds all eight `CommercialScore` dimensions with an identical value. The `CommercialScore` struct and `Score` type exist in `repogate-core` and are used by the scoring engine, but Claude Code is never asked to populate per-dimension values, so all eight dimensions carry the same number and the weighted scoring formula produces no differentiation.

**Gap 3 — License detection uses a signature-phrase heuristic, not `askalono` (ADR-006).**
The module docstring in `crates/repogate-licensing/src/detect.rs` explicitly states the gap: "askalono requires shipping a multi-megabyte license cache, so for the MVP we use a lightweight signature-phrase matcher here and treat askalono as a drop-in upgrade behind `identify_license_text`." The heuristic matches hardcoded uppercase phrases and returns a fixed confidence of 0.9 regardless of actual text similarity. License texts that deviate from verbatim standard text (e.g., customised Apache notices, older GPL preambles, or non-English translations) will not match. The integration point `identify_license_text` is already isolated and well-named.

**Gap 4 — Silent fallbacks reduce observability (cross-cutting: ADR-009, ADR-013).**
ADR-009 defines a job state machine with structured failure modes and requires that budget exhaustion produce a partial report with `incomplete: true`. ADR-013 requires structured signals when analysis is degraded. In the MVP, the three silent-fallback paths (arch-mapping heuristic, feature-discovery module skip, risk-analysis default) produce no structured signal: no `tracing` event, no `completeness_metadata` field in the pipeline output, and no `is_degraded` flag on the affected module assessments or the final `PipelineOutput`. A consumer receiving `is_complete: true` cannot distinguish a fully-analyzed result from one where several modules were silently skipped due to parse failures.

**Minor convention divergence (noted for the record).**
Schema types in `claude_schemas.rs` use `String` for timestamp fields rather than `chrono::DateTime<Utc>`. This was intentional for `JsonSchema`-friendliness (schemars derives cleaner JSON Schema for `String` than for chrono types) and is not worth reverting. Recorded here to prevent future confusion about why the schema uses `String` when the database layer uses `chrono`.

---

## Decision

The four gaps are addressed as a sequenced remediation, ordered by analytical leverage. All items below are **planned decisions** — they describe what will be implemented, not what has been implemented.

**Remediation 1 — Wire schema enforcement for all phases (highest leverage).**

For each phase invocation, `write_schema<T>` is called to write the phase's schema to a temp file, and `schema_path` is set on `ClaudeInvocation`:

- `arch_mapping.rs`: write `ModuleManifest` (or a new `ArchMapOutput` type) schema → set `schema_path`.
- `feature_discovery.rs`: write `ModuleAssessment` schema → set `schema_path` on both the repomix single-session path and every per-module fan-out invocation.
- `synthesis.rs`: write `SynthesisOutput` schema → set `schema_path`.
- `risk_analysis.rs`: add `schemars::JsonSchema` derive to `RiskAnalysisOutput`, write its schema → set `schema_path`.

Failure handling is upgraded from silent fallback to explicit retry-then-surface:
- On a `serde_json` deserialization error after schema-constrained output, the orchestrator retries the phase once (new session, same prompt and schema).
- If the retry also fails, the phase returns `OrchestratorError::SchemaViolation` (already defined). The job transitions to `failed` for that module with the partial output preserved, consistent with ADR-009.
- Silent `unwrap_or(candidates)`, `continue`, and `unwrap_or_default()` fallbacks are removed from the schema-constrained paths.

**Remediation 2 — Extend `ModuleAssessment` to carry a full `CommercialScore` and wire it into scoring.**

`ModuleAssessment` in `claude_schemas.rs` gains a `commercial_score: Option<CommercialScore>` field (the eight `Score` dimensions defined in `repogate-core`). The existing `commercial_value_estimate: Option<f32>` is retained as a fallback for backward compatibility with stored assessments that pre-date this change.

The feature-discovery prompt is extended to instruct Claude Code to populate each of the eight dimensions explicitly (adoption value, enterprise buyer value, commercial leverage, competitive sensitivity, operational value, security sensitivity, support burden, strategic importance), each 0–10, with a one-sentence rationale.

`build_scoring_inputs` in `runner.rs` is updated to:
1. Use `assessment.commercial_score` directly when present.
2. Fall back to `uniform_commercial(estimate)` when `commercial_score` is `None` (backward compatibility and budget-exhausted partial assessments).

The `uniform_commercial` function is retained but is now a degradation path, not the primary path. Its use is logged as a structured event (see Remediation 4).

**Remediation 3 — Integrate `askalono` behind the `identify_license_text` seam.**

`askalono` is added as a dependency of `repogate-licensing` behind a Cargo feature flag (`askalono-corpus`). When the feature is enabled:
- The `askalono` SPDX license cache is bundled (embedded at compile time via `include_bytes!` or loaded from a path configured at startup).
- `identify_license_text` is replaced by an `askalono`-backed implementation that returns the SPDX identifier and a genuine similarity-based confidence score.
- Detections below a configurable confidence threshold (default: 0.75, matching the existing `REVIEW_THRESHOLD`) set `needs_review: true`.

When the feature is disabled (the default for zero-dependency builds), the existing signature-phrase heuristic remains active as the fallback. The feature flag can be set per-deployment; the containerized production build enables it.

No changes are required outside `detect.rs` — the `identify_license_text` function signature is the documented drop-in point.

**Remediation 4 — Emit structured degradation signals.**

All silent-fallback paths emit `tracing::warn!` events with structured fields identifying the phase, module, and failure reason. The `PipelineOutput` struct gains a `completeness_metadata: CompletenessMetadata` field:

```rust
pub struct CompletenessMetadata {
    /// Modules for which the schema-constrained session failed and heuristics were used.
    pub degraded_modules: Vec<String>,
    /// Modules skipped due to budget exhaustion.
    pub budget_skipped_modules: Vec<String>,
    /// Whether the license detection used the heuristic fallback (askalono not available).
    pub license_detection_degraded: bool,
    /// Whether any scoring dimension used the uniform fallback instead of real per-dimension scores.
    pub scoring_degraded_modules: Vec<String>,
}
```

`is_complete` (existing field) is narrowed: it is `true` only when `degraded_modules`, `budget_skipped_modules`, and `scoring_degraded_modules` are all empty. The report rendering layer (ADR-011) surfaces `completeness_metadata` in both the canonical JSON artifact and the Markdown report, enabling consumers to distinguish a deep result from a degraded one.

---

## Consequences

**Positive:**
- After Remediation 1, the ~99.8% schema compliance guaranteed by ADR-007 is actually exercised at runtime. Parse failures become explicit errors rather than silent quality degradations.
- After Remediation 2, tier recommendations are differentiated across the 8 dimensions as designed in ADR-010. The synthesis phase receives genuinely varied per-module scores, producing more accurate and traceable gating recommendations.
- After Remediation 3, license-text matching handles non-verbatim license texts and modified notices that the heuristic misses entirely. Confidence scores are grounded in corpus similarity rather than fixed at 0.9.
- After Remediation 4, operators can inspect `completeness_metadata` to determine whether an assessment is trustworthy or requires re-running at higher budget or with `askalono` enabled.
- Each remediation is independently shippable — they do not depend on each other and can be implemented in sequence without breaking the existing pipeline.

**Negative / Trade-offs:**
- Remediation 1 removes the silent-fallback safety net. Jobs that previously produced a degraded-but-complete output will now produce a `failed` job with partial results when schema compliance fails after retry. This is the correct behavior per ADR-007 but is a user-visible breaking change for integrations that expect `is_complete: true` even on degraded runs.
- Remediation 2 requires prompt changes and a schema migration. Assessments stored by the MVP have no `commercial_score` field; the fallback to `uniform_commercial` handles this, but the gap between old and new assessments is a permanent record in the database.
- Remediation 3 adds a large compile-time artifact (the askalono SPDX corpus, ~2–4 MB) to the production binary when the feature flag is enabled. Container image size increases accordingly.
- Remediation 4 adds a new required field to `PipelineOutput`. Any code that constructs `PipelineOutput` directly (currently only `runner.rs` and tests) must be updated.
- The sequencing means Remediation 4 (observability) ships last, so the interim period between Remediations 1–3 still has incomplete degradation visibility. This is an acceptable trade-off given that Remediations 1–3 each reduce the incidence of degradation.

---

## Alternatives Considered

**Accept the MVP gaps as permanent simplifications** — The silent fallbacks produce a working pipeline and the tests pass. Rejected: the gaps directly compromise the platform's core value proposition (deep, not surface-level analysis) and the scoring model's traceability guarantee. Leaving them in place would make the ADR documentation materially misleading.

**Implement all four remediations simultaneously** — Reduces the period of partial improvement. Rejected: the remediations are independent but each requires careful testing. Sequential shipping reduces the risk of a regression in multiple systems simultaneously.

**Replace the heuristic license matcher with Claude Code inference (no askalono)** — Claude Code could reason about license text without an SPDX corpus. Rejected: ADR-006 explicitly requires a deterministic, auditable mechanism for license identification. LLM inference is acceptable as a secondary check on flagged files but not as the primary identifier.

**Surface degradation via HTTP 206 Partial Content** — Signal incomplete results at the HTTP layer rather than in the JSON body. Rejected: a `206` response to `GET /assessments/:id/report` is semantically odd (the report is complete as a document; it is the analysis that is incomplete). The `completeness_metadata` field in the JSON body is the correct mechanism.

**Add `schemars::JsonSchema` derive to `RiskAnalysisOutput` without moving it** — Keep `RiskAnalysisOutput` local to `risk_analysis.rs` and just add the derive. Rejected: the schema types that are exported to the `claude --json-schema` flag belong in `repogate-core::claude_schemas` to ensure the Rust type and the JSON Schema contract remain co-located and versioned together. `RiskAnalysisOutput` should be moved alongside `ModuleAssessment` and `SynthesisOutput`.

---

## Relationships

This ADR amends and extends the following ADRs. The original ADRs remain in force; this ADR records the gap between their intent and the MVP implementation, and the remediation decisions.

| ADR | Nature of amendment |
|---|---|
| ADR-006 (License and Dependency Analysis Stack) | Gap 3: `askalono` is not yet integrated; heuristic active. Remediation 3 closes this. |
| ADR-007 (Schema-Enforced Structured Output) | Gap 1: `--json-schema` not wired; silent fallbacks active. Remediation 1 closes this. |
| ADR-009 (Multi-Phase Pipeline with Partial-Result Persistence) | Gap 4 (partial): silent fallbacks are not surfaced in job state or partial-report metadata. Remediation 4 closes this. |
| ADR-010 (Commercial Value Scoring Model) | Gap 2: 8-dimension scoring seeded from a single float, not per-dimension model inference. Remediation 2 closes this. |
| ADR-013 (Token-Budget Enforcement and Pre-Run Cost Estimation) | Gap 4 (partial): budget-skipped modules are not reflected in `completeness_metadata`. Remediation 4 closes this. |
