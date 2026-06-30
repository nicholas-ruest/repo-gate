//! Canonical JSON output (the durable artifact, ADR-011).

use repogate_core::Assessment;

use crate::ReportError;

/// Write an assessment as pretty JSON to `writer`.
pub fn write_json(assessment: &Assessment, writer: impl std::io::Write) -> Result<(), ReportError> {
    serde_json::to_writer_pretty(writer, assessment)?;
    Ok(())
}

/// Serialize an assessment to pretty-printed JSON bytes.
pub fn to_json_bytes(assessment: &Assessment) -> Result<Vec<u8>, ReportError> {
    Ok(serde_json::to_vec_pretty(assessment)?)
}
