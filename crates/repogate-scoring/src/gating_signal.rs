//! Derivation of a coarse gating signal from the composite score (ADR-010).

use repogate_core::{CompositeScore, GatingSignal, Score};

/// Derive a [`GatingSignal`] from the effective composite and adoption value.
///
/// High adoption value pulls a low-composite module toward `OpenCandidate` to
/// protect community adoption.
pub fn derive_gating_signal(
    effective_composite: CompositeScore,
    adoption_value: Option<Score>,
) -> GatingSignal {
    let score = effective_composite.get();
    let adoption = adoption_value.map(|s| s.get()).unwrap_or(5.0);

    if score >= 7.0 {
        GatingSignal::StrongGateCandidate
    } else if score >= 5.0 {
        GatingSignal::WeakGateCandidate
    } else if score <= 0.0 && adoption <= 0.0 {
        // No discriminating signal at all — insufficient data to recommend.
        GatingSignal::Undetermined
    } else {
        GatingSignal::OpenCandidate
    }
}
