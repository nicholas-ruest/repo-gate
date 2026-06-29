# ADR-010 — Commercial Value Scoring Model

**Status:** Accepted

---

## Context

RepoGate's primary output is a per-module gating recommendation: should this module remain open source, become source-available, move to a paid tier, or require legal review before any decision?

This recommendation must be:
1. **Traceable** — users must understand why a module was rated as enterprise-tier vs. open.
2. **Configurable** — different organizations have different commercialization strategies; a security company weights security sensitivity differently than a developer-tools company.
3. **Deterministic** — given the same `ModuleAssessment` JSON and the same weights, the scoring engine always produces the same output.
4. **Separate from inference** — the scoring engine is deterministic Rust code. Claude Code produces the `ModuleAssessment` inputs (dimensional scores 0–10); the Rust engine maps those inputs to gating tiers. This separation ensures the final recommendation is auditable and not subject to model non-determinism.

---

## Decision

**Scoring dimensions:**

Each module is scored on 8 dimensions, each 0–10, produced by Claude Code as part of the `ModuleAssessment` schema:

| Dimension | Description |
|---|---|
| `adoption_value` | How much does this module drive open-source adoption and developer trust? |
| `enterprise_leverage` | How much does this module provide value specifically to enterprise buyers? |
| `competitive_sensitivity` | How much would open-sourcing this module benefit competitors? |
| `operational_value` | How much operational/infrastructure value does this module provide? |
| `security_sensitivity` | Does this module contain security-sensitive logic that warrants access control? |
| `support_burden` | How much support overhead would this module create if widely adopted for free? |
| `strategic_importance` | How central is this module to the project's long-term competitive position? |
| `gating_suitability` | How suitable is this module for gating without damaging community trust? |

Each dimension has a configurable weight (default weights defined in `repogate-core`; overridable per-job via the API or config file).

**License risk sub-score:**

A separate `license_risk_score` (0–10) is produced by `repogate-licensing` based on the copyleft risk matrix (ADR-006). A high `license_risk_score` can override gating tiers: a module with an AGPL dependency cannot be safely placed in a closed commercial tier regardless of its commercial value scores.

**Composite score:**

```
composite = sum(dimension_score[i] * weight[i]) / sum(weight[i])
```

The composite score is a weighted average, normalized to 0–10.

**Tier mapping (deterministic Rust rules engine):**

| Composite range | License risk | Tier |
|---|---|---|
| 0.0–2.5 | any | `open` |
| 2.5–4.5 | low | `source_available` |
| 4.5–6.5 | low | `pro_tier` |
| 6.5–8.0 | low | `enterprise_tier` |
| 8.0–10.0 | low | `managed_cloud` |
| any | high | `legal_review` |
| any | critical (AGPL/GPL transitive) | `not_recommended` |

The mapping is implemented as a pure function in `repogate-scoring`: `fn map_to_tier(composite: f32, license_risk: f32, weights: &Weights) -> GatingTier`. No model inference is involved in tier assignment.

**Confidence indicator:**

The orchestrator attaches a `confidence` field to each module assessment based on how completely Claude Code was able to analyze the module (number of files read, session completeness flag). Low-confidence assessments are flagged in the report for human review.

---

## Consequences

**Positive:**
- Traceability: the report includes per-dimension scores, weights, composite, and tier mapping — users see exactly why a module was rated as it was.
- Configurability: weights are a first-class input, tunable per organization or per job without code changes.
- Determinism: the scoring engine is pure Rust, no randomness. The same inputs always produce the same tier.
- Separation of concerns: Claude Code is responsible only for producing dimensional scores (0–10 per dimension). The Rust engine is responsible for the tier assignment. This makes the system easier to test and audit.

**Negative / Trade-offs:**
- The 8 dimensions are a design choice that reflects common open-core commercialization considerations. They may not perfectly fit all use cases; the weighting system partially compensates but cannot add new dimensions without a schema change.
- The tier thresholds in the rules engine are initial values. They will need calibration against real-world assessments.
- Claude Code's dimensional scores are model inference — they are not perfectly reproducible between runs. Two runs on the same module may produce slightly different 0–10 scores, leading to different composite values (and potentially different tiers near boundaries).

---

## Alternatives Considered

**Single LLM-produced tier recommendation** — Ask Claude Code directly "should this module be open or enterprise tier?" without a numerical scoring layer. Faster but not traceable, not configurable, and not deterministic. Rejected.

**ML classifier trained on labeled data** — Train a classifier on a labeled dataset of modules and gating decisions. Requires labeled training data that does not yet exist. May be explored post-MVP as a calibration tool. Rejected for MVP.

**Equal weights (no configurability)** — Simpler scoring but does not accommodate different organizational priorities. The overhead of making weights configurable is low. Rejected.
