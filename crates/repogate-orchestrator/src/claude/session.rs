//! Execution of a single Claude Code session.

use std::io::BufReader;

use super::invocation::ClaudeInvocation;
use super::stream::{ClaudeEvent, StreamParser, UsageStats};
use crate::OrchestratorError;

/// The outcome of a completed Claude Code session.
#[derive(Debug, Clone)]
pub struct SessionResult {
    pub session_id: String,
    pub output: String,
    pub usage: UsageStats,
}

/// Run a Claude Code session to completion and return its structured result.
///
/// The full stdout is collected and then parsed as newline-delimited JSON. A
/// terminal `error` event or a non-zero exit status yields
/// [`OrchestratorError::SessionFailed`].
pub async fn run_session(invocation: ClaudeInvocation) -> Result<SessionResult, OrchestratorError> {
    let std_cmd = invocation.build_command();
    let output = tokio::process::Command::from(std_cmd)
        .output()
        .await
        .map_err(|e| OrchestratorError::SessionFailed(format!("spawn failed: {e}")))?;

    let result = parse_session_output(&output.stdout)?;

    if !output.status.success() {
        return Err(OrchestratorError::SessionFailed(format!(
            "claude exited with status {}",
            output.status.code().unwrap_or(-1)
        )));
    }

    Ok(result)
}

/// Parse collected stdout bytes into a [`SessionResult`].
///
/// Pulled out of [`run_session`] so it can be unit-tested with canned output
/// and no live process.
pub(crate) fn parse_session_output(stdout: &[u8]) -> Result<SessionResult, OrchestratorError> {
    let reader = BufReader::new(stdout);
    let mut session_id = String::new();
    let mut output = String::new();
    let mut usage = UsageStats::default();

    for event in StreamParser::parse_stream(reader) {
        match event {
            Ok(ClaudeEvent::Init { session_id: sid }) => session_id = sid,
            Ok(ClaudeEvent::Result {
                content,
                usage: used,
            }) => {
                output = content;
                usage = used;
            }
            Ok(ClaudeEvent::Error { message, code }) => {
                return Err(OrchestratorError::SessionFailed(format!(
                    "session error {code}: {message}"
                )));
            }
            Ok(_) => {}
            Err(e) => {
                return Err(OrchestratorError::SessionFailed(format!(
                    "stream parse error: {e}"
                )));
            }
        }
    }

    Ok(SessionResult {
        session_id,
        output,
        usage,
    })
}
