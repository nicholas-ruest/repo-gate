# ADR-009 — Multi-Phase Analysis Pipeline with Partial-Result Persistence

**Status:** Accepted

---

## Context

A RepoGate analysis job for a large repository can take tens of minutes: cloning, file-tree walking, spawning multiple Claude Code module sessions, running license analysis, and synthesizing results. If any phase fails partway through — network interruption, model error, process crash, token budget exhaustion — all prior work is lost in a stateless pipeline.

This is unacceptable for two reasons:
1. **User experience**: re-running a job from scratch after a failure wastes time and money.
2. **Cost**: API token costs for completed module analyses are incurred regardless of whether the job finishes. Losing completed work means re-paying those costs on retry.

Additionally, the web UI (ADR-015) needs to display real-time job progress. A progress model requires a defined phase sequence with observable state transitions.

---

## Decision

The analysis pipeline is modelled as a state machine with the following phases:

```
queued → ingesting → analyzing → scoring → reporting → complete
                                                      ↘ failed
```

| Phase | Description |
|---|---|
| `queued` | Job created; waiting for worker capacity |
| `ingesting` | Cloning repo, walking file tree, building module manifest, running license scan |
| `analyzing` | Spawning Claude Code module sessions; collecting `ModuleAssessment` outputs |
| `scoring` | Running the deterministic scoring engine over validated module assessments |
| `reporting` | Rendering output artifacts (canonical JSON, Markdown, optional PDF) |
| `complete` | All artifacts available; job is read-only |
| `failed` | A non-recoverable error occurred; partial results are preserved |

**Persistence after each phase:**

The job record and all intermediate outputs are persisted to the database (ADR-014) after each phase completes. Specifically:
- After `ingesting`: the `ModuleManifest` is stored.
- After each module session within `analyzing`: the `ModuleAssessment` is stored (keyed by module name). A partially-completed `analyzing` phase preserves all assessments received so far.
- After `scoring`: the `ScoringResult` is stored.
- After `reporting`: artifact paths and the canonical JSON are stored.

**Crash recovery:**

On restart after a crash, the orchestrator queries the database for jobs in `ingesting`, `analyzing`, or `scoring` state. It resumes from the last completed checkpoint:
- If `ingesting` was in progress, it re-runs ingestion (fast, no API cost).
- If `analyzing` was in progress, it re-runs only the module sessions whose `ModuleAssessment` is not yet stored.
- If `scoring` or `reporting` was in progress, it re-runs that phase with the already-stored module assessments.

**Budget exhaustion:**

If the token budget (ADR-013) is exhausted during `analyzing`, the job transitions to `failed` but all completed `ModuleAssessment` records are preserved. The partial report (covering analyzed modules) is rendered and marked as incomplete. The user can re-run the job with a higher budget.

---

## Consequences

**Positive:**
- Work is never lost: a crash mid-`analyzing` preserves all completed module assessments.
- The progress model enables real-time UI feedback (the web dashboard polls job status and shows phase + completion percentage).
- Budget exhaustion produces a partial report rather than nothing, giving users actionable output even when interrupted.
- The state machine is a clear contract for the orchestrator, the API server, and the web UI.

**Negative / Trade-offs:**
- Each phase transition requires a database write. For the `analyzing` phase (one write per module assessment), this is many small writes. SQLite is adequate for local/dev; Postgres for production under concurrent job load.
- Crash recovery logic adds orchestrator complexity: it must correctly identify which module sessions need to be re-run vs. which are already complete.
- The `failed` state with partial results may confuse users who expect a complete report. The API response must clearly indicate which modules were analyzed and which were not.

---

## Alternatives Considered

**Stateless pipeline (no persistence between phases)** — Simpler to implement but loses all work on crash. Unacceptable for long-running jobs. Rejected.

**File-based checkpointing** — Write phase outputs to temporary files rather than the database. Works for single-node deployments but does not generalize to multi-worker deployments and is harder to query for progress. Rejected in favor of database persistence.

**Workflow engine (Temporal, Prefect)** — Provides built-in state persistence, retry logic, and observability. Adds significant operational complexity (external service dependency) for MVP. May be considered post-MVP if multi-tenancy and complex retry policies are needed. Rejected for MVP.

**Re-run from scratch on failure** — Simple but expensive and slow. Rejected.
