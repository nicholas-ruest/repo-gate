//! Gating-strategy synthesis (always Opus, ADR-012).

use repogate_core::{GatingStrategy, SynthesisOutput, TierAssignment};
use repogate_licensing::LicenseReport;
use repogate_scoring::ValuationReport;

use super::arch_mapping::ArchitectureMap;
use super::llm_adapter::FunctionalityInventory;
use crate::claude::{run_structured, ClaudeInvocation, ClaudeModel, SessionRunner};
use crate::OrchestratorError;

/// Run the synthesis phase: an Opus session reasons over the JSON summaries and
/// produces an open-core narrative; the per-module tier assignments are taken
/// deterministically from the valuation so they are always populated.
pub async fn run_synthesis_phase(
    valuation: &ValuationReport,
    inventory: &FunctionalityInventory,
    license_report: &LicenseReport,
    arch_map: &ArchitectureMap,
    session_runner: &impl SessionRunner,
) -> Result<GatingStrategy, OrchestratorError> {
    let prompt = format!(
        "Synthesize an open-core commercialization strategy from these summaries. \
         Return a SynthesisOutput.\n\nVALUATION: {}\n\nINVENTORY: {}\n\nLICENSE: {}",
        serde_json::to_string(valuation).unwrap_or_default(),
        serde_json::to_string(inventory).unwrap_or_default(),
        serde_json::to_string(license_report).unwrap_or_default(),
    );

    let invocation = ClaudeInvocation {
        prompt,
        model: ClaudeModel::Opus,
        schema_json: None, // set by run_structured
        allowed_tools: vec![],
        system_prompt: None,
        working_dir: None,
        session_id: None,
    };

    // Schema-enforced with retry (ADR-016 R1). Tier assignments come
    // deterministically from the valuation regardless, so a failed boundary
    // narrative degrades gracefully to None with a warning.
    let boundary_description = match run_structured::<SynthesisOutput>(session_runner, invocation)
        .await
    {
        Ok(structured) => structured.value.gating_strategy,
        Err(_) => {
            tracing::warn!("synthesis produced no schema-valid output; boundary narrative omitted");
            None
        }
    };

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

    Ok(GatingStrategy {
        tier_assignments,
        boundary_description,
    })
}
