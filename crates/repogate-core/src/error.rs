use thiserror::Error;

/// Top-level error enum for the RepoGate platform.
#[allow(clippy::enum_variant_names)]
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

/// Returned when a `Score` value is outside `[0.0, 10.0]`.
#[derive(Debug, Clone, Error)]
#[error("score {value} is outside valid range [0.0, 10.0]")]
pub struct ScoreRangeError {
    pub value: f32,
}

/// Returned when `ScoreWeights` field values violate invariants.
#[derive(Debug, Clone, Error)]
#[error("weight validation failed: {0}")]
pub struct WeightError(pub String);

/// Returned when structured output from Claude fails schema validation.
#[derive(Debug, Clone, Error)]
#[error("schema violation: {0}")]
pub struct SchemaViolationError(pub String);

/// Returned on persistence failures.
#[derive(Debug, Clone, Error)]
#[error("store error: {0}")]
pub struct StoreError(pub String);

/// Returned on orchestrator failures.
#[derive(Debug, Clone, Error)]
#[error("orchestrator error: {0}")]
pub struct OrchestratorError(pub String);
