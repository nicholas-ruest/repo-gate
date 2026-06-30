//! End-to-end pipeline runner sequencing all analysis phases.

use std::path::Path;

use repogate_core::{
    CommercialScore, CompletenessMetadata, GatingStrategy, Score, ScoreWeights, TokenBudget,
};
use repogate_ingestion::{build_manifest, RepoManifest};
use repogate_licensing::{analyze, CopyleftTier, LicenseReport};
use repogate_scoring::{score_all_modules, ModuleScoringInput, ValuationReport};

use super::arch_mapping::{run_architecture_mapping_phase, ArchitectureMap};
use super::feature_discovery::run_feature_discovery_phase;
use super::llm_adapter::FunctionalityInventory;
use super::risk_analysis::{run_risk_analysis_phase, RiskProfile};
use super::synthesis::run_synthesis_phase;
use crate::claude::SessionRunner;
use crate::job::{BudgetTracker, InMemoryModuleAssessmentStore, ModuleAssessmentStore};
use crate::OrchestratorError;

/// The complete output of an assessment pipeline run.
#[derive(Debug, Clone)]
pub struct PipelineOutput {
    pub manifest: RepoManifest,
    pub arch_map: ArchitectureMap,
    pub license_report: LicenseReport,
    pub inventory: FunctionalityInventory,
    pub valuation: ValuationReport,
    pub strategy: GatingStrategy,
    pub risk_profile: RiskProfile,
    pub is_complete: bool,
    /// Where the run degraded vs. ran fully (ADR-016 Remediation 4).
    pub completeness: CompletenessMetadata,
}

/// Whether the heuristic license detector was used (true unless the askalono
/// corpus feature is compiled in) — informational completeness signal.
fn license_detection_is_degraded() -> bool {
    !cfg!(feature = "askalono-corpus")
}

const JOB_ID: &str = "pipeline-job";

/// Sequences ingestion → licensing → architecture → discovery → scoring →
/// synthesis → risk over an already-cloned repository at `repo_path`.
pub struct PipelineRunner<R: SessionRunner> {
    runner: R,
    module_store: InMemoryModuleAssessmentStore,
    budget: BudgetTracker,
    max_concurrent: usize,
}

impl<R: SessionRunner> PipelineRunner<R> {
    pub fn new(runner: R, budget: TokenBudget) -> Self {
        Self {
            runner,
            module_store: InMemoryModuleAssessmentStore::new(),
            budget: BudgetTracker::new(budget),
            max_concurrent: 4,
        }
    }

    /// Run the full pipeline against a local repository checkout.
    pub async fn run(
        &self,
        repo_url: &str,
        repo_path: &Path,
        weights: &ScoreWeights,
    ) -> Result<PipelineOutput, OrchestratorError> {
        let manifest = build_manifest(repo_url, repo_path)
            .await
            .map_err(|e| OrchestratorError::SessionFailed(format!("ingestion: {e}")))?;

        let license_report = analyze(&manifest, repo_path)
            .await
            .map_err(|e| OrchestratorError::SessionFailed(format!("licensing: {e}")))?;

        let arch_map = run_architecture_mapping_phase(&manifest, repo_path, &self.runner).await?;

        let inventory = run_feature_discovery_phase(
            &arch_map,
            repo_path,
            &self.runner,
            &self.module_store,
            &self.budget,
            JOB_ID,
            self.max_concurrent,
        )
        .await?;

        let (scoring_inputs, scoring_degraded_modules) =
            self.build_scoring_inputs(&arch_map, &license_report).await;
        let valuation = score_all_modules(&scoring_inputs, weights)
            .map_err(|e| OrchestratorError::SchemaViolation(format!("scoring: {e}")))?;

        let strategy = run_synthesis_phase(
            &valuation,
            &inventory,
            &license_report,
            &arch_map,
            &self.runner,
        )
        .await?;

        let risk_profile = run_risk_analysis_phase(
            &strategy,
            &valuation,
            &license_report,
            &inventory,
            &self.runner,
        )
        .await?;

        let completeness = CompletenessMetadata {
            degraded_modules: inventory.degraded_modules.clone(),
            budget_skipped_modules: inventory.budget_skipped_modules.clone(),
            license_detection_degraded: license_detection_is_degraded(),
            scoring_degraded_modules,
        };

        Ok(PipelineOutput {
            manifest,
            arch_map,
            license_report,
            inventory,
            valuation,
            strategy,
            risk_profile,
            is_complete: completeness.is_complete() && !self.budget.is_exceeded(),
            completeness,
        })
    }

    /// Build per-module scoring inputs from stored assessments and the repo's
    /// license posture. Uses the model's real 8-dimension `commercial_score`
    /// when present; otherwise falls back to a uniform seed from the single
    /// estimate and records the module as scoring-degraded (ADR-016 R2/R4).
    ///
    /// Returns the inputs and the list of modules that used the fallback.
    async fn build_scoring_inputs(
        &self,
        arch_map: &ArchitectureMap,
        license_report: &LicenseReport,
    ) -> (Vec<ModuleScoringInput>, Vec<String>) {
        let license_tier = repo_license_tier(license_report.overall_risk_score);
        let mut inputs = Vec::new();
        let mut degraded = Vec::new();

        for module in &arch_map.modules {
            let assessment = self
                .module_store
                .find_by_module(JOB_ID, &module.id)
                .await
                .ok()
                .flatten();

            let commercial_score = match assessment
                .as_ref()
                .and_then(|a| a.commercial_score.clone())
            {
                Some(score) => score,
                None => {
                    let estimate = assessment
                        .as_ref()
                        .and_then(|a| a.commercial_value_estimate)
                        .unwrap_or(5.0);
                    tracing::warn!(module = %module.id, "scoring used uniform fallback (no per-dimension scores)");
                    degraded.push(module.id.clone());
                    uniform_commercial(estimate)
                }
            };

            inputs.push(ModuleScoringInput {
                module_id: module.id.clone(),
                commercial_score,
                license_tier,
            });
        }
        (inputs, degraded)
    }
}

fn uniform_commercial(value: f32) -> CommercialScore {
    let s = Score::new(value.clamp(0.0, 10.0)).unwrap_or_else(|_| Score::new(5.0).unwrap());
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

fn repo_license_tier(overall_risk_score: f32) -> CopyleftTier {
    if overall_risk_score >= 8.0 {
        CopyleftTier::StrongCopyleft
    } else if overall_risk_score >= 3.0 {
        CopyleftTier::SourceAvailableNonOsi
    } else {
        CopyleftTier::Permissive
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::{ClaudeInvocation, SessionResult, UsageStats};
    use repogate_core::{RiskAnalysisOutput, RiskFinding};

    /// Returns a fixed canned output for every invocation; phases that cannot
    /// parse it fall back to deterministic defaults.
    struct CannedRunner {
        output: String,
    }

    impl SessionRunner for CannedRunner {
        async fn run(
            &self,
            _invocation: ClaudeInvocation,
        ) -> Result<SessionResult, OrchestratorError> {
            Ok(SessionResult {
                session_id: "s".to_string(),
                output: self.output.clone(),
                usage: UsageStats::default(),
            })
        }
    }

    fn budget() -> TokenBudget {
        TokenBudget {
            total_limit: 1_000_000,
            per_phase_limit: 1_000_000,
            per_session_limit: 1_000_000,
            warn_threshold: 0.8,
        }
    }

    #[tokio::test]
    async fn pipeline_completes_with_canned_runner() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "// x\n").unwrap();

        let pipeline = PipelineRunner::new(crate::claude::DeterministicMockRunner, budget());
        let out = pipeline
            .run(
                "https://example.com/x",
                dir.path(),
                &ScoreWeights::default(),
            )
            .await
            .unwrap();

        // The phase-aware mock returns schema-valid output with real per-dimension
        // scores, so the run is fully complete (no degradation).
        assert!(out.is_complete);
        assert!(out.completeness.is_complete());
        assert!(out.completeness.scoring_degraded_modules.is_empty());
        assert!(!out.valuation.module_scores.is_empty());
        // Synthesis always populates tier assignments from the valuation.
        assert_eq!(
            out.strategy.tier_assignments.len(),
            out.valuation.module_scores.len()
        );
    }

    #[tokio::test]
    async fn synthesis_populates_tier_assignments() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "// x\n").unwrap();

        let synthesis = repogate_core::SynthesisOutput {
            gating_strategy: Some("keep core open; gate enterprise".to_string()),
            tier_assignments: None,
        };
        let runner = CannedRunner {
            output: serde_json::to_string(&synthesis).unwrap(),
        };
        let pipeline = PipelineRunner::new(runner, budget());
        let out = pipeline
            .run(
                "https://example.com/x",
                dir.path(),
                &ScoreWeights::default(),
            )
            .await
            .unwrap();
        assert!(!out.strategy.tier_assignments.is_empty());
    }

    #[tokio::test]
    async fn risk_blocking_flag_propagates() {
        let output = RiskAnalysisOutput {
            risks: vec![RiskFinding {
                category: "license".to_string(),
                severity: repogate_core::Severity::High,
                description: "AGPL in open tier".to_string(),
                mitigation_suggestion: "relicense or gate".to_string(),
                is_blocking: true,
            }],
        };
        let profile = run_risk_analysis_phase(
            &GatingStrategy {
                tier_assignments: vec![],
                boundary_description: None,
            },
            &score_all_modules(&[], &ScoreWeights::default()).unwrap(),
            &dummy_license_report(),
            &dummy_inventory(),
            &CannedRunner {
                output: serde_json::to_string(&output).unwrap(),
            },
        )
        .await
        .unwrap();

        assert_eq!(profile.blocking_risk_count, 1);
        assert_eq!(profile.overall_risk_level, "high");
        assert!(profile.risks[0].is_blocking);
    }

    fn dummy_license_report() -> LicenseReport {
        LicenseReport {
            repo_id: "r".to_string(),
            detections: vec![],
            dependency_licenses: vec![],
            copyleft_exposure: 0.0,
            missing_licenses: false,
            conflicts: vec![],
            overall_risk_score: 0.0,
        }
    }

    fn dummy_inventory() -> FunctionalityInventory {
        FunctionalityInventory {
            repo_id: "r".to_string(),
            items: vec![],
            total_count: 0,
            hidden_count: 0,
            enterprise_count: 0,
            api_entry_points: vec![],
            degraded_modules: vec![],
            budget_skipped_modules: vec![],
        }
    }
}
