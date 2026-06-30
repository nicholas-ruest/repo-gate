//! Agent-in-the-loop commands: `ingest` and `synthesize`.
//!
//! These let Claude Code itself be the reasoning engine — the authenticated
//! in-session agent does the deep per-module analysis between the two steps,
//! instead of RepoGate shelling out to a separate `claude -p` subprocess.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use repogate_core::{ModuleAssessment, ScoreWeights};
use repogate_ingestion::git::{validate_repo_url, GitProvider, SubprocessGit};
use repogate_ingestion::{build_manifest, RepoManifest};
use repogate_licensing::{analyze, LicenseReport};
use repogate_orchestrator::pipeline::arch_mapping::{
    detect_modules_heuristic, generate_ascii_diagram, ArchitectureMap,
};
use repogate_orchestrator::pipeline::{assemble_offline, AgentAnalysis};

use crate::analyze::write_output;
use crate::cli::{IngestArgs, SynthesizeArgs};

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> anyhow::Result<()> {
    std::fs::write(path, serde_json::to_vec_pretty(value)?)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))
}

/// `repogate ingest <url>`: clone, walk, license-scan, and detect modules; write
/// the artifacts and an analysis template for the agent to fill.
pub async fn run_ingest(args: IngestArgs) -> anyhow::Result<()> {
    validate_repo_url(&args.repo_url).context("invalid repository URL")?;
    let out = PathBuf::from(&args.out);
    std::fs::create_dir_all(&out)?;
    let repo_path = out.join("repo");

    if !repo_path.exists() {
        eprintln!(
            "[ingest] cloning {} -> {}",
            args.repo_url,
            repo_path.display()
        );
        SubprocessGit
            .clone(&args.repo_url, &repo_path)
            .await
            .context("cloning repository")?;
    } else {
        eprintln!(
            "[ingest] reusing existing checkout at {}",
            repo_path.display()
        );
    }

    let manifest = build_manifest(&args.repo_url, &repo_path).await?;
    let license: LicenseReport = analyze(&manifest, &repo_path).await?;
    let modules = detect_modules_heuristic(&manifest, &repo_path);
    let ascii_diagram = generate_ascii_diagram(&modules, &[]);
    let arch_map = ArchitectureMap {
        modules: modules.clone(),
        edges: vec![],
        ascii_diagram,
    };

    write_json(&out.join("manifest.json"), &manifest)?;
    write_json(&out.join("arch_map.json"), &arch_map)?;
    write_json(&out.join("license.json"), &license)?;

    let template = AgentAnalysis {
        modules: arch_map
            .modules
            .iter()
            .map(|m| ModuleAssessment {
                module_name: m.name.clone(),
                module_path: m.path.clone(),
                capabilities: vec![],
                commercial_score: None,
                commercial_value_estimate: None,
                estimated_tier: None,
                risks: vec![],
            })
            .collect(),
        gating_strategy: None,
        risks: vec![],
    };
    write_json(&out.join("analysis.template.json"), &template)?;

    println!(
        "Ingested {} ({} files, {} LOC).",
        manifest.url, manifest.total_files, manifest.total_loc
    );
    println!("Modules detected ({}):", arch_map.modules.len());
    for m in &arch_map.modules {
        println!("  - {} ({:?}) at {}", m.name, m.layer, m.path);
    }
    println!(
        "\nNext: read the code under {}, then write {}/analysis.json \
         (copy {}/analysis.template.json and fill each module's capabilities + \
         8-dimension commercial_score, plus gating_strategy and risks). \
         Then run: repogate synthesize --dir {}",
        repo_path.display(),
        out.display(),
        out.display(),
        out.display(),
    );
    Ok(())
}

/// `repogate synthesize --dir <dir>`: score the agent's analysis and render the report.
pub async fn run_synthesize(args: SynthesizeArgs) -> anyhow::Result<()> {
    let dir = PathBuf::from(&args.dir);
    let manifest: RepoManifest = read_json(&dir.join("manifest.json"))?;
    let arch_map: ArchitectureMap = read_json(&dir.join("arch_map.json"))?;
    let license: LicenseReport = read_json(&dir.join("license.json"))?;
    let analysis_path = args
        .analysis
        .map(PathBuf::from)
        .unwrap_or_else(|| dir.join("analysis.json"));
    let analysis: AgentAnalysis = read_json(&analysis_path)?;

    let output = assemble_offline(
        &manifest,
        &arch_map,
        &license,
        &analysis,
        &ScoreWeights::default(),
    )
    .map_err(|e| anyhow::anyhow!("synthesis failed: {e}"))?;

    let assessment = repogate_report::assemble(&output, &now_timestamp());
    let path = write_output(&assessment, &args.output, args.output_file)?;

    eprintln!(
        "Synthesized: {} modules scored, {} strong-gate, {} open, {} legal-review. is_complete={}",
        output.valuation.module_scores.len(),
        output.valuation.strong_gate_count,
        output.valuation.open_count,
        output.valuation.legal_review_count,
        output.is_complete,
    );
    println!("Report written: {path}");
    Ok(())
}

fn now_timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}
