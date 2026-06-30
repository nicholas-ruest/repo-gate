//! Parsing of the `--output-format stream-json` event stream.
//!
//! The event shapes match the `claude` CLI's actual stream-json output: a
//! `system`/`init` event carrying the session id, `assistant` events, and a
//! terminal `result` event carrying the text (`result`), optional
//! schema-validated `structured_output`, usage, and an `is_error` flag.

use std::io::BufRead;

use serde::{Deserialize, Serialize};

/// One newline-delimited event emitted by `claude --output-format stream-json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeEvent {
    /// System lifecycle event; `subtype: "init"` carries the session id.
    #[serde(rename = "system")]
    System {
        #[serde(default)]
        subtype: Option<String>,
        #[serde(default)]
        session_id: Option<String>,
    },

    #[serde(rename = "assistant")]
    Assistant {
        #[serde(default)]
        session_id: Option<String>,
    },

    #[serde(rename = "user")]
    User {
        #[serde(default)]
        session_id: Option<String>,
    },

    /// Terminal event with the final text, optional structured output, and usage.
    #[serde(rename = "result")]
    Result {
        #[serde(default)]
        subtype: Option<String>,
        #[serde(default)]
        is_error: bool,
        #[serde(default)]
        result: Option<String>,
        /// Schema-validated object when `--json-schema` was used.
        #[serde(default)]
        structured_output: Option<serde_json::Value>,
        #[serde(default)]
        session_id: Option<String>,
        #[serde(default)]
        usage: UsageStats,
    },

    /// Any other event type is ignored.
    #[serde(other)]
    Other,
}

/// Token-usage accounting reported in the terminal `result` event. Unknown
/// fields in the CLI's usage object are ignored.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageStats {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

/// Stateless parser over a buffered reader of newline-delimited JSON events.
pub struct StreamParser;

impl StreamParser {
    /// Parse each non-empty line into a [`ClaudeEvent`].
    pub fn parse_stream(
        reader: impl BufRead,
    ) -> impl Iterator<Item = Result<ClaudeEvent, StreamError>> {
        reader.lines().filter_map(|line| {
            let line = line.ok()?;
            if line.trim().is_empty() {
                return None;
            }
            Some(
                serde_json::from_str::<ClaudeEvent>(&line)
                    .map_err(|e| StreamError::DeserializeFailed(e.to_string())),
            )
        })
    }
}

/// Errors produced while parsing the event stream.
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("deserialize failed: {0}")]
    DeserializeFailed(String),

    #[error("stream ended unexpectedly")]
    Truncated,
}
