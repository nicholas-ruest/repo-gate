//! License-risk adjustment to a module's composite score (ADR-010 sub-score).

use repogate_core::CompositeScore;
use repogate_licensing::CopyleftTier;

/// Adjust a composite score for license/copyleft exposure.
///
/// Returns the adjusted score and the applied adjustment (negative). Strong
/// copyleft effectively caps the module near the floor and flags it for legal
/// review downstream (see [`crate::tier::map_to_tier`]).
pub fn apply_license_risk(
    composite: CompositeScore,
    exposure: &CopyleftTier,
) -> (CompositeScore, Option<f32>) {
    let adjustment = match exposure {
        CopyleftTier::StrongCopyleft => -8.0,
        CopyleftTier::WeakCopyleft => -2.0,
        CopyleftTier::SourceAvailableNonOsi => -1.0,
        CopyleftTier::Permissive | CopyleftTier::PublicDomain => 0.0,
        CopyleftTier::Unknown => -0.5,
    };

    let adjusted = CompositeScore::new(composite.get() + adjustment);
    (adjusted, Some(adjustment))
}
