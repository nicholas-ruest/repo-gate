# RepoGate — Complete Implementation Prompt Series (P01–P17)

This is the full set of dependency-ordered implementation prompts for building RepoGate, concatenated into a single document. Dispatch each prompt to a coding agent **one at a time, in order**, and reach green on its acceptance criteria before moving to the next. The authoritative plan is [BUILD-MANIFEST.md](BUILD-MANIFEST.md); individual prompt files live alongside this document (`P01-*.md` … `P17-*.md`).

**Rules that apply to every prompt:** Rust-first (TypeScript only for the P16 Next.js dashboard); Claude Code driven headlessly via `claude --bare -p --output-format stream-json --json-schema`; subprocess `git` clone for the MVP; schema-enforced structured output; no live Claude API calls in tests (use a mock `SessionRunner`).

---

## Table of Contents

- [P01 — Cargo Workspace Skeleton + CI](#p01-cargo-workspace-skeleton-ci)
- [P02 — `repogate-core`: Domain Types, Error Types, JSON Schema Contracts](#p02-repogate-core-domain-types-error-types-json-schema-contracts)
- [P03 — `repogate-ingestion`: Git Clone, File Walk, Language Detection, Binary Filtering](#p03-repogate-ingestion-git-clone-file-walk-language-detection-binary-filtering)
- [P04 — `repogate-ingestion`: Dependency Manifest Parsing + `syft` SBOM](#p04-repogate-ingestion-dependency-manifest-parsing-syft-sbom)
- [P05 — `repogate-licensing`: License Detection, SPDX Parsing, Copyleft Risk Matrix](#p05-repogate-licensing-license-detection-spdx-parsing-copyleft-risk-matrix)
- [P06 — `repogate-orchestrator`: Claude Code Subprocess Driver](#p06-repogate-orchestrator-claude-code-subprocess-driver)
- [P07 — `repogate-orchestrator`: AssessmentJob State Machine, Token Budget, Crash Recovery](#p07-repogate-orchestrator-assessmentjob-state-machine-token-budget-crash-recovery)
- [P08 — `repogate-orchestrator`: Architecture Mapping Phase](#p08-repogate-orchestrator-architecture-mapping-phase)
- [P09 — `repogate-orchestrator`: Functionality Discovery Fan-Out Phase](#p09-repogate-orchestrator-functionality-discovery-fan-out-phase)
- [P10 — `repogate-scoring`: Commercial Value Scoring Engine + Tier Classifier](#p10-repogate-scoring-commercial-value-scoring-engine-tier-classifier)
- [P11 — `repogate-orchestrator`: Synthesis Phase (Gating Strategy + Risk Analysis)](#p11-repogate-orchestrator-synthesis-phase-gating-strategy-risk-analysis)
- [P12 — `repogate-report`: Report Assembly, `minijinja` Templates, Canonical JSON](#p12-repogate-report-report-assembly-minijinja-templates-canonical-json)
- [P13 — `sqlx` Schema, Migrations, Store Implementations](#p13-sqlx-schema-migrations-store-implementations)
- [P14 — `repogate-cli`: CLI Entry Point, `repogate analyze`, Cost Estimation, Progress](#p14-repogate-cli-cli-entry-point-repogate-analyze-cost-estimation-progress)
- [P15 — `repogate-server`: `axum` HTTP Server, API Endpoints, Static Serving](#p15-repogate-server-axum-http-server-api-endpoints-static-serving)
- [P16 — `repogate-web`: Next.js Dashboard (TypeScript)](#p16-repogate-web-nextjs-dashboard-typescript)
- [P17 — End-to-End Integration Tests + Repomix Small-Repo Path](#p17-end-to-end-integration-tests-repomix-small-repo-path)

---

# P01 — Cargo Workspace Skeleton + CI

## Context

RepoGate is a deep repository assessment platform that analyzes full open-source codebases to determine what should remain open source versus become part of commercial tiers, using Claude Code as the reasoning engine and Rust as the primary implementation language.

**You are implementing exactly ONE build unit: the Cargo workspace skeleton and CI setup.** Do not start work on other units. Build and tests must be green before this prompt is considered done.

**Prerequisites:** None — this is the foundation.

---

## Phase & Dependencies

- **Phase:** Foundations
- **Depends on:** Nothing

---

## Scope & Deliverables

Your task is to set up the Cargo workspace and CI/CD infrastructure for the entire RepoGate project.

### Cargo Workspace Root

Create `/workspaces/repo-gate/Cargo.toml` as the workspace manifest declaring all 8 member crates:

```toml
[workspace]
members = [
    "crates/repogate-core",
    "crates/repogate-ingestion",
    "crates/repogate-licensing",
    "crates/repogate-orchestrator",
    "crates/repogate-scoring",
    "crates/repogate-report",
    "crates/repogate-cli",
    "crates/repogate-server",
]
resolver = "2"

[workspace.dependencies]
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
schemars = { version = "0.8", features = ["derive"] }
sqlx = { version = "0.8", features = ["runtime-tokio-native-tls", "sqlite", "postgres"] }
axum = "0.7"
clap = { version = "4.5", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1.0"
thiserror = "1.0"
```

### Member Crate Initialization

For each of the 8 crates, create a minimal directory structure:

- `crates/repogate-{name}/`
  - `Cargo.toml` (package manifest, importing workspace deps)
  - `src/lib.rs` or `src/main.rs` with one passing placeholder test
  - `.gitkeep` or similar

**Example `crates/repogate-core/Cargo.toml`:**

```toml
[package]
name = "repogate-core"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
schemars = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["full"] }
```

**Example `crates/repogate-core/src/lib.rs`:**

```rust
#![doc = "RepoGate core types and schemas."]

pub mod error;
pub mod types;

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder() {
        assert!(true);
    }
}
```

### Root-Level Configuration

- **`.gitignore`** — Standard Rust patterns (target/, *.swp, .DS_Store, Cargo.lock, etc.)
- **`rust-toolchain.toml`** — Pin stable toolchain:
  ```toml
  [toolchain]
  channel = "stable"
  ```
- **`tests/fixtures/dev.db`** — Empty SQLite database file (0 bytes or minimal schema) for compile-time `sqlx` checks in CI. Create the file with `touch tests/fixtures/dev.db` and add a `.gitkeep` if needed.

### GitHub Actions CI Workflow

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --workspace --verbose
      - run: cargo test --workspace --verbose
      - run: cargo clippy --workspace -- -D warnings
      - run: cargo fmt --check
```

---

## Source Documents to Read

- **`docs/adr/ADR-001-rust-primary-language.md`** — Workspace structure, crate list, dependency strategy
- **`docs/adr/ADR-014-persistence-sqlx-sqlite-postgres.md`** — Reference to dev.db for compile-time checks

---

## Acceptance Criteria

- ✅ `cargo build --workspace` completes with zero errors and zero warnings
- ✅ `cargo test --workspace` runs and all placeholder tests pass
- ✅ `cargo clippy --workspace -- -D warnings` passes (no lint failures)
- ✅ `cargo fmt --check` passes (code is formatted)
- ✅ All 8 crates are present in `crates/` with minimal valid `Cargo.toml` and `src/` structure
- ✅ `.github/workflows/ci.yml` is syntactically valid (can validate with `yamllint` or GitHub's workflow parser)
- ✅ `tests/fixtures/dev.db` file exists

---

## Language

**Rust** — All crate manifests and build configuration.

---

## Out-of-Scope

- Do NOT implement any domain logic, types, or business code in this prompt
- Do NOT add external crates beyond those listed in `workspace.dependencies`
- Do NOT configure logging, tracing, or database connection pools yet
- Do NOT write integration tests or complex test fixtures


---

# P02 — `repogate-core`: Domain Types, Error Types, JSON Schema Contracts

## Context

RepoGate is a deep repository assessment platform that analyzes full open-source codebases to determine what should remain open source versus become part of commercial tiers, using Claude Code as the reasoning engine and Rust as the primary implementation language.

**You are implementing exactly ONE build unit: the core types, enums, and error definitions for the entire system.** These are the canonical domain models and contracts. Do not start work on other units. Build and tests must be green before this prompt is considered done.

**Prerequisites:** P01 (Cargo workspace skeleton) is complete.

---

## Phase & Dependencies

- **Phase:** Foundations
- **Depends on:** P01

---

## Scope & Deliverables

Implement the core types and schemas in `crates/repogate-core/src/`. All types must derive `serde::Serialize`, `serde::Deserialize`, and `schemars::JsonSchema`.

### File: `src/types.rs` — Value Objects & Enums

Implement these Rust enums and newtypes with exact signatures:

**Gating Tier:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GatingTier {
    Open,
    SourceAvailable,
    ProTier,
    EnterpriseTier,
    ManagedCloud,
    LegalReview,
    NotRecommended,
}
```

**Severity & Risk Kind:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub enum Severity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub enum RiskKind {
    OverGating,
    CommunityBacklash,
    LicenseConflict,
    CompetitiveExposure,
    SecurityExposure,
    AccidentalOpenSource,
    UnderGating,
}
```

**Layer & Language:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub enum Layer {
    Core,
    Api,
    Sdk,
    Cli,
    Connector,
    Integration,
    Deployment,
    Test,
    Documentation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub enum Language {
    Rust,
    TypeScript,
    Python,
    Go,
    Java,
    CSharp,
    Ruby,
    #[serde(untagged)]
    Other(String),
}
```

**Score Newtype (with validation):**
```rust
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
pub struct Score(f32);

impl Score {
    pub fn new(v: f32) -> Result<Self, ScoreRangeError> {
        if v < 0.0 || v > 10.0 {
            Err(ScoreRangeError { value: v })
        } else {
            Ok(Score(v))
        }
    }
    
    pub fn get(&self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CompositeScore(f32);  // 0.0–10.0

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CommercialScore {
    pub adoption_value: Score,
    pub enterprise_buyer_value: Score,
    pub commercial_leverage: Score,
    pub competitive_sensitivity: Score,
    pub operational_value: Score,
    pub security_sensitivity: Score,
    pub support_burden: Score,
    pub strategic_importance: Score,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScoreWeights {
    pub adoption_value: f32,
    pub enterprise_buyer_value: f32,
    pub commercial_leverage: f32,
    pub competitive_sensitivity: f32,
    pub operational_value: f32,
    pub security_sensitivity: f32,
    pub support_burden: f32,
    pub strategic_importance: f32,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        // Expert-tuned default weights (subject to ADR-010)
        Self {
            adoption_value: 1.2,
            enterprise_buyer_value: 1.1,
            commercial_leverage: 1.0,
            competitive_sensitivity: 0.9,
            operational_value: 0.8,
            security_sensitivity: 0.7,
            support_burden: -0.6,  // Negative: higher burden reduces score
            strategic_importance: 1.0,
        }
    }
}

impl ScoreWeights {
    pub fn new(...) -> Result<Self, WeightError> {
        // Validate all weights >= 0.0 (except support_burden can be negative)
        // Return error if invalid
    }
}
```

**Gating Signal & Token Budget:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum GatingSignal {
    StrongGateCandidate,
    WeakGateCandidate,
    OpenCandidate,
    Undetermined,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TokenBudget {
    pub total_limit: u64,
    pub per_phase_limit: u64,
    pub per_session_limit: u64,
    pub warn_threshold: f32,  // 0.0–1.0, e.g., 0.8
}

impl TokenBudget {
    pub fn is_exceeded(&self, used: u64) -> bool {
        used >= self.total_limit
    }
    
    pub fn remaining(&self, used: u64) -> u64 {
        self.total_limit.saturating_sub(used)
    }
}
```

### File: `src/model.rs` — Domain Aggregates

Implement these core domain models:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Repository {
    pub id: String,
    pub url: String,
    pub name: String,
    pub description: Option<String>,
    pub license: Option<String>,
    pub metrics: RepositoryMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RepositoryMetrics {
    pub total_files: usize,
    pub total_loc: usize,
    pub language_stats: std::collections::HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Module {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub path: String,
    pub layer: Layer,
    pub file_count: usize,
    pub loc: usize,
    pub commercial_score: Option<CommercialScore>,
    pub recommended_tier: Option<GatingTier>,
    pub risks: Vec<Risk>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Internal,
    Experimental,
    Undocumented,
    Enterprise,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Capability {
    pub name: String,
    pub description: Option<String>,
    pub visibility: Visibility,
    pub affects_tiers: Vec<GatingTier>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SourceLocation {
    pub file_path: String,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Risk {
    pub kind: RiskKind,
    pub severity: Severity,
    pub description: String,
    pub mitigation: Option<String>,
    pub is_blocking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Assessment {
    pub repo_id: String,
    pub schema_version: String,  // "1.0"
    pub generated_at: String,  // ISO 8601 timestamp
    pub is_complete: bool,
    pub repository: Repository,
    pub modules: Vec<Module>,
    pub gating_strategy: Option<GatingStrategy>,
    pub risks: Vec<Risk>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GatingStrategy {
    pub tier_assignments: Vec<TierAssignment>,
    pub boundary_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TierAssignment {
    pub module_id: String,
    pub module_name: String,
    pub tier: GatingTier,
    pub rationale: Option<String>,
}
```

### File: `src/claude_schemas.rs` — Claude Output Schemas

Implement schemas that Claude Code will output JSON matching:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum DiscoveryMethod {
    PublicApi,
    TestCoverage,
    ExampleCode,
    CliInspection,
    SourceTracing,
    ConfigAnalysis,
    DocumentationCross,
    LlmInference,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CapabilityFinding {
    pub name: String,
    pub description: String,
    pub is_enterprise: bool,
    pub is_undocumented: bool,
    pub discovery_method: DiscoveryMethod,
    pub source_locations: Option<Vec<SourceLocation>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModuleAssessment {
    pub module_name: String,
    pub module_path: String,
    pub capabilities: Vec<CapabilityFinding>,
    pub commercial_value_estimate: Option<f32>,
    pub estimated_tier: Option<String>,
    pub risks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SynthesisOutput {
    pub gating_strategy: Option<String>,
    pub tier_assignments: Option<Vec<serde_json::Value>>,  // Flexible structure
}

/// Write JSON Schema for a type to a file.
pub fn write_schema<T: schemars::JsonSchema>(path: &std::path::Path) -> anyhow::Result<()> {
    let schema = schemars::schema_for!(T);
    let json = serde_json::to_string_pretty(&schema)?;
    std::fs::write(path, json)?;
    Ok(())
}
```

### File: `src/error.rs` — Error Types

Implement comprehensive error types using `thiserror`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepogateError {
    #[error("score out of range: {0}")]
    ScoreRangeError(f32),
    
    #[error("weight validation failed: {0}")]
    WeightError(String),
    
    #[error("schema violation: {0}")]
    SchemaViolationError(String),
    
    #[error("store error: {0}")]
    StoreError(String),
    
    #[error("orchestrator error: {0}")]
    OrchestratorError(String),
}

#[derive(Debug, Clone, Error)]
#[error("score {value} is outside valid range [0.0, 10.0]")]
pub struct ScoreRangeError {
    pub value: f32,
}

#[derive(Debug, Clone, Error)]
#[error("weight validation failed: {0}")]
pub struct WeightError(pub String);

#[derive(Debug, Clone, Error)]
#[error("schema violation: {0}")]
pub struct SchemaViolationError(pub String);

#[derive(Debug, Clone, Error)]
#[error("store error: {0}")]
pub struct StoreError(pub String);

#[derive(Debug, Clone, Error)]
#[error("orchestrator error: {0}")]
pub struct OrchestratorError(pub String);
```

### File: `src/lib.rs`

```rust
#![doc = "RepoGate core types, schemas, and domain models."]

pub mod types;
pub mod model;
pub mod claude_schemas;
pub mod error;

pub use types::*;
pub use model::*;
pub use claude_schemas::*;
pub use error::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_valid() {
        assert!(Score::new(5.0).is_ok());
    }

    #[test]
    fn score_out_of_range() {
        assert!(Score::new(-1.0).is_err());
        assert!(Score::new(11.0).is_err());
    }

    #[test]
    fn token_budget_is_exceeded() {
        let budget = TokenBudget {
            total_limit: 100,
            per_phase_limit: 50,
            per_session_limit: 30,
            warn_threshold: 0.8,
        };
        assert!(!budget.is_exceeded(50));
        assert!(budget.is_exceeded(100));
    }

    #[test]
    fn assessment_round_trip() {
        let assessment = Assessment {
            repo_id: "test".to_string(),
            schema_version: "1.0".to_string(),
            generated_at: "2024-01-01T00:00:00Z".to_string(),
            is_complete: true,
            repository: Repository {
                id: "r1".to_string(),
                url: "https://example.com/repo".to_string(),
                name: "test-repo".to_string(),
                description: None,
                license: None,
                metrics: RepositoryMetrics {
                    total_files: 100,
                    total_loc: 5000,
                    language_stats: Default::default(),
                },
            },
            modules: vec![],
            gating_strategy: None,
            risks: vec![],
        };
        let json = serde_json::to_string(&assessment).unwrap();
        let restored: Assessment = serde_json::from_str(&json).unwrap();
        assert_eq!(assessment.repo_id, restored.repo_id);
    }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-007-schema-enforced-structured-output.md`** — `schemars` + JSON Schema export strategy
- **`docs/adr/ADR-010-commercial-value-scoring-model.md`** — 8 scoring dimensions, tier ranges, weighting
- **`docs/adr/ADR-011-assessment-output-formats.md`** — `schema_version`, canonical output structure
- **`docs/ddd/commercial-valuation.md`** — Value objects, scoring invariants
- **`docs/ddd/functionality-discovery.md`** — `Visibility`, `DiscoveryMethod`
- **`docs/ddd/assessment-orchestration.md`** — `TokenBudget`

---

## Acceptance Criteria

- ✅ `cargo build -p repogate-core` completes with zero warnings
- ✅ All structs implement `Serialize`, `Deserialize`, `JsonSchema`
- ✅ `Score::new(-1.0)` returns `Err`; `Score::new(5.0)` returns `Ok(Score(5.0))`
- ✅ `Score::new(10.0)` succeeds; `Score::new(10.1)` fails
- ✅ `write_schema::<ModuleAssessment>` writes a valid JSON Schema file without error
- ✅ `TokenBudget::is_exceeded(50)` returns false for limit 100; returns true for 100
- ✅ `Assessment` can round-trip through `serde_json` (serialize → deserialize → equal)
- ✅ All unit tests pass: `cargo test -p repogate-core`

---

## Language

**Rust** — All type definitions, derives, and test logic.

---

## Out-of-Scope

- Do NOT implement database logic or storage
- Do NOT implement Claude Code invocation or subprocess management
- Do NOT write complex business logic; just data structures and contracts
- Do NOT add validation beyond what's shown in `Score::new()` and `ScoreWeights::new()`


---

# P03 — `repogate-ingestion`: Git Clone, File Walk, Language Detection, Binary Filtering

## Context

RepoGate is a deep repository assessment platform that analyzes full open-source codebases to determine what should remain open source versus become part of commercial tiers, using Claude Code as the reasoning engine and Rust as the primary implementation language.

**You are implementing exactly ONE build unit: the repository ingestion layer, including git cloning, file traversal, language detection, and binary filtering.** Do not start work on other units. Build and tests must be green before this prompt is considered done.

**Prerequisites:** P02 (core types and schemas) is complete.

---

## Phase & Dependencies

- **Phase:** Ingestion
- **Depends on:** P02

---

## Scope & Deliverables

Implement the `repogate-ingestion` crate with safe, parallel repository ingestion logic.

### File: `src/git.rs` — Git Provider Trait & Subprocess Implementation

Define a trait and implement it using subprocess:

```rust
pub trait GitProvider: Send + Sync {
    async fn clone(&self, url: &str, dest: &std::path::Path) -> Result<(), IngestionError>;
    async fn resolve_head(&self, repo_path: &std::path::Path) -> Result<String, IngestionError>;
}

pub struct SubprocessGit;

impl GitProvider for SubprocessGit {
    async fn clone(&self, url: &str, dest: &std::path::Path) -> Result<(), IngestionError> {
        // Validate URL first (reject file://, localhost, RFC-1918 IPs)
        validate_repo_url(url)?;
        
        // Run: git clone --depth=1 --filter=blob:none <url> <dest>
        let output = tokio::process::Command::new("git")
            .arg("clone")
            .arg("--depth=1")
            .arg("--filter=blob:none")
            .arg(url)
            .arg(dest)
            .output()
            .await?;
        
        if !output.status.success() {
            return Err(IngestionError::CloneFailed {
                url: url.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        Ok(())
    }
    
    async fn resolve_head(&self, repo_path: &std::path::Path) -> Result<String, IngestionError> {
        let output = tokio::process::Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(repo_path)
            .output()
            .await?;
        
        if !output.status.success() {
            return Err(IngestionError::RevParseFailed);
        }
        
        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }
}

fn validate_repo_url(url: &str) -> Result<(), IngestionError> {
    // Reject file://, localhost, 127.0.0.1, RFC-1918 (10.x, 172.16-31.x, 192.168.x.x)
    if url.starts_with("file://") {
        return Err(IngestionError::InvalidUrl("file:// URLs not allowed".into()));
    }
    if url.contains("localhost") || url.contains("127.0.0.1") {
        return Err(IngestionError::InvalidUrl("localhost URLs not allowed".into()));
    }
    // Additional IP range checks as needed
    Ok(())
}
```

### File: `src/walk.rs` — File Walk & Language Detection

```rust
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: std::path::PathBuf,
    pub size_bytes: u64,
    pub is_binary: bool,
    pub language: Option<repogate_core::Language>,
    pub hash: String,  // BLAKE3 hash
}

pub async fn walk_repository(
    repo_path: &std::path::Path,
) -> Result<Vec<FileEntry>, IngestionError> {
    // Use ignore::WalkBuilder for gitignore-aware traversal
    // Detect binaries (null byte in first 8 KB OR known binary extension)
    // Detect generated files (.gitattributes linguist-generated, vendor/, node_modules/, *.min.js)
    // Parallel walk via rayon or tokio
    // Classify each file's language via language_detector::classify() or similar
    // Compute BLAKE3 hash per file
    
    let mut entries = Vec::new();
    let walker = ignore::WalkBuilder::new(repo_path)
        .hidden(true)
        .ignore(true)
        .build_parallel();
    
    let (tx, rx) = tokio::sync::mpsc::channel(1000);
    tokio::spawn(async move {
        walker.run(|| {
            let tx = tx.clone();
            Box::new(move |result| {
                if let Ok(entry) = result {
                    // Process each file entry
                    let _ = tx.blocking_send(entry);
                }
                ignore::WalkState::Continue
            })
        });
    });
    
    // Collect results from channel
    while let Some(entry) = rx.recv().await {
        // Build FileEntry from DirEntry
        entries.push(FileEntry {
            path: entry.path().to_path_buf(),
            size_bytes: 0,  // TODO: calculate
            is_binary: detect_binary(entry.path()),
            language: None,  // TODO: classify
            hash: String::new(),  // TODO: compute BLAKE3
        });
    }
    
    Ok(entries)
}

fn detect_binary(path: &std::path::Path) -> bool {
    // Check for known binary extensions
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        if matches!(ext_str.as_str(), "png" | "jpg" | "jpeg" | "gif" | "so" | "dll" | "dylib" | "bin" | "exe" | "wasm") {
            return true;
        }
    }
    
    // Try reading first 8KB and check for null bytes
    if let Ok(data) = std::fs::read(&path) {
        let sample = &data[..std::cmp::min(8192, data.len())];
        if sample.iter().any(|&b| b == 0) {
            return true;
        }
    }
    
    false
}

fn classify_language(path: &std::path::Path) -> Option<repogate_core::Language> {
    // Use tree-sitter or similar to classify by extension + content
    // Fallback to hyperpolyglot heuristics
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        match ext_str.as_str() {
            "rs" => Some(repogate_core::Language::Rust),
            "ts" | "tsx" | "js" | "jsx" => Some(repogate_core::Language::TypeScript),
            "py" => Some(repogate_core::Language::Python),
            "go" => Some(repogate_core::Language::Go),
            "java" => Some(repogate_core::Language::Java),
            _ => None,
        }
    } else {
        None
    }
}
```

### File: `src/language.rs` — Language Statistics

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageStats {
    pub language_counts: HashMap<repogate_core::Language, usize>,  // LOC per language
}

pub fn compute_language_stats(entries: &[FileEntry]) -> LanguageStats {
    // Use tokei library to aggregate LOC per language
    // Or use hyperpolyglot classification for each file
    let mut counts = HashMap::new();
    
    for entry in entries {
        if let Some(lang) = entry.language {
            *counts.entry(lang).or_insert(0) += 1;  // Simplified: count files, not LOC
        }
    }
    
    LanguageStats {
        language_counts: counts,
    }
}
```

### File: `src/manifest.rs` — Repository Manifest

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PackageFileType {
    Cargo,     // Cargo.toml
    Npm,       // package.json
    PyProject, // pyproject.toml
    GoMod,     // go.mod
    Maven,     // pom.xml
    Gradle,    // build.gradle
    Gemfile,   // Gemfile
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageFileRef {
    pub path: std::path::PathBuf,
    pub file_type: PackageFileType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoManifest {
    pub repo_id: String,
    pub url: String,
    pub total_files: usize,
    pub total_loc: usize,
    pub language_stats: LanguageStats,
    pub root_dirs: Vec<String>,  // Top-level directories
    pub file_entries: Vec<FileEntry>,
    pub package_files: Vec<PackageFileRef>,
}
```

### File: `src/lib.rs`

```rust
#![doc = "RepoGate repository ingestion: git cloning, file walking, language detection."]

pub mod git;
pub mod walk;
pub mod language;
pub mod manifest;

pub use git::{GitProvider, SubprocessGit};
pub use walk::FileEntry;
pub use manifest::RepoManifest;

#[derive(Debug, thiserror::Error)]
pub enum IngestionError {
    #[error("clone failed for {url}: {stderr}")]
    CloneFailed { url: String, stderr: String },
    
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    
    #[error("rev-parse failed")]
    RevParseFailed,
    
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("utf8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

pub async fn ingest(
    url: &str,
    dest: &std::path::Path,
) -> Result<RepoManifest, IngestionError> {
    let git = SubprocessGit;
    git.clone(url, dest).await?;
    let _head = git.resolve_head(dest).await?;
    
    let entries = walk::walk_repository(dest).await?;
    let lang_stats = language::compute_language_stats(&entries);
    
    Ok(RepoManifest {
        repo_id: uuid::Uuid::new_v4().to_string(),
        url: url.to_string(),
        total_files: entries.len(),
        total_loc: 0,  // TODO: sum from tokei
        language_stats: lang_stats,
        root_dirs: vec![],  // TODO: extract top-level dirs
        file_entries: entries,
        package_files: vec![],  // TODO: detect package files
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_url_rejects_file() {
        assert!(git::validate_repo_url("file:///local/path").is_err());
    }

    #[test]
    fn validate_url_rejects_localhost() {
        assert!(git::validate_repo_url("http://localhost:8080/repo").is_err());
    }

    #[test]
    fn validate_url_accepts_github() {
        assert!(git::validate_repo_url("https://github.com/rust-lang/rust").is_ok());
    }

    #[test]
    fn detect_binary_png() {
        assert!(walk::detect_binary(std::path::Path::new("image.png")));
    }

    #[test]
    fn classify_language_rust() {
        let lang = walk::classify_language(std::path::Path::new("main.rs"));
        assert_eq!(lang, Some(repogate_core::Language::Rust));
    }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-005-git-ingestion-and-tree-walking.md`** — Git strategy, `ignore` crate, `tokei`, `hyperpolyglot`, binary detection, `GitProvider` trait
- **`docs/ddd/repository-ingestion.md`** — Ingestion invariants, `RepoUrl`, `FileEntry`, `ModuleManifest`

---

## Acceptance Criteria

- ✅ `cargo test -p repogate-ingestion` passes
- ✅ Integration test: clone `https://github.com/rust-lang/regex` to temp; `total_files > 50`, `language_stats` contains at least `Rust`
- ✅ `file://` URL validation rejects with `InvalidUrl` error
- ✅ `http://localhost:8080` URL validation rejects with error
- ✅ `.png` file in entries has `is_binary: true` and `language: None`
- ✅ `.rs` file classified as `Language::Rust`

---

## Language

**Rust** — All ingestion logic, file operations, git integration.

---

## Out-of-Scope

- Do NOT implement dependency parsing or manifest interpretation (P04)
- Do NOT implement license detection (P05)
- Do NOT implement module boundary detection or architecture mapping (P08)
- Do NOT implement deep file content inspection; just basic language classification


---

# P04 — `repogate-ingestion`: Dependency Manifest Parsing + `syft` SBOM

## Context

RepoGate is a deep repository assessment platform that analyzes full open-source codebases to determine what should remain open source versus become part of commercial tiers, using Claude Code as the reasoning engine and Rust as the primary implementation language.

**You are implementing exactly ONE build unit: dependency extraction from package manifests and SBOM generation via syft.** Do not start work on other units. Build and tests must be green before this prompt is considered done.

**Prerequisites:** P03 (git clone, file walk) is complete.

---

## Phase & Dependencies

- **Phase:** Ingestion
- **Depends on:** P03

---

## Scope & Deliverables

Extend `repogate-ingestion` with dependency parsing.

### File: `src/deps/cargo.rs` — Cargo Manifest Parsing

```rust
pub struct DependencyRecord {
    pub name: String,
    pub version: String,
    pub ecosystem: Ecosystem,
    pub spdx_license: Option<String>,  // Preserved verbatim; not parsed
    pub is_direct: bool,
    pub is_transitive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ecosystem {
    Cargo,
    Npm,
    PyPi,
    Go,
    Maven,
    Gradle,
    Ruby,
    Unknown,
}

pub async fn parse_cargo_deps(repo_path: &std::path::Path) -> Result<Vec<DependencyRecord>, IngestionError> {
    // Run: cargo metadata --format-version 1
    let output = tokio::process::Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(repo_path)
        .output()
        .await?;
    
    if !output.status.success() {
        return Err(IngestionError::CargoMetadataFailed);
    }
    
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let mut deps = Vec::new();
    
    // Extract from metadata.packages[]
    if let Some(packages) = metadata["packages"].as_array() {
        for pkg in packages {
            deps.push(DependencyRecord {
                name: pkg["name"].as_str().unwrap_or("").to_string(),
                version: pkg["version"].as_str().unwrap_or("").to_string(),
                ecosystem: Ecosystem::Cargo,
                spdx_license: pkg["license"].as_str().map(|s| s.to_string()),
                is_direct: true,  // Simplification; parse resolve graph for accuracy
                is_transitive: false,
            });
        }
    }
    
    Ok(deps)
}
```

### File: `src/deps/sbom.rs` — Syft SBOM Parsing

```rust
pub async fn extract_sbom_via_syft(repo_path: &std::path::Path) -> Result<Vec<DependencyRecord>, IngestionError> {
    // Run: syft <repo-path> -o spdx-json --quiet
    let output = tokio::process::Command::new("syft")
        .arg(repo_path)
        .arg("-o")
        .arg("spdx-json")
        .arg("--quiet")
        .output()
        .await;
    
    match output {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(IngestionError::SyftNotFound);
        }
        Err(e) => return Err(IngestionError::Io(e)),
        Ok(out) => {
            if !out.status.success() {
                return Ok(vec![]);  // Graceful fallback
            }
            
            // Parse SPDX JSON output
            let spdx: serde_json::Value = serde_json::from_slice(&out.stdout)?;
            let mut deps = Vec::new();
            
            // Extract from spdx["packages"][]
            if let Some(packages) = spdx["packages"].as_array() {
                for pkg in packages {
                    deps.push(DependencyRecord {
                        name: pkg["name"].as_str().unwrap_or("").to_string(),
                        version: pkg["versionInfo"].as_str().unwrap_or("").to_string(),
                        ecosystem: infer_ecosystem(&pkg["name"].as_str().unwrap_or("")),
                        spdx_license: pkg["licenseConcluded"].as_str().map(|s| s.to_string()),
                        is_direct: false,  // SBOM reports all
                        is_transitive: true,
                    });
                }
            }
            
            Ok(deps)
        }
    }
}

fn infer_ecosystem(name: &str) -> Ecosystem {
    // Heuristic: check package name patterns
    // (Simplified; real impl would use package registry APIs)
    Ecosystem::Unknown
}
```

### File: `src/deps/mod.rs`

```rust
pub mod cargo;
pub mod sbom;

pub use cargo::{DependencyRecord, Ecosystem};

pub async fn extract_dependencies(
    manifest: &RepoManifest,
    repo_path: &std::path::Path,
) -> Result<Vec<DependencyRecord>, IngestionError> {
    let mut all_deps = Vec::new();
    
    // Detect manifest types and extract accordingly
    let has_cargo = manifest.package_files.iter().any(|pf| {
        matches!(pf.file_type, PackageFileType::Cargo)
    });
    
    if has_cargo {
        all_deps.extend(cargo::parse_cargo_deps(repo_path).await?);
    }
    
    // Run syft and merge
    if let Ok(sbom_deps) = sbom::extract_sbom_via_syft(repo_path).await {
        all_deps.extend(sbom_deps);
    }
    
    // Dedup by (name, version, ecosystem)
    all_deps.sort_by(|a, b| (a.name.cmp(&b.name), a.version.cmp(&b.version)).cmp(&(a.name.cmp(&b.name), a.version.cmp(&b.version))));
    all_deps.dedup_by(|a, b| a.name == b.name && a.version == b.version && a.ecosystem == b.ecosystem);
    
    Ok(all_deps)
}
```

### File: `src/lib.rs` — Extend Manifest

```rust
// In RepoManifest:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoManifest {
    pub repo_id: String,
    pub url: String,
    pub total_files: usize,
    pub total_loc: usize,
    pub language_stats: LanguageStats,
    pub root_dirs: Vec<String>,
    pub file_entries: Vec<FileEntry>,
    pub package_files: Vec<PackageFileRef>,
    pub dependencies: Vec<DependencyRecord>,  // NEW
}

#[derive(Debug, thiserror::Error)]
pub enum IngestionError {
    // ... previous variants ...
    #[error("cargo metadata failed")]
    CargoMetadataFailed,
    
    #[error("syft not found")]
    SyftNotFound,
}

pub async fn ingest(
    url: &str,
    dest: &std::path::Path,
) -> Result<RepoManifest, IngestionError> {
    let git = SubprocessGit;
    git.clone(url, dest).await?;
    
    let entries = walk::walk_repository(dest).await?;
    let lang_stats = language::compute_language_stats(&entries);
    let package_files = detect_package_files(&entries);
    
    let mut manifest = RepoManifest {
        repo_id: uuid::Uuid::new_v4().to_string(),
        url: url.to_string(),
        total_files: entries.len(),
        total_loc: 0,
        language_stats: lang_stats,
        root_dirs: vec![],
        file_entries: entries,
        package_files,
        dependencies: vec![],
    };
    
    // Extract dependencies
    manifest.dependencies = deps::extract_dependencies(&manifest, dest).await.unwrap_or_default();
    
    Ok(manifest)
}

fn detect_package_files(entries: &[FileEntry]) -> Vec<PackageFileRef> {
    entries.iter()
        .filter_map(|e| {
            let file_name = e.path.file_name()?.to_string_lossy();
            let file_type = match file_name.as_ref() {
                "Cargo.toml" => PackageFileType::Cargo,
                "package.json" => PackageFileType::Npm,
                "pyproject.toml" => PackageFileType::PyProject,
                "go.mod" => PackageFileType::GoMod,
                "pom.xml" => PackageFileType::Maven,
                "build.gradle" => PackageFileType::Gradle,
                "Gemfile" => PackageFileType::Gemfile,
                _ => return None,
            };
            Some(PackageFileRef {
                path: e.path.clone(),
                file_type,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_record_preserves_license() {
        let record = DependencyRecord {
            name: "serde".to_string(),
            version: "1.0".to_string(),
            ecosystem: Ecosystem::Cargo,
            spdx_license: Some("MIT OR Apache-2.0".to_string()),
            is_direct: true,
            is_transitive: false,
        };
        assert_eq!(record.spdx_license.unwrap(), "MIT OR Apache-2.0");
    }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-006-license-dependency-analysis.md`** — `cargo_metadata`, `syft` subprocess, multi-ecosystem SBOM parsing
- **`docs/ddd/repository-ingestion.md`** — `package_files` structure

---

## Acceptance Criteria

- ✅ `cargo test -p repogate-ingestion` passes
- ✅ Rust repo analysis produces non-empty `Vec<DependencyRecord>` with `Cargo` ecosystem
- ✅ `syft` missing → `Err(SyftNotFound)`, no panic
- ✅ Cargo license string `"MIT OR Apache-2.0"` is preserved verbatim (not parsed)
- ✅ `DependencyRecord` serializes and deserializes via serde
- ✅ Deduplication removes duplicate (name, version, ecosystem) tuples

---

## Language

**Rust** — Manifest parsing, subprocess coordination, dependency extraction.

---

## Out-of-Scope

- Do NOT parse or interpret SPDX license expressions (P05)
- Do NOT implement supply-chain risk assessment
- Do NOT implement transitive dependency graph building


---

# P05 — `repogate-licensing`: License Detection, SPDX Parsing, Copyleft Risk Matrix

## Context

RepoGate is a deep repository assessment platform that analyzes full open-source codebases to determine what should remain open source versus become part of commercial tiers, using Claude Code as the reasoning engine and Rust as the primary implementation language.

**You are implementing exactly ONE build unit: license detection and copyleft risk analysis.** Do not start work on other units. Build and tests must be green before this prompt is considered done.

**Prerequisites:** P02 (core types), P04 (dependencies) are complete.

---

## Phase & Dependencies

- **Phase:** Ingestion (parallel with P03/P04)
- **Depends on:** P02, P04

---

## Scope & Deliverables

Implement `repogate-licensing` crate for license analysis.

### File: `src/detect.rs` — License Detection

```rust
use serde::{Serialize, Deserialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum DetectionMethod {
    LicenseFile,
    SpdxHeader,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LicenseDetection {
    pub file_path: String,
    pub spdx_expression: String,
    pub confidence: f32,  // 0.0–1.0
    pub detection_method: DetectionMethod,
    pub needs_review: bool,  // confidence < 0.75
}

pub async fn detect_licenses(repo_path: &std::path::Path) -> Result<Vec<LicenseDetection>, LicensingError> {
    let mut detections = Vec::new();
    
    // Look for LICENSE*, COPYING*, NOTICE*, LICENCE* files
    for entry in walkdir::WalkDir::new(repo_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let file_name = entry.file_name().to_string_lossy().to_uppercase();
        if file_name.starts_with("LICENSE") || file_name.starts_with("COPYING") 
            || file_name.starts_with("NOTICE") || file_name.starts_with("LICENCE") {
            
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                // Use askalono to detect license
                if let Ok(license_info) = askalono::identify(&content) {
                    detections.push(LicenseDetection {
                        file_path: entry.path().to_string_lossy().to_string(),
                        spdx_expression: license_info.name.to_string(),
                        confidence: license_info.confidence as f32,
                        detection_method: DetectionMethod::LicenseFile,
                        needs_review: license_info.confidence < 0.75,
                    });
                }
            }
        }
    }
    
    // Scan first 30 lines of source files for SPDX-License-Identifier
    for entry in walkdir::WalkDir::new(repo_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| is_source_file(e.path()))
    {
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            for (i, line) in content.lines().enumerate() {
                if i >= 30 { break; }
                if let Some(expr) = extract_spdx_header(line) {
                    detections.push(LicenseDetection {
                        file_path: entry.path().to_string_lossy().to_string(),
                        spdx_expression: expr,
                        confidence: 0.95,
                        detection_method: DetectionMethod::SpdxHeader,
                        needs_review: false,
                    });
                    break;
                }
            }
        }
    }
    
    Ok(detections)
}

fn is_source_file(path: &std::path::Path) -> bool {
    if let Some(ext) = path.extension() {
        matches!(ext.to_string_lossy().as_ref(), "rs" | "ts" | "js" | "py" | "go" | "java" | "rb")
    } else {
        false
    }
}

fn extract_spdx_header(line: &str) -> Option<String> {
    if line.contains("SPDX-License-Identifier:") {
        line.split("SPDX-License-Identifier:").nth(1).map(|s| s.trim().to_string())
    } else {
        None
    }
}
```

### File: `src/spdx.rs` — SPDX Expression Parsing

```rust
use spdx::Expression;

pub fn parse_and_normalize(expr_str: &str) -> Result<String, LicensingError> {
    let expr = Expression::parse(expr_str)
        .map_err(|e| LicensingError::SpdxParseFailed(e.to_string()))?;
    
    // Return normalized expression
    Ok(expr.to_string())
}

pub fn extract_base_identifiers(expr_str: &str) -> Result<Vec<String>, LicensingError> {
    let expr = Expression::parse(expr_str)?;
    
    // Walk the AST to collect all license IDs
    let mut ids = Vec::new();
    collect_ids(&expr, &mut ids);
    
    Ok(ids)
}

fn collect_ids(expr: &Expression, ids: &mut Vec<String>) {
    // Recursively extract license identifiers from expression tree
}
```

### File: `src/copyleft.rs` — Copyleft Classification

```rust
use serde::{Serialize, Deserialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum CopyleftTier {
    StrongCopyleft,      // GPL-3.0, AGPL-3.0
    WeakCopyleft,        // LGPL, MPL-2.0
    SourceAvailableNonOsi, // BSL-1.1, EUPLv1.1
    Permissive,          // MIT, Apache-2.0, BSD
    PublicDomain,        // Unlicense, CC0
    Unknown,
}

pub fn classify_license(spdx_id: &str) -> CopyleftTier {
    match spdx_id {
        "GPL-3.0" | "GPL-3.0-or-later" | "AGPL-3.0" | "AGPL-3.0-or-later" => CopyleftTier::StrongCopyleft,
        "GPL-2.0" | "GPL-2.0-or-later" => CopyleftTier::StrongCopyleft,
        "LGPL-2.1" | "LGPL-3.0" | "MPL-2.0" => CopyleftTier::WeakCopyleft,
        "BSL-1.1" => CopyleftTier::SourceAvailableNonOsi,
        "MIT" | "Apache-2.0" | "BSD-2-Clause" | "BSD-3-Clause" => CopyleftTier::Permissive,
        "Unlicense" | "CC0-1.0" => CopyleftTier::PublicDomain,
        _ => CopyleftTier::Unknown,
    }
}

pub fn copyleft_risk_score(tier: CopyleftTier) -> f32 {
    match tier {
        CopyleftTier::StrongCopyleft => 9.0,
        CopyleftTier::WeakCopyleft => 4.0,
        CopyleftTier::SourceAvailableNonOsi => 3.0,
        CopyleftTier::Permissive => 0.0,
        CopyleftTier::PublicDomain => 0.0,
        CopyleftTier::Unknown => 2.0,
    }
}
```

### File: `src/report.rs` — License Report

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LicenseReport {
    pub repo_id: String,
    pub detections: Vec<LicenseDetection>,
    pub dependency_licenses: Vec<(String, String)>,  // (dep_name, spdx_expr)
    pub copyleft_exposure: f32,  // 0.0–10.0
    pub missing_licenses: bool,
    pub conflicts: Vec<String>,
    pub overall_risk_score: f32,  // 0.0–10.0
}

pub fn build_report(
    repo_id: &str,
    detections: Vec<LicenseDetection>,
    deps: &[repogate_ingestion::DependencyRecord],
) -> Result<LicenseReport, LicensingError> {
    // Analyze detections for copyleft, conflicts
    let mut copyleft_exposure = 0.0;
    let mut overall_risk = 0.0;
    let mut dep_licenses = Vec::new();
    
    for detection in &detections {
        let tier = classify_license(&detection.spdx_expression);
        copyleft_exposure = copyleft_exposure.max(copyleft_risk_score(tier));
    }
    
    for dep in deps {
        if let Some(license) = &dep.spdx_license {
            dep_licenses.push((dep.name.clone(), license.clone()));
            let tier = classify_license(license);
            overall_risk = overall_risk.max(copyleft_risk_score(tier));
        }
    }
    
    Ok(LicenseReport {
        repo_id: repo_id.to_string(),
        detections,
        dependency_licenses: dep_licenses,
        copyleft_exposure,
        missing_licenses: overall_risk == 0.0 && detections.is_empty(),
        conflicts: vec![],  // TODO: detect mixed-license conflicts
        overall_risk_score: overall_risk,
    })
}
```

### File: `src/lib.rs`

```rust
#![doc = "RepoGate license detection and copyleft analysis."]

pub mod detect;
pub mod spdx;
pub mod copyleft;
pub mod report;

pub use detect::{LicenseDetection, DetectionMethod};
pub use copyleft::CopyleftTier;
pub use report::LicenseReport;

#[derive(Debug, thiserror::Error)]
pub enum LicensingError {
    #[error("SPDX parse failed: {0}")]
    SpdxParseFailed(String),
    
    #[error("license detection failed")]
    DetectionFailed,
    
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub async fn analyze(
    manifest: &repogate_ingestion::RepoManifest,
    repo_path: &std::path::Path,
) -> Result<LicenseReport, LicensingError> {
    let detections = detect::detect_licenses(repo_path).await?;
    let report = report::build_report(&manifest.repo_id, detections, &manifest.dependencies)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_gpl3() {
        assert_eq!(copyleft::classify_license("GPL-3.0"), CopyleftTier::StrongCopyleft);
    }

    #[test]
    fn classify_mit() {
        assert_eq!(copyleft::classify_license("MIT"), CopyleftTier::Permissive);
    }

    #[test]
    fn classify_bsl() {
        assert_eq!(copyleft::classify_license("BSL-1.1"), CopyleftTier::SourceAvailableNonOsi);
    }

    #[test]
    fn risk_score_agpl() {
        let score = copyleft::copyleft_risk_score(CopyleftTier::StrongCopyleft);
        assert!(score >= 8.0);
    }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-006-license-dependency-analysis.md`** — `askalono`, `spdx` parsing, copyleft classification
- **`docs/adr/ADR-010-commercial-value-scoring-model.md`** — License risk sub-score contribution
- **`docs/ddd/license-compliance.md`** — License model, copyleft tiers, risk invariants

---

## Acceptance Criteria

- ✅ `cargo test -p repogate-licensing` passes
- ✅ `classify_license("AGPL-3.0")` → `CopyleftTier::StrongCopyleft`
- ✅ `classify_license("MIT")` → `CopyleftTier::Permissive`
- ✅ `classify_license("BSL-1.1")` → `CopyleftTier::SourceAvailableNonOsi`
- ✅ Repo with `"GPL-3.0"` license → `overall_risk_score >= 8.0`
- ✅ SPDX expression `"GPL-2.0-only WITH Classpath-exception-2.0"` parses without error
- ✅ LicenseReport round-trips through JSON

---

## Language

**Rust** — License detection, SPDX parsing, copyleft classification, risk scoring.

---

## Out-of-Scope

- Do NOT implement module-level license inference; focus on repo and dependencies
- Do NOT implement automatic license compliance remediation
- Do NOT implement detailed SPDX license conflict resolution (flag for manual review)


---

# P06 — `repogate-orchestrator`: Claude Code Subprocess Driver

## Context

RepoGate is a deep repository assessment platform using Claude Code as the reasoning engine.

**You are implementing exactly ONE build unit: the lowest-level Claude Code integration (subprocess, streaming, schema enforcement).** Do not start work on other units. Build and tests must be green before this prompt is considered done.

**Prerequisites:** P02 (core types), P03 (ingestion basics) are complete.

---

## Phase & Dependencies

- **Phase:** Orchestration core
- **Depends on:** P02, P03

---

## Scope & Deliverables

Implement `repogate-orchestrator/src/claude/` module for headless Claude invocation.

### File: `src/claude/invocation.rs` — Invocation Builder

```rust
#[derive(Debug, Clone)]
pub enum ClaudeModel {
    Opus,   // claude-opus-4-8
    Sonnet, // claude-sonnet-4-6
}

#[derive(Debug, Clone)]
pub struct ClaudeInvocation {
    pub prompt: String,
    pub model: ClaudeModel,
    pub schema_path: Option<std::path::PathBuf>,
    pub allowed_tools: Vec<String>,
    pub system_prompt: Option<String>,
    pub working_dir: Option<std::path::PathBuf>,
    pub session_id: Option<String>,
}

impl ClaudeInvocation {
    pub fn build_command(&self) -> std::process::Command {
        let model_id = match self.model {
            ClaudeModel::Opus => "claude-opus-4-8",
            ClaudeModel::Sonnet => "claude-sonnet-4-6",
        };
        
        let mut cmd = std::process::Command::new("claude");
        cmd.arg("--bare")
            .arg("-p")
            .arg(&self.prompt);
        
        cmd.arg("--output-format").arg("stream-json");
        
        if let Some(schema_path) = &self.schema_path {
            cmd.arg("--json-schema").arg(schema_path);
        }
        
        let tools = self.allowed_tools.join(",");
        if !tools.is_empty() {
            cmd.arg("--allowedTools").arg(tools);
        }
        
        if let Some(sys) = &self.system_prompt {
            cmd.arg("--append-system-prompt").arg(sys);
        }
        
        cmd.arg("--model").arg(model_id);
        
        if let Some(session) = &self.session_id {
            cmd.arg("--resume").arg(session);
        }
        
        if let Some(wd) = &self.working_dir {
            cmd.current_dir(wd);
        }
        
        cmd
    }
}
```

### File: `src/claude/stream.rs` — Stream Parsing

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeEvent {
    #[serde(rename = "init")]
    Init { session_id: String },
    
    #[serde(rename = "assistant")]
    Assistant { content: String },
    
    #[serde(rename = "tool_result")]
    ToolResult { result: serde_json::Value },
    
    #[serde(rename = "result")]
    Result { content: String, usage: UsageStats },
    
    #[serde(rename = "error")]
    Error { message: String, code: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
}

pub struct StreamParser;

impl StreamParser {
    pub fn parse_stream(reader: impl std::io::BufRead) -> impl Iterator<Item = Result<ClaudeEvent, StreamError>> {
        reader.lines().filter_map(move |line| {
            let line = line.ok()?;
            if line.trim().is_empty() {
                return None;
            }
            match serde_json::from_str::<ClaudeEvent>(&line) {
                Ok(event) => Some(Ok(event)),
                Err(e) => Some(Err(StreamError::DeserializeFailed(e.to_string()))),
            }
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("deserialize failed: {0}")]
    DeserializeFailed(String),
    
    #[error("stream ended unexpectedly")]
    Truncated,
}
```

### File: `src/claude/session.rs` — Session Execution

```rust
pub struct SessionResult {
    pub session_id: String,
    pub output: String,
    pub usage: UsageStats,
}

pub async fn run_session(invocation: ClaudeInvocation) -> Result<SessionResult, OrchestratorError> {
    let mut cmd = invocation.build_command();
    let mut child = cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| OrchestratorError(format!("spawn failed: {e}")))?;
    
    let stdout = child.stdout.take().ok_or_else(|| 
        OrchestratorError("no stdout".into()))?;
    
    let reader = std::io::BufReader::new(stdout);
    let mut session_id = String::new();
    let mut final_output = String::new();
    let mut usage = UsageStats {
        input_tokens: 0,
        output_tokens: 0,
        cache_read_input_tokens: 0,
    };
    
    for event in stream::StreamParser::parse_stream(reader) {
        match event {
            Ok(stream::ClaudeEvent::Init { session_id: sid }) => {
                session_id = sid;
            }
            Ok(stream::ClaudeEvent::Result { content, usage: u }) => {
                final_output = content;
                usage = u;
            }
            Ok(stream::ClaudeEvent::Error { message, code }) => {
                return Err(OrchestratorError(format!("session error: {code}: {message}")));
            }
            _ => {}
        }
    }
    
    let status = child.wait().await
        .map_err(|e| OrchestratorError(format!("wait failed: {e}")))?;
    
    if !status.success() {
        return Err(OrchestratorError(format!("exit code: {}", status.code().unwrap_or(-1))));
    }
    
    Ok(SessionResult {
        session_id,
        output: final_output,
        usage,
    })
}
```

### File: `src/claude/routing.rs` — Model Selection

```rust
#[derive(Debug, Clone, Copy)]
pub enum Phase {
    Synthesis,
    ManifestSummarization,
    FeatureDiscovery,
    RiskAnalysis,
}

pub fn select_model(module_name: &str, phase: Phase) -> ClaudeModel {
    match phase {
        Phase::Synthesis => ClaudeModel::Opus,
        Phase::ManifestSummarization => ClaudeModel::Sonnet,
        Phase::RiskAnalysis => ClaudeModel::Sonnet,
        Phase::FeatureDiscovery => {
            // Use Opus for large/complex/enterprise modules
            let enterprise_keywords = ["auth", "rbac", "audit", "billing", "enterprise", "compliance", "security"];
            if enterprise_keywords.iter().any(|kw| module_name.to_lowercase().contains(kw)) {
                ClaudeModel::Opus
            } else {
                ClaudeModel::Sonnet
            }
        }
    }
}
```

### File: `src/claude/schema.rs` — Schema Export

```rust
pub fn write_phase_schema(phase: Phase, dir: &std::path::Path) -> Result<std::path::PathBuf, SchemaError> {
    use repogate_core::JsonSchema;
    
    let path = dir.join(format!("{:?}-schema.json", phase));
    
    match phase {
        Phase::Synthesis => {
            repogate_core::write_schema::<repogate_core::SynthesisOutput>(&path)?;
        }
        Phase::FeatureDiscovery => {
            repogate_core::write_schema::<repogate_core::ModuleAssessment>(&path)?;
        }
        Phase::RiskAnalysis => {
            // Similar pattern for risk output schema
        }
        _ => {}
    }
    
    Ok(path)
}

#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("schema write failed: {0}")]
    WriteFailed(String),
}
```

### File: `src/lib.rs`

```rust
pub mod claude;

#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("orchestrator error: {0}")]
    SessionFailed(String),
    
    #[error("schema violation: {0}")]
    SchemaViolation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_command_contains_bare() {
        let inv = claude::invocation::ClaudeInvocation {
            prompt: "test".to_string(),
            model: claude::invocation::ClaudeModel::Opus,
            schema_path: None,
            allowed_tools: vec![],
            system_prompt: None,
            working_dir: None,
            session_id: None,
        };
        let cmd = inv.build_command();
        // Verify command structure (integration test required for full verification)
    }

    #[test]
    fn parse_canned_json() {
        let json = r#"{"type": "init", "session_id": "test-123"}"#;
        let event: claude::stream::ClaudeEvent = serde_json::from_str(json).unwrap();
        if let claude::stream::ClaudeEvent::Init { session_id } = event {
            assert_eq!(session_id, "test-123");
        }
    }

    #[test]
    fn select_model_synthesis() {
        let model = claude::routing::select_model("any", claude::routing::Phase::Synthesis);
        assert!(matches!(model, claude::invocation::ClaudeModel::Opus));
    }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-002-claude-code-analysis-engine.md`** — Claude Code as engine
- **`docs/adr/ADR-003-headless-claude-code-invocation.md`** — Invocation flags, subprocess
- **`docs/adr/ADR-007-schema-enforced-structured-output.md`** — Schema enforcement
- **`docs/adr/ADR-012-model-routing.md`** — Model selection strategy

---

## Acceptance Criteria

- ✅ `cargo test -p repogate-orchestrator` passes (mock command output; no live API)
- ✅ `build_command()` contains `--bare`, `--output-format stream-json`, `--json-schema`, `--allowedTools`
- ✅ `parse_stream` deserializes `Init`, `Result`, `Error` from newline-delimited JSON
- ✅ `select_model("auth", Synthesis)` → `Opus`; `select_model("utils", ManifestSummarization)` → `Sonnet`
- ✅ No live Claude calls in tests (mock canned JSON)

---

## Language

**Rust** — Subprocess command building, JSON stream parsing, subprocess execution.

---

## Out-of-Scope

- Do NOT implement Claude Code CLI authentication
- Do NOT call live Claude API; mock all tests
- Do NOT implement result storage or persistence


---

# P07 — `repogate-orchestrator`: AssessmentJob State Machine, Token Budget, Crash Recovery

## Context

RepoGate uses Claude Code as the reasoning engine. **You are implementing the durable job state machine, budget tracking, and crash recovery framework.**

**Prerequisites:** P06 (Claude driver) is complete.

---

## Phase & Dependencies

- **Phase:** Orchestration core
- **Depends on:** P06

---

## Scope & Deliverables

Implement persistent job state machine and crash recovery in `repogate-orchestrator/src/job/`.

### File: `src/job/state.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Queued, Ingesting, Analyzing, Scoring, Strategizing, RiskAnalyzing, Reporting, Complete, Failed, Recovering,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseKind {
    Ingestion, LicenseScan, ArchitectureMapping, FeatureDiscovery, Scoring, Strategy, RiskAnalysis, ReportAssembly, ManifestSummarization, Synthesis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisPhase {
    pub id: String,
    pub job_id: String,
    pub phase_kind: PhaseKind,
    pub status: JobStatus,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub tokens_used: u64,
    pub session_ids: Vec<String>,
    pub error: Option<String>,
    pub retry_count: u32,
}
```

### File: `src/job/budget.rs`

```rust
use std::sync::atomic::{AtomicU64, Ordering};

pub struct BudgetTracker {
    budget: repogate_core::TokenBudget,
    used: Arc<AtomicU64>,
}

pub enum BudgetStatus {
    Ok,
    Warning,
    Exceeded,
}

impl BudgetTracker {
    pub fn new(budget: repogate_core::TokenBudget) -> Self {
        Self {
            budget,
            used: Arc::new(AtomicU64::new(0)),
        }
    }
    
    pub fn record_usage(&self, input: u64, output: u64, cache_read: u64) -> BudgetStatus {
        let cost = input + output + (cache_read / 10);  // cache at 10%
        let prev = self.used.fetch_add(cost, Ordering::SeqCst);
        let new_total = prev + cost;
        
        if new_total >= self.budget.total_limit {
            BudgetStatus::Exceeded
        } else if new_total as f32 >= self.budget.total_limit as f32 * self.budget.warn_threshold {
            BudgetStatus::Warning
        } else {
            BudgetStatus::Ok
        }
    }
    
    pub fn is_exceeded(&self) -> bool {
        self.used.load(Ordering::SeqCst) >= self.budget.total_limit
    }
    
    pub fn remaining(&self) -> u64 {
        let used = self.used.load(Ordering::SeqCst);
        self.budget.total_limit.saturating_sub(used)
    }
    
    pub fn estimated_cost_usd(&self) -> f64 {
        let used = self.used.load(Ordering::SeqCst);
        // Opus: $3/$15 per 1M input/output; Sonnet: $3/$15
        (used as f64) * 0.003 / 1_000_000.0  // Simplified
    }
}
```

### File: `src/job/checkpoint.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCheckpoint {
    pub job_id: String,
    pub last_completed_phase: Option<PhaseKind>,
    pub completed_module_ids: Vec<String>,
    pub token_usage_so_far: u64,
    pub partial_results: serde_json::Value,
    pub saved_at: String,
}

pub trait CheckpointStore: Send + Sync {
    async fn save(&self, checkpoint: JobCheckpoint) -> Result<(), StoreError>;
    async fn load(&self, job_id: &str) -> Result<Option<JobCheckpoint>, StoreError>;
}

pub struct InMemoryCheckpointStore {
    checkpoints: Arc<Mutex<HashMap<String, JobCheckpoint>>>,
}

impl CheckpointStore for InMemoryCheckpointStore {
    async fn save(&self, checkpoint: JobCheckpoint) -> Result<(), StoreError> {
        self.checkpoints.lock().unwrap().insert(checkpoint.job_id.clone(), checkpoint);
        Ok(())
    }
    
    async fn load(&self, job_id: &str) -> Result<Option<JobCheckpoint>, StoreError> {
        Ok(self.checkpoints.lock().unwrap().get(job_id).cloned())
    }
}

pub fn phases_to_run(checkpoint: &JobCheckpoint) -> Vec<PhaseKind> {
    let all_phases = vec![
        PhaseKind::Ingestion,
        PhaseKind::LicenseScan,
        PhaseKind::ArchitectureMapping,
        PhaseKind::FeatureDiscovery,
        PhaseKind::Scoring,
        PhaseKind::Strategy,
        PhaseKind::RiskAnalysis,
        PhaseKind::ReportAssembly,
    ];
    
    if let Some(last) = checkpoint.last_completed_phase {
        all_phases.into_iter().skip_while(|p| p != &last).skip(1).collect()
    } else {
        all_phases
    }
}
```

### File: `src/job/store.rs`

```rust
pub trait AssessmentJobStore: Send + Sync {
    async fn save(&self, job: AssessmentJob) -> Result<(), StoreError>;
    async fn find_by_id(&self, job_id: &str) -> Result<Option<AssessmentJob>, StoreError>;
    async fn find_by_status(&self, status: JobStatus) -> Result<Vec<AssessmentJob>, StoreError>;
    async fn find_concurrent_for_repo(&self, repo_url: &str) -> Result<Vec<AssessmentJob>, StoreError>;
}

pub trait ModuleAssessmentStore: Send + Sync {
    async fn save(&self, assessment: ModuleAssessment) -> Result<(), StoreError>;
    async fn find_by_module_id(&self, module_id: &str) -> Result<Option<ModuleAssessment>, StoreError>;
}

pub struct InMemoryAssessmentJobStore {
    jobs: Arc<Mutex<HashMap<String, AssessmentJob>>>,
}

// Implement trait for InMemoryAssessmentJobStore
impl AssessmentJobStore for InMemoryAssessmentJobStore {
    async fn save(&self, job: AssessmentJob) -> Result<(), StoreError> {
        self.jobs.lock().unwrap().insert(job.id.clone(), job);
        Ok(())
    }
    
    async fn find_by_id(&self, job_id: &str) -> Result<Option<AssessmentJob>, StoreError> {
        Ok(self.jobs.lock().unwrap().get(job_id).cloned())
    }
    
    // ... other methods
}
```

### File: `src/job/gates.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub enum PhaseGateError {
    #[error("phase gate failed: {0}")]
    Precondition(String),
}

pub fn validate_gate(from_phase: PhaseKind, state: &AssessmentJob) -> Result<(), PhaseGateError> {
    match from_phase {
        PhaseKind::Ingestion => {
            // Next: Analysis; require manifest non-empty
            if state.manifest.as_ref().map(|m| m.total_files == 0).unwrap_or(true) {
                return Err(PhaseGateError::Precondition("manifest empty".into()));
            }
        }
        PhaseKind::FeatureDiscovery => {
            // Next: Scoring; require all modules have ≥1 stored assessment
            // Check state.module_assessments
        }
        _ => {}
    }
    Ok(())
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-009-multi-phase-pipeline-crash-recovery.md`** — State machine, persistence, recovery
- **`docs/adr/ADR-013-token-budget-enforcement.md`** — Budget tracking, hard/soft limits, dollar tracking

---

## Acceptance Criteria

- ✅ `BudgetTracker::record_usage(500, 200, 0)` → `total 700`; `is_exceeded()` true when over limit
- ✅ `phases_to_run()` at checkpoint `LicenseScan` returns remaining phases
- ✅ `validate_gate(FeatureDiscovery→Scoring)` fails when not all assessments stored
- ✅ Round-trip save/load via `InMemoryAssessmentJobStore`
- ✅ `cargo test -p repogate-orchestrator` passes

---

## Language

**Rust** — State machine, async storage traits, budget arithmetic.

---

## Out-of-Scope

- Do NOT implement database persistence (P13 handles sqlx)
- Do NOT implement complex retry logic; focus on gate validation


---

# P08 — `repogate-orchestrator`: Architecture Mapping Phase

## Context

**You are implementing the architecture mapping phase: deterministic module boundary detection and manifest summarization via Claude.**

**Prerequisites:** P07 (state machine) is complete.

---

## Phase & Dependencies

- **Phase:** Analysis pipeline
- **Depends on:** P07

---

## Scope & Deliverables

### File: `src/pipeline/arch_mapping.rs`

```rust
pub struct ModuleNode {
    pub id: String,
    pub name: String,
    pub path: String,
    pub layer: repogate_core::Layer,
    pub centrality: f32,  // 0.0–1.0
    pub file_count: usize,
    pub loc: usize,
    pub has_public_interface: bool,
}

pub struct ArchitectureMap {
    pub modules: Vec<ModuleNode>,
    pub edges: Vec<(String, String)>,  // (from_id, to_id)
    pub ascii_diagram: String,
}

pub async fn run_architecture_mapping_phase(
    manifest: &RepoManifest,
    repo_path: &Path,
    session_runner: impl SessionRunner,
) -> Result<ArchitectureMap, OrchestratorError> {
    // Deterministic heuristics: top-level dirs, Cargo workspaces, npm workspaces, language clusters, size caps
    let mut module_candidates = detect_modules_heuristic(manifest, repo_path);
    
    // Claude manifest summarization (Sonnet)
    let prompt = format!("Analyze module boundaries. Return ModuleNode schema for: {:?}", module_candidates);
    let result = session_runner.run(
        ClaudeInvocation {
            prompt,
            model: ClaudeModel::Sonnet,
            schema_path: Some("module_summary_schema.json".into()),
            allowed_tools: vec![],
            system_prompt: None,
            working_dir: Some(repo_path.to_path_buf()),
            session_id: None,
        }
    ).await?;
    
    let modules: Vec<ModuleNode> = serde_json::from_str(&result.output)?;
    let edges = compute_dependencies(&modules, repo_path)?;
    let ascii_diagram = generate_ascii_diagram(&modules, &edges);
    
    Ok(ArchitectureMap { modules, edges, ascii_diagram })
}

fn detect_modules_heuristic(manifest: &RepoManifest, repo_path: &Path) -> Vec<ModuleNode> {
    let mut candidates = Vec::new();
    
    // Top-level dir grouping
    for dir in &["src", "cli", "lib", "tests", "examples", "docs"] {
        let path = repo_path.join(dir);
        if path.exists() {
            candidates.push(ModuleNode {
                id: dir.to_string(),
                name: dir.to_string(),
                path: dir.to_string(),
                layer: match *dir {
                    "src" => repogate_core::Layer::Core,
                    "cli" => repogate_core::Layer::Cli,
                    "lib" => repogate_core::Layer::Core,
                    "tests" => repogate_core::Layer::Test,
                    "examples" => repogate_core::Layer::Documentation,
                    "docs" => repogate_core::Layer::Documentation,
                    _ => repogate_core::Layer::Core,
                },
                centrality: 0.5,
                file_count: 0,
                loc: 0,
                has_public_interface: true,
            });
        }
    }
    
    candidates
}

fn compute_dependencies(modules: &[ModuleNode], _repo_path: &Path) -> Result<Vec<(String, String)>, OrchestratorError> {
    // Simplified: analyze imports to find edges
    Ok(vec![])
}

fn generate_ascii_diagram(modules: &[ModuleNode], _edges: &[(String, String)]) -> String {
    // Generate simple text-art tree
    modules.iter()
        .map(|m| format!("  ├─ {}", m.name))
        .collect::<Vec<_>>()
        .join("\n")
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-008-deep-traversal-map-reduce.md`** — Boundary heuristics, Sonnet summarization
- **`docs/ddd/architecture-mapping.md`** — ModuleNode, DependencyEdge, Layer, Centrality

---

## Acceptance Criteria

- ✅ Heuristic: repo with `src/`, `cli/`, `tests/` → 3 module candidates
- ✅ Cargo workspace with 3 members → 3 modules
- ✅ ArchitectureMap serializes to valid JSON
- ✅ ASCII diagram generates without panic
- ✅ `cargo test -p repogate-orchestrator` passes

---

## Language

**Rust** — Heuristics, Claude integration, graph generation.

---

## Out-of-Scope

- Do NOT implement complex dependency graph analysis
- Do NOT call live Claude; use mock SessionRunner in tests


---

# P09 — `repogate-orchestrator`: Functionality Discovery Fan-Out Phase

## Context

**You are implementing the fan-out phase where each module gets analyzed by Claude for deep functionality discovery.**

**Prerequisites:** P08 (architecture mapping) is complete.

---

## Phase & Dependencies

- **Phase:** Analysis pipeline
- **Depends on:** P08

---

## Scope & Deliverables

### File: `src/pipeline/feature_discovery.rs`

```rust
pub async fn run_feature_discovery_phase(
    arch_map: &ArchitectureMap,
    repo_path: &Path,
    session_runner: impl SessionRunner + Clone,
    module_store: &dyn ModuleAssessmentStore,
    budget: &BudgetTracker,
    job_id: &str,
    max_concurrent: usize,
) -> Result<FunctionalityInventory, OrchestratorError> {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));
    let mut handles = vec![];
    
    for module in &arch_map.modules {
        // Skip if already analyzed (crash recovery)
        if module_store.find_by_module_id(&module.id).await.ok().flatten().is_some() {
            continue;
        }
        
        let module_clone = module.clone();
        let sem = semaphore.clone();
        let runner = session_runner.clone();
        let store = module_store.clone();
        let repo = repo_path.to_path_buf();
        let model = select_model(&module.name, Phase::FeatureDiscovery);
        
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.ok()?;
            
            let prompt = format!("Analyze module {} at {}. Discover all capabilities (public, internal, experimental, undocumented, enterprise). Return ModuleAssessment schema.", module_clone.name, module_clone.path);
            
            let result = runner.run(
                ClaudeInvocation {
                    prompt,
                    model,
                    schema_path: Some("module_assessment_schema.json".into()),
                    allowed_tools: vec!["Read".into(), "Glob".into(), "Bash(grep)".into(), "Bash(find)".into()],
                    system_prompt: None,
                    working_dir: Some(repo),
                    session_id: None,
                }
            ).await.ok()?;
            
            let assessment: ModuleAssessment = serde_json::from_str(&result.output).ok()?;
            store.save(assessment).await.ok()?;
            
            Some(result.usage.input_tokens + result.usage.output_tokens)
        });
        
        handles.push(handle);
        
        if budget.is_exceeded() {
            break;
        }
    }
    
    // Collect results
    let mut total_tokens = 0u64;
    for handle in handles {
        if let Ok(Some(tokens)) = handle.await {
            total_tokens += tokens;
        }
    }
    
    Ok(FunctionalityInventory {
        repo_id: job_id.to_string(),
        items: vec![],  // Populated from module assessments
        total_count: arch_map.modules.len(),
        hidden_count: 0,
        enterprise_count: 0,
        api_entry_points: vec![],
    })
}
```

### File: `src/pipeline/llm_adapter.rs`

```rust
pub fn parse_module_assessment(raw: &str) -> Result<ModuleAssessment, SchemaViolationError> {
    serde_json::from_str(raw).map_err(|e| SchemaViolationError(e.to_string()))
}

pub fn map_to_functionality_items(
    assessment: &ModuleAssessment,
    module_path: &str,
) -> Vec<FunctionalityItem> {
    assessment.capabilities.iter().map(|cap| {
        FunctionalityItem {
            name: cap.name.clone(),
            description: cap.description.clone(),
            visibility: if cap.is_enterprise {
                Visibility::Enterprise
            } else if cap.is_undocumented {
                Visibility::Undocumented
            } else {
                Visibility::Public
            },
            source_location: None,
            discovery_method: cap.discovery_method.clone(),
            is_confirmed: cap.source_locations.is_some(),
        }
    }).collect()
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-008-deep-traversal-map-reduce.md`** — Sub-agent-per-module, tool allowlist, concurrency
- **`docs/adr/ADR-003-headless-claude-code-invocation.md`** — Tool allowlist

---

## Acceptance Criteria

- ✅ Mock SessionRunner with canned JSON: saves assessments, skips already-analyzed modules
- ✅ Respects concurrency cap (Semaphore)
- ✅ `is_enterprise: true` → `Visibility::Enterprise`
- ✅ Budget exhaustion stops new sessions, preserves stored assessments
- ✅ `cargo test -p repogate-orchestrator` passes

---

## Language

**Rust** — Async fan-out, Claude invocation, schema adaptation.

---

## Out-of-Scope

- Do NOT implement repomix small-repo path (P17)
- Do NOT implement deep source code tracing; rely on Claude


---

# P10 — `repogate-scoring`: Commercial Value Scoring Engine + Tier Classifier

## Context

**You are implementing the commercial value scoring and gating tier classification engine.**

**Prerequisites:** P02 (core types), P05 (licensing), P09 (feature discovery) are complete.

---

## Phase & Dependencies

- **Phase:** Scoring
- **Depends on:** P02, P05, P09

---

## Scope & Deliverables

Implement `repogate-scoring/src/` with scoring engine and tier mapping.

### File: `src/scoring/engine.rs`

```rust
pub fn compute_composite(scores: &CommercialScore, weights: &ScoreWeights) -> CompositeScore {
    let weighted_sum = 
        scores.adoption_value.get() * weights.adoption_value +
        scores.enterprise_buyer_value.get() * weights.enterprise_buyer_value +
        scores.commercial_leverage.get() * weights.commercial_leverage +
        scores.competitive_sensitivity.get() * weights.competitive_sensitivity +
        scores.operational_value.get() * weights.operational_value +
        scores.security_sensitivity.get() * weights.security_sensitivity +
        scores.support_burden.get() * weights.support_burden +  // Negative; subtracts
        scores.strategic_importance.get() * weights.strategic_importance;
    
    let sum_weights: f32 = weights.adoption_value + weights.enterprise_buyer_value + 
        weights.commercial_leverage + weights.competitive_sensitivity +
        weights.operational_value + weights.security_sensitivity + 
        weights.support_burden.abs() + weights.strategic_importance;
    
    let composite = (weighted_sum / sum_weights).max(0.0).min(10.0);
    CompositeScore(composite)
}
```

### File: `src/scoring/license_risk.rs`

```rust
pub fn apply_license_risk(
    composite: CompositeScore,
    exposure: &CopyleftTier,
) -> (CompositeScore, Option<f32>) {
    let adjustment = match exposure {
        CopyleftTier::StrongCopyleft => -8.0,  // Cap at 2.0
        CopyleftTier::WeakCopyleft => -2.0,
        CopyleftTier::SourceAvailableNonOsi => -1.0,
        CopyleftTier::Permissive | CopyleftTier::PublicDomain => 0.0,
        CopyleftTier::Unknown => -0.5,
    };
    
    let adjusted = (composite.0 + adjustment).max(0.0);
    (CompositeScore(adjusted), Some(adjustment))
}
```

### File: `src/scoring/tier.rs`

```rust
pub fn map_to_tier(effective_composite: CompositeScore, license_risk: Option<f32>) -> GatingTier {
    let score = effective_composite.0;
    
    // Check for license issues that force legal review
    if license_risk.map(|r| r < -5.0).unwrap_or(false) {
        return GatingTier::LegalReview;
    }
    
    match score {
        s if s < 2.5 => GatingTier::Open,
        s if s < 4.5 => GatingTier::SourceAvailable,
        s if s < 6.5 => GatingTier::ProTier,
        s if s < 8.0 => GatingTier::EnterpriseTier,
        _ => GatingTier::ManagedCloud,
    }
}
```

### File: `src/scoring/gating_signal.rs`

```rust
pub fn derive_gating_signal(
    effective_composite: CompositeScore,
    adoption_value: Option<Score>,
) -> GatingSignal {
    let adoption = adoption_value.map(|s| s.get()).unwrap_or(5.0);
    
    match effective_composite.0 {
        s if s >= 7.0 => GatingSignal::StrongGateCandidate,
        s if s >= 5.0 && s < 7.0 => GatingSignal::WeakGateCandidate,
        s if s < 5.0 && adoption >= 8.0 => GatingSignal::OpenCandidate,
        s if s < 5.0 => GatingSignal::OpenCandidate,
        _ => GatingSignal::Undetermined,
    }
}
```

### File: `src/scoring/report.rs`

```rust
pub struct ValuationReport {
    pub module_scores: Vec<ModuleValuation>,
    pub strong_gate_count: usize,
    pub open_count: usize,
    pub legal_review_count: usize,
}

pub struct ModuleValuation {
    pub module_id: String,
    pub composite_score: CompositeScore,
    pub tier: GatingTier,
    pub signal: GatingSignal,
}

pub fn score_all_modules(
    assessments: &[ModuleAssessment],
    inventory: &FunctionalityInventory,
    license_report: &LicenseReport,
    weights: &ScoreWeights,
) -> Result<ValuationReport, ScoringError> {
    let mut valuations = Vec::new();
    
    for assessment in assessments {
        let composite = compute_composite(&assessment.commercial_score, weights);
        let (adjusted, _) = apply_license_risk(composite, &CopyleftTier::Permissive);
        let tier = map_to_tier(adjusted, None);
        let signal = derive_gating_signal(adjusted, None);
        
        valuations.push(ModuleValuation {
            module_id: assessment.module_id.clone(),
            composite_score: adjusted,
            tier,
            signal,
        });
    }
    
    let strong_count = valuations.iter().filter(|v| v.tier == GatingTier::EnterpriseTier || v.tier == GatingTier::ManagedCloud).count();
    let open_count = valuations.iter().filter(|v| v.tier == GatingTier::Open).count();
    let legal_count = valuations.iter().filter(|v| v.tier == GatingTier::LegalReview).count();
    
    Ok(ValuationReport {
        module_scores: valuations,
        strong_gate_count: strong_count,
        open_count,
        legal_review_count: legal_count,
    })
}
```

### File: `src/lib.rs`

```rust
pub mod scoring;
pub mod license_risk;
pub mod tier;
pub mod gating_signal;
pub mod report;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composite_equal_weights() {
        let scores = CommercialScore {
            adoption_value: Score::new(5.0).unwrap(),
            // ... all 5.0
        };
        let weights = ScoreWeights::default();
        let composite = scoring::engine::compute_composite(&scores, &weights);
        assert!(composite.0 >= 4.5 && composite.0 <= 5.5);
    }

    #[test]
    fn test_tier_mapping() {
        let tier = tier::map_to_tier(CompositeScore(7.5), None);
        assert_eq!(tier, GatingTier::EnterpriseTier);
    }

    #[test]
    fn test_license_risk_agpl() {
        let (adjusted, _) = license_risk::apply_license_risk(CompositeScore(8.0), &CopyleftTier::StrongCopyleft);
        assert!(adjusted.0 <= 2.0);
    }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-010-commercial-value-scoring-model.md`** — Full scoring model, dimensions, tier ranges, weights
- **`docs/ddd/commercial-valuation.md`** — Value objects, scoring invariants

---

## Acceptance Criteria

- ✅ All scores 5.0 with equal weights → composite ~5.0
- ✅ `support_burden` 10.0 with others 5.0 → composite < 5.0
- ✅ AGPL module → `LegalReview`
- ✅ `map_to_tier(7.5, None)` → `EnterpriseTier`
- ✅ `derive_gating_signal(3.0, 9.0)` → `OpenCandidate`
- ✅ 100% coverage on tier classifier
- ✅ `cargo test -p repogate-scoring` passes

---

## Language

**Rust** — Scoring arithmetic, enum mapping, weighted aggregation.

---

## Out-of-Scope

- Do NOT implement complex machine learning; use deterministic scoring
- Do NOT implement per-language scoring variations


---

# P11 — `repogate-orchestrator`: Synthesis Phase (Gating Strategy + Risk Analysis)

## Context

**You are implementing the synthesis and risk analysis phases: orchestrating Claude for high-level strategy and risk assessment.**

**Prerequisites:** P10 (scoring) is complete.

---

## Phase & Dependencies

- **Phase:** Synthesis
- **Depends on:** P10, P09

---

## Scope & Deliverables

### File: `src/pipeline/synthesis.rs`

```rust
pub async fn run_synthesis_phase(
    valuation: &ValuationReport,
    inventory: &FunctionalityInventory,
    license_report: &LicenseReport,
    arch_map: &ArchitectureMap,
    session_runner: impl SessionRunner,
) -> Result<GatingStrategy, OrchestratorError> {
    let prompt = format!(
        "Based on the valuation and functionality inventory, synthesize an open-core strategy.\
         Return SynthesisOutput schema with tier_assignments.",
        // Include JSON summaries of valuation, inventory, license_report
    );
    
    let result = session_runner.run(
        ClaudeInvocation {
            prompt,
            model: ClaudeModel::Opus,  // Always Opus for synthesis
            schema_path: Some("synthesis_output_schema.json".into()),
            allowed_tools: vec![],
            system_prompt: None,
            working_dir: None,
            session_id: None,
        }
    ).await?;
    
    let synthesis_output: SynthesisOutput = serde_json::from_str(&result.output)?;
    
    let tier_assignments = valuation.module_scores.iter().map(|score| {
        TierAssignment {
            module_id: score.module_id.clone(),
            module_name: score.module_id.clone(),  // Lookup from arch_map
            tier: score.tier,
            rationale: Some(format!("Score: {:.1}/10", score.composite_score.0)),
        }
    }).collect();
    
    Ok(GatingStrategy {
        tier_assignments,
        boundary_description: synthesis_output.strategy_notes,
    })
}
```

### File: `src/pipeline/risk_analysis.rs`

```rust
pub struct RiskProfile {
    pub risks: Vec<Risk>,
    pub blocking_risk_count: usize,
    pub high_severity_count: usize,
    pub overall_risk_level: String,  // "low" | "medium" | "high"
}

pub async fn run_risk_analysis_phase(
    strategy: &GatingStrategy,
    valuation: &ValuationReport,
    license_report: &LicenseReport,
    inventory: &FunctionalityInventory,
    session_runner: impl SessionRunner,
) -> Result<RiskProfile, OrchestratorError> {
    let prompt = format!(
        "Analyze risks in this gating strategy. Return RiskAnalysisOutput with identified risks.",
        // Include strategy, valuation, license, inventory summaries
    );
    
    let result = session_runner.run(
        ClaudeInvocation {
            prompt,
            model: ClaudeModel::Sonnet,  // Risk analysis uses Sonnet
            schema_path: Some("risk_analysis_output_schema.json".into()),
            allowed_tools: vec![],
            system_prompt: None,
            working_dir: None,
            session_id: None,
        }
    ).await?;
    
    let risk_output: RiskAnalysisOutput = serde_json::from_str(&result.output)?;
    
    let blocking_count = risk_output.risks.iter().filter(|r| r.is_blocking).count();
    let high_count = risk_output.risks.iter().filter(|r| r.severity == Severity::High).count();
    
    Ok(RiskProfile {
        risks: map_risks(&risk_output.risks),
        blocking_risk_count: blocking_count,
        high_severity_count: high_count,
        overall_risk_level: if blocking_count > 0 { "high" } else if high_count > 2 { "medium" } else { "low" }.into(),
    })
}

fn map_risks(findings: &[RiskFinding]) -> Vec<Risk> {
    findings.iter().map(|f| {
        Risk {
            kind: RiskKind::OverGating,  // Simplified mapping
            severity: f.severity.clone(),
            description: f.description.clone(),
            mitigation: Some(f.mitigation_suggestion.clone()),
            is_blocking: f.is_blocking,
        }
    }).collect()
}
```

### File: `src/pipeline/runner.rs`

```rust
pub struct PipelineOutput {
    pub manifest: RepoManifest,
    pub arch_map: ArchitectureMap,
    pub license_report: LicenseReport,
    pub inventory: FunctionalityInventory,
    pub valuation: ValuationReport,
    pub strategy: GatingStrategy,
    pub risk_profile: RiskProfile,
    pub is_complete: bool,
}

pub struct PipelineRunner {
    session_runner: Box<dyn SessionRunner>,
    checkpoint_store: Box<dyn CheckpointStore>,
    job_store: Box<dyn AssessmentJobStore>,
    budget: Arc<BudgetTracker>,
}

impl PipelineRunner {
    pub async fn run(
        &self,
        url: &str,
        budget_limit: u64,
        weights: &ScoreWeights,
    ) -> Result<PipelineOutput, OrchestratorError> {
        // P03: Ingest
        let manifest = ingest::ingest(url, &Path::new("/tmp/repo")).await?;
        
        // P04–P05: License scan (parallel)
        let license_report = licensing::analyze(&manifest, &Path::new("/tmp/repo")).await?;
        
        // P08: Architecture mapping
        let arch_map = arch_mapping::run_architecture_mapping_phase(
            &manifest, &Path::new("/tmp/repo"), &self.session_runner
        ).await?;
        
        // P09: Feature discovery
        let inventory = feature_discovery::run_feature_discovery_phase(
            &arch_map, &Path::new("/tmp/repo"), &self.session_runner,
            &*self.job_store, &self.budget, "job-1", 4
        ).await?;
        
        // P10: Scoring
        let valuation = scoring::score_all_modules(&[], &inventory, &license_report, weights)?;
        
        // P11: Synthesis
        let strategy = synthesis::run_synthesis_phase(
            &valuation, &inventory, &license_report, &arch_map, &self.session_runner
        ).await?;
        
        // P11: Risk analysis
        let risk_profile = risk_analysis::run_risk_analysis_phase(
            &strategy, &valuation, &license_report, &inventory, &self.session_runner
        ).await?;
        
        Ok(PipelineOutput {
            manifest,
            arch_map,
            license_report,
            inventory,
            valuation,
            strategy,
            risk_profile,
            is_complete: true,
        })
    }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-008-deep-traversal-map-reduce.md`** — Synthesis pass, JSON summaries
- **`docs/adr/ADR-012-model-routing.md`** — Synthesis Opus, risk Sonnet

---

## Acceptance Criteria

- ✅ Mock canned SynthesisOutput → GatingStrategy with populated tier_assignments
- ✅ Canned RiskAnalysisOutput `is_blocking: true` → Risk::is_blocking: true
- ✅ PipelineRunner::run with mock session runners completes
- ✅ `cargo test -p repogate-orchestrator` passes

---

## Language

**Rust** — Pipeline orchestration, Claude invocation, JSON mapping.

---

## Out-of-Scope

- Do NOT implement detailed risk categorization; focus on structure
- Do NOT call live Claude API


---

# P12 — `repogate-report`: Report Assembly, `minijinja` Templates, Canonical JSON

## Context

**You are implementing report generation: assembling Assessment from pipeline output and rendering JSON + Markdown.**

**Prerequisites:** P11 (synthesis + risk) is complete.

---

## Phase & Dependencies

- **Phase:** Reporting
- **Depends on:** P11

---

## Scope & Deliverables

Implement `repogate-report/src/` for report generation.

### File: `src/assembly.rs`

```rust
pub fn assemble(output: &PipelineOutput, generated_at: &str) -> Assessment {
    Assessment {
        repo_id: output.manifest.repo_id.clone(),
        schema_version: "1.0".to_string(),
        generated_at: generated_at.to_string(),
        is_complete: output.is_complete,
        repository: output.manifest.repository.clone(),
        modules: output.arch_map.modules.iter().map(|m| {
            Module {
                id: m.id.clone(),
                name: m.name.clone(),
                description: None,
                path: m.path.clone(),
                layer: m.layer,
                file_count: m.file_count,
                loc: m.loc,
                commercial_score: None,  // TODO: lookup from valuation
                recommended_tier: None,
                risks: vec![],
            }
        }).collect(),
        gating_strategy: Some(output.strategy.clone()),
        risks: output.risk_profile.risks.clone(),
    }
}
```

### File: `src/json.rs`

```rust
pub fn write_json(assessment: &Assessment, writer: impl std::io::Write) -> Result<(), ReportError> {
    serde_json::to_writer_pretty(writer, assessment)?;
    Ok(())
}

pub fn to_json_bytes(assessment: &Assessment) -> Result<Vec<u8>, ReportError> {
    serde_json::to_vec_pretty(assessment).map_err(|e| ReportError::JsonError(e.to_string()))
}
```

### File: `src/markdown.rs`

```rust
pub fn render_markdown(assessment: &Assessment) -> Result<String, ReportError> {
    let template = r#"
# RepoGate Assessment Report

## Executive Summary

Repository: {{ repo.name }}
Schema Version: {{ schema_version }}

## Gating Recommendations

{% for assignment in gating_strategy.tier_assignments %}
- **{{ assignment.module_name }}**: {{ assignment.tier }}
{% endfor %}

## Risk Analysis

{% for risk in risks %}
- **{{ risk.kind }}** ({{ risk.severity }}): {{ risk.description }}
{% endfor %}

## Modules

{% for module in modules %}
### {{ module.name }}
- Path: {{ module.path }}
- LOC: {{ module.loc }}
{% endfor %}
"#;
    
    use minijinja::Environment;
    let mut env = Environment::new();
    env.add_template("report", template)?;
    
    let tmpl = env.get_template("report")?;
    let rendered = tmpl.render(minijinja::context! {
        repo => &assessment.repository,
        schema_version => &assessment.schema_version,
        gating_strategy => &assessment.gating_strategy,
        risks => &assessment.risks,
        modules => &assessment.modules,
    })?;
    
    Ok(rendered)
}
```

### File: `src/pdf.rs`

```rust
pub fn render_pdf(markdown: &str, output_path: &Path) -> Result<(), ReportError> {
    let mut child = tokio::process::Command::new("pandoc")
        .arg("-f").arg("markdown")
        .arg("-t").arg("pdf")
        .arg("-o").arg(output_path)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ReportError::PandocNotFound
            } else {
                ReportError::PandocError(e.to_string())
            }
        })?;
    
    child.stdin.as_mut().unwrap().write_all(markdown.as_bytes())?;
    drop(child.stdin.take());
    
    child.wait()?;
    Ok(())
}
```

### File: `src/naming.rs`

```rust
pub fn report_stem(repo_url: &str, completed_at: &str) -> String {
    let parts: Vec<&str> = repo_url.trim_end_matches('/').split('/').collect();
    let owner = parts.get(parts.len() - 2).unwrap_or(&"unknown");
    let repo = parts.get(parts.len() - 1).unwrap_or(&"repo");
    
    let slugified_owner = owner.to_lowercase().replace("_", "-");
    let slugified_repo = repo.to_lowercase().replace("_", "-");
    
    format!("repogate-{}-{}-{}", slugified_owner, slugified_repo, completed_at)
}
```

### File: `src/lib.rs`

```rust
pub mod assembly;
pub mod json;
pub mod markdown;
pub mod pdf;
pub mod naming;

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("json error: {0}")]
    JsonError(String),
    
    #[error("pandoc not found")]
    PandocNotFound,
    
    #[error("pandoc error: {0}")]
    PandocError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_stem_github() {
        let stem = naming::report_stem("https://github.com/acme/myproject", "20240101-120000");
        assert!(stem.starts_with("repogate-acme-myproject"));
    }

    #[test]
    fn render_markdown_minimal() {
        let assessment = Assessment {
            // Minimal test assessment
        };
        let md = markdown::render_markdown(&assessment).unwrap();
        assert!(md.contains("Executive Summary"));
    }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-011-assessment-output-formats.md`** — Canonical JSON, minijinja, schema_version, pandoc PDF
- **`docs/ddd/report-delivery.md`** — Report structure, delivery mechanisms

---

## Acceptance Criteria

- ✅ `assemble()` → `is_complete: true` when pipeline complete
- ✅ `render_markdown()` of minimal Assessment contains "Executive Summary" and "Gating Recommendations"
- ✅ `report_stem("https://github.com/acme/myproject", ...)` → `"repogate-acme-myproject-<ts>"`
- ✅ JSON round-trip: write → read → equal
- ✅ `cargo test -p repogate-report` passes

---

## Language

**Rust** — JSON serialization, minijinja templating, PDF subprocess.

---

## Out-of-Scope

- Do NOT implement HTML export
- Do NOT implement report signing or encryption


---

# P13 — `sqlx` Schema, Migrations, Store Implementations

## Context

**You are implementing durable persistence: SQL migrations and sqlx-backed store implementations.**

**Prerequisites:** P07 (state machine), P12 (report assembly) are complete.

---

## Phase & Dependencies

- **Phase:** Persistence
- **Depends on:** P07, P12

---

## Scope & Deliverables

Implement sqlx stores in `repogate-server/src/db/`.

### Directory: `repogate-server/migrations/`

**`0001_jobs.sql`**
```sql
CREATE TABLE jobs (
    id TEXT PRIMARY KEY,
    repo_url TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    budget_limit INTEGER NOT NULL,
    token_usage INTEGER NOT NULL DEFAULT 0,
    error_message TEXT
);
```

**`0002_module_assessments.sql`**
```sql
CREATE TABLE module_assessments (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES jobs(id),
    module_id TEXT NOT NULL,
    module_name TEXT NOT NULL,
    assessment_json TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(job_id, module_id)
);
```

**`0003_checkpoints.sql`**
```sql
CREATE TABLE checkpoints (
    job_id TEXT PRIMARY KEY REFERENCES jobs(id),
    last_completed_phase TEXT,
    completed_modules TEXT NOT NULL,  -- JSON array
    token_usage INTEGER NOT NULL,
    partial_results TEXT,
    saved_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**`0004_reports.sql`**
```sql
CREATE TABLE reports (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL UNIQUE REFERENCES jobs(id),
    assessment_json TEXT NOT NULL,
    markdown_content TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**`0005_cache.sql`**
```sql
CREATE TABLE analysis_cache (
    repo_url TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    cached_assessment TEXT NOT NULL,
    ttl_days INTEGER NOT NULL DEFAULT 30,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (repo_url, commit_sha)
);
```

### File: `src/db/job_store.rs`

```rust
pub struct SqlxAssessmentJobStore {
    pool: sqlx::AnyPool,
}

#[async_trait]
impl AssessmentJobStore for SqlxAssessmentJobStore {
    async fn save(&self, job: AssessmentJob) -> Result<(), StoreError> {
        sqlx::query!(
            r#"
            INSERT INTO jobs (id, repo_url, status, budget_limit, token_usage)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(id) DO UPDATE SET status = ?3, updated_at = CURRENT_TIMESTAMP
            "#,
            job.id,
            job.repo_url,
            format!("{:?}", job.status),  // Simplified
            job.budget_limit,
            job.token_usage,
        ).execute(&self.pool).await?;
        Ok(())
    }
    
    async fn find_by_id(&self, job_id: &str) -> Result<Option<AssessmentJob>, StoreError> {
        let row = sqlx::query!("SELECT * FROM jobs WHERE id = ?1", job_id)
            .fetch_optional(&self.pool)
            .await?;
        
        Ok(row.map(|r| AssessmentJob {
            id: r.id,
            repo_url: r.repo_url,
            status: JobStatus::Queued,  // Parse from r.status
            // ... other fields
        }))
    }
    
    async fn find_by_status(&self, status: JobStatus) -> Result<Vec<AssessmentJob>, StoreError> {
        let status_str = format!("{:?}", status);
        let rows = sqlx::query!("SELECT * FROM jobs WHERE status = ?1", status_str)
            .fetch_all(&self.pool)
            .await?;
        
        Ok(rows.into_iter().map(|_| AssessmentJob { /* ... */ }).collect())
    }
    
    async fn find_concurrent_for_repo(&self, repo_url: &str) -> Result<Vec<AssessmentJob>, StoreError> {
        // Find jobs for same repo running concurrently
        Ok(vec![])
    }
}
```

### File: `src/db/pool.rs`

```rust
pub async fn create_pool(database_url: &str) -> Result<sqlx::AnyPool, sqlx::Error> {
    let pool = sqlx::AnyPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    
    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;
    
    Ok(pool)
}
```

### File: `src/db/cache.rs`

```rust
pub struct AnalysisCacheStore {
    pool: sqlx::AnyPool,
}

impl AnalysisCacheStore {
    pub async fn get(&self, repo_url: &str, commit_sha: &str) -> Result<Option<Assessment>, StoreError> {
        let row = sqlx::query!("SELECT cached_assessment FROM analysis_cache WHERE repo_url = ?1 AND commit_sha = ?2 AND created_at > datetime('now', '-' || ttl_days || ' days')", repo_url, commit_sha)
            .fetch_optional(&self.pool)
            .await?;
        
        Ok(row.and_then(|r| serde_json::from_str(&r.cached_assessment).ok()))
    }
    
    pub async fn set(&self, repo_url: &str, commit_sha: &str, assessment: &Assessment, ttl_days: i32) -> Result<(), StoreError> {
        let json = serde_json::to_string(assessment)?;
        sqlx::query!("INSERT INTO analysis_cache (repo_url, commit_sha, cached_assessment, ttl_days) VALUES (?1, ?2, ?3, ?4)", repo_url, commit_sha, json, ttl_days)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    
    pub async fn invalidate(&self, repo_url: &str) -> Result<(), StoreError> {
        sqlx::query!("DELETE FROM analysis_cache WHERE repo_url = ?1", repo_url)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
```

### File: `scripts/prepare-sqlx.sh`

```bash
#!/bin/bash
cd repogate-server
cargo sqlx prepare -- --lib
```

---

## Source Documents to Read

- **`docs/adr/ADR-014-persistence-sqlx-sqlite-postgres.md`** — sqlx, SQLite dev/Postgres prod, compile-time queries, offline mode

---

## Acceptance Criteria

- ✅ `cargo build -p repogate-server` with sqlx offline mode (`sqlx-data.json` committed)
- ✅ Migrations run cleanly on fresh SQLite DB
- ✅ SqlxAssessmentJobStore save→find_by_id round-trips
- ✅ Cache set→get returns stored assessment; past-TTL → None
- ✅ `cargo test -p repogate-server` passes (in-memory SQLite)

---

## Language

**Rust** — SQL, sqlx query macros, async database operations.

---

## Out-of-Scope

- Do NOT implement connection pooling configuration beyond defaults
- Do NOT implement complex SQL queries


---

# P14 — `repogate-cli`: CLI Entry Point, `repogate analyze`, Cost Estimation, Progress

## Context

**You are implementing the CLI interface: argument parsing, cost estimation, progress reporting.**

**Prerequisites:** P11 (orchestration), P12 (report), P13 (stores) are complete.

---

## Phase & Dependencies

- **Phase:** UX
- **Depends on:** P11, P12, P13

---

## Scope & Deliverables

Implement `repogate-cli/src/`.

### File: `src/main.rs`

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "repogate", about = "Deep repository assessment for open-core gating")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Analyze(AnalyzeArgs),
    Cache(CacheArgs),
}

#[derive(Parser)]
struct AnalyzeArgs {
    /// Repository URL
    #[arg(value_name = "URL")]
    repo_url: String,
    
    /// Budget in USD (required)
    #[arg(long, required = true)]
    budget: f32,
    
    /// Output format: json | markdown | pdf
    #[arg(long, default_value = "markdown")]
    output: String,
    
    /// Output file path
    #[arg(long)]
    output_file: Option<String>,
    
    /// Weights JSON file
    #[arg(long)]
    weights: Option<String>,
    
    /// Model override: opus | sonnet
    #[arg(long)]
    model_override: Option<String>,
    
    /// Max concurrent modules
    #[arg(long, default_value = "4")]
    max_concurrent: usize,
    
    /// Skip confirmation
    #[arg(long)]
    yes: bool,
    
    /// Verbose output
    #[arg(long)]
    verbose: bool,
}

#[derive(Parser)]
struct CacheArgs {
    #[command(subcommand)]
    command: CacheCommands,
}

#[derive(Subcommand)]
enum CacheCommands {
    Invalidate { repo_url: String },
    List,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Analyze(args) => commands::analyze::run(args).await?,
        Commands::Cache(args) => commands::cache::run(args).await?,
    }
    
    Ok(())
}
```

### File: `src/commands/analyze.rs`

```rust
pub async fn run(args: AnalyzeArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Validate URL
    git::validate_repo_url(&args.repo_url)?;
    
    // Instantiate pipeline runner
    let runner = PipelineRunner::new(
        Box::new(MockSessionRunner::default()),
        Box::new(InMemoryCheckpointStore::new()),
        Box::new(InMemoryAssessmentJobStore::new()),
    );
    
    // Estimate cost
    let (min_cost, max_cost) = estimate_cost(&args.repo_url, &args).await?;
    eprintln!("Estimated cost: ${:.2} – ${:.2}", min_cost, max_cost);
    
    if !args.yes {
        eprintln!("Proceed? [y/N] ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            eprintln!("Cancelled.");
            return Ok(());
        }
    }
    
    // Run pipeline with progress
    let reporter = StderrProgressReporter::new();
    let output = runner.run(
        &args.repo_url,
        (args.budget * 1_000_000.0) as u64,  // Tokens equiv
        &ScoreWeights::default(),
    ).await?;
    
    // Assemble report
    let assessment = assembly::assemble(&output, &chrono::Utc::now().to_rfc3339());
    
    // Write output
    match args.output.as_str() {
        "json" => {
            let file = args.output_file.unwrap_or_else(|| "assessment.json".to_string());
            let writer = std::fs::File::create(&file)?;
            json::write_json(&assessment, writer)?;
            println!("Written: {}", file);
        }
        "markdown" | _ => {
            let md = markdown::render_markdown(&assessment)?;
            let file = args.output_file.unwrap_or_else(|| "assessment.md".to_string());
            std::fs::write(&file, md)?;
            println!("Written: {}", file);
        }
        "pdf" => {
            let md = markdown::render_markdown(&assessment)?;
            let file = args.output_file.unwrap_or_else(|| "assessment.pdf".to_string());
            pdf::render_pdf(&md, Path::new(&file))?;
            println!("Written: {}", file);
        }
    }
    
    if !output.is_complete {
        eprintln!("Warning: analysis incomplete due to budget exhaustion.");
        return Err("Budget exceeded".into());
    }
    
    Ok(())
}

async fn estimate_cost(repo_url: &str, args: &AnalyzeArgs) -> Result<(f32, f32), Box<dyn std::error::Error>> {
    // Heuristic: small repos ~$1, large repos ~$15
    Ok((1.0, 15.0))
}
```

### File: `src/progress.rs`

```rust
pub trait ProgressReporter: Send {
    fn report(&self, phase: &str, message: &str);
}

pub struct StderrProgressReporter;

impl ProgressReporter for StderrProgressReporter {
    fn report(&self, phase: &str, message: &str) {
        eprintln!("[{}] {}", phase, message);
    }
}
```

### File: `src/lib.rs`

```rust
pub mod commands;
pub mod progress;

#[cfg(test)]
mod tests {
    #[test]
    fn cli_help_works() {
        // Integration test: run `repogate --help`
    }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-013-token-budget-enforcement.md`** — Confirmation flow, `--yes`, estimate, hard budget
- **`docs/adr/ADR-015-web-api-layer-axum-nextjs.md`** — CLI vs. server modes

---

## Acceptance Criteria

- ✅ `repogate analyze --help` shows `--budget` as required
- ✅ Missing `--budget` → error exit code
- ✅ `--yes` skips confirmation (test with small repo)
- ✅ Budget exhaustion → partial report `is_complete: false`; non-zero exit
- ✅ `cargo build -p repogate-cli` produces binary

---

## Language

**Rust** — clap argument parsing, cost estimation, progress reporting.

---

## Out-of-Scope

- Do NOT implement interactive TUI
- Do NOT implement watch mode or continuous monitoring


---

# P15 — `repogate-server`: `axum` HTTP Server, API Endpoints, Static Serving

## Context

**You are implementing the HTTP API server: REST endpoints, job management, report serving.**

**Prerequisites:** P11 (orchestration), P12 (report), P13 (stores) are complete.

---

## Phase & Dependencies

- **Phase:** UX
- **Depends on:** P11, P12, P13

---

## Scope & Deliverables

Implement `repogate-server/src/main.rs` and route handlers.

### File: `src/main.rs`

```rust
use axum::{
    extract::{Path, Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, delete},
    Router,
};
use std::sync::{Arc, Mutex};
use tower_http::services::ServeDir;

#[derive(Clone)]
struct AppState {
    pool: sqlx::AnyPool,
    pipeline_runner: Arc<PipelineRunner>,
    job_queue: Arc<Mutex<VecDeque<String>>>,
}

#[derive(serde::Deserialize)]
struct AnalysisRequest {
    repo_url: String,
    budget_usd: f32,
    model_override: Option<String>,
    weights: Option<serde_json::Value>,
}

#[derive(serde::Serialize)]
struct JobResponse {
    job_id: String,
    estimated_cost_min: f32,
    estimated_cost_max: f32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let pool = create_pool(&args.database_url).await?;
    
    let app_state = AppState {
        pool,
        pipeline_runner: Arc::new(PipelineRunner::new(/* ... */)),
        job_queue: Arc::new(Mutex::new(VecDeque::new())),
    };
    
    let app = Router::new()
        .route("/health", get(health))
        .route("/assessments", post(create_assessment))
        .route("/assessments/:id", get(get_assessment))
        .route("/assessments/:id/status", get(get_assessment_status))
        .route("/assessments/:id/report", get(get_report))
        .route("/assessments/:id/report.pdf", get(get_report_pdf))
        .route("/assessments/:id", delete(delete_assessment))
        .nest_service("/", ServeDir::new("static"))
        .layer(axum::middleware::Next::layer(auth_middleware))
        .with_state(app_state)
        .fallback(|| async { (StatusCode::NOT_FOUND, "404") })
        .into_make_service_with_connect_info::<std::net::SocketAddr>();
    
    let listener = tokio::net::TcpListener::bind(&args.listen).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn create_assessment(
    State(state): State<AppState>,
    Json(req): Json<AnalysisRequest>,
) -> Result<Json<JobResponse>, AppError> {
    let job_id = uuid::Uuid::new_v4().to_string();
    
    // Estimate cost
    let (min, max) = estimate_cost(&req.repo_url).await?;
    
    // Queue job
    state.job_queue.lock().unwrap().push_back(job_id.clone());
    
    Ok(Json(JobResponse {
        job_id,
        estimated_cost_min: min,
        estimated_cost_max: max,
    }))
}

async fn get_assessment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Fetch from DB
    Ok(Json(serde_json::json!({})))
}

async fn get_assessment_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<StatusResponse>, AppError> {
    Ok(Json(StatusResponse {
        status: "queued".to_string(),
        current_phase: "ingesting".to_string(),
        progress_pct: 10,
        tokens_used: 0,
    }))
}

async fn get_report(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<String, AppError> {
    // Fetch markdown from DB
    Ok("# Report".to_string())
}

async fn get_report_pdf(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Vec<u8>, AppError> {
    Err(AppError::NotFound)
}

async fn delete_assessment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn auth_middleware(
    request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::http::Response<axum::body::Body> {
    if request.uri().path() == "/health" {
        return next.run(request).await;
    }
    
    if let Some(auth) = request.headers().get("Authorization") {
        if let Ok(auth_str) = auth.to_str() {
            if auth_str.starts_with("Bearer ") {
                return next.run(request).await;
            }
        }
    }
    
    axum::http::Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(axum::body::Body::from("Unauthorized"))
        .unwrap()
}

#[derive(serde::Serialize)]
struct StatusResponse {
    status: String,
    current_phase: String,
    progress_pct: u8,
    tokens_used: u64,
}

#[derive(Debug)]
enum AppError {
    NotFound,
    InternalError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not found").into_response(),
            AppError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response(),
        }
    }
}

#[derive(clap::Parser)]
struct Args {
    #[arg(long, default_value = "0.0.0.0:8080")]
    listen: String,
    
    #[arg(long, default_value = "sqlite://repogate.db")]
    database_url: String,
    
    #[arg(long, default_value = "static")]
    static_dir: String,
    
    #[arg(long)]
    api_key: Option<String>,
}

async fn estimate_cost(repo_url: &str) -> Result<(f32, f32), AppError> {
    Ok((1.0, 15.0))
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-015-web-api-layer-axum-nextjs.md`** — axum endpoints, bearer auth, polling, static export

---

## Acceptance Criteria

- ✅ `cargo build -p repogate-server` produces executable
- ✅ `POST /assessments` with valid body → 200 JSON
- ✅ `POST /assessments` without `Authorization` → 401
- ✅ `GET /assessments/:id/status` → `{status: "queued"}`
- ✅ `GET /health` → 200 without auth
- ✅ Integration test: submit → poll → fetch

---

## Language

**Rust** — axum, async HTTP, bearer auth, static file serving.

---

## Out-of-Scope

- Do NOT implement WebSocket (polling is MVP)
- Do NOT implement job history pagination


---

# P16 — `repogate-web`: Next.js Dashboard (TypeScript)

## Context

**You are implementing the web dashboard: form submission, job polling, report viewing.**

**Prerequisites:** P15 (server API) is complete.

---

## Phase & Dependencies

- **Phase:** UX
- **Depends on:** P15

---

## Scope & Deliverables

Implement `repogate-web/` as a Next.js 14+ static export.

### File: `next.config.js`

```javascript
/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'export',
  trailingSlash: true,
  rewrites: async () => [
    {
      source: '/api/:path*',
      destination: 'http://localhost:8080/api/:path*',
    },
  ],
};

module.exports = nextConfig;
```

### File: `src/app/page.tsx`

```typescript
'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';

export default function Home() {
  const router = useRouter();
  const [url, setUrl] = useState('');
  const [budget, setBudget] = useState('5');
  const [loading, setLoading] = useState(false);
  const [apiKey, setApiKey] = useState(() => {
    if (typeof window !== 'undefined') {
      return localStorage.getItem('repogate-api-key') || '';
    }
    return '';
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    
    try {
      const response = await fetch('/api/assessments', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${apiKey}`,
        },
        body: JSON.stringify({
          repo_url: url,
          budget_usd: parseFloat(budget),
        }),
      });
      
      if (!response.ok) throw new Error('Failed to submit');
      
      const data = await response.json();
      localStorage.setItem('repogate-api-key', apiKey);
      router.push(`/jobs/${data.job_id}`);
    } catch (error) {
      alert('Error: ' + (error as Error).message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <main className="min-h-screen bg-gradient-to-br from-slate-900 to-slate-800 flex items-center justify-center p-4">
      <div className="bg-white rounded-lg shadow-2xl p-8 max-w-md w-full">
        <h1 className="text-2xl font-bold text-slate-900 mb-6">RepoGate</h1>
        <form onSubmit={handleSubmit} className="space-y-4">
          <input
            type="password"
            placeholder="API Key"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            className="w-full px-4 py-2 border rounded-lg"
            required
          />
          <input
            type="url"
            placeholder="Repository URL"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            className="w-full px-4 py-2 border rounded-lg"
            required
          />
          <input
            type="number"
            placeholder="Budget (USD)"
            value={budget}
            onChange={(e) => setBudget(e.target.value)}
            className="w-full px-4 py-2 border rounded-lg"
            required
          />
          <button
            type="submit"
            disabled={loading}
            className="w-full bg-blue-600 text-white py-2 rounded-lg hover:bg-blue-700 disabled:opacity-50"
          >
            {loading ? 'Submitting...' : 'Analyze'}
          </button>
        </form>
      </div>
    </main>
  );
}
```

### File: `src/app/jobs/[id]/page.tsx`

```typescript
'use client';

import { useEffect, useState } from 'react';
import { useParams } from 'next/navigation';
import ReportViewer from '@/components/ReportViewer';

export default function JobPage() {
  const params = useParams();
  const jobId = params.id as string;
  const [status, setStatus] = useState<any>(null);
  const [report, setReport] = useState<any>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const poll = async () => {
      try {
        const res = await fetch(`/api/assessments/${jobId}/status`);
        const data = await res.json();
        setStatus(data);
        
        if (data.status === 'complete') {
          const reportRes = await fetch(`/api/assessments/${jobId}/report`);
          const reportData = await reportRes.json();
          setReport(reportData);
        }
      } catch (error) {
        console.error('Poll error:', error);
      } finally {
        setLoading(false);
      }
    };

    const interval = setInterval(poll, 3000);
    poll();
    
    return () => clearInterval(interval);
  }, [jobId]);

  return (
    <main className="min-h-screen bg-slate-50 p-8">
      <h1 className="text-3xl font-bold mb-6">Assessment Status</h1>
      
      {loading ? (
        <div>Loading...</div>
      ) : status?.status === 'complete' && report ? (
        <ReportViewer report={report} />
      ) : (
        <div>
          <p>Status: {status?.status}</p>
          <p>Phase: {status?.current_phase}</p>
          <progress value={status?.progress_pct} max={100} />
        </div>
      )}
    </main>
  );
}
```

### File: `src/components/ReportViewer.tsx`

```typescript
'use client';

import { useState } from 'react';

interface ReportViewerProps {
  report: any;
}

export default function ReportViewer({ report }: ReportViewerProps) {
  const [activeTab, setActiveTab] = useState('summary');

  const tabs = [
    { id: 'summary', label: 'Executive Summary' },
    { id: 'modules', label: 'Modules' },
    { id: 'gating', label: 'Gating Recommendations' },
    { id: 'licensing', label: 'Licensing' },
    { id: 'inventory', label: 'Full Inventory' },
  ];

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <div className="flex gap-4 border-b mb-6">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`px-4 py-2 font-medium ${
              activeTab === tab.id
                ? 'border-b-2 border-blue-600 text-blue-600'
                : 'text-slate-600 hover:text-slate-900'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      <div>
        {activeTab === 'summary' && (
          <div>
            <h2 className="text-xl font-bold mb-4">Executive Summary</h2>
            {/* Render summary content */}
          </div>
        )}
        {activeTab === 'modules' && (
          <div>
            <h2 className="text-xl font-bold mb-4">Modules</h2>
            {/* Render module list */}
          </div>
        )}
        {/* Other tabs similarly */}
      </div>
    </div>
  );
}
```

### File: `src/lib/api.ts`

```typescript
export interface SubmissionRequest {
  repo_url: string;
  budget_usd: number;
  model_override?: string;
  weights?: Record<string, number>;
}

export interface JobStatus {
  status: string;
  current_phase: string;
  progress_pct: number;
  tokens_used: number;
}

export async function submitAssessment(req: SubmissionRequest, apiKey: string) {
  const response = await fetch('/api/assessments', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${apiKey}`,
    },
    body: JSON.stringify(req),
  });
  
  if (!response.ok) throw new Error('Submission failed');
  return response.json();
}

export async function pollStatus(jobId: string, apiKey: string): Promise<JobStatus> {
  const response = await fetch(`/api/assessments/${jobId}/status`, {
    headers: { 'Authorization': `Bearer ${apiKey}` },
  });
  
  if (!response.ok) throw new Error('Poll failed');
  return response.json();
}

export async function fetchReport(jobId: string, apiKey: string) {
  const response = await fetch(`/api/assessments/${jobId}/report`, {
    headers: { 'Authorization': `Bearer ${apiKey}` },
  });
  
  if (!response.ok) throw new Error('Fetch failed');
  return response.json();
}
```

### File: `package.json`

```json
{
  "name": "repogate-web",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "start": "next start",
    "lint": "next lint"
  },
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "next": "^14.0.0"
  },
  "devDependencies": {
    "@types/node": "^20.0.0",
    "@types/react": "^18.0.0",
    "@types/react-dom": "^18.0.0",
    "typescript": "^5.0.0",
    "tailwindcss": "^3.0.0",
    "postcss": "^8.0.0",
    "autoprefixer": "^10.0.0"
  }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-015-web-api-layer-axum-nextjs.md`** — Static export, 3s polling, 5 tabs, dev proxy
- **`docs/adr/ADR-004-rust-native-orchestration-typescript-scope.md`** — TypeScript scope (web only)

---

## Acceptance Criteria

- ✅ `npm run build` → no TypeScript errors
- ✅ `npm run dev` → form submission calls `POST /assessments`
- ✅ With running server + completed assessment, viewer renders all 5 tabs
- ✅ `next export` → static `out/`; server serves with `--static-dir out/`
- ✅ No console errors

---

## Language

**TypeScript** — React, Next.js, client-side API integration.

---

## Out-of-Scope

- Do NOT implement real-time updates (polling is MVP)
- Do NOT implement user authentication beyond API key


---

# P17 — End-to-End Integration Tests + Repomix Small-Repo Path

## Context

**You are implementing end-to-end tests and the small-repo optimization path using repomix.**

**Prerequisites:** P14 (CLI), P15 (server), P16 (web) are complete.

---

## Phase & Dependencies

- **Phase:** Hardening
- **Depends on:** P14, P15, P16

---

## Scope & Deliverables

### File: `tests/integration/e2e_pipeline.rs`

```rust
#[tokio::test]
async fn test_full_pipeline_with_mock() {
    // Create a mock small local repo (use a git fixture)
    let repo_path = Path::new("tests/fixtures/sample-repo");
    
    let runner = PipelineRunner::new(
        Box::new(MockSessionRunner::with_canned_responses(
            vec![
                "module_assessment_1.json",
                "module_assessment_2.json",
                "synthesis_output.json",
                "risk_output.json",
            ]
        )),
        Box::new(InMemoryCheckpointStore::new()),
        Box::new(InMemoryAssessmentJobStore::new()),
    );
    
    let output = runner.run(
        "https://github.com/example/test-repo",
        20_000_000,  // 20M tokens
        &ScoreWeights::default(),
    ).await.expect("pipeline should complete");
    
    assert!(output.is_complete);
    assert!(!output.arch_map.modules.is_empty());
    assert!(!output.valuation.module_scores.is_empty());
    assert!(output.strategy.tier_assignments.len() > 0);
}

#[tokio::test]
async fn test_crash_recovery() {
    let runner = PipelineRunner::new(
        Box::new(MockSessionRunner::default()),
        Box::new(InMemoryCheckpointStore::new()),
        Box::new(InMemoryAssessmentJobStore::new()),
    );
    
    // Simulate crash after module 2 of 5
    let checkpoint = JobCheckpoint {
        job_id: "test-job".to_string(),
        last_completed_phase: Some(PhaseKind::FeatureDiscovery),
        completed_module_ids: vec!["mod1".to_string(), "mod2".to_string()],
        token_usage_so_far: 500_000,
        partial_results: serde_json::json!({}),
        saved_at: chrono::Utc::now().to_rfc3339(),
    };
    
    let phases_to_run = phases_to_run(&checkpoint);
    
    // Should resume from next phase
    assert!(phases_to_run.contains(&PhaseKind::Scoring));
    assert!(!phases_to_run.contains(&PhaseKind::FeatureDiscovery));
}

#[test]
fn test_repomix_single_session_path() {
    let manifest = RepoManifest {
        total_loc: 30_000,  // < 50k
        // ...
    };
    
    assert!(should_use_repomix(&manifest));
}

fn should_use_repomix(manifest: &RepoManifest) -> bool {
    manifest.total_loc < 50_000
}
```

### File: `src/pipeline/feature_discovery.rs` — Repomix Integration

```rust
pub async fn run_feature_discovery_phase_with_repomix(
    arch_map: &ArchitectureMap,
    repo_path: &Path,
    session_runner: impl SessionRunner,
    module_store: &dyn ModuleAssessmentStore,
) -> Result<FunctionalityInventory, OrchestratorError> {
    let total_loc = compute_total_loc(repo_path);
    
    if total_loc < 50_000 {
        // Single-session repomix path
        run_single_session_analysis(repo_path, session_runner).await
    } else {
        // Standard fan-out path
        run_fan_out_analysis(arch_map, repo_path, session_runner, module_store).await
    }
}

async fn run_single_session_analysis(
    repo_path: &Path,
    session_runner: impl SessionRunner,
) -> Result<FunctionalityInventory, OrchestratorError> {
    // Run: repomix --output-format xml <path>
    let output = tokio::process::Command::new("repomix")
        .arg("--output-format").arg("xml")
        .arg(repo_path)
        .output()
        .await;
    
    match output {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Fallback to fan-out if repomix not installed
            return Err(OrchestratorError("repomix not found, falling back".into()));
        }
        Err(e) => return Err(OrchestratorError(format!("repomix error: {}", e))),
        Ok(out) => {
            if !out.status.success() {
                return Err(OrchestratorError("repomix failed".into()));
            }
            
            let xml_content = String::from_utf8(out.stdout)?;
            
            // Single Claude session over full repo
            let prompt = format!(
                "Analyze this repository XML output. Identify all capabilities.\nReturn ModuleAssessment schema with module_name: 'all'.\n\n{}",
                xml_content
            );
            
            let result = session_runner.run(
                ClaudeInvocation {
                    prompt,
                    model: ClaudeModel::Sonnet,
                    schema_path: Some("module_assessment_schema.json".into()),
                    allowed_tools: vec![],
                    system_prompt: None,
                    working_dir: Some(repo_path.to_path_buf()),
                    session_id: None,
                }
            ).await?;
            
            let assessment: ModuleAssessment = serde_json::from_str(&result.output)?;
            
            Ok(FunctionalityInventory {
                repo_id: uuid::Uuid::new_v4().to_string(),
                items: map_to_functionality_items(&assessment, ""),
                total_count: 1,
                hidden_count: 0,
                enterprise_count: 0,
                api_entry_points: vec![],
            })
        }
    }
}

fn compute_total_loc(repo_path: &Path) -> usize {
    // Use tokei to aggregate LOC
    0  // Placeholder
}
```

### File: `.github/workflows/e2e.yml`

```yaml
name: E2E Tests

on:
  push:
    branches: [main]

jobs:
  e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      
      - name: Build CLI
        run: cargo build -p repogate-cli --release
      
      - name: Run E2E with mock sessions
        env:
          REPOGATE_MOCK_SESSIONS: "true"
        run: |
          ./target/release/repogate analyze \
            https://github.com/BurntSushi/toml \
            --budget 0.50 \
            --yes \
            --output json \
            --output-file assessment.json
      
      - name: Validate JSON output
        run: |
          jq '.schema_version' assessment.json | grep -q '"1.0"'
      
      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: assessment
          path: assessment.json
```

### Directory: `tests/fixtures/`

- `canned_module_assessment.json` — Sample ModuleAssessment response
- `canned_synthesis_output.json` — Sample SynthesisOutput response
- `canned_risk_output.json` — Sample RiskAnalysisOutput response
- `dev.db` — SQLite fixture (created by P01)

---

## Source Documents to Read

- **`docs/adr/ADR-008-deep-traversal-map-reduce.md`** — Repomix small-repo path (<50k LOC single-session)
- **`docs/adr/ADR-009-multi-phase-pipeline-crash-recovery.md`** — Crash recovery testing
- **`docs/adr/ADR-013-token-budget-enforcement.md`** — Partial results on budget exhaustion

---

## Acceptance Criteria

- ✅ `cargo test -p repogate-orchestrator --test e2e_pipeline` passes (0 live API calls)
- ✅ Crash recovery: resume after crash at module 2 → only modules 3–5 analyzed (3 sessions)
- ✅ Repomix path: <50k LOC manifest → exactly 1 Claude session
- ✅ `repogate analyze <small-repo> --budget 0.50 --yes` with `REPOGATE_MOCK_SESSIONS=true` → exit 0 + JSON
- ✅ All CI jobs pass on clean checkout

---

## Language

**Rust** (tests), **YAML** (workflow).

---

## Out-of-Scope

- Do NOT implement real live API e2e tests (use mocks)
- Do NOT implement performance benchmarking


---

