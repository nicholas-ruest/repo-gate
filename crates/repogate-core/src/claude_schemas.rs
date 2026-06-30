use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::model::SourceLocation;

/// How a capability was discovered during analysis.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum DiscoveryMethod {
    PublicApi,
    TestCoverage,
    ExampleCode,
    CliInspection,
    SourceTracing,
    ConfigAnalysis,
    DocumentationCross,
    LlmInference,
}

/// A single capability finding produced by a Claude Code session.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CapabilityFinding {
    pub name: String,
    pub description: String,
    pub is_enterprise: bool,
    pub is_undocumented: bool,
    pub discovery_method: DiscoveryMethod,
    pub source_locations: Option<Vec<SourceLocation>>,
}

/// Structured output Claude Code produces for one module's assessment phase.
///
/// This struct is exported as JSON Schema and passed to `--json-schema` to
/// constrain Claude Code's output (see ADR-007).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModuleAssessment {
    pub module_name: String,
    pub module_path: String,
    pub capabilities: Vec<CapabilityFinding>,
    pub commercial_value_estimate: Option<f32>,
    pub estimated_tier: Option<String>,
    pub risks: Vec<String>,
}

/// Structured output for the synthesis phase (gating strategy generation).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SynthesisOutput {
    pub gating_strategy: Option<String>,
    /// Flexible tier assignment objects as returned by Claude Code.
    pub tier_assignments: Option<Vec<serde_json::Value>>,
}

/// Write the JSON Schema for type `T` to `path`.
///
/// The schema is derived from the `schemars::JsonSchema` implementation of `T`
/// and serialized as pretty-printed JSON. Used by the orchestrator to generate
/// schema files passed to `claude --json-schema` (ADR-007).
pub fn write_schema<T: schemars::JsonSchema>(path: &std::path::Path) -> anyhow::Result<()> {
    let schema = schemars::schema_for!(T);
    let json = serde_json::to_string_pretty(&schema)?;
    std::fs::write(path, json)?;
    Ok(())
}
