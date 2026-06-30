//! Job and phase state for the assessment pipeline (ADR-009).

use repogate_core::ModuleAssessment;
use repogate_ingestion::RepoManifest;
use serde::{Deserialize, Serialize};

/// Overall status of an assessment job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Ingesting,
    Analyzing,
    Scoring,
    Strategizing,
    RiskAnalyzing,
    Reporting,
    Complete,
    Failed,
    Recovering,
}

/// A discrete unit of pipeline work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseKind {
    Ingestion,
    LicenseScan,
    ArchitectureMapping,
    FeatureDiscovery,
    Scoring,
    Strategy,
    RiskAnalysis,
    ReportAssembly,
    ManifestSummarization,
    Synthesis,
}

/// Execution record for one phase of a job.
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

/// The durable aggregate root for a single repository assessment run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentJob {
    pub id: String,
    pub repo_url: String,
    pub status: JobStatus,
    pub current_phase: Option<PhaseKind>,
    pub tokens_used: u64,
    pub created_at: String,
    pub updated_at: String,
    pub error: Option<String>,
    /// The ingestion manifest, once produced.
    pub manifest: Option<RepoManifest>,
    /// Module identifiers (names) expected to be assessed.
    pub module_ids: Vec<String>,
    /// Stored per-module assessments, keyed implicitly by `module_name`.
    pub module_assessments: Vec<ModuleAssessment>,
}

impl AssessmentJob {
    /// Create a fresh queued job for `repo_url`.
    pub fn new(repo_url: &str, now: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            repo_url: repo_url.to_string(),
            status: JobStatus::Queued,
            current_phase: None,
            tokens_used: 0,
            created_at: now.to_string(),
            updated_at: now.to_string(),
            error: None,
            manifest: None,
            module_ids: Vec::new(),
            module_assessments: Vec::new(),
        }
    }

    /// True when every expected module has a stored assessment.
    pub fn all_modules_assessed(&self) -> bool {
        self.module_ids
            .iter()
            .all(|id| self.module_assessments.iter().any(|a| &a.module_name == id))
    }
}
