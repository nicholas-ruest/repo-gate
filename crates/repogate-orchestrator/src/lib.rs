#![doc = "RepoGate orchestration: headless Claude Code integration and pipeline coordination."]

pub mod claude;
pub mod job;
pub mod pipeline;

/// Errors produced by the orchestrator.
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("session failed: {0}")]
    SessionFailed(String),

    #[error("schema violation: {0}")]
    SchemaViolation(String),
}

#[cfg(test)]
mod tests {
    use super::claude::*;

    fn args_of(inv: &ClaudeInvocation) -> Vec<String> {
        inv.build_command()
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect()
    }

    #[test]
    fn build_command_contains_required_flags() {
        let inv = ClaudeInvocation {
            prompt: "assess".to_string(),
            model: ClaudeModel::Opus,
            schema_path: Some("/tmp/schema.json".into()),
            allowed_tools: vec!["Read".to_string(), "Glob".to_string()],
            system_prompt: Some("be precise".to_string()),
            working_dir: None,
            session_id: None,
        };
        let args = args_of(&inv);
        for flag in [
            "--bare",
            "--output-format",
            "stream-json",
            "--json-schema",
            "--allowedTools",
            "--model",
        ] {
            assert!(args.iter().any(|a| a == flag), "missing flag: {flag}");
        }
        assert!(args.iter().any(|a| a == "claude-opus-4-8"));
        assert!(args.iter().any(|a| a == "Read,Glob"));
    }

    #[test]
    fn build_command_resume_when_session_set() {
        let inv = ClaudeInvocation {
            prompt: "x".to_string(),
            model: ClaudeModel::Sonnet,
            schema_path: None,
            allowed_tools: vec![],
            system_prompt: None,
            working_dir: None,
            session_id: Some("sess-42".to_string()),
        };
        let args = args_of(&inv);
        assert!(args.iter().any(|a| a == "--resume"));
        assert!(args.iter().any(|a| a == "sess-42"));
        assert!(!args.iter().any(|a| a == "--allowedTools"));
    }

    #[test]
    fn parse_init_event() {
        let json = r#"{"type":"init","session_id":"test-123"}"#;
        let event: ClaudeEvent = serde_json::from_str(json).unwrap();
        match event {
            ClaudeEvent::Init { session_id } => assert_eq!(session_id, "test-123"),
            other => panic!("expected Init, got {other:?}"),
        }
    }

    #[test]
    fn parse_stream_init_result() {
        let stdout = concat!(
            r#"{"type":"init","session_id":"s1"}"#,
            "\n",
            r#"{"type":"result","content":"done","usage":{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":2}}"#,
            "\n",
        );
        let result = session::parse_session_output(stdout.as_bytes()).unwrap();
        assert_eq!(result.session_id, "s1");
        assert_eq!(result.output, "done");
        assert_eq!(result.usage.input_tokens, 10);
    }

    #[test]
    fn parse_stream_surfaces_error_event() {
        let stdout = concat!(
            r#"{"type":"init","session_id":"s1"}"#,
            "\n",
            r#"{"type":"error","message":"boom","code":"E1"}"#,
            "\n",
        );
        let err = session::parse_session_output(stdout.as_bytes()).unwrap_err();
        assert!(matches!(err, crate::OrchestratorError::SessionFailed(_)));
    }

    #[test]
    fn select_model_routing() {
        assert_eq!(select_model("any", Phase::Synthesis), ClaudeModel::Opus);
        assert_eq!(
            select_model("utils", Phase::ManifestSummarization),
            ClaudeModel::Sonnet
        );
        assert_eq!(
            select_model("auth", Phase::FeatureDiscovery),
            ClaudeModel::Opus
        );
        assert_eq!(
            select_model("widgets", Phase::FeatureDiscovery),
            ClaudeModel::Sonnet
        );
    }

    #[test]
    fn write_phase_schema_emits_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_phase_schema(Phase::FeatureDiscovery, dir.path()).unwrap();
        let contents = std::fs::read_to_string(&path).unwrap();
        let _: serde_json::Value = serde_json::from_str(&contents).unwrap();
    }
}

#[cfg(test)]
mod job_tests {
    use crate::job::*;
    use repogate_core::{ModuleAssessment, TokenBudget};

    fn budget(total: u64) -> TokenBudget {
        TokenBudget {
            total_limit: total,
            per_phase_limit: total,
            per_session_limit: total,
            warn_threshold: 0.8,
        }
    }

    fn module_assessment(name: &str) -> ModuleAssessment {
        ModuleAssessment {
            module_name: name.to_string(),
            module_path: format!("src/{name}"),
            capabilities: vec![],
            commercial_value_estimate: None,
            estimated_tier: None,
            risks: vec![],
        }
    }

    #[test]
    fn budget_records_and_detects_exceeded() {
        let tracker = BudgetTracker::new(budget(1000));
        let status = tracker.record_usage(500, 200, 0);
        assert_eq!(tracker.used(), 700);
        assert_eq!(status, BudgetStatus::Ok);
        assert!(!tracker.is_exceeded());

        let status = tracker.record_usage(400, 0, 0); // total 1100 >= 1000
        assert_eq!(status, BudgetStatus::Exceeded);
        assert!(tracker.is_exceeded());
        assert_eq!(tracker.remaining(), 0);
    }

    #[test]
    fn budget_cache_read_billed_at_ten_percent() {
        let tracker = BudgetTracker::new(budget(10_000));
        tracker.record_usage(0, 0, 1000); // 1000/10 = 100
        assert_eq!(tracker.used(), 100);
    }

    #[test]
    fn phases_to_run_resumes_after_license_scan() {
        let checkpoint = JobCheckpoint {
            job_id: "j1".to_string(),
            last_completed_phase: Some(PhaseKind::LicenseScan),
            completed_module_ids: vec![],
            token_usage_so_far: 0,
            partial_results: serde_json::Value::Null,
            saved_at: "2026-06-30T00:00:00Z".to_string(),
        };
        let remaining = phases_to_run(&checkpoint);
        assert_eq!(remaining.first(), Some(&PhaseKind::ArchitectureMapping));
        assert!(!remaining.contains(&PhaseKind::Ingestion));
        assert!(!remaining.contains(&PhaseKind::LicenseScan));
        assert!(remaining.contains(&PhaseKind::ReportAssembly));
    }

    #[test]
    fn feature_discovery_gate_requires_all_assessments() {
        let mut job = AssessmentJob::new("https://example.com/x", "2026-06-30T00:00:00Z");
        job.module_ids = vec!["auth".to_string(), "billing".to_string()];

        // Missing assessments -> gate fails.
        assert!(validate_gate(PhaseKind::FeatureDiscovery, &job).is_err());

        job.module_assessments.push(module_assessment("auth"));
        assert!(validate_gate(PhaseKind::FeatureDiscovery, &job).is_err());

        job.module_assessments.push(module_assessment("billing"));
        assert!(validate_gate(PhaseKind::FeatureDiscovery, &job).is_ok());
    }

    #[tokio::test]
    async fn job_store_round_trip() {
        let store = InMemoryAssessmentJobStore::new();
        let job = AssessmentJob::new("https://example.com/x", "2026-06-30T00:00:00Z");
        let id = job.id.clone();
        store.save(job).await.unwrap();

        let loaded = store.find_by_id(&id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().repo_url, "https://example.com/x");

        let queued = store.find_by_status(JobStatus::Queued).await.unwrap();
        assert_eq!(queued.len(), 1);
    }

    #[tokio::test]
    async fn module_store_exists_and_load_all() {
        let store = InMemoryModuleAssessmentStore::new();
        store.save("j1", module_assessment("auth")).await.unwrap();
        assert!(store.exists("j1", "auth").await.unwrap());
        assert!(!store.exists("j1", "billing").await.unwrap());
        assert_eq!(store.load_all("j1").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn checkpoint_store_round_trip() {
        let store = InMemoryCheckpointStore::new();
        let cp = JobCheckpoint {
            job_id: "j1".to_string(),
            last_completed_phase: Some(PhaseKind::Ingestion),
            completed_module_ids: vec![],
            token_usage_so_far: 42,
            partial_results: serde_json::Value::Null,
            saved_at: "2026-06-30T00:00:00Z".to_string(),
        };
        store.save(cp).await.unwrap();
        let loaded = store.load("j1").await.unwrap().unwrap();
        assert_eq!(loaded.token_usage_so_far, 42);
    }
}
