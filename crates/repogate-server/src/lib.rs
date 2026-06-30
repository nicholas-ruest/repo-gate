#![doc = "RepoGate server library: persistence and the HTTP API."]

pub mod db;
pub mod http;

#[cfg(test)]
mod tests {
    use super::db::*;
    use repogate_orchestrator::job::{
        AssessmentJob, AssessmentJobStore, CheckpointStore, JobCheckpoint, JobStatus,
        ModuleAssessmentStore, PhaseKind,
    };

    async fn temp_pool() -> (tempfile::TempDir, sqlx::SqlitePool) {
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}/test.db", dir.path().display());
        let pool = create_pool(&url).await.unwrap();
        (dir, pool)
    }

    fn module_assessment(name: &str) -> repogate_core::ModuleAssessment {
        repogate_core::ModuleAssessment {
            module_name: name.to_string(),
            module_path: format!("src/{name}"),
            capabilities: vec![],
            commercial_score: None,
            commercial_value_estimate: Some(6.0),
            estimated_tier: None,
            risks: vec![],
        }
    }

    fn minimal_assessment() -> repogate_core::Assessment {
        repogate_core::Assessment {
            repo_id: "r1".to_string(),
            schema_version: "1.0".to_string(),
            generated_at: "2026-06-30T00:00:00Z".to_string(),
            is_complete: true,
            repository: repogate_core::Repository {
                id: "r1".to_string(),
                url: "https://github.com/acme/x".to_string(),
                name: "x".to_string(),
                description: None,
                license: None,
                metrics: repogate_core::RepositoryMetrics {
                    total_files: 1,
                    total_loc: 1,
                    language_stats: std::collections::HashMap::new(),
                },
            },
            modules: vec![],
            gating_strategy: None,
            risks: vec![],
            completeness: None,
        }
    }

    #[tokio::test]
    async fn migrations_run_clean() {
        let (_dir, _pool) = temp_pool().await;
    }

    #[tokio::test]
    async fn job_store_round_trip() {
        let (_dir, pool) = temp_pool().await;
        let store = SqlxAssessmentJobStore::new(pool);
        let job = AssessmentJob::new("https://github.com/acme/x", "2026-06-30T00:00:00Z");
        let id = job.id.clone();
        store.save(job).await.unwrap();

        let loaded = store.find_by_id(&id).await.unwrap().unwrap();
        assert_eq!(loaded.repo_url, "https://github.com/acme/x");

        let queued = store.find_by_status(JobStatus::Queued).await.unwrap();
        assert_eq!(queued.len(), 1);

        let concurrent = store
            .find_concurrent_for_repo("https://github.com/acme/x")
            .await
            .unwrap();
        assert_eq!(concurrent.len(), 1);
    }

    #[tokio::test]
    async fn module_store_round_trip() {
        let (_dir, pool) = temp_pool().await;
        let store = SqlxModuleAssessmentStore::new(pool);
        store.save("job1", module_assessment("auth")).await.unwrap();
        // Saving again upserts, not duplicates.
        store.save("job1", module_assessment("auth")).await.unwrap();

        assert!(store.exists("job1", "auth").await.unwrap());
        assert!(!store.exists("job1", "billing").await.unwrap());
        assert_eq!(store.load_all("job1").await.unwrap().len(), 1);
        let found = store.find_by_module("job1", "auth").await.unwrap().unwrap();
        assert_eq!(found.commercial_value_estimate, Some(6.0));
    }

    #[tokio::test]
    async fn checkpoint_store_round_trip() {
        let (_dir, pool) = temp_pool().await;
        let store = SqlxCheckpointStore::new(pool);
        let cp = JobCheckpoint {
            job_id: "job1".to_string(),
            last_completed_phase: Some(PhaseKind::LicenseScan),
            completed_module_ids: vec!["a".to_string()],
            token_usage_so_far: 123,
            partial_results: serde_json::Value::Null,
            saved_at: "2026-06-30T00:00:00Z".to_string(),
        };
        store.save(cp).await.unwrap();
        let loaded = store.load("job1").await.unwrap().unwrap();
        assert_eq!(loaded.token_usage_so_far, 123);
        assert_eq!(loaded.last_completed_phase, Some(PhaseKind::LicenseScan));
    }

    #[tokio::test]
    async fn cache_set_get_and_ttl() {
        let (_dir, pool) = temp_pool().await;
        let cache = AnalysisCacheStore::new(pool);
        let assessment = minimal_assessment();

        cache
            .set("https://github.com/acme/x", "sha1", &assessment, 30)
            .await
            .unwrap();
        let got = cache
            .get("https://github.com/acme/x", "sha1")
            .await
            .unwrap();
        assert!(got.is_some());
        assert_eq!(got.unwrap().repo_id, "r1");

        // ttl_days = 0 -> immediately expired.
        cache
            .set("https://github.com/acme/y", "sha2", &assessment, 0)
            .await
            .unwrap();
        let expired = cache
            .get("https://github.com/acme/y", "sha2")
            .await
            .unwrap();
        assert!(expired.is_none());

        cache.invalidate("https://github.com/acme/x").await.unwrap();
        assert!(cache
            .get("https://github.com/acme/x", "sha1")
            .await
            .unwrap()
            .is_none());
    }
}
