# ADR-001 — Rust as the Primary Implementation Language

**Status:** Accepted

---

## Context

RepoGate is an I/O-heavy platform: it clones repositories, walks file trees, spawns subprocess analysis sessions, streams JSON events, persists job state, and serves an HTTP API — all with significant concurrency. The platform must handle large repositories (hundreds of thousands of files) without GC pauses degrading throughput or introducing unpredictable latency spikes.

The analysis pipeline is safety-critical in the sense that correctness matters: incorrectly parsed license expressions, missed modules, or data corruption during job persistence could cause wrong commercialization recommendations. A strong type system that enforces invariants at compile time reduces this class of error materially.

The team evaluated Go, Python, and TypeScript as alternatives. Go is competitive for I/O concurrency but lacks the algebraic type system and zero-cost abstractions useful for the scoring engine and structured-output parsing. Python has excellent ML tooling but GIL constraints and runtime type errors are poor fits for a production orchestration daemon. TypeScript is a strong fit for the web UI layer but is a poor substrate for subprocess orchestration and binary-level file walking.

A Cargo workspace allows the platform to be split into focused crates with clear boundaries while sharing a single lock file, toolchain, and CI configuration.

---

## Decision

Rust is the primary implementation language for RepoGate. The codebase is structured as a Cargo workspace with the following member crates:

| Crate | Responsibility |
|---|---|
| `repogate-core` | Shared domain types, error types, scoring model structs |
| `repogate-ingestion` | Repository cloning, file-tree walking, language detection, SBOM extraction |
| `repogate-licensing` | License text matching, SPDX expression parsing, copyleft risk matrix |
| `repogate-orchestrator` | Claude Code subprocess lifecycle, session management, multi-turn orchestration |
| `repogate-scoring` | Deterministic rules engine mapping module assessments to gating tiers |
| `repogate-report` | Template rendering (JSON canonical output, Markdown, optional PDF) |
| `repogate-cli` | `repogate` binary — CLI entry point for local/CI usage |
| `repogate-server` | `axum`-based HTTP server; serves the Next.js build in production |

TypeScript is used exclusively for the Next.js web dashboard (see ADR-015). No Rust crate depends on any TypeScript artifact at compile time.

The async runtime is **Tokio**. All I/O (subprocess spawning, HTTP serving, database access, file reads) is async.

---

## Consequences

**Positive:**
- Memory safety without a garbage collector; no GC pauses during large file-tree walks or concurrent analysis sessions.
- Tokio's async I/O is a direct match for the workload: subprocess streams, HTTP connections, and DB queries are all naturally concurrent without threads-per-connection overhead.
- The type system enforces the data model at compile time — a `GatingTier` cannot be constructed from an invalid string; a `LicenseExpression` is validated on construction.
- Cargo workspaces give clean crate-level API boundaries, enabling independent testing and future extraction of crates (e.g., `repogate-licensing` as a standalone library).
- `cargo test`, `cargo clippy`, and `cargo audit` integrate cleanly into CI.

**Negative / Trade-offs:**
- Longer onboarding time for contributors unfamiliar with Rust ownership and lifetimes.
- No official Anthropic Rust SDK — the orchestrator must drive Claude Code via subprocess (see ADR-003, ADR-004).
- Compile times are longer than Go or Python; incremental compilation and `sccache` mitigate this in CI.
- Async Rust (`.await`, `Pin`, `Send` bounds) adds complexity compared to goroutines or async Python.

---

## Alternatives Considered

**Go** — Excellent concurrency model and fast compile times. Rejected because the algebraic type system (enums with associated data, `Result`/`Option`) is superior in Rust for modelling the complex state machine and scoring model without runtime panics.

**Python** — Rich ecosystem for text processing and ML. Rejected due to GIL limitations under concurrent subprocess orchestration, runtime type errors, and lack of a clean crate-boundary model for a multi-component platform.

**TypeScript (Node.js) for everything** — The web UI naturally lives in TypeScript, but using Node.js as the orchestration runtime introduces callback complexity for subprocess lifecycle management and loses the compile-time safety guarantees. Scoped to dashboard only (ADR-015).
