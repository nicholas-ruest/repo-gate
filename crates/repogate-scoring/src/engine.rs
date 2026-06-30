//! Weighted composite scoring (ADR-010, recalibrated per ADR-017).

use repogate_core::{CommercialScore, CompositeScore, ScoreWeights};

/// Maximum fraction of the gate pressure that high openness can discount away.
const MAX_OPENNESS_DISCOUNT: f32 = 0.6;

/// Compute a module's composite **gating score** from its 8 dimensions and
/// weights.
///
/// The composite is a 0–10 "how gateable is this" score, not a generic average:
///
/// - **Gate-positive dimensions** raise it — `enterprise_buyer_value`,
///   `commercial_leverage`, `competitive_sensitivity`, `operational_value`, and
///   `support_burden` (heavy support is an enterprise-tier signal, ADR-010).
///   Their weighted average is the *gate pressure*.
/// - **Openness dimensions** discount it — high `adoption_value` and
///   `strategic_importance` mean "keep open for the community flywheel", so they
///   reduce the gating score (up to [`MAX_OPENNESS_DISCOUNT`]).
/// - `security_sensitivity` does not affect the tier here; it is surfaced
///   separately as a review/risk signal.
///
/// Consequences: a low-value module scores low → Open; a high-IP module with
/// low adoption scores high → Enterprise; a commodity with high adoption is
/// discounted back toward Open even if it has some commercial value.
pub fn compute_composite(scores: &CommercialScore, weights: &ScoreWeights) -> CompositeScore {
    let gate_dims = [
        (
            scores.enterprise_buyer_value.get(),
            weights.enterprise_buyer_value.abs(),
        ),
        (
            scores.commercial_leverage.get(),
            weights.commercial_leverage.abs(),
        ),
        (
            scores.competitive_sensitivity.get(),
            weights.competitive_sensitivity.abs(),
        ),
        (
            scores.operational_value.get(),
            weights.operational_value.abs(),
        ),
        (scores.support_burden.get(), weights.support_burden.abs()),
    ];
    let gate_weight: f32 = gate_dims.iter().map(|(_, w)| w).sum();
    let gate_pressure = if gate_weight > 0.0 {
        gate_dims.iter().map(|(s, w)| s * w).sum::<f32>() / gate_weight
    } else {
        0.0
    };

    let open_weight = weights.adoption_value.abs() + weights.strategic_importance.abs();
    let openness = if open_weight > 0.0 {
        (scores.adoption_value.get() * weights.adoption_value.abs()
            + scores.strategic_importance.get() * weights.strategic_importance.abs())
            / open_weight
    } else {
        0.0
    };

    let discount = (openness / 10.0).clamp(0.0, 1.0) * MAX_OPENNESS_DISCOUNT;
    CompositeScore::new(gate_pressure * (1.0 - discount))
}
