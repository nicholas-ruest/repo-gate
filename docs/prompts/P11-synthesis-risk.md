# P11 — `repogate-orchestrator`: Synthesis Phase (Gating Strategy + Risk Analysis)

## Context

**You are implementing the synthesis and risk analysis phases: orchestrating Claude for high-level strategy and risk assessment.**

**Prerequisites:** P10 (scoring) is complete.

---

## Phase & Dependencies

- **Phase:** Synthesis
- **Depends on:** P10, P09

---

## Scope & Deliverables

### File: `src/pipeline/synthesis.rs`

```rust
pub async fn run_synthesis_phase(
    valuation: &ValuationReport,
    inventory: &FunctionalityInventory,
    license_report: &LicenseReport,
    arch_map: &ArchitectureMap,
    session_runner: impl SessionRunner,
) -> Result<GatingStrategy, OrchestratorError> {
    let prompt = format!(
        "Based on the valuation and functionality inventory, synthesize an open-core strategy.\
         Return SynthesisOutput schema with tier_assignments.",
        // Include JSON summaries of valuation, inventory, license_report
    );
    
    let result = session_runner.run(
        ClaudeInvocation {
            prompt,
            model: ClaudeModel::Opus,  // Always Opus for synthesis
            schema_path: Some("synthesis_output_schema.json".into()),
            allowed_tools: vec![],
            system_prompt: None,
            working_dir: None,
            session_id: None,
        }
    ).await?;
    
    let synthesis_output: SynthesisOutput = serde_json::from_str(&result.output)?;
    
    let tier_assignments = valuation.module_scores.iter().map(|score| {
        TierAssignment {
            module_id: score.module_id.clone(),
            module_name: score.module_id.clone(),  // Lookup from arch_map
            tier: score.tier,
            rationale: Some(format!("Score: {:.1}/10", score.composite_score.0)),
        }
    }).collect();
    
    Ok(GatingStrategy {
        tier_assignments,
        boundary_description: synthesis_output.strategy_notes,
    })
}
```

### File: `src/pipeline/risk_analysis.rs`

```rust
pub struct RiskProfile {
    pub risks: Vec<Risk>,
    pub blocking_risk_count: usize,
    pub high_severity_count: usize,
    pub overall_risk_level: String,  // "low" | "medium" | "high"
}

pub async fn run_risk_analysis_phase(
    strategy: &GatingStrategy,
    valuation: &ValuationReport,
    license_report: &LicenseReport,
    inventory: &FunctionalityInventory,
    session_runner: impl SessionRunner,
) -> Result<RiskProfile, OrchestratorError> {
    let prompt = format!(
        "Analyze risks in this gating strategy. Return RiskAnalysisOutput with identified risks.",
        // Include strategy, valuation, license, inventory summaries
    );
    
    let result = session_runner.run(
        ClaudeInvocation {
            prompt,
            model: ClaudeModel::Sonnet,  // Risk analysis uses Sonnet
            schema_path: Some("risk_analysis_output_schema.json".into()),
            allowed_tools: vec![],
            system_prompt: None,
            working_dir: None,
            session_id: None,
        }
    ).await?;
    
    let risk_output: RiskAnalysisOutput = serde_json::from_str(&result.output)?;
    
    let blocking_count = risk_output.risks.iter().filter(|r| r.is_blocking).count();
    let high_count = risk_output.risks.iter().filter(|r| r.severity == Severity::High).count();
    
    Ok(RiskProfile {
        risks: map_risks(&risk_output.risks),
        blocking_risk_count: blocking_count,
        high_severity_count: high_count,
        overall_risk_level: if blocking_count > 0 { "high" } else if high_count > 2 { "medium" } else { "low" }.into(),
    })
}

fn map_risks(findings: &[RiskFinding]) -> Vec<Risk> {
    findings.iter().map(|f| {
        Risk {
            kind: RiskKind::OverGating,  // Simplified mapping
            severity: f.severity.clone(),
            description: f.description.clone(),
            mitigation: Some(f.mitigation_suggestion.clone()),
            is_blocking: f.is_blocking,
        }
    }).collect()
}
```

### File: `src/pipeline/runner.rs`

```rust
pub struct PipelineOutput {
    pub manifest: RepoManifest,
    pub arch_map: ArchitectureMap,
    pub license_report: LicenseReport,
    pub inventory: FunctionalityInventory,
    pub valuation: ValuationReport,
    pub strategy: GatingStrategy,
    pub risk_profile: RiskProfile,
    pub is_complete: bool,
}

pub struct PipelineRunner {
    session_runner: Box<dyn SessionRunner>,
    checkpoint_store: Box<dyn CheckpointStore>,
    job_store: Box<dyn AssessmentJobStore>,
    budget: Arc<BudgetTracker>,
}

impl PipelineRunner {
    pub async fn run(
        &self,
        url: &str,
        budget_limit: u64,
        weights: &ScoreWeights,
    ) -> Result<PipelineOutput, OrchestratorError> {
        // P03: Ingest
        let manifest = ingest::ingest(url, &Path::new("/tmp/repo")).await?;
        
        // P04–P05: License scan (parallel)
        let license_report = licensing::analyze(&manifest, &Path::new("/tmp/repo")).await?;
        
        // P08: Architecture mapping
        let arch_map = arch_mapping::run_architecture_mapping_phase(
            &manifest, &Path::new("/tmp/repo"), &self.session_runner
        ).await?;
        
        // P09: Feature discovery
        let inventory = feature_discovery::run_feature_discovery_phase(
            &arch_map, &Path::new("/tmp/repo"), &self.session_runner,
            &*self.job_store, &self.budget, "job-1", 4
        ).await?;
        
        // P10: Scoring
        let valuation = scoring::score_all_modules(&[], &inventory, &license_report, weights)?;
        
        // P11: Synthesis
        let strategy = synthesis::run_synthesis_phase(
            &valuation, &inventory, &license_report, &arch_map, &self.session_runner
        ).await?;
        
        // P11: Risk analysis
        let risk_profile = risk_analysis::run_risk_analysis_phase(
            &strategy, &valuation, &license_report, &inventory, &self.session_runner
        ).await?;
        
        Ok(PipelineOutput {
            manifest,
            arch_map,
            license_report,
            inventory,
            valuation,
            strategy,
            risk_profile,
            is_complete: true,
        })
    }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-008-deep-traversal-map-reduce.md`** — Synthesis pass, JSON summaries
- **`docs/adr/ADR-012-model-routing.md`** — Synthesis Opus, risk Sonnet

---

## Acceptance Criteria

- ✅ Mock canned SynthesisOutput → GatingStrategy with populated tier_assignments
- ✅ Canned RiskAnalysisOutput `is_blocking: true` → Risk::is_blocking: true
- ✅ PipelineRunner::run with mock session runners completes
- ✅ `cargo test -p repogate-orchestrator` passes

---

## Language

**Rust** — Pipeline orchestration, Claude invocation, JSON mapping.

---

## Out-of-Scope

- Do NOT implement detailed risk categorization; focus on structure
- Do NOT call live Claude API
