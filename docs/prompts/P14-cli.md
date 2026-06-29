# P14 — `repogate-cli`: CLI Entry Point, `repogate analyze`, Cost Estimation, Progress

## Context

**You are implementing the CLI interface: argument parsing, cost estimation, progress reporting.**

**Prerequisites:** P11 (orchestration), P12 (report), P13 (stores) are complete.

---

## Phase & Dependencies

- **Phase:** UX
- **Depends on:** P11, P12, P13

---

## Scope & Deliverables

Implement `repogate-cli/src/`.

### File: `src/main.rs`

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "repogate", about = "Deep repository assessment for open-core gating")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Analyze(AnalyzeArgs),
    Cache(CacheArgs),
}

#[derive(Parser)]
struct AnalyzeArgs {
    /// Repository URL
    #[arg(value_name = "URL")]
    repo_url: String,
    
    /// Budget in USD (required)
    #[arg(long, required = true)]
    budget: f32,
    
    /// Output format: json | markdown | pdf
    #[arg(long, default_value = "markdown")]
    output: String,
    
    /// Output file path
    #[arg(long)]
    output_file: Option<String>,
    
    /// Weights JSON file
    #[arg(long)]
    weights: Option<String>,
    
    /// Model override: opus | sonnet
    #[arg(long)]
    model_override: Option<String>,
    
    /// Max concurrent modules
    #[arg(long, default_value = "4")]
    max_concurrent: usize,
    
    /// Skip confirmation
    #[arg(long)]
    yes: bool,
    
    /// Verbose output
    #[arg(long)]
    verbose: bool,
}

#[derive(Parser)]
struct CacheArgs {
    #[command(subcommand)]
    command: CacheCommands,
}

#[derive(Subcommand)]
enum CacheCommands {
    Invalidate { repo_url: String },
    List,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Analyze(args) => commands::analyze::run(args).await?,
        Commands::Cache(args) => commands::cache::run(args).await?,
    }
    
    Ok(())
}
```

### File: `src/commands/analyze.rs`

```rust
pub async fn run(args: AnalyzeArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Validate URL
    git::validate_repo_url(&args.repo_url)?;
    
    // Instantiate pipeline runner
    let runner = PipelineRunner::new(
        Box::new(MockSessionRunner::default()),
        Box::new(InMemoryCheckpointStore::new()),
        Box::new(InMemoryAssessmentJobStore::new()),
    );
    
    // Estimate cost
    let (min_cost, max_cost) = estimate_cost(&args.repo_url, &args).await?;
    eprintln!("Estimated cost: ${:.2} – ${:.2}", min_cost, max_cost);
    
    if !args.yes {
        eprintln!("Proceed? [y/N] ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            eprintln!("Cancelled.");
            return Ok(());
        }
    }
    
    // Run pipeline with progress
    let reporter = StderrProgressReporter::new();
    let output = runner.run(
        &args.repo_url,
        (args.budget * 1_000_000.0) as u64,  // Tokens equiv
        &ScoreWeights::default(),
    ).await?;
    
    // Assemble report
    let assessment = assembly::assemble(&output, &chrono::Utc::now().to_rfc3339());
    
    // Write output
    match args.output.as_str() {
        "json" => {
            let file = args.output_file.unwrap_or_else(|| "assessment.json".to_string());
            let writer = std::fs::File::create(&file)?;
            json::write_json(&assessment, writer)?;
            println!("Written: {}", file);
        }
        "markdown" | _ => {
            let md = markdown::render_markdown(&assessment)?;
            let file = args.output_file.unwrap_or_else(|| "assessment.md".to_string());
            std::fs::write(&file, md)?;
            println!("Written: {}", file);
        }
        "pdf" => {
            let md = markdown::render_markdown(&assessment)?;
            let file = args.output_file.unwrap_or_else(|| "assessment.pdf".to_string());
            pdf::render_pdf(&md, Path::new(&file))?;
            println!("Written: {}", file);
        }
    }
    
    if !output.is_complete {
        eprintln!("Warning: analysis incomplete due to budget exhaustion.");
        return Err("Budget exceeded".into());
    }
    
    Ok(())
}

async fn estimate_cost(repo_url: &str, args: &AnalyzeArgs) -> Result<(f32, f32), Box<dyn std::error::Error>> {
    // Heuristic: small repos ~$1, large repos ~$15
    Ok((1.0, 15.0))
}
```

### File: `src/progress.rs`

```rust
pub trait ProgressReporter: Send {
    fn report(&self, phase: &str, message: &str);
}

pub struct StderrProgressReporter;

impl ProgressReporter for StderrProgressReporter {
    fn report(&self, phase: &str, message: &str) {
        eprintln!("[{}] {}", phase, message);
    }
}
```

### File: `src/lib.rs`

```rust
pub mod commands;
pub mod progress;

#[cfg(test)]
mod tests {
    #[test]
    fn cli_help_works() {
        // Integration test: run `repogate --help`
    }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-013-token-budget-enforcement.md`** — Confirmation flow, `--yes`, estimate, hard budget
- **`docs/adr/ADR-015-web-api-layer-axum-nextjs.md`** — CLI vs. server modes

---

## Acceptance Criteria

- ✅ `repogate analyze --help` shows `--budget` as required
- ✅ Missing `--budget` → error exit code
- ✅ `--yes` skips confirmation (test with small repo)
- ✅ Budget exhaustion → partial report `is_complete: false`; non-zero exit
- ✅ `cargo build -p repogate-cli` produces binary

---

## Language

**Rust** — clap argument parsing, cost estimation, progress reporting.

---

## Out-of-Scope

- Do NOT implement interactive TUI
- Do NOT implement watch mode or continuous monitoring
