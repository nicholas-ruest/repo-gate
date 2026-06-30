# Architecture Decision Records — RepoGate

This directory contains the Architecture Decision Records (ADRs) for RepoGate. Each ADR documents a significant architectural decision: the context that motivated it, the decision made, its consequences, and the alternatives that were considered and rejected.

ADRs are append-only. Superseded decisions are marked with a reference to the superseding ADR rather than being deleted.

---

## Index

| ADR | Title | Status | Area |
|---|---|---|---|
| [ADR-001](ADR-001-rust-primary-language.md) | Rust as the Primary Implementation Language | Accepted | Language & Runtime |
| [ADR-002](ADR-002-claude-code-analysis-engine.md) | Claude Code as the Analysis and Reasoning Engine | Accepted | Analysis Engine |
| [ADR-003](ADR-003-headless-claude-code-invocation.md) | Headless Claude Code Invocation via the `claude` CLI | Accepted | Analysis Engine |
| [ADR-004](ADR-004-rust-native-orchestration-typescript-scope.md) | Rust-Native Orchestration; TypeScript Scoped to the Web Dashboard Only | Accepted | Orchestration |
| [ADR-005](ADR-005-git-ingestion-and-tree-walking.md) | Git Ingestion (subprocess `git` for MVP → `gix`), Tree Walking with `ignore`, Language Stats via `tokei` + `hyperpolyglot` | Accepted | Ingestion |
| [ADR-006](ADR-006-license-dependency-analysis.md) | License and Dependency Analysis Stack | Accepted | Licensing |
| [ADR-007](ADR-007-schema-enforced-structured-output.md) | Schema-Enforced Structured Output via `--json-schema` | Accepted | Analysis Engine |
| [ADR-008](ADR-008-deep-traversal-map-reduce.md) | Deep-Not-Surface Traversal: Sub-Agent-Per-Module and Map-Reduce Synthesis | Accepted | Analysis Engine |
| [ADR-009](ADR-009-multi-phase-pipeline-crash-recovery.md) | Multi-Phase Analysis Pipeline with Partial-Result Persistence | Accepted | Orchestration |
| [ADR-010](ADR-010-commercial-value-scoring-model.md) | Commercial Value Scoring Model | Accepted | Scoring |
| [ADR-011](ADR-011-assessment-output-formats.md) | Assessment Output Formats | Accepted | Reporting |
| [ADR-012](ADR-012-model-routing.md) | Model Routing | Accepted | Analysis Engine |
| [ADR-013](ADR-013-token-budget-enforcement.md) | Token-Budget Enforcement and Pre-Run Cost Estimation | Accepted | Cost & Operations |
| [ADR-014](ADR-014-persistence-sqlx-sqlite-postgres.md) | Persistence with `sqlx`: SQLite for Dev/Local, Postgres for Production | Accepted | Data Layer |
| [ADR-015](ADR-015-web-api-layer-axum-nextjs.md) | Web and API Layer: `axum` HTTP Server and Next.js Dashboard | Accepted | API & UI |
| [ADR-016](ADR-016-closing-analysis-fidelity-gaps.md) | Closing Analysis-Fidelity Gaps Between the MVP and ADR Design Intent | Accepted | Cross-cutting |
| [ADR-017](ADR-017-recalibrate-gating-score.md) | Recalibrate the Composite as a Gating-Pressure Score | Accepted | Scoring |

---

## Key Decision Summary

**Language:** Rust is the primary language for all analysis, orchestration, scoring, and server logic. TypeScript is used only for the Next.js web dashboard (ADR-001, ADR-004).

**Analysis engine:** Claude Code (`claude` CLI), driven headlessly via `tokio::process::Command`, not the Anthropic API directly (ADR-002, ADR-003). No TypeScript sidecar (ADR-004).

**Traversal strategy:** Map-reduce — Rust builds a module manifest, one Claude Code sub-agent per module produces a schema-validated assessment, a synthesis pass consumes the JSON summaries (ADR-008).

**Structured output:** All Claude Code invocations use `--json-schema` for schema enforcement; no post-hoc parsing (ADR-007).

**Ingestion stack:** subprocess `git` for the MVP (→ `gix` post-MVP) for cloning, `ignore` (tree walk), `tokei` + `hyperpolyglot` (language), `askalono` + `spdx` + `cargo_metadata` + `syft` (licensing) (ADR-005, ADR-006).

**Scoring:** Deterministic Rust rules engine over 8 weighted Claude-produced dimensions; maps to discrete gating tiers (ADR-010).

**Models:** `claude-opus-4-8` for synthesis and complex modules; `claude-sonnet-4-6` for bulk classification (ADR-012).

**Persistence:** `sqlx` with SQLite (dev) / Postgres (production); compile-time SQL validation (ADR-014).

**Pipeline:** Multi-phase state machine with per-phase persistence for crash recovery and partial reporting (ADR-009).

**Budget:** Dollar-denominated budget enforcement with pre-run cost estimation; note June 15 2026 billing change — programmatic `claude` draws from a separate credit pool at API list prices (ADR-013).

**API + UI:** `axum` REST API + Next.js static dashboard; single binary serves both in production (ADR-015).

**MVP fidelity gaps and remediation:** Four gaps between ADR design intent and the MVP implementation — schema enforcement not wired, 8-dimension scoring seeded from a single float, heuristic license detection instead of `askalono`, and silent fallbacks without structured observability signals — are documented with sequenced remediations in ADR-016.

---

## ADR Format

Each ADR uses the following structure:

```
# ADR-NNN — Title

**Status:** Accepted | Superseded by ADR-NNN

## Context
## Decision
## Consequences
## Alternatives Considered
```
