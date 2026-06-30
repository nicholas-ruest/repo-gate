//! RepoGate CLI entry point.

use std::process::ExitCode;

use clap::Parser;
use repogate_cli::cli::{Cli, Commands};

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Analyze(args) => {
            if args.verbose {
                let _ = tracing_subscriber::fmt::try_init();
            }
            repogate_cli::analyze::run_analyze(args).await
        }
        Commands::Ingest(args) => repogate_cli::agent_flow::run_ingest(args).await,
        Commands::Synthesize(args) => repogate_cli::agent_flow::run_synthesize(args).await,
        Commands::Cache(args) => repogate_cli::cache_cmd::run_cache(args).await,
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
