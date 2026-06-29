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
