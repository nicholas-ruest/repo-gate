#![doc = "RepoGate report assembly: canonical JSON and Markdown rendering."]

pub mod assembly;
pub mod json;
pub mod markdown;
pub mod naming;
pub mod pdf;

pub use assembly::assemble;
pub use json::{to_json_bytes, write_json};
pub use markdown::render_markdown;
pub use naming::report_stem;
pub use pdf::render_pdf;

/// Errors produced during report generation.
#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("template render error: {0}")]
    Render(String),

    #[error("pdf engine (weasyprint) not found")]
    PdfEngineNotFound,

    #[error("pdf engine error: {0}")]
    PdfEngineError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use repogate_core::{
        Assessment, GatingStrategy, GatingTier, Repository, RepositoryMetrics, TierAssignment,
    };
    use std::collections::HashMap;

    fn minimal_assessment() -> Assessment {
        Assessment {
            repo_id: "r1".to_string(),
            schema_version: "1.0".to_string(),
            generated_at: "2026-06-30T00:00:00Z".to_string(),
            is_complete: true,
            repository: Repository {
                id: "r1".to_string(),
                url: "https://github.com/acme/myproject".to_string(),
                name: "myproject".to_string(),
                description: None,
                license: Some("MIT".to_string()),
                metrics: RepositoryMetrics {
                    total_files: 3,
                    total_loc: 100,
                    language_stats: HashMap::new(),
                },
            },
            modules: vec![],
            gating_strategy: Some(GatingStrategy {
                tier_assignments: vec![TierAssignment {
                    module_id: "core".to_string(),
                    module_name: "core".to_string(),
                    tier: GatingTier::Open,
                    rationale: Some("keep open".to_string()),
                }],
                boundary_description: Some("open core".to_string()),
            }),
            risks: vec![],
            completeness: None,
        }
    }

    #[test]
    fn report_stem_github() {
        let stem = report_stem("https://github.com/acme/my_project", "20260630-000000");
        assert_eq!(stem, "repogate-acme-my-project-20260630-000000");
    }

    #[test]
    fn render_markdown_contains_sections() {
        let md = render_markdown(&minimal_assessment()).unwrap();
        assert!(md.contains("Executive Summary"));
        assert!(md.contains("Gating Recommendations"));
        assert!(md.contains("myproject"));
        assert!(md.contains("core"));
    }

    #[test]
    fn json_round_trip() {
        let assessment = minimal_assessment();
        let bytes = to_json_bytes(&assessment).unwrap();
        let back: Assessment = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            serde_json::to_value(&assessment).unwrap(),
            serde_json::to_value(&back).unwrap()
        );
    }

    #[tokio::test]
    async fn assemble_marks_completion() {
        use repogate_core::ScoreWeights;
        use repogate_orchestrator::claude::DeterministicMockRunner;
        use repogate_orchestrator::pipeline::{PipelineOutput, PipelineRunner};

        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "// x\n").unwrap();

        let budget = repogate_core::TokenBudget {
            total_limit: 1_000_000,
            per_phase_limit: 1_000_000,
            per_session_limit: 1_000_000,
            warn_threshold: 0.8,
        };
        let pipeline = PipelineRunner::new(DeterministicMockRunner, budget);
        let output: PipelineOutput = pipeline
            .run(
                "https://github.com/acme/myproject",
                dir.path(),
                &ScoreWeights::default(),
            )
            .await
            .unwrap();

        let assessment = assemble(&output, "2026-06-30T00:00:00Z");
        assert!(assessment.is_complete);
        assert!(assessment.completeness.is_some());
        assert_eq!(assessment.schema_version, "1.0");
        assert_eq!(assessment.repository.name, "myproject");
    }
}
