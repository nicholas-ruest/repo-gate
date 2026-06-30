//! In-memory job and module-assessment stores (sqlx-backed stores land in P13).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use repogate_core::ModuleAssessment;

use super::state::{AssessmentJob, JobStatus};
use super::StoreError;

/// Persistence boundary for assessment jobs.
#[allow(async_fn_in_trait)]
pub trait AssessmentJobStore: Send + Sync {
    async fn save(&self, job: AssessmentJob) -> Result<(), StoreError>;
    async fn find_by_id(&self, job_id: &str) -> Result<Option<AssessmentJob>, StoreError>;
    async fn find_by_status(&self, status: JobStatus) -> Result<Vec<AssessmentJob>, StoreError>;
    async fn find_concurrent_for_repo(
        &self,
        repo_url: &str,
    ) -> Result<Vec<AssessmentJob>, StoreError>;
}

/// Persistence boundary for per-module assessments, keyed by `module_name`.
#[allow(async_fn_in_trait)]
pub trait ModuleAssessmentStore: Send + Sync {
    async fn save(&self, job_id: &str, assessment: ModuleAssessment) -> Result<(), StoreError>;
    async fn find_by_module(
        &self,
        job_id: &str,
        module_id: &str,
    ) -> Result<Option<ModuleAssessment>, StoreError>;
    async fn exists(&self, job_id: &str, module_id: &str) -> Result<bool, StoreError>;
    async fn load_all(&self, job_id: &str) -> Result<Vec<ModuleAssessment>, StoreError>;
}

/// In-memory [`AssessmentJobStore`].
#[derive(Default)]
pub struct InMemoryAssessmentJobStore {
    jobs: Arc<Mutex<HashMap<String, AssessmentJob>>>,
}

impl InMemoryAssessmentJobStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl AssessmentJobStore for InMemoryAssessmentJobStore {
    async fn save(&self, job: AssessmentJob) -> Result<(), StoreError> {
        self.jobs
            .lock()
            .map_err(|_| StoreError::Lock)?
            .insert(job.id.clone(), job);
        Ok(())
    }

    async fn find_by_id(&self, job_id: &str) -> Result<Option<AssessmentJob>, StoreError> {
        Ok(self
            .jobs
            .lock()
            .map_err(|_| StoreError::Lock)?
            .get(job_id)
            .cloned())
    }

    async fn find_by_status(&self, status: JobStatus) -> Result<Vec<AssessmentJob>, StoreError> {
        Ok(self
            .jobs
            .lock()
            .map_err(|_| StoreError::Lock)?
            .values()
            .filter(|j| j.status == status)
            .cloned()
            .collect())
    }

    async fn find_concurrent_for_repo(
        &self,
        repo_url: &str,
    ) -> Result<Vec<AssessmentJob>, StoreError> {
        Ok(self
            .jobs
            .lock()
            .map_err(|_| StoreError::Lock)?
            .values()
            .filter(|j| {
                j.repo_url == repo_url
                    && !matches!(j.status, JobStatus::Complete | JobStatus::Failed)
            })
            .cloned()
            .collect())
    }
}

/// In-memory [`ModuleAssessmentStore`], keyed by `(job_id, module_name)`.
#[derive(Default)]
pub struct InMemoryModuleAssessmentStore {
    assessments: Arc<Mutex<HashMap<(String, String), ModuleAssessment>>>,
}

impl InMemoryModuleAssessmentStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ModuleAssessmentStore for InMemoryModuleAssessmentStore {
    async fn save(&self, job_id: &str, assessment: ModuleAssessment) -> Result<(), StoreError> {
        let key = (job_id.to_string(), assessment.module_name.clone());
        self.assessments
            .lock()
            .map_err(|_| StoreError::Lock)?
            .insert(key, assessment);
        Ok(())
    }

    async fn find_by_module(
        &self,
        job_id: &str,
        module_id: &str,
    ) -> Result<Option<ModuleAssessment>, StoreError> {
        Ok(self
            .assessments
            .lock()
            .map_err(|_| StoreError::Lock)?
            .get(&(job_id.to_string(), module_id.to_string()))
            .cloned())
    }

    async fn exists(&self, job_id: &str, module_id: &str) -> Result<bool, StoreError> {
        Ok(self
            .assessments
            .lock()
            .map_err(|_| StoreError::Lock)?
            .contains_key(&(job_id.to_string(), module_id.to_string())))
    }

    async fn load_all(&self, job_id: &str) -> Result<Vec<ModuleAssessment>, StoreError> {
        Ok(self
            .assessments
            .lock()
            .map_err(|_| StoreError::Lock)?
            .iter()
            .filter(|((jid, _), _)| jid == job_id)
            .map(|(_, a)| a.clone())
            .collect())
    }
}
