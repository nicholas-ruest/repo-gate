//! Crash-recovery checkpoints and phase-resumption logic (ADR-009).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::state::PhaseKind;
use super::StoreError;

/// A durable snapshot of a job's progress, used to resume after a crash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCheckpoint {
    pub job_id: String,
    pub last_completed_phase: Option<PhaseKind>,
    pub completed_module_ids: Vec<String>,
    pub token_usage_so_far: u64,
    pub partial_results: serde_json::Value,
    pub saved_at: String,
}

/// Persistence boundary for job checkpoints.
#[allow(async_fn_in_trait)]
pub trait CheckpointStore: Send + Sync {
    async fn save(&self, checkpoint: JobCheckpoint) -> Result<(), StoreError>;
    async fn load(&self, job_id: &str) -> Result<Option<JobCheckpoint>, StoreError>;
}

/// In-memory [`CheckpointStore`] for the CLI path and tests.
#[derive(Default)]
pub struct InMemoryCheckpointStore {
    checkpoints: Arc<Mutex<HashMap<String, JobCheckpoint>>>,
}

impl InMemoryCheckpointStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CheckpointStore for InMemoryCheckpointStore {
    async fn save(&self, checkpoint: JobCheckpoint) -> Result<(), StoreError> {
        self.checkpoints
            .lock()
            .map_err(|_| StoreError::Lock)?
            .insert(checkpoint.job_id.clone(), checkpoint);
        Ok(())
    }

    async fn load(&self, job_id: &str) -> Result<Option<JobCheckpoint>, StoreError> {
        Ok(self
            .checkpoints
            .lock()
            .map_err(|_| StoreError::Lock)?
            .get(job_id)
            .cloned())
    }
}

/// The canonical ordered phase sequence of the pipeline.
pub fn all_phases() -> Vec<PhaseKind> {
    vec![
        PhaseKind::Ingestion,
        PhaseKind::LicenseScan,
        PhaseKind::ArchitectureMapping,
        PhaseKind::FeatureDiscovery,
        PhaseKind::Scoring,
        PhaseKind::Strategy,
        PhaseKind::RiskAnalysis,
        PhaseKind::ReportAssembly,
    ]
}

/// Phases that still need to run given a checkpoint — everything after the last
/// completed phase.
pub fn phases_to_run(checkpoint: &JobCheckpoint) -> Vec<PhaseKind> {
    let phases = all_phases();
    match checkpoint.last_completed_phase {
        Some(last) => phases
            .into_iter()
            .skip_while(|p| *p != last)
            .skip(1)
            .collect(),
        None => phases,
    }
}
