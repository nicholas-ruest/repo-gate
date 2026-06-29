# P10 — `repogate-scoring`: Commercial Value Scoring Engine + Tier Classifier

## Context

**You are implementing the commercial value scoring and gating tier classification engine.**

**Prerequisites:** P02 (core types), P05 (licensing), P09 (feature discovery) are complete.

---

## Phase & Dependencies

- **Phase:** Scoring
- **Depends on:** P02, P05, P09

---

## Scope & Deliverables

Implement `repogate-scoring/src/` with scoring engine and tier mapping.

### File: `src/scoring/engine.rs`

```rust
pub fn compute_composite(scores: &CommercialScore, weights: &ScoreWeights) -> CompositeScore {
    let weighted_sum = 
        scores.adoption_value.get() * weights.adoption_value +
        scores.enterprise_buyer_value.get() * weights.enterprise_buyer_value +
        scores.commercial_leverage.get() * weights.commercial_leverage +
        scores.competitive_sensitivity.get() * weights.competitive_sensitivity +
        scores.operational_value.get() * weights.operational_value +
        scores.security_sensitivity.get() * weights.security_sensitivity +
        scores.support_burden.get() * weights.support_burden +  // Negative; subtracts
        scores.strategic_importance.get() * weights.strategic_importance;
    
    let sum_weights: f32 = weights.adoption_value + weights.enterprise_buyer_value + 
        weights.commercial_leverage + weights.competitive_sensitivity +
        weights.operational_value + weights.security_sensitivity + 
        weights.support_burden.abs() + weights.strategic_importance;
    
    let composite = (weighted_sum / sum_weights).max(0.0).min(10.0);
    CompositeScore(composite)
}
```

### File: `src/scoring/license_risk.rs`

```rust
pub fn apply_license_risk(
    composite: CompositeScore,
    exposure: &CopyleftTier,
) -> (CompositeScore, Option<f32>) {
    let adjustment = match exposure {
        CopyleftTier::StrongCopyleft => -8.0,  // Cap at 2.0
        CopyleftTier::WeakCopyleft => -2.0,
        CopyleftTier::SourceAvailableNonOsi => -1.0,
        CopyleftTier::Permissive | CopyleftTier::PublicDomain => 0.0,
        CopyleftTier::Unknown => -0.5,
    };
    
    let adjusted = (composite.0 + adjustment).max(0.0);
    (CompositeScore(adjusted), Some(adjustment))
}
```

### File: `src/scoring/tier.rs`

```rust
pub fn map_to_tier(effective_composite: CompositeScore, license_risk: Option<f32>) -> GatingTier {
    let score = effective_composite.0;
    
    // Check for license issues that force legal review
    if license_risk.map(|r| r < -5.0).unwrap_or(false) {
        return GatingTier::LegalReview;
    }
    
    match score {
        s if s < 2.5 => GatingTier::Open,
        s if s < 4.5 => GatingTier::SourceAvailable,
        s if s < 6.5 => GatingTier::ProTier,
        s if s < 8.0 => GatingTier::EnterpriseTier,
        _ => GatingTier::ManagedCloud,
    }
}
```

### File: `src/scoring/gating_signal.rs`

```rust
pub fn derive_gating_signal(
    effective_composite: CompositeScore,
    adoption_value: Option<Score>,
) -> GatingSignal {
    let adoption = adoption_value.map(|s| s.get()).unwrap_or(5.0);
    
    match effective_composite.0 {
        s if s >= 7.0 => GatingSignal::StrongGateCandidate,
        s if s >= 5.0 && s < 7.0 => GatingSignal::WeakGateCandidate,
        s if s < 5.0 && adoption >= 8.0 => GatingSignal::OpenCandidate,
        s if s < 5.0 => GatingSignal::OpenCandidate,
        _ => GatingSignal::Undetermined,
    }
}
```

### File: `src/scoring/report.rs`

```rust
pub struct ValuationReport {
    pub module_scores: Vec<ModuleValuation>,
    pub strong_gate_count: usize,
    pub open_count: usize,
    pub legal_review_count: usize,
}

pub struct ModuleValuation {
    pub module_id: String,
    pub composite_score: CompositeScore,
    pub tier: GatingTier,
    pub signal: GatingSignal,
}

pub fn score_all_modules(
    assessments: &[ModuleAssessment],
    inventory: &FunctionalityInventory,
    license_report: &LicenseReport,
    weights: &ScoreWeights,
) -> Result<ValuationReport, ScoringError> {
    let mut valuations = Vec::new();
    
    for assessment in assessments {
        let composite = compute_composite(&assessment.commercial_score, weights);
        let (adjusted, _) = apply_license_risk(composite, &CopyleftTier::Permissive);
        let tier = map_to_tier(adjusted, None);
        let signal = derive_gating_signal(adjusted, None);
        
        valuations.push(ModuleValuation {
            module_id: assessment.module_id.clone(),
            composite_score: adjusted,
            tier,
            signal,
        });
    }
    
    let strong_count = valuations.iter().filter(|v| v.tier == GatingTier::EnterpriseTier || v.tier == GatingTier::ManagedCloud).count();
    let open_count = valuations.iter().filter(|v| v.tier == GatingTier::Open).count();
    let legal_count = valuations.iter().filter(|v| v.tier == GatingTier::LegalReview).count();
    
    Ok(ValuationReport {
        module_scores: valuations,
        strong_gate_count: strong_count,
        open_count,
        legal_review_count: legal_count,
    })
}
```

### File: `src/lib.rs`

```rust
pub mod scoring;
pub mod license_risk;
pub mod tier;
pub mod gating_signal;
pub mod report;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composite_equal_weights() {
        let scores = CommercialScore {
            adoption_value: Score::new(5.0).unwrap(),
            // ... all 5.0
        };
        let weights = ScoreWeights::default();
        let composite = scoring::engine::compute_composite(&scores, &weights);
        assert!(composite.0 >= 4.5 && composite.0 <= 5.5);
    }

    #[test]
    fn test_tier_mapping() {
        let tier = tier::map_to_tier(CompositeScore(7.5), None);
        assert_eq!(tier, GatingTier::EnterpriseTier);
    }

    #[test]
    fn test_license_risk_agpl() {
        let (adjusted, _) = license_risk::apply_license_risk(CompositeScore(8.0), &CopyleftTier::StrongCopyleft);
        assert!(adjusted.0 <= 2.0);
    }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-010-commercial-value-scoring-model.md`** — Full scoring model, dimensions, tier ranges, weights
- **`docs/ddd/commercial-valuation.md`** — Value objects, scoring invariants

---

## Acceptance Criteria

- ✅ All scores 5.0 with equal weights → composite ~5.0
- ✅ `support_burden` 10.0 with others 5.0 → composite < 5.0
- ✅ AGPL module → `LegalReview`
- ✅ `map_to_tier(7.5, None)` → `EnterpriseTier`
- ✅ `derive_gating_signal(3.0, 9.0)` → `OpenCandidate`
- ✅ 100% coverage on tier classifier
- ✅ `cargo test -p repogate-scoring` passes

---

## Language

**Rust** — Scoring arithmetic, enum mapping, weighted aggregation.

---

## Out-of-Scope

- Do NOT implement complex machine learning; use deterministic scoring
- Do NOT implement per-language scoring variations
