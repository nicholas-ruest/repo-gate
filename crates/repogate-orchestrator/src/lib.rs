#![doc = "RepoGate orchestration: headless Claude Code integration and pipeline coordination."]

pub mod claude;

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
