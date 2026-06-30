//! Durable job state machine, token budgeting, checkpoints, stores, and phase gates.

pub mod budget;
pub mod checkpoint;
pub mod gates;
pub mod state;
pub mod store;

pub use budget::{BudgetStatus, BudgetTracker};
pub use checkpoint::{
    all_phases, phases_to_run, CheckpointStore, InMemoryCheckpointStore, JobCheckpoint,
};
pub use gates::{validate_gate, PhaseGateError};
pub use state::{AnalysisPhase, AssessmentJob, JobStatus, PhaseKind};
pub use store::{
    AssessmentJobStore, InMemoryAssessmentJobStore, InMemoryModuleAssessmentStore,
    ModuleAssessmentStore,
};

/// Errors produced by job stores.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("store lock poisoned")]
    Lock,

    #[error("store backend error: {0}")]
    Backend(String),
}
