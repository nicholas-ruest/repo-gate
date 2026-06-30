//! Risk-analysis phase (Sonnet, ADR-012).

use repogate_core::{Risk, RiskAnalysisOutput, RiskFinding, RiskKind, Severity};
use repogate_licensing::LicenseReport;
use repogate_scoring::ValuationReport;
use serde::{Deserialize, Serialize};

use super::llm_adapter::FunctionalityInventory;
use crate::claude::{run_structured, ClaudeInvocation, ClaudeModel, SessionRunner};
use crate::OrchestratorError;

/// The aggregated risk profile for an assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskProfile {
    pub risks: Vec<Risk>,
    pub blocking_risk_count: usize,
    pub high_severity_count: usize,
    pub overall_risk_level: String,
}

/// Run the risk-analysis phase over the proposed strategy and supporting data.
pub async fn run_risk_analysis_phase(
    strategy: &repogate_core::GatingStrategy,
    valuation: &ValuationReport,
    license_report: &LicenseReport,
    inventory: &FunctionalityInventory,
    session_runner: &impl SessionRunner,
) -> Result<RiskProfile, OrchestratorError> {
    let prompt = format!(
        "Identify risks in this gating strategy (over-gating, community backlash, \
         license conflicts, competitive exposure, security exposure). Return a \
         RiskAnalysisOutput.\n\nSTRATEGY: {}\n\nVALUATION: {}\n\nLICENSE: {}\n\nINVENTORY: {}",
        serde_json::to_string(strategy).unwrap_or_default(),
        serde_json::to_string(valuation).unwrap_or_default(),
        serde_json::to_string(license_report).unwrap_or_default(),
        serde_json::to_string(inventory).unwrap_or_default(),
    );

    let invocation = ClaudeInvocation {
        prompt,
        model: ClaudeModel::Sonnet,
        schema_json: None, // set by run_structured
        allowed_tools: vec![],
        system_prompt: None,
        working_dir: None,
        session_id: None,
    };

    // Schema-enforced with retry (ADR-016 R1). Risk analysis is advisory: if the
    // session cannot produce valid output, fall back to an empty risk set with a
    // warning rather than failing the whole assessment.
    let output = match run_structured::<RiskAnalysisOutput>(session_runner, invocation).await {
        Ok(structured) => structured.value,
        Err(_) => {
            tracing::warn!("risk analysis produced no schema-valid output; reporting no risks");
            RiskAnalysisOutput::default()
        }
    };

    let blocking_risk_count = output.risks.iter().filter(|r| r.is_blocking).count();
    let high_severity_count = output
        .risks
        .iter()
        .filter(|r| r.severity == Severity::High)
        .count();

    let overall_risk_level = if blocking_risk_count > 0 {
        "high"
    } else if high_severity_count > 2 {
        "medium"
    } else {
        "low"
    }
    .to_string();

    Ok(RiskProfile {
        risks: map_risks(&output.risks),
        blocking_risk_count,
        high_severity_count,
        overall_risk_level,
    })
}

/// Build a [`RiskProfile`] directly from risk findings (used by the
/// agent-in-the-loop offline path).
pub fn risk_profile_from_findings(findings: &[RiskFinding]) -> RiskProfile {
    let blocking_risk_count = findings.iter().filter(|r| r.is_blocking).count();
    let high_severity_count = findings
        .iter()
        .filter(|r| r.severity == Severity::High)
        .count();
    let overall_risk_level = if blocking_risk_count > 0 {
        "high"
    } else if high_severity_count > 2 {
        "medium"
    } else {
        "low"
    }
    .to_string();
    RiskProfile {
        risks: map_risks(findings),
        blocking_risk_count,
        high_severity_count,
        overall_risk_level,
    }
}

fn map_risks(findings: &[RiskFinding]) -> Vec<Risk> {
    findings
        .iter()
        .map(|f| Risk {
            kind: classify_kind(&f.category),
            severity: f.severity,
            description: f.description.clone(),
            mitigation: Some(f.mitigation_suggestion.clone()),
            is_blocking: f.is_blocking,
        })
        .collect()
}

fn classify_kind(category: &str) -> RiskKind {
    match category.to_lowercase().as_str() {
        c if c.contains("backlash") || c.contains("community") => RiskKind::CommunityBacklash,
        c if c.contains("license") || c.contains("copyleft") => RiskKind::LicenseConflict,
        c if c.contains("competit") => RiskKind::CompetitiveExposure,
        c if c.contains("security") => RiskKind::SecurityExposure,
        c if c.contains("under") => RiskKind::UnderGating,
        c if c.contains("accidental") || c.contains("open") => RiskKind::AccidentalOpenSource,
        _ => RiskKind::OverGating,
    }
}
