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
    fn composite_all_five_leans_open() {
        // With moderate gate pressure (5) and a moderate openness discount, an
        // all-average module leans toward open rather than sitting at the
        // midpoint (ADR-017 recalibration).
        let composite = compute_composite(&uniform_scores(5.0), &ScoreWeights::default());
        assert!(
            composite.get() > 2.5 && composite.get() < 4.5,
            "composite was {}",
            composite.get()
        );
    }

    #[test]
    fn high_support_burden_raises_gateability() {
        // Heavy support burden is an enterprise-tier signal (ADR-010), so it now
        // increases the gating composite rather than lowering it.
        let base = compute_composite(&uniform_scores(5.0), &ScoreWeights::default());
        let mut scores = uniform_scores(5.0);
        scores.support_burden = Score::new(10.0).unwrap();
        let with_burden = compute_composite(&scores, &ScoreWeights::default());
        assert!(
            with_burden.get() > base.get(),
            "{} should exceed {}",
            with_burden.get(),
            base.get()
        );
    }

    #[test]
    fn high_adoption_commodity_scores_open_while_ip_scores_gated() {
        // A high-adoption commodity (low IP) lands near Open; a high-IP, low-
        // adoption module lands clearly higher (ADR-017 separation).
        let weights = ScoreWeights::default();
        let commodity = CommercialScore {
            adoption_value: Score::new(9.0).unwrap(),
            enterprise_buyer_value: Score::new(3.0).unwrap(),
            commercial_leverage: Score::new(3.0).unwrap(),
            competitive_sensitivity: Score::new(2.0).unwrap(),
            operational_value: Score::new(4.0).unwrap(),
            security_sensitivity: Score::new(2.0).unwrap(),
            support_burden: Score::new(3.0).unwrap(),
            strategic_importance: Score::new(8.0).unwrap(),
        };
        let ip = CommercialScore {
            adoption_value: Score::new(4.0).unwrap(),
            enterprise_buyer_value: Score::new(8.0).unwrap(),
            commercial_leverage: Score::new(8.0).unwrap(),
            competitive_sensitivity: Score::new(9.0).unwrap(),
            operational_value: Score::new(4.0).unwrap(),
            security_sensitivity: Score::new(3.0).unwrap(),
            support_burden: Score::new(6.0).unwrap(),
            strategic_importance: Score::new(6.0).unwrap(),
        };
        let commodity_c = compute_composite(&commodity, &weights).get();
        let ip_c = compute_composite(&ip, &weights).get();
        assert!(
            commodity_c < 2.5,
            "commodity composite {commodity_c} should be Open-range"
        );
        assert!(
            ip_c >= 4.5,
            "ip composite {ip_c} should be at least ProTier-range"
        );
        assert!(ip_c > commodity_c + 2.0);
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
