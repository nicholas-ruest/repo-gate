//! Command-line argument definitions.

use clap::{Parser, Subcommand};

/// RepoGate — deep repository assessment for open-core gating.
#[derive(Parser, Debug)]
#[command(
    name = "repogate",
    about = "Deep repository assessment for open-core gating"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Analyze a repository and produce a gating assessment (drives `claude` CLI).
    Analyze(AnalyzeArgs),
    /// Agent-in-the-loop step 1: clone + ingest a repo for Claude Code to analyze.
    Ingest(IngestArgs),
    /// Agent-in-the-loop step 2: score the agent's analysis and render the report.
    Synthesize(SynthesizeArgs),
    /// Manage the analysis cache.
    Cache(CacheArgs),
}

#[derive(Parser, Debug)]
pub struct IngestArgs {
    /// Repository URL.
    #[arg(value_name = "URL")]
    pub repo_url: String,

    /// Working directory to write the ingestion artifacts and cloned repo into.
    #[arg(long, default_value = "repogate-work")]
    pub out: String,
}

#[derive(Parser, Debug)]
pub struct SynthesizeArgs {
    /// Working directory produced by `ingest` (contains manifest/arch_map/license + your analysis.json).
    #[arg(long, default_value = "repogate-work")]
    pub dir: String,

    /// Path to the agent analysis JSON (defaults to <dir>/analysis.json).
    #[arg(long)]
    pub analysis: Option<String>,

    /// Output format: json | markdown | pdf.
    #[arg(long, default_value = "markdown")]
    pub output: String,

    /// Output file path.
    #[arg(long)]
    pub output_file: Option<String>,
}

#[derive(Parser, Debug)]
pub struct AnalyzeArgs {
    /// Repository URL.
    #[arg(value_name = "URL")]
    pub repo_url: String,

    /// Budget in USD (required — forces explicit cost acknowledgment).
    #[arg(long, required = true)]
    pub budget: f32,

    /// Output format: json | markdown | pdf.
    #[arg(long, default_value = "markdown")]
    pub output: String,

    /// Output file path.
    #[arg(long)]
    pub output_file: Option<String>,

    /// Weights JSON file (optional; defaults to expert weights).
    #[arg(long)]
    pub weights: Option<String>,

    /// Model override: opus | sonnet.
    #[arg(long)]
    pub model_override: Option<String>,

    /// Max concurrent module analyses.
    #[arg(long, default_value = "4")]
    pub max_concurrent: usize,

    /// Skip the confirmation prompt.
    #[arg(long)]
    pub yes: bool,

    /// Verbose logging.
    #[arg(long)]
    pub verbose: bool,
}

#[derive(Parser, Debug)]
pub struct CacheArgs {
    #[command(subcommand)]
    pub command: CacheCommands,
}

#[derive(Subcommand, Debug)]
pub enum CacheCommands {
    /// Invalidate all cache entries for a repository URL.
    Invalidate {
        #[arg(value_name = "URL")]
        repo_url: String,
    },
    /// List cache status.
    List,
}
