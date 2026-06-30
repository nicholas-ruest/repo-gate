//! The `repogate analyze` command.

use std::io::BufRead;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context};
use repogate_core::{Assessment, ScoreWeights, TokenBudget};
use repogate_ingestion::git::{validate_repo_url, GitProvider, SubprocessGit};
use repogate_orchestrator::claude::ClaudeCliRunner;
use repogate_orchestrator::pipeline::PipelineRunner;
use repogate_report::{render_markdown, render_pdf, to_json_bytes};

use crate::cli::AnalyzeArgs;
use crate::progress::{ProgressReporter, StderrProgressReporter};

/// Approximate blended token price: ~$3 per million tokens.
const USD_PER_MILLION_TOKENS: f32 = 3.0;

/// Convert a USD budget to an approximate token limit.
pub fn budget_to_tokens(budget_usd: f32) -> u64 {
    ((budget_usd / USD_PER_MILLION_TOKENS) * 1_000_000.0).max(0.0) as u64
}

/// Rough pre-run cost estimate (min, max) in USD.
pub fn estimate_cost(_repo_url: &str) -> (f32, f32) {
    (1.0, 15.0)
}

/// Decide whether to proceed, reading a confirmation line unless `yes`.
pub fn should_proceed(yes: bool, mut reader: impl BufRead) -> bool {
    if yes {
        return true;
    }
    let mut input = String::new();
    if reader.read_line(&mut input).is_err() {
        return false;
    }
    input.trim().eq_ignore_ascii_case("y")
}

fn now_timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

/// Write the assessment in the requested format; returns the output path.
pub fn write_output(
    assessment: &Assessment,
    format: &str,
    output_file: Option<String>,
) -> anyhow::Result<String> {
    match format {
        "json" => {
            let path = output_file.unwrap_or_else(|| "assessment.json".to_string());
            std::fs::write(&path, to_json_bytes(assessment)?)?;
            Ok(path)
        }
        "pdf" => {
            let md = render_markdown(assessment)?;
            let path = output_file.unwrap_or_else(|| "assessment.pdf".to_string());
            render_pdf(&md, Path::new(&path))?;
            Ok(path)
        }
        _ => {
            let md = render_markdown(assessment)?;
            let path = output_file.unwrap_or_else(|| "assessment.md".to_string());
            std::fs::write(&path, md)?;
            Ok(path)
        }
    }
}

/// Run the analyze command end-to-end.
pub async fn run_analyze(args: AnalyzeArgs) -> anyhow::Result<()> {
    validate_repo_url(&args.repo_url).context("invalid repository URL")?;

    let reporter = StderrProgressReporter;

    let (min_cost, max_cost) = estimate_cost(&args.repo_url);
    eprintln!(
        "Estimated cost: ${min_cost:.2} – ${max_cost:.2} (budget ${:.2})",
        args.budget
    );

    if !should_proceed(args.yes, std::io::stdin().lock()) {
        eprintln!("Cancelled.");
        return Ok(());
    }

    // Clone the repository to a temporary working directory.
    let workdir = tempfile::tempdir().context("creating temp dir")?;
    let repo_path = workdir.path().join("repo");
    reporter.report("ingesting", &format!("cloning {}", args.repo_url));
    SubprocessGit
        .clone(&args.repo_url, &repo_path)
        .await
        .context("cloning repository")?;

    let budget = TokenBudget {
        total_limit: budget_to_tokens(args.budget),
        per_phase_limit: budget_to_tokens(args.budget),
        per_session_limit: budget_to_tokens(args.budget),
        warn_threshold: 0.8,
    };

    reporter.report("analyzing", "running assessment pipeline");
    let pipeline = PipelineRunner::new(ClaudeCliRunner, budget);
    let output = pipeline
        .run(&args.repo_url, &repo_path, &ScoreWeights::default())
        .await
        .map_err(|e| anyhow!("pipeline failed: {e}"))?;

    reporter.report("reporting", "assembling report");
    let assessment = repogate_report::assemble(&output, &now_timestamp());
    let path = write_output(&assessment, &args.output, args.output_file)?;
    eprintln!("Report written: {path}");

    if !output.is_complete {
        eprintln!("Warning: analysis incomplete due to budget exhaustion.");
        return Err(anyhow!("budget exceeded; partial report written to {path}"));
    }

    Ok(())
}
