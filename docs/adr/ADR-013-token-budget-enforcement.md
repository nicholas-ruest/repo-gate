# ADR-013 — Token-Budget Enforcement and Pre-Run Cost Estimation

**Status:** Accepted

---

## Context

Running a RepoGate analysis job involves many Claude Code invocations, each consuming tokens billed at API list prices. For large repositories (hundreds of modules, complex synthesis), uncontrolled token usage could result in unexpectedly high costs.

A critical billing change effective June 15, 2026 must be understood:

> Programmatic `claude` CLI invocations (i.e., headless, non-interactive usage) draw from a **separate dollar-denominated credit pool** at **API list prices**, distinct from the subscription quota used for interactive Claude.ai sessions.

This means the cost of a RepoGate analysis job is not covered by a user's existing Claude subscription. It is charged to their API credits at the full API list price per token. This has direct implications for cost transparency and budget control.

Without budget enforcement:
- A single large-repository analysis could exhaust an operator's credit balance unexpectedly.
- Users have no visibility into expected cost before committing to a run.
- Token runaway (e.g., a module session that enters an unexpectedly deep traversal loop) has no circuit breaker.

---

## Decision

**Pre-run cost estimation:**

Before any Claude Code session is spawned, the `repogate-orchestrator` produces a cost estimate:
1. Count total files and estimated LOC from the module manifest.
2. Apply per-phase token estimates (configurable constants in `repogate-core`):
   - Manifest summarization: ~500 tokens input + ~200 output per module.
   - Module assessment (Sonnet): ~3,000 tokens input + ~800 output per module.
   - Module assessment (Opus): ~5,000 tokens input + ~1,200 output per module.
   - Synthesis: ~8,000 tokens input + ~2,000 output.
3. Multiply by API list prices for the selected models (`claude-opus-4-8`, `claude-sonnet-4-6`).
4. Return the estimate as `{ min_usd, max_usd, model_mix }` in the `POST /assessments` response and in the CLI output before prompting for confirmation.

**Budget enforcement:**

Each job is created with a `budget_usd` field (required, no default — must be set explicitly by the caller). The orchestrator tracks cumulative token usage against the budget in real time:
- After each Claude Code invocation, the token counts from the `usage` field of the `result` event are accumulated.
- When accumulated cost exceeds `budget_usd`, no new Claude Code sessions are spawned. The current session is allowed to complete.
- The job transitions to `failed` with `failure_reason: "budget_exhausted"`. Completed module assessments are preserved (ADR-009).
- A partial report is rendered from the completed assessments and marked `incomplete: true`.

**Budget types:**

| Type | Behavior |
|---|---|
| `hard` (default) | Job stops immediately when budget is exceeded. |
| `soft` | Job logs a warning and continues; operator is notified. Intended for dev/test only. |

**CLI confirmation:**

The `repogate-cli` shows the cost estimate and prompts for confirmation before starting a job (overridable with `--yes`):
```
Estimated cost: $0.42 – $1.20 (70% Sonnet / 30% Opus)
Budget set to: $5.00
Proceed? [y/N]
```

**Token tracking implementation:**

`repogate-orchestrator` accumulates `usage` events from the stream-JSON output. The `usage` object in each `result` event contains `input_tokens`, `output_tokens`, and (when applicable) `cache_read_input_tokens`. Cached tokens are billed at a reduced rate (currently ~10% of base input token price); the budget tracker accounts for this.

---

## Consequences

**Positive:**
- Operators and users always know the expected cost before committing to a run.
- Hard budget limits prevent runaway spending on a single job.
- Partial reports with `incomplete: true` give users value even when the budget is insufficient for a full analysis.
- The billing model (separate credit pool at API list prices) is surfaced clearly in the CLI and API documentation, preventing billing surprises.

**Negative / Trade-offs:**
- Pre-run cost estimates are approximations. Actual token usage depends on model behavior (tool calls, reasoning length) and repository characteristics. The estimate can be off by 2–3× in extreme cases.
- The `budget_usd` requirement adds friction for first-time users who must understand the billing model before starting a job. The CLI pre-run estimate partially mitigates this.
- Token tracking via stream events adds parsing complexity in the orchestrator: the `usage` field must be extracted from every `result` event and accumulated thread-safely across concurrent module sessions.
- If the model changes its token usage characteristics (e.g., longer reasoning chains in a new version), the estimate constants require recalibration.

---

## Alternatives Considered

**No budget limit (unlimited spending)** — Simple but irresponsible. A runaway session or a very large repository could exhaust credits unexpectedly. Rejected.

**Subscription-quota tracking only** — The June 15, 2026 billing change means programmatic usage is charged to a separate credit pool, not the subscription quota. Subscription-level tracking is irrelevant here. Not applicable.

**Count-based limits (max module sessions)** — Cap the number of module sessions rather than dollar spend. Simpler to implement but does not map to actual cost (Opus sessions cost more than Sonnet sessions). Rejected in favor of dollar-denominated budgets.

**Pre-purchase credits with a fixed pool** — Reserve credits upfront before starting a job. The Anthropic API does not support reserved credit pools per-job. Not supported by the platform.
