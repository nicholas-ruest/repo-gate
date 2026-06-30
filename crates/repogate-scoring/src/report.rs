//! Aggregate valuation across all modules.

use repogate_core::{CommercialScore, CompositeScore, GatingSignal, GatingTier, ScoreWeights};
use repogate_licensing::CopyleftTier;
use serde::{Deserialize, Serialize};

use crate::engine::compute_composite;
use crate::gating_signal::derive_gating_signal;
use crate::license_risk::apply_license_risk;
use crate::tier::map_to_tier;

/// One module's commercial score input: its 8-dimension score and license tier.
#[derive(Debug, Clone)]
pub struct ModuleScoringInput {
    pub module_id: String,
    pub commercial_score: CommercialScore,
    pub license_tier: CopyleftTier,
}

/// The computed valuation for a single module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleValuation {
    pub module_id: String,
    pub composite_score: f32,
    pub tier: GatingTier,
    pub signal: GatingSignal,
}

/// Aggregate valuation across all scored modules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuationReport {
    pub module_scores: Vec<ModuleValuation>,
    pub strong_gate_count: usize,
    pub open_count: usize,
    pub legal_review_count: usize,
}

/// Errors produced while scoring modules.
#[derive(Debug, thiserror::Error)]
pub enum ScoringError {
    #[error("scoring failed: {0}")]
    Failed(String),
}

/// Score every module: compute composite, apply license risk, map to tier and
/// gating signal, then aggregate tier counts.
pub fn score_all_modules(
    inputs: &[ModuleScoringInput],
    weights: &ScoreWeights,
) -> Result<ValuationReport, ScoringError> {
    let mut module_scores = Vec::new();

    for input in inputs {
        let composite = compute_composite(&input.commercial_score, weights);
        let (adjusted, adjustment) = apply_license_risk(composite, &input.license_tier);
        let tier = map_to_tier(adjusted, adjustment);
        let signal = derive_gating_signal(adjusted, Some(input.commercial_score.adoption_value));

        module_scores.push(ModuleValuation {
            module_id: input.module_id.clone(),
            composite_score: composite_value(adjusted),
            tier,
            signal,
        });
    }

    let strong_gate_count = module_scores
        .iter()
        .filter(|v| {
            matches!(
                v.tier,
                GatingTier::EnterpriseTier | GatingTier::ManagedCloud
            )
        })
        .count();
    let open_count = module_scores
        .iter()
        .filter(|v| v.tier == GatingTier::Open)
        .count();
    let legal_review_count = module_scores
        .iter()
        .filter(|v| v.tier == GatingTier::LegalReview)
        .count();

    Ok(ValuationReport {
        module_scores,
        strong_gate_count,
        open_count,
        legal_review_count,
    })
}

fn composite_value(score: CompositeScore) -> f32 {
    score.get()
}
