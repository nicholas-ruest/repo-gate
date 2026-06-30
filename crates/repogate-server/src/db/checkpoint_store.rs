//! sqlx-backed [`CheckpointStore`].

use repogate_orchestrator::job::{CheckpointStore, JobCheckpoint, StoreError};
use sqlx::{Row, SqlitePool};

fn backend(e: impl std::fmt::Display) -> StoreError {
    StoreError::Backend(e.to_string())
}

/// Persists job checkpoints in the `checkpoints` table.
pub struct SqlxCheckpointStore {
    pool: SqlitePool,
}

impl SqlxCheckpointStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl CheckpointStore for SqlxCheckpointStore {
    async fn save(&self, checkpoint: JobCheckpoint) -> Result<(), StoreError> {
        let json = serde_json::to_string(&checkpoint).map_err(backend)?;
        sqlx::query(
            r#"INSERT INTO checkpoints (job_id, checkpoint_json)
               VALUES (?1, ?2)
               ON CONFLICT(job_id) DO UPDATE SET
                 checkpoint_json = excluded.checkpoint_json,
                 saved_at = CURRENT_TIMESTAMP"#,
        )
        .bind(&checkpoint.job_id)
        .bind(json)
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn load(&self, job_id: &str) -> Result<Option<JobCheckpoint>, StoreError> {
        let row = sqlx::query("SELECT checkpoint_json FROM checkpoints WHERE job_id = ?1")
            .bind(job_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        match row {
            Some(r) => {
                let json: String = r.try_get("checkpoint_json").map_err(backend)?;
                Ok(Some(serde_json::from_str(&json).map_err(backend)?))
            }
            None => Ok(None),
        }
    }
}
