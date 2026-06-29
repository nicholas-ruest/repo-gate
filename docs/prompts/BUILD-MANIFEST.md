# RepoGate — Build-Order Manifest (17 Implementation Prompts)

This manifest defines the dependency-ordered sequence of implementation prompts for building RepoGate. It reconciles `docs/initial_spec.md`, `docs/architecture.md`, all 15 ADRs (`docs/adr/`), and all 10 DDD bounded contexts (`docs/ddd/`).

The manifest is strictly **bottom-up**: nothing in step N depends on anything not yet built in steps 1..N-1. Each `Pxx` unit is expanded into a standalone prompt file (`docs/prompts/Pxx-<slug>.md`). A coding agent (Claude Code) receives exactly one prompt file at a time and works to green status on its acceptance criteria before the next prompt is dispatched.

**Crate-name note:** `docs/architecture.md` and the ADRs use `repogate-*` prefixes; some DDD docs use `rg-*` prefixes. **The architecture doc is authoritative for crate names.** DDD bounded-context → crate mappings are noted per prompt.

---

## Build Phases

| Phase | Prompts |
|-------|---------|
| Foundations | P01, P02 |
| Ingestion | P03, P04, P05 |
| Orchestration core | P06, P07 |
| Analysis pipeline | P08, P09 |
| Scoring | P10 |
| Synthesis | P11 |
| Reporting | P12 |
| Persistence | P13 |
| UX | P14, P15, P16 |
| Hardening | P17 |

---

## P01 — Cargo Workspace Skeleton + CI

- **Phase:** Foundations
- **Depends on:** nothing
- **Language:** Rust (all crates)

**Scope / deliverables:**
- `Cargo.toml` workspace root declaring all member crates: `repogate-core`, `repogate-ingestion`, `repogate-licensing`, `repogate-orchestrator`, `repogate-scoring`, `repogate-report`, `repogate-cli`, `repogate-server`
- Each crate initialized as a minimal `lib.rs`/`main.rs` with one passing placeholder test
- Shared workspace dependency versions pinned in `[workspace.dependencies]`: `tokio` (full), `serde` + `serde_json`, `schemars`, `sqlx`, `axum`, `clap`, `tracing`, `anyhow`, `thiserror`
- `.github/workflows/ci.yml`: `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`
- `.gitignore`, `rust-toolchain.toml` pinning stable toolchain
- `tests/fixtures/dev.db` placeholder (empty SQLite) for compile-time `sqlx` checking in CI

**Source docs:** ADR-001 (workspace + crate list), ADR-014 (dev.db for sqlx compile-time checking)

**Acceptance criteria:**
- `cargo build --workspace` — zero errors, zero warnings
- `cargo test --workspace` — all placeholder tests pass
- `cargo clippy --workspace -- -D warnings` passes
- CI workflow is syntactically valid (`actionlint`/`yamllint`)

---

## P02 — `repogate-core`: Domain Types, Error Types, JSON Schema Contracts

- **Phase:** Foundations
- **Depends on:** P01
- **Language:** Rust

**Scope / deliverables:** Core Rust types with `serde::Serialize/Deserialize` + `schemars::JsonSchema` derives.

`types.rs` — value objects & enums:
- `GatingTier`: `Open | SourceAvailable | ProTier | EnterpriseTier | ManagedCloud | LegalReview | NotRecommended`
- `Severity`: `Low | Medium | High`
- `RiskKind`: `OverGating | CommunityBacklash | LicenseConflict | CompetitiveExposure | SecurityExposure | AccidentalOpenSource | UnderGating`
- `Layer`: `Core | Api | Sdk | Cli | Connector | Integration | Deployment | Test | Documentation`
- `Language` enum (Rust, TypeScript, Python, Go, Java, …; `Other(String)`)
- `Score` newtype over `f32`, invariant `0.0 <= v <= 10.0`, `Score::new(f32) -> Result<Score, ScoreRangeError>`
- `CompositeScore` newtype over `f32` (0.0–10.0)
- `CommercialScore`: 8 `Score` fields (ADR-010 dims): `adoption_value`, `enterprise_buyer_value`, `commercial_leverage`, `competitive_sensitivity`, `operational_value`, `security_sensitivity`, `support_burden`, `strategic_importance`
- `ScoreWeights`: 8 `f32` fields; `default()` (expert-tuned); `new(...) -> Result<_, WeightError>` enforcing weights >= 0.0
- `GatingSignal`: `StrongGateCandidate | WeakGateCandidate | OpenCandidate | Undetermined`
- `TokenBudget`: `total_limit/per_phase_limit/per_session_limit: u64`, `warn_threshold: f32`; `is_exceeded(used)`, `remaining(used)`

`model.rs` — domain aggregates: `Repository`, `Module`, `Capability`, `Visibility` (`Public | Internal | Experimental | Undocumented | Enterprise`), `SourceLocation`, `Risk`, `RepositoryMetrics`, `Assessment` (canonical output incl. `schema_version`, `generated_at`, `is_complete`), `GatingStrategy`, `TierAssignment`.

`claude_schemas.rs` — Claude output schemas: `ModuleAssessment`, `CapabilityFinding`, `DiscoveryMethod` (`PublicApi | TestCoverage | ExampleCode | CliInspection | SourceTracing | ConfigAnalysis | DocumentationCross | LlmInference`), `SynthesisOutput`, and `pub fn write_schema<T: schemars::JsonSchema>(path: &Path) -> anyhow::Result<()>`.

`error.rs` — `RepogateError` (thiserror) + `ScoreRangeError`, `WeightError`, `SchemaViolationError`, `StoreError`, `OrchestratorError`.

**Source docs:** ADR-007 (schemars + schema export), ADR-010 (8 dims + tier ranges), ADR-011 (`schema_version`), architecture.md (Data Model), DDD: CommercialValuation, AssessmentOrchestration (`TokenBudget`), FunctionalityDiscovery (`Visibility`, `DiscoveryMethod`, `SourceLocation`)

**Acceptance criteria:**
- `cargo build -p repogate-core` — zero warnings; all structs impl `Serialize`/`Deserialize`/`JsonSchema`
- `Score::new(-1.0)` → `Err`; `Score::new(5.0)` → `Ok`
- `write_schema::<ModuleAssessment>` writes valid JSON Schema
- Unit tests: Score range, ScoreWeights validation, `TokenBudget::is_exceeded`, `Assessment` round-trip

---

## P03 — `repogate-ingestion`: Git Clone, File Walk, Language Detection, Binary Filtering

- **Phase:** Ingestion
- **Depends on:** P02
- **Language:** Rust

**Scope / deliverables:**
- `git.rs` — `GitProvider` trait + `SubprocessGit` impl: `git clone --depth=1 --filter=blob:none <url> <dest>` via `tokio::process::Command`; resolve `HEAD` via `git rev-parse HEAD`; `IngestionError::CloneFailed` on non-zero exit; URL validation rejecting `file://`, `localhost`, RFC-1918 IPs (security boundary per `RepoUrl` invariant)
- `walk.rs` — `ignore::WalkBuilder` (gitignore-aware); emit `FileEntry { path, size_bytes, is_binary, language, hash: FileHash (BLAKE3) }`; binary detection (null byte in first 8 KB OR known-binary extension); generated-file detection (`.gitattributes linguist-generated`, `vendor/`, `node_modules/`, `*.min.js`); parallel walk
- `language.rs` — `tokei` (library) aggregate LOC stats + `hyperpolyglot` per-file classification → `repogate-core::Language`; `LanguageStats: HashMap<Language, LocCount>`
- `manifest.rs` — `RepoManifest { repo_id, total_files, total_loc, language_stats, root_dirs, file_entries, package_files }`; `PackageFileRef` with type `Cargo | Npm | PyProject | GoMod | Maven | Gradle | Gemfile | Unknown`
- `lib.rs` — `pub async fn ingest(url, dest) -> Result<RepoManifest, IngestionError>`

**Source docs:** ADR-005 (subprocess git, ignore, tokei, hyperpolyglot, binary filter, GitProvider trait), DDD: RepositoryIngestion (`FileEntry`, `ModuleManifest`, invariants, `RepoUrl`)

**Acceptance criteria:**
- `cargo test -p repogate-ingestion` passes
- Integration test: clone `https://github.com/rust-lang/regex` to temp; `total_files > 50`, `language_stats` contains `Rust`, no binary entry has a language
- `file://` and `http://localhost` → `Err(InvalidUrl)`
- `.png` entry → `is_binary: true`, `language: None`

---

## P04 — `repogate-ingestion`: Dependency Manifest Parsing + `syft` SBOM

- **Phase:** Ingestion
- **Depends on:** P03
- **Language:** Rust

**Scope / deliverables:**
- `deps/cargo.rs` — `cargo metadata --format-version 1` subprocess parsed via `cargo_metadata`; extract name, version, SPDX license string, repo URL, transitive deps → `Vec<DependencyRecord>` (ecosystem Cargo)
- `deps/sbom.rs` — `syft <repo-path> -o spdx-json --quiet` subprocess; parse SPDX JSON; `Err(SyftNotFound)` (no panic) if missing; merge + dedup with Cargo output
- `deps/mod.rs` — `extract_dependencies(manifest, repo_path) -> Result<Vec<DependencyRecord>, IngestionError>`; detect manifest types from `PackageFileRef`; graceful fallback; attach `dependencies: Vec<DependencyRecord>` to `RepoManifest`
- `DependencyRecord { name, version, ecosystem, spdx_license: Option<String>, is_direct, is_transitive }`; `Ecosystem { Cargo | Npm | PyPi | Go | Maven | Gradle | Ruby | Unknown }`

**Source docs:** ADR-006 (cargo_metadata, syft subprocess, multi-ecosystem SBOM), DDD: RepositoryIngestion (`package_files`)

**Acceptance criteria:**
- Rust repo → non-empty `Vec<DependencyRecord>` with correct ecosystems
- `syft` missing → `Err(SyftNotFound)`, no panic
- Cargo license `"MIT OR Apache-2.0"` preserved verbatim (parsing is licensing's job)
- `cargo test -p repogate-ingestion` still passes

---

## P05 — `repogate-licensing`: License Detection, SPDX Parsing, Copyleft Risk Matrix

- **Phase:** Ingestion (parallel with P03/P04)
- **Depends on:** P02, P04
- **Language:** Rust

**Scope / deliverables:**
- `detect.rs` — `askalono` detection over `LICENSE*`/`COPYING*`/`NOTICE*`/`LICENCE*` + inline `SPDX-License-Identifier:` headers (first 30 lines); `LicenseDetection { file_path, spdx_expression, confidence, detection_method }`; confidence < 0.75 → `needs_review`
- `spdx.rs` — `spdx::Expression::parse` to validate/normalize; identify compound (`MIT OR Apache-2.0`), exceptions (`GPL-2.0-only WITH Classpath-exception-2.0`), unknowns; extract base identifiers
- `copyleft.rs` — `CopyleftTier { StrongCopyleft | WeakCopyleft | SourceAvailableNonOsi | Permissive | PublicDomain | Unknown }`; `classify_license(spdx_id) -> CopyleftTier`; `copyleft_risk_score(tier) -> f32`
- `report.rs` — `LicenseReport { repo_id, detections, dependency_licenses, copyleft_exposure, missing_licenses, conflicts, overall_risk_score }`; `build_report(detections, deps)`
- `lib.rs` — `pub async fn analyze(manifest, repo_path) -> Result<LicenseReport, LicensingError>`

**Source docs:** ADR-006 (askalono, spdx, copyleft matrix), ADR-010 (license risk sub-score), DDD: LicenseCompliance

**Acceptance criteria:**
- `classify_license("AGPL-3.0")` → StrongCopyleft; `"MIT"` → Permissive; `"BSL-1.1"` → SourceAvailableNonOsi
- askalono identifies MIT text with confidence > 0.90
- `"GPL-2.0-only WITH Classpath-exception-2.0"` parses without error
- Repo with `license: "GPL-3.0"` → `overall_risk_score >= 8.0`
- `cargo test -p repogate-licensing` passes

---

## P06 — `repogate-orchestrator`: Claude Code Subprocess Driver

- **Phase:** Orchestration core
- **Depends on:** P02, P03
- **Language:** Rust

**Scope / deliverables:** `repogate-orchestrator/src/claude/` — lowest-level Claude Code integration.
- `claude/invocation.rs` — `ClaudeInvocation { prompt, model: ClaudeModel, schema_path, allowed_tools, system_prompt, working_dir, session_id }`; `ClaudeModel { Opus /*claude-opus-4-8*/, Sonnet /*claude-sonnet-4-6*/ }`; `build_command()` → `claude --bare -p "<prompt>" --output-format stream-json --json-schema <path> --allowedTools "<tools>" --append-system-prompt "<sys>" --model <id> [--resume <id>]`
- `claude/stream.rs` — `ClaudeEvent { Init{session_id} | Assistant{content} | ToolResult{..} | Result{content, usage} | Error{message, code} }`; `UsageStats { input_tokens, output_tokens, cache_read_input_tokens }`; `parse_stream(reader) -> impl Stream<Item = Result<ClaudeEvent, StreamError>>` over newline-delimited JSON; capture `session_id` from first `Init`
- `claude/session.rs` — `run_session(invocation) -> Result<SessionResult, OrchestratorError>` (`SessionResult { session_id, output, usage }`); non-zero exit/`Error` → `SessionFailed`; failed deserialize → `SchemaViolation`
- `claude/routing.rs` — `select_model(module, phase) -> ClaudeModel`: Synthesis → Opus; ManifestSummarization → Sonnet; module analysis → Opus if `files > 50` OR name/path matches `auth|rbac|audit|billing|enterprise|compliance|security`, else Sonnet
- `claude/schema.rs` — `write_phase_schema(phase, dir) -> Result<PathBuf, SchemaError>` via `repogate-core::write_schema<T>()`

**Source docs:** ADR-002, ADR-003 (invocation flags), ADR-004 (pure Rust, no TS sidecar), ADR-007 (schema-enforced), ADR-012 (model routing)

**Acceptance criteria:**
- `build_command()` contains `--bare`, `--output-format stream-json`, `--json-schema`, `--allowedTools`
- `parse_stream` deserializes `Init`/`Result`/`Error` from canned newline-delimited JSON
- `select_model`: synthesis → Opus; small non-enterprise module → Sonnet; 60-file module → Opus
- `cargo test -p repogate-orchestrator` passes (mock command output; no live Claude calls)

---

## P07 — `repogate-orchestrator`: AssessmentJob State Machine, Token Budget, Crash Recovery

- **Phase:** Orchestration core
- **Depends on:** P06
- **Language:** Rust

**Scope / deliverables:** `repogate-orchestrator/src/job/` — durable process manager.
- `job/state.rs` — `JobStatus { Queued | Ingesting | Analyzing | Scoring | Strategizing | RiskAnalyzing | Reporting | Complete | Failed | Recovering }`; `PhaseKind { Ingestion | LicenseScan | ArchitectureMapping | FeatureDiscovery | Scoring | Strategy | RiskAnalysis | ReportAssembly | ManifestSummarization | Synthesis }`; `PhaseStatus`; `AnalysisPhase { id, job_id, phase_kind, status, started_at, completed_at, tokens_used, session_ids, error, retry_count }`
- `job/budget.rs` — `BudgetTracker` over `TokenBudget` with `AtomicU64`; `record_usage(input, output, cache_read) -> BudgetStatus { Ok | Warning | Exceeded }` (cache_read billed at 10% of input price); `estimated_cost_usd() -> f64` from Opus/Sonnet list prices (constants in core)
- `job/checkpoint.rs` — `JobCheckpoint { job_id, last_completed_phase, completed_module_ids, token_usage_so_far, partial_results, saved_at }`; `CheckpointStore` trait + `InMemoryCheckpointStore`; `phases_to_run(checkpoint) -> Vec<PhaseKind>` (only un-completed)
- `job/store.rs` — `AssessmentJobStore` + `ModuleAssessmentStore` traits (+ `find_concurrent_for_repo`) and `InMemory*` impls (`Arc<Mutex<HashMap>>`)
- `job/gates.rs` — `validate_gate(phase, state) -> Result<(), PhaseGateError>`: Ingestion→Analysis (manifest non-empty); Analysis→FeatureDiscovery (`LicensesScanned` + `DependencyGraphBuilt`); FeatureDiscovery→Scoring (all modules have ≥1 stored `ModuleAssessment`)

**Source docs:** ADR-009 (state machine, per-phase persistence, crash recovery, budget exhaustion), ADR-013 (budget, hard/soft modes, dollar tracking, partial reports), DDD: AssessmentOrchestration (state machine, `AnalysisPhase`, `TokenBudget`, `JobCheckpoint`, phase-gate invariants)

**Acceptance criteria:**
- `BudgetTracker`: `record_usage(500,200,0)` → total 700; `is_exceeded` true when over limit
- `phases_to_run` at checkpoint `LicenseScan` returns `ArchitectureMapping` onward
- `validate_gate(FeatureDiscovery→Scoring)` fails when not all module assessments stored
- `InMemoryAssessmentJobStore` save→find_by_id round-trips
- `cargo test -p repogate-orchestrator` passes

---

## P08 — `repogate-orchestrator`: Architecture Mapping Phase

- **Phase:** Analysis pipeline
- **Depends on:** P07 (impl); runtime needs P04 + P05 complete
- **Language:** Rust

**Scope / deliverables:** `pipeline/arch_mapping.rs` — `run_architecture_mapping_phase(manifest, repo_path, session_runner, model) -> Result<ArchitectureMap, OrchestratorError>`.
- Deterministic module-boundary heuristics (pre-LLM): top-level dir grouping (`src/`, `cli/`, `lib/`, `tests/`, `examples/`, `docs/`); Cargo workspace members; npm `workspaces`; language clustering (>80% one language = one module); size cap (>500 files split by subdir)
- Claude manifest-summarization session (Sonnet): identify primary modules + roles + key deps; schema `ArchitectureMapOutput { modules: Vec<ModuleSummary>, dependency_edges: Vec<(String,String)>, architecture_notes }`; `ModuleSummary { name, path, layer, file_count, loc, centrality }`
- `ArchitectureMap { modules: Vec<ModuleNode>, edges, ascii_diagram }`; `ModuleNode { id, name, path, layer, centrality, file_count, loc, has_public_interface }`
- ASCII diagram generator (text-art tree)

**Source docs:** ADR-008 (boundary heuristics, Sonnet manifest summarization), DDD: ArchitectureMapping (`ModuleNode`, `DependencyEdge`, `Layer`, `Centrality`), ADR-012 (ManifestSummarization → Sonnet)

**Acceptance criteria:**
- Heuristic test: fake manifest with `src/`, `cli/`, `tests/`, `examples/` → 4 module candidates
- Cargo workspace with 3 members → exactly 3 modules
- `ArchitectureMap` (3 nodes, 2 edges) serializes to valid JSON
- `cargo test -p repogate-orchestrator` passes

---

## P09 — `repogate-orchestrator`: Functionality Discovery Fan-Out Phase

- **Phase:** Analysis pipeline
- **Depends on:** P08
- **Language:** Rust

**Scope / deliverables:** `pipeline/feature_discovery.rs` — `run_feature_discovery_phase(arch_map, repo_path, session_runner, module_store, budget, job_id, max_concurrent) -> Result<FunctionalityInventory, OrchestratorError>`.
- Fan-out one Claude session per `ModuleNode`; skip if already stored (crash recovery); `select_model(module, FeatureDiscovery)`; context prompt instructs deep inspection (public/internal/experimental/undocumented/enterprise capabilities, API/CLI/SDK entry points); `--allowedTools "Read,Glob,Bash(grep *),Bash(find *)"`; on success deserialize `ModuleAssessment` → store + record usage; on `BudgetExceeded` stop spawning, let in-flight finish, mark budget-exhausted; concurrency via `tokio::sync::Semaphore` (default 4)
- `pipeline/llm_adapter.rs` — `parse_module_assessment(raw) -> Result<ModuleAssessment, SchemaViolationError>`; `map_to_functionality_items(assessment, module_path) -> Vec<FunctionalityItem>` (Visibility from `is_enterprise`/`is_undocumented`; LLM-inferred items without source location → `discovery_method: LlmInference`, `is_confirmed: false`)
- `FunctionalityInventory { repo_id, items, total_count, hidden_count, enterprise_count, api_entry_points }`

**Source docs:** ADR-008 (sub-agent-per-module, tool allowlist, concurrency, `ModuleAssessment` gate), ADR-003 (tool allowlist), ADR-013 (budget enforcement), ADR-012 (per-module routing), DDD: FunctionalityDiscovery (invariants, Visibility rules, `LlmOutputAdapter`)

**Acceptance criteria:**
- Mock `SessionRunner` with canned JSON: saves assessments, skips already-analyzed modules, respects concurrency cap
- `is_enterprise: true` → `Visibility::Enterprise`; `is_undocumented: true` → `Visibility::Undocumented`
- Budget exhaustion: no new sessions after exceeded; stored assessments preserved
- `cargo test -p repogate-orchestrator` passes

---

## P10 — `repogate-scoring`: Commercial Value Scoring Engine + Tier Classifier

- **Phase:** Scoring
- **Depends on:** P02, P05, P09
- **Language:** Rust

**Scope / deliverables:** `repogate-scoring/src/`.
- `scoring/engine.rs` — `compute_composite(scores, weights) -> CompositeScore`: weighted average normalized to [0,10]; `support_burden` is SUBTRACTED; clamp [0,10]
- `scoring/license_risk.rs` — `apply_license_risk(composite, exposure) -> (CompositeScore, Option<LicenseRiskSubScore>)`: StrongCopyleft → cap 2.0; WeakCopyleft → −2.0 (min 0); SourceAvailableNonOsi → −1.0; Permissive/PublicDomain → no change; `effective_composite <= composite`
- `scoring/tier.rs` — `map_to_tier(effective_composite, license_risk) -> GatingTier`: 0–2.5 Open; 2.5–4.5 (low risk) SourceAvailable; 4.5–6.5 ProTier; 6.5–8.0 EnterpriseTier; 8.0–10.0 ManagedCloud; high license risk → LegalReview; critical (AGPL transitive) → NotRecommended
- `scoring/gating_signal.rs` — `derive_gating_signal(effective_composite, adoption_value)`: ≥7.0 Strong; [5,7) Weak; <5 OR adoption≥8 Open; all-zero Undetermined
- `scoring/report.rs` — `score_all_modules(module_assessments, inventory, license_report, weights) -> ValuationReport { module_scores, strong_gate_count, open_count, legal_review_count }`

**Source docs:** ADR-010 (full scoring model), DDD: CommercialValuation (value objects, invariants)

**Acceptance criteria:**
- All scores 5.0, equal weights (except support_burden) → composite 5.0
- `support_burden` 10.0 with others 5.0 → composite < 5.0
- AGPL module → `LegalReview`
- `map_to_tier(7.5, None)` → `EnterpriseTier`
- `derive_gating_signal(3.0, 9.0)` → `OpenCandidate` (adoption overrides)
- `cargo test -p repogate-scoring` passes; 100% coverage on tier classifier

---

## P11 — `repogate-orchestrator`: Synthesis Phase (Gating Strategy + Risk Analysis)

- **Phase:** Synthesis
- **Depends on:** P10, P09
- **Language:** Rust

**Scope / deliverables:**
- `pipeline/synthesis.rs` — `run_synthesis_phase(valuation, inventory, license_report, arch_map, session_runner) -> Result<GatingStrategy, OrchestratorError>`; model always Opus; prompt over JSON summaries (not raw code); schema `SynthesisOutput`; map → `GatingStrategy` with `tier_assignments` from `ValuationReport`
- `pipeline/risk_analysis.rs` — `run_risk_analysis_phase(strategy, valuation, license_report, inventory, session_runner) -> Result<RiskProfile, OrchestratorError>`; model Sonnet; schema `RiskAnalysisOutput { risks: Vec<RiskFinding> }`; map → `Vec<Risk>`
- `RiskProfile { risks, blocking_risk_count, high_severity_count, overall_risk_level }`
- `pipeline/runner.rs` — `PipelineRunner::run(url, budget, weights) -> Result<PipelineOutput, OrchestratorError>` sequencing P03→P04→(P05‖P08)→P09→P10→P11; `PipelineOutput { manifest, arch_map, license_report, inventory, valuation, strategy, risk_profile, is_complete }`

**Source docs:** ADR-008 (synthesis pass, Opus, JSON summaries), ADR-012 (synthesis Opus, risk Sonnet), DDD: GatingStrategy, RiskAnalysis

**Acceptance criteria:**
- Mock canned `SynthesisOutput` → `GatingStrategy` with populated `tier_assignments`
- Canned `RiskAnalysisOutput` `is_blocking: true` → `Risk::is_blocking: true`
- `PipelineRunner::run` with mock session runners + small local repo completes
- `cargo test -p repogate-orchestrator` passes

---

## P12 — `repogate-report`: Report Assembly, `minijinja` Templates, Canonical JSON

- **Phase:** Reporting
- **Depends on:** P11
- **Language:** Rust

**Scope / deliverables:** `repogate-report/src/`.
- `assembly.rs` — `assemble(output, generated_at) -> Assessment` filling every field; executive summary from strategy + top risks; inventory sorted enterprise-first; `is_complete` from pipeline
- `json.rs` — `write_json` / `to_json_bytes` via `serde_json::to_writer_pretty`; validate `schema_version` ("1.0")
- `markdown.rs` — `minijinja` rendering of `templates/report.md.jinja2` (embedded via `include_str!`); sections: Exec Summary, Module Table, Full Dimensional Scores, License/Dependency Posture, Legal Review Flags, Gating Recommendations + Rationale, Risk Analysis; `render_markdown(assessment) -> Result<String, ReportError>`
- `pdf.rs` — `render_pdf(markdown, output_path)` via `pandoc` subprocess; `Err(PandocNotFound)` if missing; opt-in
- `naming.rs` — `report_stem(repo_url, completed_at) -> "repogate-{owner}-{repo}-{YYYYMMDD-HHmmss}"` (slugified)

**Source docs:** ADR-011 (canonical JSON, minijinja, schema_version, pandoc PDF, naming), DDD: ReportDelivery

**Acceptance criteria:**
- `assemble` → `is_complete: true` when pipeline complete
- `render_markdown` of minimal `Assessment` contains "Executive Summary" and "Gating Recommendations"
- `report_stem("https://github.com/acme/myproject", …)` → `"repogate-acme-myproject-<ts>"`
- `write_json` + `from_str::<Assessment>` round-trips
- `cargo test -p repogate-report` passes

---

## P13 — `sqlx` Schema, Migrations, Store Implementations

- **Phase:** Persistence
- **Depends on:** P07, P12
- **Language:** Rust

**Scope / deliverables:** sqlx-backed stores in `repogate-server/src/db/` (server owns persistence; CLI links it for local SQLite).
- Migrations (`repogate-server/migrations/`): `0001_jobs.sql`, `0002_module_assessments.sql`, `0003_checkpoints.sql`, `0004_reports.sql`, `0005_cache.sql` (composite PK `(repo_url, commit_sha)`, TTL)
- `db/job_store.rs` — `SqlxAssessmentJobStore` (compile-time `sqlx::query!`; upsert; `find_by_status`; `find_concurrent_for_repo`)
- `db/module_store.rs` — `SqlxModuleAssessmentStore`
- `db/checkpoint_store.rs` — `SqlxCheckpointStore`
- `db/cache.rs` — `AnalysisCacheStore { get, set(ttl_days), invalidate }`
- `db/pool.rs` — `create_pool(database_url) -> Result<sqlx::AnyPool, sqlx::Error>`; runs `sqlx::migrate!()`; supports `sqlite://` + `postgresql://`
- `cargo sqlx prepare` → commit `sqlx-data.json`; `scripts/prepare-sqlx.sh`

**Source docs:** ADR-014 (sqlx, SQLite dev / Postgres prod, compile-time queries, offline mode, cache TTL, recovery queries), DDD: AssessmentOrchestration (store traits)

**Acceptance criteria:**
- `cargo build -p repogate-server` with sqlx offline mode (`sqlx-data.json` committed)
- `sqlx migrate run --database-url sqlite://test.db` clean on fresh DB
- `SqlxAssessmentJobStore` save→find_by_id round-trips full job
- Cache set→get returns stored assessment; past-TTL → `None`
- `cargo test -p repogate-server` passes (in-memory `sqlite://:memory:`)

---

## P14 — `repogate-cli`: CLI Entry Point, `repogate analyze`, Cost Estimation, Progress

- **Phase:** UX
- **Depends on:** P11, P12, P13
- **Language:** Rust

**Scope / deliverables:** `repogate-cli/src/`.
- clap structure: `analyze <repo-url>` with `--output <json|markdown|pdf>` (default markdown), `--budget <USD>` (required), `--output-file`, `--weights <json>`, `--model-override <opus|sonnet>`, `--max-concurrent <N>` (default 4), `--yes`, `--verbose`; `cache invalidate <url>` / `cache list`
- `commands/analyze.rs` — validate URL; instantiate `PipelineRunner` with `InMemoryCheckpointStore` + `SqlxModuleAssessmentStore` (SQLite `~/.config/repogate/repogate.db`); pre-run cost estimate to stderr (`Estimated cost: $X.XX – $Y.YY`); confirmation prompt unless `--yes`; run with progress callback (`[ingesting] …`, `[analyzing] module 3/12 …`, `[scoring] …`, `[reporting] …`); write output per `--output`; summary to stderr; non-zero exit on error with partial-output path
- `commands/cache.rs` — wrappers over `AnalysisCacheStore`
- `progress.rs` — `ProgressReporter` trait + stderr impl

**Source docs:** ADR-013 (confirmation flow, `--yes`, estimate, hard budget), ADR-015 (CLI uses SQLite, not server), DDD: UserWorkflow (`AnalysisRequest`, `SubmittedUrl`, `Submission` idempotency), architecture.md (CLI mode)

**Acceptance criteria:**
- `repogate analyze --help` shows `--budget` as required
- Missing `--budget` → non-zero exit + clear error
- `--yes` skips confirmation (integration test, small repo)
- Budget exhaustion → partial report `is_complete: false`; non-zero exit
- `cargo build -p repogate-cli` → binary; `--help` works

---

## P15 — `repogate-server`: `axum` HTTP Server, API Endpoints, Static Serving

- **Phase:** UX
- **Depends on:** P11, P12, P13
- **Language:** Rust

**Scope / deliverables:** `repogate-server/src/`.
- `main.rs` — clap args `--listen` (default `0.0.0.0:8080`), `--database-url`, `--static-dir`, `--api-key`; init pool + migrations; build `axum::Router`; background `tokio::task` running `PipelineRunner` (one job at a time for MVP; `Arc<Mutex<VecDeque<JobId>>>` queue)
- Routes: `POST /assessments` (→ `{job_id, estimated_cost_min, estimated_cost_max}`), `GET /assessments/:id`, `GET /assessments/:id/status` (`{status, current_phase, progress_pct, tokens_used}`), `GET /assessments/:id/report` (`text/markdown`), `GET /assessments/:id/report.pdf` (404 if none), `DELETE /assessments/:id`, `GET /health`
- Request body: `{ repo_url, budget_usd, model_override, weights }`
- Auth: `tower` layer checking `Authorization: Bearer <key>` (401 on mismatch; `/health` exempt)
- Static serving via `tower-http::ServeDir` (Next.js export); `GET /` → `index.html`
- Error handling: `IntoResponse` for `RepogateError`; 400/404/500 JSON `{error, code}`

**Source docs:** ADR-015 (axum, ServeDir, all endpoints, Next.js static export, bearer auth, 3s polling, no WebSocket for MVP), ADR-014 (Postgres prod / SQLite dev), DDD: UserWorkflow (`JobView`, `ResultView` DTOs)

**Acceptance criteria:**
- `cargo build -p repogate-server` — zero warnings
- `POST /assessments` valid body → 200 `{job_id, estimated_cost_min, estimated_cost_max}`
- `POST /assessments` without `Authorization` → 401
- `GET /assessments/:id/status` → `{status: "queued"}` for fresh job
- `GET /health` → 200 without auth
- Integration: submit → poll → fetch JSON → assert `schema_version`

---

## P16 — `repogate-web`: Next.js Dashboard (TypeScript)

- **Phase:** UX
- **Depends on:** P15 (API contract stable)
- **Language:** TypeScript

**Scope / deliverables:** `repogate-web/`.
- Setup: `next.config.js` (`output: 'export'`, `trailingSlash: true`, dev proxy rewrites to `:8080`); `tsconfig.json`; `package.json` (`next`, `react`, `react-dom`, `typescript`, `tailwindcss`)
- `src/app/page.tsx` — landing form (URL input + budget input + submit) → `POST /assessments` → redirect `/jobs/[id]`; API key in `localStorage`
- `src/app/jobs/[id]/page.tsx` — poll `GET /assessments/:id/status` every 3s; step indicator (Ingesting→Analyzing→Scoring→Reporting→Complete); progress bar; on complete fetch full report + render tabs
- `src/components/ReportViewer.tsx` — tabs: Executive Summary | Module Map | Gating Recommendations | License Posture | Full Inventory
- `src/lib/api.ts` — typed client (`submitAssessment`, `pollStatus`, `fetchReport`); TS interfaces mirror `repogate-core` JSON
- Build: `next build` → static `out/` → passed as `--static-dir`

**Source docs:** ADR-015 (static export, 3s polling, 5 tabs, localStorage key, dev proxy), ADR-004 (TS only for web), DDD: UserWorkflow (`JobView`/`ResultView`)

**Acceptance criteria:**
- `npm run build` — no TypeScript errors
- `npm run dev` — submitting a URL calls `POST /assessments`
- With running server + completed assessment, viewer renders all 5 tabs, no console errors
- `next export` → static `out/`; `repogate-server --static-dir out/` serves `GET /`

---

## P17 — End-to-End Integration Tests + Repomix Small-Repo Path

- **Phase:** Hardening
- **Depends on:** P14, P15, P16
- **Language:** Rust (+ TS web e2e if added)

**Scope / deliverables:**
- `tests/integration/e2e_pipeline.rs` — full `PipelineRunner::run` against a real small repo with a mock `SessionRunner` (deterministic canned JSON, no live API); assert `is_complete`, non-empty arch/valuation/strategy; crash-recovery test (simulate crash at FeatureDiscovery after 2/5 modules; resume re-analyzes only 3)
- Repomix small-repo path (in P09 `feature_discovery.rs`): if `total_loc < 50_000`, single Claude session over full repo (`repomix --output-format xml <path>` → single-session prompt; fallback to fan-out if repomix missing); single-session output schema = `ModuleAssessment` with `module_name: "all"`
- `.github/workflows/e2e.yml` — on merge to main: clone pinned small repo (e.g. `https://github.com/BurntSushi/toml`); `repogate analyze … --budget 0.50 --yes --output json` with mock sessions; assert exit 0 + valid `Assessment`
- `tests/fixtures/`: `canned_module_assessment.json`, `canned_synthesis_output.json`, `canned_risk_output.json`, `dev.db`

**Source docs:** ADR-008 (<50k LOC Repomix single-session; >500k tree-sitter deferred), ADR-009 (crash recovery), ADR-013 (partial results), ADR-003 (tool allowlist boundary)

**Acceptance criteria:**
- `cargo test -p repogate-orchestrator --test e2e_pipeline` passes, 0 live API calls
- Crash recovery: resume after crash at module 2 → only modules 3–5 (3 sessions)
- Repomix path: <50k LOC manifest → exactly 1 session
- `repogate-cli analyze https://github.com/BurntSushi/toml --budget 0.50 --yes` (with `REPOGATE_MOCK_SESSIONS=true`) → exit 0 + JSON file
- All CI jobs pass on clean checkout

---

## Dependency Graph

```
P01 workspace
 └ P02 core types + schemas
    ├ P03 git clone + file walk
    │  └ P04 deps + syft
    │     └ P05 licensing ─────────────┐
    ├ P05 licensing (needs P04)         │
    ├ P06 Claude driver (needs P02,P03) │
    │  └ P07 state machine + budget     │
    │     └ P08 arch mapping ───────────┤
    │        └ P09 fan-out discovery    │ (P09 also needs P05 for license risk downstream)
    P09 + P05 → P10 scoring engine ─────┘
    P10 + P09 → P11 synthesis + risk
    P11 → P12 report assembly
    P07 + P12 → P13 sqlx stores
    P11 + P12 + P13 → P14 CLI
    P11 + P12 + P13 → P15 axum server
    P15 → P16 Next.js web
    P14 + P15 + P16 → P17 e2e + repomix path
```

**Parallelism:**
- P05 (licensing) concurrent with P03/P04 (only needs core + `DependencyRecord` from P04)
- P06 (Claude driver) concurrent with P03/P04 (only needs core)
- P14 (CLI) and P15 (server) fully parallel (both depend on P11/P12/P13, not each other)
- P08 (arch mapping) and P05 (licensing) parallel in the runtime pipeline
