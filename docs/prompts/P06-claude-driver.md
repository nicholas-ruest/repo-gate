# P06 — `repogate-orchestrator`: Claude Code Subprocess Driver

## Context

RepoGate is a deep repository assessment platform using Claude Code as the reasoning engine.

**You are implementing exactly ONE build unit: the lowest-level Claude Code integration (subprocess, streaming, schema enforcement).** Do not start work on other units. Build and tests must be green before this prompt is considered done.

**Prerequisites:** P02 (core types), P03 (ingestion basics) are complete.

---

## Phase & Dependencies

- **Phase:** Orchestration core
- **Depends on:** P02, P03

---

## Scope & Deliverables

Implement `repogate-orchestrator/src/claude/` module for headless Claude invocation.

### File: `src/claude/invocation.rs` — Invocation Builder

```rust
#[derive(Debug, Clone)]
pub enum ClaudeModel {
    Opus,   // claude-opus-4-8
    Sonnet, // claude-sonnet-4-6
}

#[derive(Debug, Clone)]
pub struct ClaudeInvocation {
    pub prompt: String,
    pub model: ClaudeModel,
    pub schema_path: Option<std::path::PathBuf>,
    pub allowed_tools: Vec<String>,
    pub system_prompt: Option<String>,
    pub working_dir: Option<std::path::PathBuf>,
    pub session_id: Option<String>,
}

impl ClaudeInvocation {
    pub fn build_command(&self) -> std::process::Command {
        let model_id = match self.model {
            ClaudeModel::Opus => "claude-opus-4-8",
            ClaudeModel::Sonnet => "claude-sonnet-4-6",
        };
        
        let mut cmd = std::process::Command::new("claude");
        cmd.arg("--bare")
            .arg("-p")
            .arg(&self.prompt);
        
        cmd.arg("--output-format").arg("stream-json");
        
        if let Some(schema_path) = &self.schema_path {
            cmd.arg("--json-schema").arg(schema_path);
        }
        
        let tools = self.allowed_tools.join(",");
        if !tools.is_empty() {
            cmd.arg("--allowedTools").arg(tools);
        }
        
        if let Some(sys) = &self.system_prompt {
            cmd.arg("--append-system-prompt").arg(sys);
        }
        
        cmd.arg("--model").arg(model_id);
        
        if let Some(session) = &self.session_id {
            cmd.arg("--resume").arg(session);
        }
        
        if let Some(wd) = &self.working_dir {
            cmd.current_dir(wd);
        }
        
        cmd
    }
}
```

### File: `src/claude/stream.rs` — Stream Parsing

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeEvent {
    #[serde(rename = "init")]
    Init { session_id: String },
    
    #[serde(rename = "assistant")]
    Assistant { content: String },
    
    #[serde(rename = "tool_result")]
    ToolResult { result: serde_json::Value },
    
    #[serde(rename = "result")]
    Result { content: String, usage: UsageStats },
    
    #[serde(rename = "error")]
    Error { message: String, code: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
}

pub struct StreamParser;

impl StreamParser {
    pub fn parse_stream(reader: impl std::io::BufRead) -> impl Iterator<Item = Result<ClaudeEvent, StreamError>> {
        reader.lines().filter_map(move |line| {
            let line = line.ok()?;
            if line.trim().is_empty() {
                return None;
            }
            match serde_json::from_str::<ClaudeEvent>(&line) {
                Ok(event) => Some(Ok(event)),
                Err(e) => Some(Err(StreamError::DeserializeFailed(e.to_string()))),
            }
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("deserialize failed: {0}")]
    DeserializeFailed(String),
    
    #[error("stream ended unexpectedly")]
    Truncated,
}
```

### File: `src/claude/session.rs` — Session Execution

```rust
pub struct SessionResult {
    pub session_id: String,
    pub output: String,
    pub usage: UsageStats,
}

pub async fn run_session(invocation: ClaudeInvocation) -> Result<SessionResult, OrchestratorError> {
    let mut cmd = invocation.build_command();
    let mut child = cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| OrchestratorError(format!("spawn failed: {e}")))?;
    
    let stdout = child.stdout.take().ok_or_else(|| 
        OrchestratorError("no stdout".into()))?;
    
    let reader = std::io::BufReader::new(stdout);
    let mut session_id = String::new();
    let mut final_output = String::new();
    let mut usage = UsageStats {
        input_tokens: 0,
        output_tokens: 0,
        cache_read_input_tokens: 0,
    };
    
    for event in stream::StreamParser::parse_stream(reader) {
        match event {
            Ok(stream::ClaudeEvent::Init { session_id: sid }) => {
                session_id = sid;
            }
            Ok(stream::ClaudeEvent::Result { content, usage: u }) => {
                final_output = content;
                usage = u;
            }
            Ok(stream::ClaudeEvent::Error { message, code }) => {
                return Err(OrchestratorError(format!("session error: {code}: {message}")));
            }
            _ => {}
        }
    }
    
    let status = child.wait().await
        .map_err(|e| OrchestratorError(format!("wait failed: {e}")))?;
    
    if !status.success() {
        return Err(OrchestratorError(format!("exit code: {}", status.code().unwrap_or(-1))));
    }
    
    Ok(SessionResult {
        session_id,
        output: final_output,
        usage,
    })
}
```

### File: `src/claude/routing.rs` — Model Selection

```rust
#[derive(Debug, Clone, Copy)]
pub enum Phase {
    Synthesis,
    ManifestSummarization,
    FeatureDiscovery,
    RiskAnalysis,
}

pub fn select_model(module_name: &str, phase: Phase) -> ClaudeModel {
    match phase {
        Phase::Synthesis => ClaudeModel::Opus,
        Phase::ManifestSummarization => ClaudeModel::Sonnet,
        Phase::RiskAnalysis => ClaudeModel::Sonnet,
        Phase::FeatureDiscovery => {
            // Use Opus for large/complex/enterprise modules
            let enterprise_keywords = ["auth", "rbac", "audit", "billing", "enterprise", "compliance", "security"];
            if enterprise_keywords.iter().any(|kw| module_name.to_lowercase().contains(kw)) {
                ClaudeModel::Opus
            } else {
                ClaudeModel::Sonnet
            }
        }
    }
}
```

### File: `src/claude/schema.rs` — Schema Export

```rust
pub fn write_phase_schema(phase: Phase, dir: &std::path::Path) -> Result<std::path::PathBuf, SchemaError> {
    use repogate_core::JsonSchema;
    
    let path = dir.join(format!("{:?}-schema.json", phase));
    
    match phase {
        Phase::Synthesis => {
            repogate_core::write_schema::<repogate_core::SynthesisOutput>(&path)?;
        }
        Phase::FeatureDiscovery => {
            repogate_core::write_schema::<repogate_core::ModuleAssessment>(&path)?;
        }
        Phase::RiskAnalysis => {
            // Similar pattern for risk output schema
        }
        _ => {}
    }
    
    Ok(path)
}

#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("schema write failed: {0}")]
    WriteFailed(String),
}
```

### File: `src/lib.rs`

```rust
pub mod claude;

#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("orchestrator error: {0}")]
    SessionFailed(String),
    
    #[error("schema violation: {0}")]
    SchemaViolation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_command_contains_bare() {
        let inv = claude::invocation::ClaudeInvocation {
            prompt: "test".to_string(),
            model: claude::invocation::ClaudeModel::Opus,
            schema_path: None,
            allowed_tools: vec![],
            system_prompt: None,
            working_dir: None,
            session_id: None,
        };
        let cmd = inv.build_command();
        // Verify command structure (integration test required for full verification)
    }

    #[test]
    fn parse_canned_json() {
        let json = r#"{"type": "init", "session_id": "test-123"}"#;
        let event: claude::stream::ClaudeEvent = serde_json::from_str(json).unwrap();
        if let claude::stream::ClaudeEvent::Init { session_id } = event {
            assert_eq!(session_id, "test-123");
        }
    }

    #[test]
    fn select_model_synthesis() {
        let model = claude::routing::select_model("any", claude::routing::Phase::Synthesis);
        assert!(matches!(model, claude::invocation::ClaudeModel::Opus));
    }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-002-claude-code-analysis-engine.md`** — Claude Code as engine
- **`docs/adr/ADR-003-headless-claude-code-invocation.md`** — Invocation flags, subprocess
- **`docs/adr/ADR-007-schema-enforced-structured-output.md`** — Schema enforcement
- **`docs/adr/ADR-012-model-routing.md`** — Model selection strategy

---

## Acceptance Criteria

- ✅ `cargo test -p repogate-orchestrator` passes (mock command output; no live API)
- ✅ `build_command()` contains `--bare`, `--output-format stream-json`, `--json-schema`, `--allowedTools`
- ✅ `parse_stream` deserializes `Init`, `Result`, `Error` from newline-delimited JSON
- ✅ `select_model("auth", Synthesis)` → `Opus`; `select_model("utils", ManifestSummarization)` → `Sonnet`
- ✅ No live Claude calls in tests (mock canned JSON)

---

## Language

**Rust** — Subprocess command building, JSON stream parsing, subprocess execution.

---

## Out-of-Scope

- Do NOT implement Claude Code CLI authentication
- Do NOT call live Claude API; mock all tests
- Do NOT implement result storage or persistence
