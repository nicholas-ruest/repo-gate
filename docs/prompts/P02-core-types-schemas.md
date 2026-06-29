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
