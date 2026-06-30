//! Weighted composite scoring (ADR-010).

use repogate_core::{CommercialScore, CompositeScore, ScoreWeights};

/// Compute a module's composite score from its 8 dimensions and weights.
///
/// The seven "positive" dimensions form a weighted average (the base score).
/// `support_burden` is modeled as a penalty relative to a neutral baseline of
/// 5.0, scaled by its weight magnitude: higher burden lowers the composite,
/// lower burden raises it. With all dimensions at 5.0 the composite is 5.0.
pub fn compute_composite(scores: &CommercialScore, weights: &ScoreWeights) -> CompositeScore {
    let positive = [
        (scores.adoption_value.get(), weights.adoption_value),
        (
            scores.enterprise_buyer_value.get(),
            weights.enterprise_buyer_value,
        ),
        (
            scores.commercial_leverage.get(),
            weights.commercial_leverage,
        ),
        (
            scores.competitive_sensitivity.get(),
            weights.competitive_sensitivity,
        ),
        (scores.operational_value.get(), weights.operational_value),
        (
            scores.security_sensitivity.get(),
            weights.security_sensitivity,
        ),
        (
            scores.strategic_importance.get(),
            weights.strategic_importance,
        ),
    ];

    let weight_sum: f32 = positive.iter().map(|(_, w)| w.abs()).sum();
    let base = if weight_sum > 0.0 {
        positive.iter().map(|(s, w)| s * w).sum::<f32>() / weight_sum
    } else {
        0.0
    };

    // Support burden penalizes relative to a neutral midpoint of 5.0.
    let support_penalty = (scores.support_burden.get() - 5.0) * weights.support_burden.abs();

    CompositeScore::new(base - support_penalty)
}
