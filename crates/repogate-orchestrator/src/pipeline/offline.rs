//! Agent-in-the-loop assembly: the deterministic pipeline parts (scoring, tier
//! mapping, report assembly) run in Rust, while the LLM reasoning (per-module
//! assessments, gating narrative, risks) is supplied by the Claude Code agent
//! that is *already authenticated in-session* — no `claude -p` subprocess.
//!
//! This realizes the spec's "runs through Claude Code" intent correctly: Claude
//! Code is the reasoning engine, RepoGate is the harness around it.

use repogate_core::{
    CompletenessMetadata, GatingStrategy, ModuleAssessment, RiskFinding, ScoreWeights,
    TierAssignment,
};
use repogate_ingestion::RepoManifest;
use repogate_licensing::LicenseReport;
use repogate_scoring::{score_all_modules, ModuleScoringInput};
use serde::{Deserialize, Serialize};

use super::arch_mapping::ArchitectureMap;
use super::llm_adapter::{map_to_functionality_items, FunctionalityInventory};
use super::risk_analysis::risk_profile_from_findings;
use super::runner::{
    license_detection_is_degraded, repo_license_tier, uniform_commercial, PipelineOutput,
};
use crate::OrchestratorError;

/// The analysis a Claude Code agent produces for a repository: one assessment
/// per module (with the eight commercial dimensions filled in), an overall
/// open-core gating narrative, and the identified risks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAnalysis {
    pub modules: Vec<ModuleAssessment>,
    #[serde(default)]
    pub gating_strategy: Option<String>,
    #[serde(default)]
    pub risks: Vec<RiskFinding>,
}

/// Assemble a full [`PipelineOutput`] from deterministic inputs plus the agent's
/// analysis — the scoring, tier mapping, risk aggregation, and inventory are all
/// computed in Rust.
pub fn assemble_offline(
    manifest: &RepoManifest,
    arch_map: &ArchitectureMap,
    license_report: &LicenseReport,
    analysis: &AgentAnalysis,
    weights: &ScoreWeights,
) -> Result<PipelineOutput, OrchestratorError> {
    let license_tier = repo_license_tier(license_report.overall_risk_score);

    let mut inputs = Vec::new();
    let mut scoring_degraded_modules = Vec::new();
    for module in &arch_map.modules {
        let assessment = analysis
            .modules
            .iter()
            .find(|a| a.module_name == module.name || a.module_name == module.id);

        let commercial_score = match assessment.and_then(|a| a.commercial_score.clone()) {
            Some(score) => score,
            None => {
                let estimate = assessment
                    .and_then(|a| a.commercial_value_estimate)
                    .unwrap_or(5.0);
                scoring_degraded_modules.push(module.id.clone());
                uniform_commercial(estimate)
            }
        };

        inputs.push(ModuleScoringInput {
            module_id: module.id.clone(),
            commercial_score,
            license_tier,
        });
    }

    let valuation = score_all_modules(&inputs, weights)
        .map_err(|e| OrchestratorError::SchemaViolation(format!("scoring: {e}")))?;

    let tier_assignments = valuation
        .module_scores
        .iter()
        .map(|score| {
            let module_name = arch_map
                .modules
                .iter()
                .find(|m| m.id == score.module_id)
                .map(|m| m.name.clone())
                .unwrap_or_else(|| score.module_id.clone());
            TierAssignment {
                module_id: score.module_id.clone(),
                module_name,
                tier: score.tier,
                rationale: Some(format!("Composite {:.1}/10", score.composite_score)),
            }
        })
        .collect();

    let strategy = GatingStrategy {
        tier_assignments,
        boundary_description: analysis.gating_strategy.clone(),
    };

    let risk_profile = risk_profile_from_findings(&analysis.risks);
    let inventory = build_inventory(&manifest.repo_id, &analysis.modules);

    let completeness = CompletenessMetadata {
        degraded_modules: Vec::new(),
        budget_skipped_modules: Vec::new(),
        license_detection_degraded: license_detection_is_degraded(),
        scoring_degraded_modules,
    };

    Ok(PipelineOutput {
        manifest: manifest.clone(),
        arch_map: arch_map.clone(),
        license_report: license_report.clone(),
        inventory,
        valuation,
        strategy,
        risk_profile,
        is_complete: completeness.is_complete(),
        completeness,
    })
}

fn build_inventory(repo_id: &str, modules: &[ModuleAssessment]) -> FunctionalityInventory {
    let mut items = Vec::new();
    let mut hidden_count = 0;
    let mut enterprise_count = 0;
    for assessment in modules {
        for item in map_to_functionality_items(assessment, &assessment.module_path) {
            match item.visibility {
                repogate_core::Visibility::Undocumented => hidden_count += 1,
                repogate_core::Visibility::Enterprise => enterprise_count += 1,
                _ => {}
            }
            items.push(item);
        }
    }
    FunctionalityInventory {
        repo_id: repo_id.to_string(),
        total_count: items.len(),
        hidden_count,
        enterprise_count,
        items,
        api_entry_points: Vec::new(),
        degraded_modules: Vec::new(),
        budget_skipped_modules: Vec::new(),
    }
}
