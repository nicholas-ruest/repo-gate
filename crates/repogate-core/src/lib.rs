#![doc = "RepoGate core types, schemas, and domain models."]

pub mod claude_schemas;
pub mod error;
pub mod model;
pub mod types;

pub use claude_schemas::*;
pub use error::*;
pub use model::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_valid() {
        // In-range values construct successfully, including the boundaries.
        assert!(Score::new(0.0).is_ok());
        assert!(Score::new(10.0).is_ok());
        // Per acceptance criteria, Score::new(5.0) yields Ok(Score(5.0)) — verify
        // the wrapped value, not merely that construction succeeds.
        let s = Score::new(5.0).expect("5.0 is in range");
        assert_eq!(s.get(), 5.0);
    }

    #[test]
    fn score_out_of_range() {
        assert!(Score::new(-1.0).is_err());
        assert!(Score::new(11.0).is_err());
        assert!(Score::new(10.1).is_err());
    }

    #[test]
    fn score_weights_validation() {
        let ok = ScoreWeights::new(1.2, 1.1, 1.0, 0.9, 0.8, 0.7, -0.6, 1.0);
        assert!(ok.is_ok());
        // Negative adoption_value (non-support_burden) must be rejected.
        let err = ScoreWeights::new(-1.0, 1.1, 1.0, 0.9, 0.8, 0.7, -0.6, 1.0);
        assert!(err.is_err());
    }

    #[test]
    fn token_budget_is_exceeded() {
        let budget = TokenBudget {
            total_limit: 100,
            per_phase_limit: 50,
            per_session_limit: 30,
            warn_threshold: 0.8,
        };
        assert!(!budget.is_exceeded(50));
        assert!(budget.is_exceeded(100));
        assert_eq!(budget.remaining(40), 60);
        assert_eq!(budget.remaining(110), 0); // saturating
    }

    #[test]
    fn assessment_round_trip() {
        let assessment = Assessment {
            repo_id: "test".to_string(),
            schema_version: "1.0".to_string(),
            generated_at: "2024-01-01T00:00:00Z".to_string(),
            is_complete: true,
            repository: Repository {
                id: "r1".to_string(),
                url: "https://example.com/repo".to_string(),
                name: "test-repo".to_string(),
                description: None,
                license: None,
                metrics: RepositoryMetrics {
                    total_files: 100,
                    total_loc: 5000,
                    language_stats: Default::default(),
                },
            },
            modules: vec![],
            gating_strategy: None,
            risks: vec![],
        };
        let json = serde_json::to_string(&assessment).unwrap();
        let restored: Assessment = serde_json::from_str(&json).unwrap();
        assert_eq!(assessment.repo_id, restored.repo_id);
        assert_eq!(assessment.schema_version, restored.schema_version);
    }

    #[test]
    fn write_schema_module_assessment() {
        let dir = std::env::temp_dir();
        let path = dir.join("repogate_module_assessment_schema.json");
        write_schema::<ModuleAssessment>(&path).expect("write_schema failed");
        let content = std::fs::read_to_string(&path).expect("schema file not written");
        let parsed: serde_json::Value =
            serde_json::from_str(&content).expect("schema is not valid JSON");
        // A valid JSON Schema has a "$schema" or "title" key, or at least is an object.
        assert!(parsed.is_object(), "schema root must be a JSON object");
        let _ = std::fs::remove_file(&path);
    }
}
