use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::{CommercialScore, GatingTier, Layer, RiskKind, Severity};

/// A repository submitted for assessment.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Repository {
    pub id: String,
    pub url: String,
    pub name: String,
    pub description: Option<String>,
    pub license: Option<String>,
    pub metrics: RepositoryMetrics,
}

/// High-level file and language statistics for a repository.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RepositoryMetrics {
    pub total_files: usize,
    pub total_loc: usize,
    /// Map of language name → lines of code.
    pub language_stats: HashMap<String, usize>,
}

/// A logical module (crate, package, directory cluster) within a repository.
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

/// Access classification of a discovered capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Internal,
    Experimental,
    Undocumented,
    Enterprise,
}

/// A discrete capability exposed by a module.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Capability {
    pub name: String,
    pub description: Option<String>,
    pub visibility: Visibility,
    pub affects_tiers: Vec<GatingTier>,
}

/// A pointer to a specific file (and optionally line range) in the repository.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SourceLocation {
    pub file_path: String,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
}

/// A gating or compliance risk identified for a module.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Risk {
    pub kind: RiskKind,
    pub severity: Severity,
    pub description: String,
    pub mitigation: Option<String>,
    pub is_blocking: bool,
}

/// The complete assessment of one repository.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Assessment {
    pub repo_id: String,
    /// Canonical output schema version; currently `"1.0"`.
    pub schema_version: String,
    /// ISO 8601 timestamp of generation.
    pub generated_at: String,
    pub is_complete: bool,
    pub repository: Repository,
    pub modules: Vec<Module>,
    pub gating_strategy: Option<GatingStrategy>,
    pub risks: Vec<Risk>,
    /// Which parts of the analysis ran degraded vs. fully (ADR-016 Remediation 4).
    #[serde(default)]
    pub completeness: Option<CompletenessMetadata>,
}

/// Records where an analysis run degraded, so a consumer can distinguish a deep
/// result from a partial/fallback one (ADR-016 Remediation 4).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct CompletenessMetadata {
    /// Modules whose schema-constrained discovery session failed (after retry).
    pub degraded_modules: Vec<String>,
    /// Modules skipped because the token budget was exhausted.
    pub budget_skipped_modules: Vec<String>,
    /// Whether license detection used the heuristic fallback (askalono unavailable).
    pub license_detection_degraded: bool,
    /// Modules scored with the uniform fallback instead of real per-dimension scores.
    pub scoring_degraded_modules: Vec<String>,
}

impl CompletenessMetadata {
    /// True when no completeness-blocking degradation occurred. License-detection
    /// degradation is informational and does not block completeness.
    pub fn is_complete(&self) -> bool {
        self.degraded_modules.is_empty()
            && self.budget_skipped_modules.is_empty()
            && self.scoring_degraded_modules.is_empty()
    }
}

/// Recommended tier assignments across all modules in the repository.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GatingStrategy {
    pub tier_assignments: Vec<TierAssignment>,
    pub boundary_description: Option<String>,
}

/// Tier recommendation for a single module.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TierAssignment {
    pub module_id: String,
    pub module_name: String,
    pub tier: GatingTier,
    pub rationale: Option<String>,
}
