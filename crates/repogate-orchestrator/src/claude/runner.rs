//! The [`SessionRunner`] abstraction over Claude Code execution.
//!
//! Pipeline phases depend on this trait rather than [`run_session`] directly, so
//! tests can inject a mock runner returning canned output with no live calls.

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
