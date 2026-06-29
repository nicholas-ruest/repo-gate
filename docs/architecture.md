# RepoGate Architecture

## System Overview

RepoGate is a deep repository assessment platform built around a Rust orchestrator that drives Claude Code for intelligent codebase analysis. The system ingests public repositories, performs multi-stage analysis (license scanning, code metrics, deep module inspection, commercial value scoring), and outputs structured gating assessments.

## Cargo Workspace

RepoGate is organized as a Rust workspace with the following member crates:

- **repogate-core** — Core data structures: repository models, module definitions, scoring schemas, gating tier definitions, and shared utilities.
- **repogate-ingestion** — Repository cloning (subprocess `git clone --depth=1 --filter=blob:none` for the MVP, behind a `GitProvider` trait that migrates to pure-Rust `gix` post-MVP), directory traversal (via `ignore`), manifest detection, and file cataloging.
- **repogate-licensing** — License detection (via `askalono`), license conflict analysis, SPDX expression parsing (via `spdx`), legal risk flagging, and supply chain scanning (via `syft` subprocess).
- **repogate-orchestrator** — Orchestration engine; manages the multi-stage pipeline, spawns Claude Code sub-agents, routes analysis tasks, aggregates results, and handles error recovery.
- **repogate-scoring** — Commercial value scoring engine; evaluates modules on adoption impact, enterprise value, competitive sensitivity, support burden, strategic importance, and gating suitability. Produces a commercial value score (0–100) per module.
- **repogate-report** — Report generation; compiles analysis results into structured JSON and Markdown reports (executive summary, functionality inventory, architecture map, module-by-module analysis, risk assessment, final recommendations).
- **repogate-cli** — Command-line interface; exposes the `repogate analyze` command via `clap`, handles input validation, invokes the orchestrator, and outputs or streams results.
- **repogate-server** — Optional HTTP API server (via `axum`); exposes analysis endpoints, stores assessment history in `sqlx`-backed database, and serves the frontend dashboard.

```
repogate-core (shared types)
    ↑
    ├─ repogate-ingestion (repo cloning, file enumeration)
    ├─ repogate-licensing (license analysis, legal review)
    ├─ repogate-orchestrator (pipeline orchestration, Claude Code spawning)
    │   └─ repogate-scoring (module valuation)
    ├─ repogate-report (JSON + Markdown rendering via `tera`)
    ├─ repogate-cli (CLI entry point via `clap`)
    └─ repogate-server (HTTP API + database via `sqlx`, `axum`)
```

## End-to-End Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. INPUT                                                        │
│ Repository URL (public GitHub, GitLab, etc.)                   │
└────────────────────────┬────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│ 2. INGESTION (repogate-ingestion)                               │
│ • Clone via subprocess git --depth=1 (gix post-MVP)             │
│ • Traverse structure (ignore .gitignore, large binaries)        │
│ • Catalog files, modules, entry points                          │
│ • Extract metrics (tokei: LOC, complexity)                      │
│ • Detect manifests (Cargo.toml, package.json, etc.)            │
└────────────────────────┬────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│ 3. LICENSE & SUPPLY CHAIN (repogate-licensing)                  │
│ • Detect licenses (askalono)                                    │
│ • Parse SPDX expressions (spdx)                                 │
│ • Identify mixed-license risk, copyleft exposure                │
│ • Scan dependencies (syft subprocess)                           │
│ • Flag files requiring legal review                             │
└────────────────────────┬────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│ 4. MODULE DISCOVERY (repogate-orchestrator)                     │
│ • Identify functional areas:                                    │
│   - Core runtime, APIs, SDKs, CLIs                              │
│   - Connectors, integrations, dashboards                        │
│   - Testing, deployment, configuration systems                  │
│ • Map inter-module dependencies                                 │
│ • Prioritize modules by commercial relevance                    │
└────────────────────────┬────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│ 5. DEEP ANALYSIS (repogate-orchestrator + Claude Code)          │
│ For each module:                                                │
│ • Spawn a Claude Code sub-agent                                 │
│ • Load module files and context                                 │
│ • Inspect implementation details                                │
│ • Uncover hidden capabilities                                   │
│ • Assess enterprise value and use cases                         │
│ • Identify undocumented features                                │
│ • Generate structured module assessment (JSON schema)           │
│ Results → repogate-core data structures                         │
└────────────────────────┬────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│ 6. COMMERCIAL SCORING (repogate-scoring)                        │
│ Per module:                                                     │
│ • Adoption impact (is it required for basic use?)               │
│ • Enterprise buyer value (does enterprise need it?)             │
│ • Commercial leverage (unique, defensible logic?)                │
│ • Competitive sensitivity (competitor risk if exposed?)          │
│ • Operational value (support/maintenance burden?)               │
│ • Strategic importance (aligns with product vision?)            │
│ • Gating suitability (can it be cleanly separated?)             │
│ Score → 0–100 per module; recommend tier                       │
└────────────────────────┬────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│ 7. GATING & RISK ANALYSIS (repogate-orchestrator)               │
│ • Assign modules to 7 gating tiers                              │
│ • Analyze over-gating risk (community backlash)                 │
│ • Identify licensing constraints on gating                      │
│ • Detect hidden enterprise functionality                        │
│ • Validate tier boundaries (cleanly separable?)                 │
│ • Flag risks and mitigation strategies                          │
└────────────────────────┬────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│ 8. REPORT GENERATION (repogate-report)                          │
│ • Compile findings into structured JSON (tera templates)        │
│ • Generate human-readable Markdown report                       │
│ • Output:                                                       │
│   - assessment.json (machine-readable)                          │
│   - assessment.md (for review and sharing)                      │
│ • Optional: Store in database (repogate-server)                 │
└────────────────────────┬────────────────────────────────────────┘
                         ↓
                    OUTPUT REPORT
           (gating tiers, recommendations, risk)
```

## Claude Code Integration

RepoGate uses Claude Code as a headless reasoning engine for deep codebase inspection. Each module spawns a dedicated sub-agent via `claude --bare -p` (programmatic mode) to:

1. **Load Context** — Feed the sub-agent a curated module context (relevant files, function signatures, tests, examples).
2. **Analyze** — Inspect implementation patterns, trace data flows, identify features and capabilities.
3. **Stream Output** — Capture Claude Code's structured JSON response (via `--json-schema`) with findings.
4. **Aggregate** — Combine sub-agent findings into a module assessment (functionality list, enterprise features, risks).

### Model Routing

- **Opus 4.8** — Deep structural analysis, architecture decisions, security implications, commercial strategy recommendations (used sparingly for high-value or high-complexity modules).
- **Sonnet 4.6** — Standard module analysis, feature discovery, dependency mapping (most modules).
- **Haiku 4.5** — Lightweight utility scanning, manifest parsing, boilerplate detection (if cost-sensitive).

Each sub-agent request is routed via the orchestrator based on module complexity, size, and strategic importance.

### Process

```rust
// Pseudocode: spawning a Claude Code sub-agent for module analysis
orchestrator.spawn_module_agent(module) {
    context = load_module_files(module) + related_tests + examples
    prompt = format_analysis_prompt(module, context)
    
    response = claude(
        --bare            // headless mode
        -p                // programmatic output
        --json-schema {   // enforce structured output
            findings: [
                { category, capability, description, is_enterprise, risk_level }
            ],
            estimated_commercial_value: 0..100,
            gating_recommendation: enum,
            reasoning: String
        },
        prompt
    )
    
    return parse_json(response)
}
```

## Data Model

### Core Types (repogate-core)

```rust
pub struct Repository {
    pub url: String,
    pub name: String,
    pub license: License,
    pub modules: Vec<Module>,
    pub metrics: RepositoryMetrics,
    pub risks: Vec<Risk>,
}

pub struct Module {
    pub id: String,
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub kind: ModuleKind,  // API, SDK, CLI, Connector, etc.
    pub files: Vec<FilePath>,
    pub dependencies: Vec<ModuleId>,
    pub capabilities: Vec<Capability>,
    pub commercial_value_score: u8,  // 0–100
    pub recommended_tier: GatingTier,
    pub risks: Vec<Risk>,
}

pub struct Capability {
    pub name: String,
    pub description: String,
    pub is_enterprise: bool,
    pub is_undocumented: bool,
    pub affects_tiers: Vec<GatingTier>,
}

pub enum GatingTier {
    Open,
    SourceAvailable,
    ProTier,
    EnterpriseTier,
    ManagedCloud,
    LegalReview,
    NotRecommended,
}

pub struct Risk {
    pub kind: RiskKind,  // OverGating, CommunityBacklash, LicenseConflict, etc.
    pub severity: Severity,
    pub description: String,
    pub mitigation: Option<String>,
}
```

### Output Schema

Both JSON and Markdown reports are generated from a unified internal assessment object:

```rust
pub struct Assessment {
    pub repository: Repository,
    pub modules: Vec<Module>,
    pub executive_summary: String,
    pub functionality_inventory: Vec<Capability>,
    pub architecture_map: String,  // ASCII diagram
    pub scoring_rationale: Vec<ModuleScoreExplanation>,
    pub gating_strategy: GatingStrategy,
    pub risks: Vec<Risk>,
    pub recommendations: Vec<String>,
    pub generated_at: DateTime<Utc>,
}
```

JSON output uses `serde` serialization; Markdown uses `tera` template rendering.

## Deployment

### CLI-Only (Single Machine)

```bash
cargo build --release
./target/release/repogate analyze <repo-url>
```

### Server Mode (API + Dashboard)

```bash
# Start the server
cargo run --release --package repogate-server -- --listen 0.0.0.0:8080

# Browser: http://localhost:8080
# API: POST /api/analyze with { url: "..." }
```

The server stores assessments in a configured `sqlx`-backed database (PostgreSQL, SQLite, etc.) for history and sharing.

## Extensibility

### Adding a New Analysis Stage

1. Create a new crate in the workspace (e.g., `repogate-security-scan`).
2. Implement the analysis logic (takes `Repository` or `Module`, returns findings).
3. Update `repogate-orchestrator` to invoke the stage in the pipeline.
4. Integrate findings into the `Assessment` struct and report templates.

### Custom Claude Code Prompts

Prompts for module analysis are centralized in `repogate-orchestrator/prompts/`. Customize per-module or per-kind prompts without rebuilding.

### Report Customization

`repogate-report` uses `tera` templates (`templates/*.tera`). Modify templates to customize JSON structure or Markdown layout without changing the data model.

## References

- **[Architecture Decision Records](adr/)** — Design rationale, Claude Code integration strategy, tier definitions, and trade-offs.
- **[Domain Model](ddd/)** — Bounded contexts, aggregate roots, domain events, and entity definitions (DDD methodology).
- **[Getting Started](getting-started.md)** — Prerequisites, build, run, and output interpretation.

## Performance & Scalability

- **Concurrency** — Multiple module analyses run in parallel via `tokio` async tasks.
- **Memory** — Large repositories are streamed; file contents are not held in memory simultaneously.
- **Claude Code Cost** — Batch module analysis under cost budgets (default: $20/assessment); adjustable via config.
- **Caching** — Repository data and license information can be cached locally to avoid re-scanning.

## Security

- Repositories are cloned to isolated temporary directories and cleaned up after analysis.
- No credentials or secrets are extracted or logged.
- License scanning respects copyright and does not republish full file contents.
- Claude Code analysis is performed in sandboxed subprocess execution.
