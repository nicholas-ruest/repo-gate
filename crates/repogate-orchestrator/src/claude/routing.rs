//! Model routing per analysis phase (ADR-012).

use super::invocation::ClaudeModel;

/// Coarse analysis phase used to route an invocation to a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Synthesis,
    ManifestSummarization,
    FeatureDiscovery,
    RiskAnalysis,
}

const ENTERPRISE_KEYWORDS: &[&str] = &[
    "auth",
    "rbac",
    "audit",
    "billing",
    "enterprise",
    "compliance",
    "security",
];

/// Select the model for a module/phase pair.
///
/// Synthesis always uses Opus; bulk classification (manifest summarization,
/// risk analysis) uses Sonnet; per-module feature discovery escalates to Opus
/// for security/enterprise-sensitive modules.
pub fn select_model(module_name: &str, phase: Phase) -> ClaudeModel {
    match phase {
        Phase::Synthesis => ClaudeModel::Opus,
        Phase::ManifestSummarization | Phase::RiskAnalysis => ClaudeModel::Sonnet,
        Phase::FeatureDiscovery => {
            let lowered = module_name.to_lowercase();
            if ENTERPRISE_KEYWORDS.iter().any(|kw| lowered.contains(kw)) {
                ClaudeModel::Opus
            } else {
                ClaudeModel::Sonnet
            }
        }
    }
}
