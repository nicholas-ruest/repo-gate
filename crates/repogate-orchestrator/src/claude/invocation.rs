//! Construction of a headless `claude` CLI invocation (ADR-003).

use std::path::PathBuf;
use std::process::Command;

/// Which Claude model to route an invocation to (ADR-012).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaudeModel {
    /// `claude-opus-4-8` — deep reasoning and final synthesis.
    Opus,
    /// `claude-sonnet-4-6` — bulk classification.
    Sonnet,
}

impl ClaudeModel {
    /// The CLI model identifier.
    pub fn model_id(self) -> &'static str {
        match self {
            ClaudeModel::Opus => "claude-opus-4-8",
            ClaudeModel::Sonnet => "claude-sonnet-4-6",
        }
    }
}

/// A fully-specified headless Claude Code invocation.
#[derive(Debug, Clone)]
pub struct ClaudeInvocation {
    pub prompt: String,
    pub model: ClaudeModel,
    pub schema_path: Option<PathBuf>,
    pub allowed_tools: Vec<String>,
    pub system_prompt: Option<String>,
    pub working_dir: Option<PathBuf>,
    pub session_id: Option<String>,
}

impl ClaudeInvocation {
    /// Build the `claude` command for this invocation.
    ///
    /// Emits: `claude --bare -p <prompt> --output-format stream-json
    /// [--json-schema <path>] [--allowedTools <csv>] [--append-system-prompt
    /// <sys>] --model <id> [--resume <session>]`.
    pub fn build_command(&self) -> Command {
        let mut cmd = Command::new("claude");
        cmd.arg("--bare").arg("-p").arg(&self.prompt);
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

        cmd.arg("--model").arg(self.model.model_id());

        if let Some(session) = &self.session_id {
            cmd.arg("--resume").arg(session);
        }

        if let Some(wd) = &self.working_dir {
            cmd.current_dir(wd);
        }

        cmd
    }
}
