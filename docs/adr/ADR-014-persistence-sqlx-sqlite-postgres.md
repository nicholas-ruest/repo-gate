# ADR-014 — Persistence with `sqlx`: SQLite for Dev/Local, Postgres for Production

**Status:** Accepted

---

## Context

RepoGate requires a relational database for:
1. **Job records**: tracking the state machine (ADR-009), job parameters, budget, timestamps, and error messages.
2. **Module assessments**: storing validated `ModuleAssessment` JSON objects keyed by job ID and module name (enables crash recovery and partial reporting).
3. **Assessment reports**: storing the canonical JSON artifact and artifact file paths.
4. **Analysis cache**: keyed by `(repo_url, commit_sha)` — if a repository at the same commit has already been analyzed, the cached assessment is returned without re-running Claude Code.

The database must support async access from Tokio (since the orchestrator and server are async Rust), compile-time SQL query validation (to catch SQL errors before they reach production), and two deployment modes: a lightweight single-file database for local/dev and a production-grade server for multi-tenant deployments.

---

## Decision

**ORM/query library: `sqlx`.**

`sqlx` provides:
- Async-native database access built on Tokio.
- Compile-time SQL query validation via the `sqlx::query!` macro: queries are checked against the database schema at compile time, catching SQL errors, column type mismatches, and missing tables during `cargo build`. This is the primary reason `sqlx` is chosen over `diesel` (which is not async-native) or `sea-orm` (which adds a higher-level abstraction at the cost of compile-time checking granularity).
- Support for SQLite and Postgres with the same query API (minor dialect differences handled via feature flags).

**SQLite for dev/local:**

SQLite is a single-file database requiring no external service. It is the default for:
- Local development (`repogate-cli` in local mode).
- CI test runs.
- Single-user self-hosted deployments.

The database file path is configured via `DATABASE_URL=sqlite://./repogate.db`.

**Postgres for production:**

Postgres is used for:
- Multi-tenant SaaS deployments of `repogate-server`.
- Deployments where multiple workers process jobs concurrently (Postgres's row-level locking is more suitable than SQLite's file-level lock under write contention).

The database URL is configured via `DATABASE_URL=postgres://user:password@host/dbname`.

**Compile-time checking:**

Compile-time SQL checking requires a live database connection during `cargo build`. For CI, a SQLite file is committed to the repository (`tests/fixtures/dev.db`) with the current schema applied. Developers run `cargo sqlx prepare` to regenerate the checked query cache when schemas change.

**Analysis cache:**

The cache table stores `(repo_url, commit_sha, assessment_id)`. Before starting a new job, the orchestrator checks for an existing completed assessment at the same commit. If found, the cached assessment is returned immediately. Cache invalidation is manual (via API or CLI) or time-based (configurable TTL, default 30 days).

---

## Consequences

**Positive:**
- Compile-time SQL validation catches query errors at build time, not at runtime in production.
- SQLite for dev requires no external service — `cargo run` works on a fresh clone.
- Postgres in production handles concurrent write load from multiple analysis workers correctly.
- The analysis cache eliminates redundant API costs when the same repository at the same commit is analyzed multiple times (common during development and testing of RepoGate itself).
- Async-native: no thread-pool overhead for database calls in the Tokio runtime.

**Negative / Trade-offs:**
- Compile-time checking requires a database connection during `cargo build`. This complicates builds in environments without database access (e.g., some CI setups). The `sqlx::query_as!` offline mode (using the `sqlx-data.json` cache file) mitigates this but requires keeping the cache file up to date.
- SQLite's file-level locking is a bottleneck under concurrent write load. Multiple workers writing `ModuleAssessment` records for the same job simultaneously can serialize badly. This is acceptable for small deployments but motivates the Postgres path for production.
- The two-database strategy means integration tests should run against both SQLite and Postgres. CI must provision a Postgres instance for the Postgres test suite.

---

## Alternatives Considered

**`diesel` (sync ORM)** — Compile-time checked queries with a strong type system. Does not natively support async Tokio; requires `spawn_blocking` wrappers, adding complexity. Rejected in favor of `sqlx`'s native async support.

**`sea-orm` (async ORM)** — Higher-level abstraction over `sqlx`, similar to ActiveRecord. Adds an additional abstraction layer that reduces compile-time checking granularity. Rejected — `sqlx` provides sufficient ergonomics with stronger query validation.

**MongoDB** — Document database; would eliminate the need to serialize/deserialize `ModuleAssessment` JSON for storage. But relational structure (job → module assessments → report) maps naturally to a relational schema, and compile-time SQL checking is a significant safety property. Rejected.

**Redis for caching only** — Using Redis for the analysis cache and a separate relational DB for job records. Adds a second database dependency. The cache can be implemented as a table in the primary database. Rejected for MVP.
