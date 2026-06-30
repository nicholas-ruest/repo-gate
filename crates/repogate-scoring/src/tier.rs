//! Deterministic mapping from composite score to a gating tier (ADR-010).

use repogate_core::{CompositeScore, GatingTier};

/// Map an effective composite score (after license adjustment) to a [`GatingTier`].
///
/// A strongly negative license adjustment forces `LegalReview` regardless of
/// the numeric score.
pub fn map_to_tier(effective_composite: CompositeScore, license_risk: Option<f32>) -> GatingTier {
    if license_risk.map(|r| r < -5.0).unwrap_or(false) {
        return GatingTier::LegalReview;
    }

    let score = effective_composite.get();
    if score < 2.5 {
        GatingTier::Open
    } else if score < 4.5 {
        GatingTier::SourceAvailable
    } else if score < 6.5 {
        GatingTier::ProTier
    } else if score < 8.0 {
        GatingTier::EnterpriseTier
    } else {
        GatingTier::ManagedCloud
    }
}
