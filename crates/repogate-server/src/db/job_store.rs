//! sqlx-backed [`AssessmentJobStore`], storing each job as a JSON blob.

use repogate_orchestrator::job::{AssessmentJob, AssessmentJobStore, JobStatus, StoreError};
use sqlx::{Row, SqlitePool};

fn backend(e: impl std::fmt::Display) -> StoreError {
    StoreError::Backend(e.to_string())
}

/// Persists [`AssessmentJob`]s in the `jobs` table.
pub struct SqlxAssessmentJobStore {
    pool: SqlitePool,
}

impl SqlxAssessmentJobStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn status_label(status: JobStatus) -> String {
    format!("{status:?}")
}

impl AssessmentJobStore for SqlxAssessmentJobStore {
    async fn save(&self, job: AssessmentJob) -> Result<(), StoreError> {
        let json = serde_json::to_string(&job).map_err(backend)?;
        sqlx::query(
            r#"INSERT INTO jobs (id, repo_url, status, job_json)
               VALUES (?1, ?2, ?3, ?4)
               ON CONFLICT(id) DO UPDATE SET
                 repo_url = excluded.repo_url,
                 status = excluded.status,
                 job_json = excluded.job_json,
                 updated_at = CURRENT_TIMESTAMP"#,
        )
        .bind(&job.id)
        .bind(&job.repo_url)
        .bind(status_label(job.status))
        .bind(json)
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn find_by_id(&self, job_id: &str) -> Result<Option<AssessmentJob>, StoreError> {
        let row = sqlx::query("SELECT job_json FROM jobs WHERE id = ?1")
            .bind(job_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        decode_optional(row)
    }

    async fn find_by_status(&self, status: JobStatus) -> Result<Vec<AssessmentJob>, StoreError> {
        let rows = sqlx::query("SELECT job_json FROM jobs WHERE status = ?1")
            .bind(status_label(status))
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        decode_all(rows)
    }

    async fn find_concurrent_for_repo(
        &self,
        repo_url: &str,
    ) -> Result<Vec<AssessmentJob>, StoreError> {
        let rows = sqlx::query(
            "SELECT job_json FROM jobs WHERE repo_url = ?1 AND status NOT IN ('Complete', 'Failed')",
        )
        .bind(repo_url)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        decode_all(rows)
    }
}

fn decode_optional(
    row: Option<sqlx::sqlite::SqliteRow>,
) -> Result<Option<AssessmentJob>, StoreError> {
    match row {
        Some(r) => {
            let json: String = r.try_get("job_json").map_err(backend)?;
            Ok(Some(serde_json::from_str(&json).map_err(backend)?))
        }
        None => Ok(None),
    }
}

fn decode_all(rows: Vec<sqlx::sqlite::SqliteRow>) -> Result<Vec<AssessmentJob>, StoreError> {
    rows.into_iter()
        .map(|r| {
            let json: String = r.try_get("job_json").map_err(backend)?;
            serde_json::from_str(&json).map_err(backend)
        })
        .collect()
}
