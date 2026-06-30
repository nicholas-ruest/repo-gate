#![doc = "RepoGate scoring: commercial value scoring and gating-tier classification."]

pub mod engine;
pub mod gating_signal;
pub mod license_risk;
pub mod report;
pub mod tier;

pub use engine::compute_composite;
pub use gating_signal::derive_gating_signal;
pub use license_risk::apply_license_risk;
pub use report::{
    score_all_modules, ModuleScoringInput, ModuleValuation, ScoringError, ValuationReport,
};
pub use tier::map_to_tier;

#[cfg(test)]
mod tests {
    use super::*;
    use repogate_core::{
        CommercialScore, CompositeScore, GatingSignal, GatingTier, Score, ScoreWeights,
    };
    use repogate_licensing::CopyleftTier;

    fn uniform_scores(v: f32) -> CommercialScore {
        let s = Score::new(v).unwrap();
        CommercialScore {
            adoption_value: s,
            enterprise_buyer_value: s,
            commercial_leverage: s,
            competitive_sensitivity: s,
            operational_value: s,
            security_sensitivity: s,
            support_burden: s,
            strategic_importance: s,
        }
    }

    #[test]
    fn composite_all_five_is_about_five() {
        let composite = compute_composite(&uniform_scores(5.0), &ScoreWeights::default());
        assert!(
            composite.get() >= 4.5 && composite.get() <= 5.5,
            "composite was {}",
            composite.get()
        );
    }

    #[test]
    fn high_support_burden_lowers_composite() {
        let mut scores = uniform_scores(5.0);
        scores.support_burden = Score::new(10.0).unwrap();
        let composite = compute_composite(&scores, &ScoreWeights::default());
        assert!(composite.get() < 5.0, "composite was {}", composite.get());
    }

    #[test]
    fn tier_mapping_all_branches() {
        assert_eq!(
            map_to_tier(CompositeScore::new(1.0), None),
            GatingTier::Open
        );
        assert_eq!(
            map_to_tier(CompositeScore::new(3.0), None),
            GatingTier::SourceAvailable
        );
        assert_eq!(
            map_to_tier(CompositeScore::new(5.0), None),
            GatingTier::ProTier
        );
        assert_eq!(
            map_to_tier(CompositeScore::new(7.5), None),
            GatingTier::EnterpriseTier
        );
        assert_eq!(
            map_to_tier(CompositeScore::new(9.0), None),
            GatingTier::ManagedCloud
        );
        assert_eq!(
            map_to_tier(CompositeScore::new(9.0), Some(-8.0)),
            GatingTier::LegalReview
        );
    }

    #[test]
    fn license_risk_strong_copyleft_caps_low() {
        let (adjusted, adj) =
            apply_license_risk(CompositeScore::new(8.0), &CopyleftTier::StrongCopyleft);
        assert!(adjusted.get() <= 2.0);
        assert!(adj.unwrap() <= -5.0);
    }

    #[test]
    fn agpl_module_routes_to_legal_review() {
        let composite = compute_composite(&uniform_scores(7.0), &ScoreWeights::default());
        let (adjusted, adj) = apply_license_risk(composite, &CopyleftTier::StrongCopyleft);
        assert_eq!(map_to_tier(adjusted, adj), GatingTier::LegalReview);
    }

    #[test]
    fn gating_signal_low_composite_high_adoption_is_open() {
        let signal = derive_gating_signal(CompositeScore::new(3.0), Some(Score::new(9.0).unwrap()));
        assert_eq!(signal, GatingSignal::OpenCandidate);
    }

    #[test]
    fn gating_signal_high_composite_is_strong() {
        let signal = derive_gating_signal(CompositeScore::new(8.0), Some(Score::new(5.0).unwrap()));
        assert_eq!(signal, GatingSignal::StrongGateCandidate);
    }

    #[test]
    fn score_all_modules_counts_tiers() {
        let permissive_high = ModuleScoringInput {
            module_id: "enterprise-mod".to_string(),
            commercial_score: uniform_scores(8.0),
            license_tier: CopyleftTier::Permissive,
        };
        let agpl = ModuleScoringInput {
            module_id: "agpl-mod".to_string(),
            commercial_score: uniform_scores(7.0),
            license_tier: CopyleftTier::StrongCopyleft,
        };
        let report = score_all_modules(&[permissive_high, agpl], &ScoreWeights::default()).unwrap();
        assert_eq!(report.module_scores.len(), 2);
        assert_eq!(report.legal_review_count, 1);
    }
}
