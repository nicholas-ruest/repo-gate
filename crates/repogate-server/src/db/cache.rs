//! Analysis cache keyed by `(repo_url, commit_sha)` with a TTL (ADR-014).

use repogate_core::Assessment;
use repogate_orchestrator::job::StoreError;
use sqlx::{Row, SqlitePool};

fn backend(e: impl std::fmt::Display) -> StoreError {
    StoreError::Backend(e.to_string())
}

/// Caches completed assessments to skip re-analysis of unchanged repositories.
pub struct AnalysisCacheStore {
    pool: SqlitePool,
}

impl AnalysisCacheStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Fetch a cached assessment if present and not past its TTL.
    pub async fn get(
        &self,
        repo_url: &str,
        commit_sha: &str,
    ) -> Result<Option<Assessment>, StoreError> {
        let row = sqlx::query(
            "SELECT cached_assessment FROM analysis_cache \
             WHERE repo_url = ?1 AND commit_sha = ?2 \
             AND created_at > datetime('now', '-' || ttl_days || ' days')",
        )
        .bind(repo_url)
        .bind(commit_sha)
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;

        match row {
            Some(r) => {
                let json: String = r.try_get("cached_assessment").map_err(backend)?;
                Ok(serde_json::from_str(&json).ok())
            }
            None => Ok(None),
        }
    }

    /// Store an assessment in the cache with a `ttl_days` lifetime.
    pub async fn set(
        &self,
        repo_url: &str,
        commit_sha: &str,
        assessment: &Assessment,
        ttl_days: i64,
    ) -> Result<(), StoreError> {
        let json = serde_json::to_string(assessment).map_err(backend)?;
        sqlx::query(
            r#"INSERT INTO analysis_cache (repo_url, commit_sha, cached_assessment, ttl_days)
               VALUES (?1, ?2, ?3, ?4)
               ON CONFLICT(repo_url, commit_sha) DO UPDATE SET
                 cached_assessment = excluded.cached_assessment,
                 ttl_days = excluded.ttl_days,
                 created_at = CURRENT_TIMESTAMP"#,
        )
        .bind(repo_url)
        .bind(commit_sha)
        .bind(json)
        .bind(ttl_days)
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    /// Remove all cache entries for a repository URL.
    pub async fn invalidate(&self, repo_url: &str) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM analysis_cache WHERE repo_url = ?1")
            .bind(repo_url)
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }
}
