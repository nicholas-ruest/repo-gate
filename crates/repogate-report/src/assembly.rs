//! Assemble a canonical [`Assessment`] from a pipeline run.

use std::collections::HashMap;

use repogate_core::{Assessment, Module, Repository, RepositoryMetrics};
use repogate_ingestion::RepoManifest;
use repogate_orchestrator::pipeline::PipelineOutput;

const SCHEMA_VERSION: &str = "1.0";

/// Build the canonical assessment from the pipeline output.
pub fn assemble(output: &PipelineOutput, generated_at: &str) -> Assessment {
    let primary_license = output
        .license_report
        .detections
        .first()
        .map(|d| d.spdx_expression.clone());
    let repository = build_repository(&output.manifest, primary_license);

    let modules = output
        .arch_map
        .modules
        .iter()
        .map(|m| {
            let recommended_tier = output
                .valuation
                .module_scores
                .iter()
                .find(|v| v.module_id == m.id)
                .map(|v| v.tier);
            Module {
                id: m.id.clone(),
                name: m.name.clone(),
                description: None,
                path: m.path.clone(),
                layer: m.layer,
                file_count: m.file_count,
                loc: m.loc,
                commercial_score: None,
                recommended_tier,
                risks: vec![],
            }
        })
        .collect();

    Assessment {
        repo_id: output.manifest.repo_id.clone(),
        schema_version: SCHEMA_VERSION.to_string(),
        generated_at: generated_at.to_string(),
        is_complete: output.is_complete,
        repository,
        modules,
        gating_strategy: Some(output.strategy.clone()),
        risks: output.risk_profile.risks.clone(),
    }
}

fn build_repository(manifest: &RepoManifest, license: Option<String>) -> Repository {
    let language_stats: HashMap<String, usize> = manifest
        .language_stats
        .language_counts
        .iter()
        .map(|(lang, loc)| (format!("{lang:?}"), *loc))
        .collect();

    Repository {
        id: manifest.repo_id.clone(),
        url: manifest.url.clone(),
        name: repo_name_from_url(&manifest.url),
        description: None,
        license,
        metrics: RepositoryMetrics {
            total_files: manifest.total_files,
            total_loc: manifest.total_loc,
            language_stats,
        },
    }
}

fn repo_name_from_url(url: &str) -> String {
    url.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("repository")
        .trim_end_matches(".git")
        .to_string()
}
