//! License detection from license files and inline SPDX headers.
//!
//! ADR-006 selects `askalono` for production-grade license-text matching against
//! the full SPDX corpus. askalono requires shipping a multi-megabyte license
//! cache, so for the MVP we use a lightweight signature-phrase matcher here and
//! treat askalono as a drop-in upgrade behind [`identify_license_text`]. SPDX
//! header detection is exact and needs no corpus.

use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::LicensingError;

/// How a license was detected for a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum DetectionMethod {
    LicenseFile,
    SpdxHeader,
}

/// A single license detection result for one file.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LicenseDetection {
    pub file_path: String,
    pub spdx_expression: String,
    pub confidence: f32,
    pub detection_method: DetectionMethod,
    pub needs_review: bool,
}

const REVIEW_THRESHOLD: f32 = 0.75;

/// Detect licenses across a repository: license files plus inline SPDX headers.
pub async fn detect_licenses(repo_path: &Path) -> Result<Vec<LicenseDetection>, LicensingError> {
    let mut detections = Vec::new();

    for entry in walkdir::WalkDir::new(repo_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let file_name = entry.file_name().to_string_lossy().to_uppercase();
        let is_license_file = file_name.starts_with("LICENSE")
            || file_name.starts_with("LICENCE")
            || file_name.starts_with("COPYING")
            || file_name.starts_with("NOTICE");

        if is_license_file {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                if let Some((spdx_id, confidence)) = identify_license_text(&content) {
                    detections.push(LicenseDetection {
                        file_path: entry.path().to_string_lossy().to_string(),
                        spdx_expression: spdx_id,
                        confidence,
                        detection_method: DetectionMethod::LicenseFile,
                        needs_review: confidence < REVIEW_THRESHOLD,
                    });
                }
            }
            continue;
        }

        if is_source_file(entry.path()) {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                for (i, line) in content.lines().enumerate() {
                    if i >= 30 {
                        break;
                    }
                    if let Some(expr) = extract_spdx_header(line) {
                        detections.push(LicenseDetection {
                            file_path: entry.path().to_string_lossy().to_string(),
                            spdx_expression: expr,
                            confidence: 0.95,
                            detection_method: DetectionMethod::SpdxHeader,
                            needs_review: false,
                        });
                        break;
                    }
                }
            }
        }
    }

    Ok(detections)
}

fn is_source_file(path: &Path) -> bool {
    path.extension()
        .map(|ext| {
            matches!(
                ext.to_string_lossy().as_ref(),
                "rs" | "ts" | "js" | "py" | "go" | "java" | "rb" | "c" | "cpp" | "h"
            )
        })
        .unwrap_or(false)
}

/// Extract an SPDX expression from a `SPDX-License-Identifier:` comment line.
pub(crate) fn extract_spdx_header(line: &str) -> Option<String> {
    line.split("SPDX-License-Identifier:")
        .nth(1)
        .map(|s| {
            s.trim()
                .trim_end_matches(['*', '/', '-'])
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
}

/// Identify a license from its full text, returning the SPDX id and a
/// confidence score.
///
/// With the `askalono-corpus` feature enabled and a corpus directory configured
/// via `REPOGATE_ASKALONO_CACHE`, this matches the text against SPDX license
/// texts by token-cosine similarity (ADR-016 Remediation 3) and returns the best
/// match above [`REVIEW_THRESHOLD`]; otherwise (or on a low score) it uses the
/// signature-phrase heuristic. The heuristic is always the fallback.
#[cfg(feature = "askalono-corpus")]
pub(crate) fn identify_license_text(text: &str) -> Option<(String, f32)> {
    if let Some((name, score)) = corpus_best_match(text) {
        if score >= REVIEW_THRESHOLD {
            return Some((name, score));
        }
    }
    heuristic_identify_license_text(text)
}

/// Heuristic-only identification (default build, or corpus fallback).
#[cfg(not(feature = "askalono-corpus"))]
pub(crate) fn identify_license_text(text: &str) -> Option<(String, f32)> {
    heuristic_identify_license_text(text)
}

/// Best corpus match by token-cosine similarity, loading SPDX license texts
/// (`<SPDX-ID>.txt`) from the directory in `REPOGATE_ASKALONO_CACHE`.
#[cfg(feature = "askalono-corpus")]
pub(crate) fn corpus_best_match(text: &str) -> Option<(String, f32)> {
    let dir = std::env::var("REPOGATE_ASKALONO_CACHE").ok()?;
    let query = normalize_tokens(text);
    let mut best: Option<(String, f32)> = None;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let Some(spdx_id) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Ok(corpus_text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let score = cosine_similarity(&query, &normalize_tokens(&corpus_text));
        if best.as_ref().map(|(_, s)| score > *s).unwrap_or(true) {
            best = Some((spdx_id.to_string(), score));
        }
    }
    best
}

/// Normalize license text into a term-frequency map of lowercased word tokens.
#[cfg(feature = "askalono-corpus")]
fn normalize_tokens(text: &str) -> std::collections::HashMap<String, f32> {
    let mut counts: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
    for raw in text.split(|c: char| !c.is_alphanumeric()) {
        if raw.is_empty() {
            continue;
        }
        *counts.entry(raw.to_lowercase()).or_insert(0.0) += 1.0;
    }
    counts
}

/// Cosine similarity between two term-frequency maps.
#[cfg(feature = "askalono-corpus")]
fn cosine_similarity(
    a: &std::collections::HashMap<String, f32>,
    b: &std::collections::HashMap<String, f32>,
) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let dot: f32 = a
        .iter()
        .filter_map(|(k, va)| b.get(k).map(|vb| va * vb))
        .sum();
    let norm_a: f32 = a.values().map(|v| v * v).sum::<f32>().sqrt();
    let norm_b: f32 = b.values().map(|v| v * v).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

/// Best-effort identification of a license from its full text by matching
/// distinctive signature phrases. The askalono-corpus feature upgrades this.
pub(crate) fn heuristic_identify_license_text(text: &str) -> Option<(String, f32)> {
    let upper = text.to_uppercase();
    let has = |needle: &str| upper.contains(needle);

    // Order matters: more specific signatures first.
    let id = if has("GNU AFFERO GENERAL PUBLIC LICENSE") {
        "AGPL-3.0"
    } else if has("GNU LESSER GENERAL PUBLIC LICENSE") {
        "LGPL-3.0"
    } else if has("GNU GENERAL PUBLIC LICENSE") && has("VERSION 3") {
        "GPL-3.0"
    } else if has("GNU GENERAL PUBLIC LICENSE") && has("VERSION 2") {
        "GPL-2.0"
    } else if has("MOZILLA PUBLIC LICENSE") && has("2.0") {
        "MPL-2.0"
    } else if has("APACHE LICENSE") && has("VERSION 2.0") {
        "Apache-2.0"
    } else if has("BUSINESS SOURCE LICENSE") {
        "BSL-1.1"
    } else if has("PERMISSION IS HEREBY GRANTED, FREE OF CHARGE") {
        "MIT"
    } else if has("REDISTRIBUTION AND USE IN SOURCE AND BINARY") && has("NEITHER THE NAME") {
        "BSD-3-Clause"
    } else if has("REDISTRIBUTION AND USE IN SOURCE AND BINARY") {
        "BSD-2-Clause"
    } else if has("THIS IS FREE AND UNENCUMBERED SOFTWARE") {
        "Unlicense"
    } else {
        return None;
    };

    Some((id.to_string(), 0.9))
}
