//! Headless Claude Code integration: invocation building, stream parsing,
//! session execution, model routing, and schema export.

pub mod invocation;
pub mod routing;
pub mod runner;
pub mod schema;
pub mod session;
pub mod stream;

pub use invocation::{ClaudeInvocation, ClaudeModel};
pub use routing::{select_model, Phase};
pub use runner::{
    run_structured, ClaudeCliRunner, DeterministicMockRunner, SessionRunner, Structured,
};
pub use schema::{write_phase_schema, SchemaError};
pub use session::{run_session, SessionResult};
pub use stream::{ClaudeEvent, StreamError, StreamParser, UsageStats};
