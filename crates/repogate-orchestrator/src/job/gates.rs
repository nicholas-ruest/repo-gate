//! Phase-gate preconditions enforced between pipeline phases (ADR-009).

use super::state::{AssessmentJob, PhaseKind};

/// Error raised when a phase-gate precondition is not met.
#[derive(Debug, thiserror::Error)]
pub enum PhaseGateError {
    #[error("phase gate failed: {0}")]
    Precondition(String),
}

/// Validate that the job may advance out of `from_phase`.
///
/// - Leaving `Ingestion` requires a non-empty manifest.
/// - Leaving `FeatureDiscovery` requires every expected module to have a stored
///   assessment.
pub fn validate_gate(from_phase: PhaseKind, state: &AssessmentJob) -> Result<(), PhaseGateError> {
    match from_phase {
        PhaseKind::Ingestion => {
            let has_files = state
                .manifest
                .as_ref()
                .map(|m| m.total_files > 0)
                .unwrap_or(false);
            if has_files {
                Ok(())
            } else {
                Err(PhaseGateError::Precondition(
                    "manifest is empty or missing".into(),
                ))
            }
        }
        PhaseKind::FeatureDiscovery => {
            if state.all_modules_assessed() {
                Ok(())
            } else {
                Err(PhaseGateError::Precondition(
                    "not all modules have a stored assessment".into(),
                ))
            }
        }
        _ => Ok(()),
    }
}
