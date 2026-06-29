# RepoGate Implementation Prompts

This directory contains 17 implementation prompts (P01–P17) for building RepoGate, a deep repository assessment platform. Each prompt is a complete, self-contained task for a coding agent (Claude Code).

**Start here:** Read [`BUILD-MANIFEST.md`](BUILD-MANIFEST.md) for the authoritative build plan, phases, and dependencies.

---

## Prompt Index

| ID | Title | Phase | Depends on | Language |
|----|----|-------|-----------|----------|
| **P01** | [Cargo Workspace Skeleton + CI](P01-workspace-skeleton.md) | Foundations | — | Rust |
| **P02** | [repogate-core: Domain Types, Error Types, JSON Schema Contracts](P02-core-types-schemas.md) | Foundations | P01 | Rust |
| **P03** | [repogate-ingestion: Git Clone, File Walk, Language Detection, Binary Filtering](P03-ingestion-git-walk.md) | Ingestion | P02 | Rust |
| **P04** | [repogate-ingestion: Dependency Manifest Parsing + syft SBOM](P04-deps-sbom.md) | Ingestion | P03 | Rust |
| **P05** | [repogate-licensing: License Detection, SPDX Parsing, Copyleft Risk Matrix](P05-licensing.md) | Ingestion | P02, P04 | Rust |
| **P06** | [repogate-orchestrator: Claude Code Subprocess Driver](P06-claude-driver.md) | Orchestration core | P02, P03 | Rust |
| **P07** | [repogate-orchestrator: AssessmentJob State Machine, Token Budget, Crash Recovery](P07-state-machine-budget.md) | Orchestration core | P06 | Rust |
| **P08** | [repogate-orchestrator: Architecture Mapping Phase](P08-arch-mapping.md) | Analysis pipeline | P07 | Rust |
| **P09** | [repogate-orchestrator: Functionality Discovery Fan-Out Phase](P09-feature-discovery.md) | Analysis pipeline | P08 | Rust |
| **P10** | [repogate-scoring: Commercial Value Scoring Engine + Tier Classifier](P10-scoring-engine.md) | Scoring | P02, P05, P09 | Rust |
| **P11** | [repogate-orchestrator: Synthesis Phase (Gating Strategy + Risk Analysis)](P11-synthesis-risk.md) | Synthesis | P10, P09 | Rust |
| **P12** | [repogate-report: Report Assembly, minijinja Templates, Canonical JSON](P12-report-assembly.md) | Reporting | P11 | Rust |
| **P13** | [sqlx Schema, Migrations, Store Implementations](P13-sqlx-stores.md) | Persistence | P07, P12 | Rust |
| **P14** | [repogate-cli: CLI Entry Point, `repogate analyze`, Cost Estimation, Progress](P14-cli.md) | UX | P11, P12, P13 | Rust |
| **P15** | [repogate-server: axum HTTP Server, API Endpoints, Static Serving](P15-server.md) | UX | P11, P12, P13 | Rust |
| **P16** | [repogate-web: Next.js Dashboard (TypeScript)](P16-web-dashboard.md) | UX | P15 | TypeScript |
| **P17** | [End-to-End Integration Tests + Repomix Small-Repo Path](P17-e2e-repomix.md) | Hardening | P14, P15, P16 | Rust |

---

## Dependency Graph

```
P01 workspace skeleton
 ↓
P02 core types + schemas
 ├─ P03 git clone + file walk
 │  └─ P04 deps + syft ─────────────────┐
 │                                      ├─ P05 licensing ───────────┐
 ├─ P05 licensing (needs P04)           │                            │
 │                                      │                            │
 ├─ P06 Claude driver (needs P02, P03)  │                            │
 │  └─ P07 state machine + budget       │                            │
 │     └─ P08 arch mapping ─────────────┤─────────────┐              │
 │        └─ P09 fan-out discovery ─────┼──────┐      │              │
 │                                      │      ├─ P10 scoring ───────┘
 └────────────────────────────────────┘      │
                                             └─ P11 synthesis + risk
                                                 └─ P12 report
                                                    ├─ P13 sqlx stores ───┐
                                                    ├─ P14 CLI ──────┐    │
                                                    ├─ P15 axum ─────┤────┘
                                                    └─ P16 web ──────┘
                                                       └─ P17 e2e
```

**Parallelism opportunities:**
- **P05** (licensing) can run parallel to P03/P04 (only needs core + dependencies from P04)
- **P06** (Claude driver) can run parallel to P03/P04 (only needs core)
- **P14** (CLI) and **P15** (server) fully parallel (both depend on P11/P12/P13, not each other)

---

## How to Use These Prompts

### Prerequisites

- Rust 1.70+ installed
- Claude CLI authenticated
- syft binary available
- Git repo initialized (P01 creates Cargo workspace)

### Workflow

1. **Read the spec:** Start with [`BUILD-MANIFEST.md`](BUILD-MANIFEST.md) to understand the 17-unit build plan and architecture.

2. **Dispatch one prompt at a time:** Copy the contents of each prompt file (e.g., `P01-workspace-skeleton.md`) and paste it as a complete task into Claude Code.

3. **Respect dependencies:** Only dispatch a prompt when its dependencies are complete. Use the dependency graph above to determine readiness. For example:
   - ✅ Dispatch P01 (no dependencies)
   - ✅ After P01 completes, dispatch P02
   - ✅ After P02 completes, dispatch P03 and P06 in parallel (independent paths)
   - ❌ Do not dispatch P04 until P03 is complete

4. **Green status before next prompt:** Each prompt must compile and pass its acceptance criteria before the next is dispatched. Tests must pass: `cargo test --workspace` must succeed.

5. **Leverage parallelism:** When a prompt has no sequential dependencies on others, dispatch multiple prompts concurrently:
   - After P02: send P03, P05, P06 together (each independent)
   - After P10/P09: send P11 (ready after both feed into it)
   - After P11/P12/P13: send P14 and P15 together (both ready, independent)

6. **Partial completions:** If a prompt times out or a coding agent needs handoff, save progress in a checkpoint (the state machine in P07 supports this). The next prompt can resume from the checkpoint.

---

## Integration Points

### Core Types (P02)
All downstream crates depend on `repogate-core` for domain models, enums, and error types. **P02 is the critical path:** ensure it compiles before any other prompt.

### Claude Integration (P06 + P09)
- P06 builds the subprocess driver and streaming JSON parser
- P09 uses P06 to spawn Claude sessions per module
- Both reference specific ADRs for invocation flags: `--bare`, `--output-format stream-json`, `--json-schema`, `--allowedTools`

### Persistence (P13)
- P07 defines in-memory stores
- P13 replaces them with sqlx implementations
- Both P14 (CLI) and P15 (server) can use either store; tests use in-memory for speed

### Reporting (P12)
- Depends on pipeline output (P11)
- Produces canonical JSON (schema_version: "1.0") and Markdown via minijinja
- CLI (P14) and server (P15) both consume P12's report generation

### Web (P16)
- Depends on server API contract (P15)
- Static export: `next build` → `out/` → served by server with `--static-dir`
- TypeScript only; no Rust code in this prompt

---

## Testing & Validation

### Unit Tests
Each prompt includes unit tests. Validate locally:
```bash
cargo test --workspace
```

### Integration Tests (P17)
- E2E pipeline with mock Claude responses
- Crash recovery with checkpoints
- Repomix small-repo path validation

### CI/CD
- GitHub Actions workflow (`.github/workflows/ci.yml`) runs on every push
- All builds, tests, lints, and formatting must pass
- E2E workflow (`.github/workflows/e2e.yml`) runs on merge to main

---

## Reference Documents

- **[BUILD-MANIFEST.md](BUILD-MANIFEST.md)** — Authoritative 17-unit build plan with acceptance criteria
- **[../architecture.md](../architecture.md)** — System design, Cargo workspace, pipeline overview
- **[../adr/](../adr/)** — Architecture Decision Records (15 total); each prompt cites relevant ADRs
- **[../ddd/](../ddd/)** — Domain-Driven Design bounded contexts (10 total); each prompt cites relevant DDD docs

---

## Example Dispatch Sequence

### Phase 1: Foundations (Serial)
1. Dispatch **P01** (Cargo workspace)
2. After P01 ✅, dispatch **P02** (core types)

### Phase 2: Ingestion & Early Orchestration (Parallel)
3. After P02 ✅, dispatch **P03**, **P05**, **P06** in parallel
4. After P03 ✅, dispatch **P04**
5. After P04 ✅ (P05 checks completion), proceed

### Phase 3: Orchestration (Sequential)
6. After P06 ✅, dispatch **P07**
7. After P07 ✅, dispatch **P08**
8. After P08 ✅, dispatch **P09**

### Phase 4: Scoring & Synthesis (Sequential)
9. After P09 + P05 ✅, dispatch **P10**
10. After P10 ✅, dispatch **P11**

### Phase 5: Reporting & Persistence (Parallel)
11. After P11 ✅, dispatch **P12** and **P13** in parallel

### Phase 6: UX (Parallel, then Sequential)
12. After P12 + P13 ✅, dispatch **P14** and **P15** in parallel
13. After P15 ✅, dispatch **P16**

### Phase 7: Hardening (Final)
14. After P14 + P15 + P16 ✅, dispatch **P17**

---

## Support & Troubleshooting

### Compilation Errors
- Check that all `workspace.dependencies` are pinned in P01's `Cargo.toml`
- Ensure `serde` and `schemars` derives are present on all domain types (P02)
- Use `cargo check` to catch issues early

### Test Failures
- Run `cargo test --workspace -- --nocapture` for detailed output
- Each prompt includes specific unit tests; refer to the acceptance criteria
- Mock external calls (Claude, git, database) in tests; see P06 and P09 for patterns

### Database Issues (P13+)
- For SQLite dev, migrations are auto-applied at startup
- For Postgres, ensure database URL is set correctly
- sqlx `prepare` step requires an accessible database or `.sqlx` offline cache

### Claude Code Integration (P06, P09, P11)
- Ensure `claude` CLI is installed and authenticated: `claude auth login`
- Tests mock Claude responses; no live API calls in test suites
- In production (CLI/server), `REPOGATE_MOCK_SESSIONS=true` env var enables mock mode for offline testing

---

## Contributing

When adding new build units or updating prompts:
1. Update the manifest ([BUILD-MANIFEST.md](BUILD-MANIFEST.md)) first
2. Update this README's prompt index and graph
3. Cite relevant ADRs and DDD docs in each prompt's "Source Documents to Read" section
4. Ensure acceptance criteria are specific and testable
5. Include a code example for key functions or types

---

**Last Updated:** June 2026  
**Manifest Version:** 1.0  
**Build Units:** 17  
**Target Platform:** Rust (primary), TypeScript (web only)
