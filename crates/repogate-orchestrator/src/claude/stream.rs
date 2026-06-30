//! Parsing of the `--output-format stream-json` event stream.

use std::io::BufRead;

use serde::{Deserialize, Serialize};

/// One newline-delimited event emitted by `claude --output-format stream-json`.
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

/// Token-usage accounting reported in the terminal `result` event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageStats {
    pub input_tokens: u64,
    pub output_tokens: u64,
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
