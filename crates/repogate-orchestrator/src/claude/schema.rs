//! Per-phase JSON Schema export for `--json-schema` enforcement (ADR-007).

use std::path::{Path, PathBuf};

use super::routing::Phase;

/// Write the JSON Schema for `phase`'s expected structured output into `dir`,
/// returning the path of the written file.
pub fn write_phase_schema(phase: Phase, dir: &Path) -> Result<PathBuf, SchemaError> {
    let path = dir.join(format!("{phase:?}-schema.json"));

    let result = match phase {
        Phase::Synthesis => repogate_core::write_schema::<repogate_core::SynthesisOutput>(&path),
        // Feature discovery and the manifest-summarization pass both emit
        // module-level assessments; risk analysis reuses the same shape until a
        // dedicated risk schema lands (P11).
        Phase::FeatureDiscovery | Phase::ManifestSummarization | Phase::RiskAnalysis => {
            repogate_core::write_schema::<repogate_core::ModuleAssessment>(&path)
        }
    };

    result.map_err(|e| SchemaError::WriteFailed(e.to_string()))?;
    Ok(path)
}

/// Errors produced while exporting a phase schema.
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("schema write failed: {0}")]
    WriteFailed(String),
}
