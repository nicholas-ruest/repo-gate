# ADR-012 — Model Routing

**Status:** Accepted

---

## Context

RepoGate's analysis pipeline uses Claude Code at multiple points with different complexity and cost profiles:

- **Module manifest summarization**: classifying files into modules, detecting languages, producing the initial manifest. This is largely pattern-matching and classification — high volume, lower reasoning depth required.
- **Bulk module assessment (most modules)**: analyzing a module's functionality, producing dimensional scores, identifying hidden capabilities. Mid-complexity reasoning; the model must read code and understand intent but most modules are not architecturally ambiguous.
- **High-value or architecturally complex modules**: deep reasoning across cross-cutting concerns, ambiguous licensing, enterprise functionality buried in internal modules. Requires the model's strongest cross-file reasoning.
- **Final synthesis**: consuming all module assessments and producing the executive summary and final gating recommendation. Requires the model to reason across the entire repository's module graph, identify cross-module interactions, and produce coherent commercialization advice.

Using the most capable (and most expensive) model for every phase would be accurate but would make costs prohibitive for large repositories. Using a cheaper model for everything would degrade quality on the phases that most benefit from depth.

Model IDs referenced here are the exact Claude model IDs as of the decision date.

---

## Decision

**Two-tier model routing:**

| Tier | Model | Use Cases |
|---|---|---|
| Deep | `claude-opus-4-8` | Final synthesis, high-value/ambiguous module assessment, cross-file architectural reasoning, license ambiguity resolution |
| Bulk | `claude-sonnet-4-6` | Module manifest summarization, bulk classification of modules scoring below a complexity threshold, routine module assessments |

**Routing logic:**

The `repogate-orchestrator` crate selects the model for each Claude Code invocation based on:

1. **Phase**: synthesis always uses `claude-opus-4-8`. Manifest construction always uses `claude-sonnet-4-6`.
2. **Module complexity indicators** (from the manifest): modules with > 50 files, cross-module dependency count > 10, or containing keywords indicating enterprise features (`auth`, `rbac`, `audit`, `billing`, `enterprise`, `compliance`) are routed to `claude-opus-4-8`. All other modules use `claude-sonnet-4-6`.
3. **Explicit override**: the job API accepts a `model_override` field that forces all phases to use a specific model (useful for cost estimation tests or high-accuracy runs).

**Model selection in CLI flags:**

The model is passed to `claude` via the `--model <model-id>` flag (or via environment variable `CLAUDE_MODEL` as a fallback).

**Cost implications:**

`claude-opus-4-8` input tokens cost approximately 5× `claude-sonnet-4-6` at list prices. Routing the majority of bulk module analyses to Sonnet and reserving Opus for synthesis + complex modules targets a 60–70% cost reduction vs. all-Opus, with minimal quality loss on routine modules.

---

## Consequences

**Positive:**
- Significant cost reduction for typical repositories where most modules are straightforward to classify.
- Quality is preserved where it matters most: synthesis and architecturally complex modules use the strongest model.
- The routing logic is explicit and auditable — the job record stores which model was used for each module session.
- `model_override` enables controlled experiments comparing Opus vs. Sonnet output quality on the same repository.

**Negative / Trade-offs:**
- Complexity indicators for routing (keyword matching, file count thresholds) are heuristics. Some complex modules may be misrouted to Sonnet; some simple modules may use Opus unnecessarily.
- Model pricing and capability relative rankings can change. The routing thresholds (file count, keyword list) may need recalibration as models evolve.
- Using two different models in the same pipeline means the synthesis pass receives assessments of varying depth. The Opus synthesis model must be prompted to account for this.

---

## Alternatives Considered

**All `claude-opus-4-8`** — Maximum quality but 5× higher cost for bulk module assessments. Unacceptable for large repositories (1000+ modules). Rejected as default; available via `model_override`.

**All `claude-sonnet-4-6`** — Lower cost but degraded quality on the synthesis and high-value module passes, which are the most critical outputs of the platform. Rejected.

**`claude-haiku-4-5-20251001` for simplest tasks** — Haiku is even cheaper for pure classification tasks. May be introduced in a future tier for initial file-type filtering and binary detection classification. Not included in MVP to avoid three-tier routing complexity.

**Dynamic routing based on real-time cost tracking** — Route to cheaper models when approaching budget limits mid-job. Adds complexity and makes output quality non-deterministic across runs. Budget enforcement (ADR-013) handles budget limits; routing is kept static per module for reproducibility. Rejected.
