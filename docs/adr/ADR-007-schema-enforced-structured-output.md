# ADR-007 — Schema-Enforced Structured Output via `--json-schema`

**Status:** Accepted

---

## Context

RepoGate's pipeline depends on machine-readable output from Claude Code at every analysis phase: module assessments, license analysis summaries, synthesis outputs, and the final gating recommendation. These outputs feed into the scoring engine (ADR-010), report rendering (ADR-011), and job persistence (ADR-014).

If model output is free text, the orchestrator must parse it — extracting structured data from prose using regex, string matching, or a secondary parsing prompt. This approach has well-known failure modes:

- Output format varies between runs; regex parsers are brittle.
- Nested structures (arrays of module assessments, each with sub-scores) are error-prone to extract from prose.
- Parsing failures mid-pipeline require fallback logic that is difficult to test comprehensively.
- Post-hoc parsing cannot guarantee that a required field is present or that a value falls within an expected enum.

Claude Code's `--json-schema` flag addresses this at the model level: the model is constrained to produce output that conforms to the provided JSON Schema before the response is returned. Anthropic reports ~99.8% compliance with schema-constrained output in production.

---

## Decision

**Every Claude Code invocation in the RepoGate pipeline uses `--json-schema <path>` to enforce structured output.** Post-hoc regex parsing of model responses is not used.

The workflow:
1. The `repogate-orchestrator` crate serializes the relevant JSON Schema (defined as Rust structs with `schemars::JsonSchema` derive) to a temp file before each invocation.
2. The `--json-schema <path>` flag is passed to `claude` at invocation time.
3. The orchestrator reads the `result` event from the stream and deserializes it using `serde_json::from_str` into the corresponding Rust struct.
4. Deserialization failure is treated as a `OrchestratorError::SchemaViolation` and causes the phase to be retried once before failing the job.

**Schema ownership:**

JSON Schemas are the source of truth for inter-component contracts. They are defined as Rust structs in `repogate-core` using `serde` and `schemars` derives, then exported to JSON Schema via `schemars::schema_for!`. This ensures the Rust type and the JSON Schema are always in sync — if the struct changes, the schema changes.

**Phase schemas include:**

| Phase | Root type |
|---|---|
| Module manifest | `ModuleManifest` |
| Module assessment | `ModuleAssessment` |
| License analysis | `LicenseAnalysis` |
| Synthesis | `SynthesisOutput` |
| Final report | `AssessmentReport` |

---

## Consequences

**Positive:**
- ~99.8% schema compliance eliminates most parsing failures without defensive fallback logic.
- Rust struct definitions are the single source of truth — the JSON Schema is derived from them, not maintained separately.
- Deserialization errors are caught immediately after the model responds, not silently propagated as missing fields downstream.
- Schema evolution is tracked in git: a change to `ModuleAssessment` is visible in the Rust diff, not buried in a string constant.

**Negative / Trade-offs:**
- The `--json-schema` flag may not be supported in all Claude Code versions. Version pinning is required (see ADR-013).
- Complex schemas (deeply nested, many optional fields) increase prompt overhead — the schema is included in the model's context.
- The ~0.2% non-compliance rate means the orchestrator must handle schema validation failures gracefully (retry once, then fail with partial output).
- `schemars` does not support all JSON Schema features; some constraints (pattern-validated strings, cross-field conditionals) require custom validation after deserialization.

---

## Alternatives Considered

**Post-hoc regex parsing** — Extract structured data from free-text responses using regular expressions or string matching. Brittle, hard to maintain, and does not enforce presence or type of required fields. Rejected.

**Two-step: free text then extraction prompt** — First call produces prose, second call extracts structure. Doubles token cost and latency per phase. Rejected.

**Response format via system prompt only** — Instruct the model to produce JSON via system prompt without using `--json-schema`. Compliance is lower (~95%) and the format is not validated at the model level before the response is returned. Rejected in favor of schema enforcement.

**Protocol Buffers / MessagePack** — Binary serialization formats. Claude Code does not support structured output in binary formats. JSON Schema is the only supported schema language for `--json-schema`. Not applicable.
