use crate::error::{ScoreRangeError, WeightError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Recommended open-core tier for a repository module.
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

/// Risk severity classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub enum Severity {
    Low,
    Medium,
    High,
}

/// Kind of gating or compliance risk identified for a module.
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

/// Architectural layer a module belongs to.
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

/// Primary programming language of a module or repository.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub enum Language {
    Rust,
    TypeScript,
    Python,
    Go,
    Java,
    CSharp,
    Ruby,
    Other(String),
}

/// A validated score in `[0.0, 10.0]`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
pub struct Score(f32);

impl Score {
    /// Construct a new `Score`, returning [`ScoreRangeError`] if `v` is outside `[0.0, 10.0]`.
    pub fn new(v: f32) -> Result<Self, ScoreRangeError> {
        if !(0.0..=10.0).contains(&v) {
            Err(ScoreRangeError { value: v })
        } else {
            Ok(Score(v))
        }
    }

    /// Return the inner `f32` value.
    pub fn get(&self) -> f32 {
        self.0
    }
}

/// Weighted composite of the 8 commercial scoring dimensions, normalised to `[0.0, 10.0]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CompositeScore(f32);

/// Eight-dimension commercial value score for a single module.
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

/// Per-dimension weights used to compute [`CompositeScore`].
///
/// All weights must be `>= 0.0` except `support_burden`, which may be negative
/// because high support burden *reduces* commercial attractiveness.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScoreWeights {
    pub adoption_value: f32,
    pub enterprise_buyer_value: f32,
    pub commercial_leverage: f32,
    pub competitive_sensitivity: f32,
    pub operational_value: f32,
    pub security_sensitivity: f32,
    /// Negative: higher burden reduces the composite score.
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
            support_burden: -0.6,
            strategic_importance: 1.0,
        }
    }
}

impl ScoreWeights {
    /// Construct and validate `ScoreWeights`.
    ///
    /// All fields except `support_burden` must be `>= 0.0`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        adoption_value: f32,
        enterprise_buyer_value: f32,
        commercial_leverage: f32,
        competitive_sensitivity: f32,
        operational_value: f32,
        security_sensitivity: f32,
        support_burden: f32,
        strategic_importance: f32,
    ) -> Result<Self, WeightError> {
        if adoption_value < 0.0
            || enterprise_buyer_value < 0.0
            || commercial_leverage < 0.0
            || competitive_sensitivity < 0.0
            || operational_value < 0.0
            || security_sensitivity < 0.0
            || strategic_importance < 0.0
        {
            return Err(WeightError(
                "all weights except support_burden must be >= 0.0".to_string(),
            ));
        }
        Ok(Self {
            adoption_value,
            enterprise_buyer_value,
            commercial_leverage,
            competitive_sensitivity,
            operational_value,
            security_sensitivity,
            support_burden,
            strategic_importance,
        })
    }
}

/// Derived signal indicating whether a module is a candidate for gating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum GatingSignal {
    StrongGateCandidate,
    WeakGateCandidate,
    OpenCandidate,
    Undetermined,
}

/// Maximum allowed token consumption for an assessment job and its phases/sessions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TokenBudget {
    /// Maximum tokens for the entire job.
    pub total_limit: u64,
    /// Maximum tokens per pipeline phase.
    pub per_phase_limit: u64,
    /// Maximum tokens per individual Claude Code session.
    pub per_session_limit: u64,
    /// Fraction `[0.0, 1.0]` at which a budget warning is emitted (e.g. `0.8`).
    pub warn_threshold: f32,
}

impl TokenBudget {
    /// Returns `true` when `used` has reached or exceeded [`total_limit`](Self::total_limit).
    pub fn is_exceeded(&self, used: u64) -> bool {
        used >= self.total_limit
    }

    /// Returns tokens remaining before [`total_limit`](Self::total_limit) is hit.
    pub fn remaining(&self, used: u64) -> u64 {
        self.total_limit.saturating_sub(used)
    }
}
