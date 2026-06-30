#![doc = "RepoGate CLI library: argument parsing, analysis, and cache commands."]

pub mod agent_flow;
pub mod analyze;
pub mod cache_cmd;
pub mod cli;
pub mod progress;

#[cfg(test)]
mod tests {
    use super::analyze::{budget_to_tokens, should_proceed, write_output};
    use super::cli::{Cli, Commands};
    use clap::Parser;
    use repogate_core::{Assessment, Repository, RepositoryMetrics};

    #[test]
    fn analyze_requires_budget() {
        // Missing --budget is an error.
        let parsed = Cli::try_parse_from(["repogate", "analyze", "https://github.com/a/b"]);
        assert!(parsed.is_err());
    }

    #[test]
    fn analyze_parses_with_budget() {
        let cli = Cli::try_parse_from([
            "repogate",
            "analyze",
            "https://github.com/a/b",
            "--budget",
            "5",
        ])
        .unwrap();
        match cli.command {
            Commands::Analyze(args) => {
                assert_eq!(args.repo_url, "https://github.com/a/b");
                assert_eq!(args.budget, 5.0);
                assert_eq!(args.output, "markdown");
                assert!(!args.yes);
            }
            _ => panic!("expected analyze"),
        }
    }

    #[test]
    fn cache_invalidate_parses() {
        let cli =
            Cli::try_parse_from(["repogate", "cache", "invalidate", "https://github.com/a/b"])
                .unwrap();
        assert!(matches!(cli.command, Commands::Cache(_)));
    }

    #[test]
    fn yes_skips_confirmation() {
        // With --yes, no input is read.
        assert!(should_proceed(true, std::io::empty()));
        // Without --yes, "y" proceeds and anything else cancels.
        assert!(should_proceed(false, "y\n".as_bytes()));
        assert!(!should_proceed(false, "n\n".as_bytes()));
    }

    #[test]
    fn budget_conversion_is_positive() {
        assert!(budget_to_tokens(3.0) >= 900_000);
        assert_eq!(budget_to_tokens(0.0), 0);
    }

    #[test]
    fn write_output_json_and_markdown() {
        let assessment = minimal_assessment();
        let dir = tempfile::tempdir().unwrap();

        let json_path = dir.path().join("out.json");
        let written = write_output(
            &assessment,
            "json",
            Some(json_path.to_string_lossy().to_string()),
        )
        .unwrap();
        assert!(std::path::Path::new(&written).exists());

        let md_path = dir.path().join("out.md");
        let written_md = write_output(
            &assessment,
            "markdown",
            Some(md_path.to_string_lossy().to_string()),
        )
        .unwrap();
        let contents = std::fs::read_to_string(&written_md).unwrap();
        assert!(contents.contains("Executive Summary"));
    }

    fn minimal_assessment() -> Assessment {
        Assessment {
            repo_id: "r1".to_string(),
            schema_version: "1.0".to_string(),
            generated_at: "0".to_string(),
            is_complete: true,
            repository: Repository {
                id: "r1".to_string(),
                url: "https://github.com/a/b".to_string(),
                name: "b".to_string(),
                description: None,
                license: None,
                metrics: RepositoryMetrics {
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
}
