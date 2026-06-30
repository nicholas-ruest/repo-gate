//! sqlx-backed [`ModuleAssessmentStore`], keyed by `(job_id, module_name)`.

use repogate_core::ModuleAssessment;
use repogate_orchestrator::job::{ModuleAssessmentStore, StoreError};
use sqlx::{Row, SqlitePool};

fn backend(e: impl std::fmt::Display) -> StoreError {
    StoreError::Backend(e.to_string())
}

/// Persists per-module assessments in the `module_assessments` table.
pub struct SqlxModuleAssessmentStore {
    pool: SqlitePool,
}

impl SqlxModuleAssessmentStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl ModuleAssessmentStore for SqlxModuleAssessmentStore {
    async fn save(&self, job_id: &str, assessment: ModuleAssessment) -> Result<(), StoreError> {
        let json = serde_json::to_string(&assessment).map_err(backend)?;
        sqlx::query(
            r#"INSERT INTO module_assessments (id, job_id, module_id, module_name, assessment_json)
               VALUES (?1, ?2, ?3, ?4, ?5)
               ON CONFLICT(job_id, module_id) DO UPDATE SET
                 assessment_json = excluded.assessment_json"#,
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(job_id)
        .bind(&assessment.module_name)
        .bind(&assessment.module_name)
        .bind(json)
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn find_by_module(
        &self,
        job_id: &str,
        module_id: &str,
    ) -> Result<Option<ModuleAssessment>, StoreError> {
        let row = sqlx::query(
            "SELECT assessment_json FROM module_assessments WHERE job_id = ?1 AND module_id = ?2",
        )
        .bind(job_id)
        .bind(module_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        match row {
            Some(r) => {
                let json: String = r.try_get("assessment_json").map_err(backend)?;
                Ok(Some(serde_json::from_str(&json).map_err(backend)?))
            }
            None => Ok(None),
        }
    }

    async fn exists(&self, job_id: &str, module_id: &str) -> Result<bool, StoreError> {
        let row =
            sqlx::query("SELECT 1 FROM module_assessments WHERE job_id = ?1 AND module_id = ?2")
                .bind(job_id)
                .bind(module_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(backend)?;
        Ok(row.is_some())
    }

    async fn load_all(&self, job_id: &str) -> Result<Vec<ModuleAssessment>, StoreError> {
        let rows = sqlx::query("SELECT assessment_json FROM module_assessments WHERE job_id = ?1")
            .bind(job_id)
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        rows.into_iter()
            .map(|r| {
                let json: String = r.try_get("assessment_json").map_err(backend)?;
                serde_json::from_str(&json).map_err(backend)
            })
            .collect()
    }
}
