//! The [`SessionRunner`] abstraction over Claude Code execution, plus the
//! schema-enforced [`run_structured`] helper (ADR-016 Remediation 1).

use schemars::JsonSchema;
use serde::de::DeserializeOwned;

use super::invocation::ClaudeInvocation;
use super::session::{run_session, SessionResult};
use crate::OrchestratorError;

/// Runs a Claude Code invocation and returns its structured result.
#[allow(async_fn_in_trait)]
pub trait SessionRunner: Send + Sync {
    async fn run(&self, invocation: ClaudeInvocation) -> Result<SessionResult, OrchestratorError>;
}

/// Production runner that shells out to the `claude` CLI.
pub struct ClaudeCliRunner;

impl SessionRunner for ClaudeCliRunner {
    async fn run(&self, invocation: ClaudeInvocation) -> Result<SessionResult, OrchestratorError> {
        run_session(invocation).await
    }
}

/// A structured session result: the parsed value plus its usage accounting.
pub struct Structured<T> {
    pub value: T,
    pub usage: super::stream::UsageStats,
}

/// Run a Claude session with `--json-schema` enforcement and retry-then-surface
/// failure handling (ADR-016 Remediation 1).
///
/// The JSON Schema for `T` is written to a temp file and attached to the
/// invocation. The output is parsed into `T`; on a parse failure the session is
/// retried once, and a second failure yields [`OrchestratorError::SchemaViolation`]
/// — no silent fallback.
pub async fn run_structured<T>(
    runner: &impl SessionRunner,
    mut invocation: ClaudeInvocation,
) -> Result<Structured<T>, OrchestratorError>
where
    T: DeserializeOwned + JsonSchema,
{
    let schema = schemars::schema_for!(T);
    let schema_json = serde_json::to_string(&schema)
        .map_err(|e| OrchestratorError::SchemaViolation(format!("serialize schema: {e}")))?;
    invocation.schema_json = Some(schema_json);

    let mut last_err = String::new();
    for attempt in 0..2 {
        let result = runner.run(invocation.clone()).await?;
        match serde_json::from_str::<T>(&result.output) {
            Ok(value) => {
                return Ok(Structured {
                    value,
                    usage: result.usage,
                })
            }
            Err(e) => {
                last_err = e.to_string();
                if attempt == 0 {
                    tracing::warn!(error = %last_err, "structured output failed to parse; retrying once");
                }
            }
        }
    }
    Err(OrchestratorError::SchemaViolation(last_err))
}

/// A deterministic, phase-aware mock runner for offline/CI use (no live Claude).
///
/// It inspects the invocation prompt and returns schema-valid canned JSON for
/// the detected phase, so the full pipeline completes without degradation.
pub struct DeterministicMockRunner;

impl SessionRunner for DeterministicMockRunner {
    async fn run(&self, invocation: ClaudeInvocation) -> Result<SessionResult, OrchestratorError> {
        let prompt = &invocation.prompt;
        let output = if prompt.contains("ModuleAssessment") {
            // Extract the module name from "Analyze module `NAME` at ...".
            let name = prompt.split('`').nth(1).unwrap_or("module").to_string();
            canned_module_assessment(&name)
        } else if prompt.contains("ModuleNode") {
            // Architecture mapping enriches heuristic candidates; an empty array
            // simply means "no refinement", which is not a degradation.
            "[]".to_string()
        } else if prompt.contains("SynthesisOutput") {
            r#"{"gating_strategy":"Keep the core open; gate enterprise features.","tier_assignments":null}"#.to_string()
        } else if prompt.contains("RiskAnalysisOutput") {
            r#"{"risks":[]}"#.to_string()
        } else {
            "{}".to_string()
        };

        Ok(SessionResult {
            session_id: "mock".to_string(),
            output,
            usage: super::stream::UsageStats::default(),
        })
    }
}

fn canned_module_assessment(name: &str) -> String {
    // A full ModuleAssessment with all eight commercial dimensions populated, so
    // scoring uses real per-dimension values (no uniform fallback).
    format!(
        r#"{{
          "module_name": "{name}",
          "module_path": "src/{name}",
          "capabilities": [],
          "commercial_score": {{
            "adoption_value": 5.0,
            "enterprise_buyer_value": 6.0,
            "commercial_leverage": 5.0,
            "competitive_sensitivity": 4.0,
            "operational_value": 5.0,
            "security_sensitivity": 3.0,
            "support_burden": 4.0,
            "strategic_importance": 6.0
          }},
          "commercial_value_estimate": 5.5,
          "estimated_tier": "ProTier",
          "risks": []
        }}"#
    )
}
