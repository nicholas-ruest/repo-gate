# P05 — `repogate-licensing`: License Detection, SPDX Parsing, Copyleft Risk Matrix

## Context

RepoGate is a deep repository assessment platform that analyzes full open-source codebases to determine what should remain open source versus become part of commercial tiers, using Claude Code as the reasoning engine and Rust as the primary implementation language.

**You are implementing exactly ONE build unit: license detection and copyleft risk analysis.** Do not start work on other units. Build and tests must be green before this prompt is considered done.

**Prerequisites:** P02 (core types), P04 (dependencies) are complete.

---

## Phase & Dependencies

- **Phase:** Ingestion (parallel with P03/P04)
- **Depends on:** P02, P04

---

## Scope & Deliverables

Implement `repogate-licensing` crate for license analysis.

### File: `src/detect.rs` — License Detection

```rust
use serde::{Serialize, Deserialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum DetectionMethod {
    LicenseFile,
    SpdxHeader,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LicenseDetection {
    pub file_path: String,
    pub spdx_expression: String,
    pub confidence: f32,  // 0.0–1.0
    pub detection_method: DetectionMethod,
    pub needs_review: bool,  // confidence < 0.75
}

pub async fn detect_licenses(repo_path: &std::path::Path) -> Result<Vec<LicenseDetection>, LicensingError> {
    let mut detections = Vec::new();
    
    // Look for LICENSE*, COPYING*, NOTICE*, LICENCE* files
    for entry in walkdir::WalkDir::new(repo_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let file_name = entry.file_name().to_string_lossy().to_uppercase();
        if file_name.starts_with("LICENSE") || file_name.starts_with("COPYING") 
            || file_name.starts_with("NOTICE") || file_name.starts_with("LICENCE") {
            
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                // Use askalono to detect license
                if let Ok(license_info) = askalono::identify(&content) {
                    detections.push(LicenseDetection {
                        file_path: entry.path().to_string_lossy().to_string(),
                        spdx_expression: license_info.name.to_string(),
                        confidence: license_info.confidence as f32,
                        detection_method: DetectionMethod::LicenseFile,
                        needs_review: license_info.confidence < 0.75,
                    });
                }
            }
        }
    }
    
    // Scan first 30 lines of source files for SPDX-License-Identifier
    for entry in walkdir::WalkDir::new(repo_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| is_source_file(e.path()))
    {
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            for (i, line) in content.lines().enumerate() {
                if i >= 30 { break; }
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
    
    Ok(detections)
}

fn is_source_file(path: &std::path::Path) -> bool {
    if let Some(ext) = path.extension() {
        matches!(ext.to_string_lossy().as_ref(), "rs" | "ts" | "js" | "py" | "go" | "java" | "rb")
    } else {
        false
    }
}

fn extract_spdx_header(line: &str) -> Option<String> {
    if line.contains("SPDX-License-Identifier:") {
        line.split("SPDX-License-Identifier:").nth(1).map(|s| s.trim().to_string())
    } else {
        None
    }
}
```

### File: `src/spdx.rs` — SPDX Expression Parsing

```rust
use spdx::Expression;

pub fn parse_and_normalize(expr_str: &str) -> Result<String, LicensingError> {
    let expr = Expression::parse(expr_str)
        .map_err(|e| LicensingError::SpdxParseFailed(e.to_string()))?;
    
    // Return normalized expression
    Ok(expr.to_string())
}

pub fn extract_base_identifiers(expr_str: &str) -> Result<Vec<String>, LicensingError> {
    let expr = Expression::parse(expr_str)?;
    
    // Walk the AST to collect all license IDs
    let mut ids = Vec::new();
    collect_ids(&expr, &mut ids);
    
    Ok(ids)
}

fn collect_ids(expr: &Expression, ids: &mut Vec<String>) {
    // Recursively extract license identifiers from expression tree
}
```

### File: `src/copyleft.rs` — Copyleft Classification

```rust
use serde::{Serialize, Deserialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum CopyleftTier {
    StrongCopyleft,      // GPL-3.0, AGPL-3.0
    WeakCopyleft,        // LGPL, MPL-2.0
    SourceAvailableNonOsi, // BSL-1.1, EUPLv1.1
    Permissive,          // MIT, Apache-2.0, BSD
    PublicDomain,        // Unlicense, CC0
    Unknown,
}

pub fn classify_license(spdx_id: &str) -> CopyleftTier {
    match spdx_id {
        "GPL-3.0" | "GPL-3.0-or-later" | "AGPL-3.0" | "AGPL-3.0-or-later" => CopyleftTier::StrongCopyleft,
        "GPL-2.0" | "GPL-2.0-or-later" => CopyleftTier::StrongCopyleft,
        "LGPL-2.1" | "LGPL-3.0" | "MPL-2.0" => CopyleftTier::WeakCopyleft,
        "BSL-1.1" => CopyleftTier::SourceAvailableNonOsi,
        "MIT" | "Apache-2.0" | "BSD-2-Clause" | "BSD-3-Clause" => CopyleftTier::Permissive,
        "Unlicense" | "CC0-1.0" => CopyleftTier::PublicDomain,
        _ => CopyleftTier::Unknown,
    }
}

pub fn copyleft_risk_score(tier: CopyleftTier) -> f32 {
    match tier {
        CopyleftTier::StrongCopyleft => 9.0,
        CopyleftTier::WeakCopyleft => 4.0,
        CopyleftTier::SourceAvailableNonOsi => 3.0,
        CopyleftTier::Permissive => 0.0,
        CopyleftTier::PublicDomain => 0.0,
        CopyleftTier::Unknown => 2.0,
    }
}
```

### File: `src/report.rs` — License Report

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LicenseReport {
    pub repo_id: String,
    pub detections: Vec<LicenseDetection>,
    pub dependency_licenses: Vec<(String, String)>,  // (dep_name, spdx_expr)
    pub copyleft_exposure: f32,  // 0.0–10.0
    pub missing_licenses: bool,
    pub conflicts: Vec<String>,
    pub overall_risk_score: f32,  // 0.0–10.0
}

pub fn build_report(
    repo_id: &str,
    detections: Vec<LicenseDetection>,
    deps: &[repogate_ingestion::DependencyRecord],
) -> Result<LicenseReport, LicensingError> {
    // Analyze detections for copyleft, conflicts
    let mut copyleft_exposure = 0.0;
    let mut overall_risk = 0.0;
    let mut dep_licenses = Vec::new();
    
    for detection in &detections {
        let tier = classify_license(&detection.spdx_expression);
        copyleft_exposure = copyleft_exposure.max(copyleft_risk_score(tier));
    }
    
    for dep in deps {
        if let Some(license) = &dep.spdx_license {
            dep_licenses.push((dep.name.clone(), license.clone()));
            let tier = classify_license(license);
            overall_risk = overall_risk.max(copyleft_risk_score(tier));
        }
    }
    
    Ok(LicenseReport {
        repo_id: repo_id.to_string(),
        detections,
        dependency_licenses: dep_licenses,
        copyleft_exposure,
        missing_licenses: overall_risk == 0.0 && detections.is_empty(),
        conflicts: vec![],  // TODO: detect mixed-license conflicts
        overall_risk_score: overall_risk,
    })
}
```

### File: `src/lib.rs`

```rust
#![doc = "RepoGate license detection and copyleft analysis."]

pub mod detect;
pub mod spdx;
pub mod copyleft;
pub mod report;

pub use detect::{LicenseDetection, DetectionMethod};
pub use copyleft::CopyleftTier;
pub use report::LicenseReport;

#[derive(Debug, thiserror::Error)]
pub enum LicensingError {
    #[error("SPDX parse failed: {0}")]
    SpdxParseFailed(String),
    
    #[error("license detection failed")]
    DetectionFailed,
    
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub async fn analyze(
    manifest: &repogate_ingestion::RepoManifest,
    repo_path: &std::path::Path,
) -> Result<LicenseReport, LicensingError> {
    let detections = detect::detect_licenses(repo_path).await?;
    let report = report::build_report(&manifest.repo_id, detections, &manifest.dependencies)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_gpl3() {
        assert_eq!(copyleft::classify_license("GPL-3.0"), CopyleftTier::StrongCopyleft);
    }

    #[test]
    fn classify_mit() {
        assert_eq!(copyleft::classify_license("MIT"), CopyleftTier::Permissive);
    }

    #[test]
    fn classify_bsl() {
        assert_eq!(copyleft::classify_license("BSL-1.1"), CopyleftTier::SourceAvailableNonOsi);
    }

    #[test]
    fn risk_score_agpl() {
        let score = copyleft::copyleft_risk_score(CopyleftTier::StrongCopyleft);
        assert!(score >= 8.0);
    }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-006-license-dependency-analysis.md`** — `askalono`, `spdx` parsing, copyleft classification
- **`docs/adr/ADR-010-commercial-value-scoring-model.md`** — License risk sub-score contribution
- **`docs/ddd/license-compliance.md`** — License model, copyleft tiers, risk invariants

---

## Acceptance Criteria

- ✅ `cargo test -p repogate-licensing` passes
- ✅ `classify_license("AGPL-3.0")` → `CopyleftTier::StrongCopyleft`
- ✅ `classify_license("MIT")` → `CopyleftTier::Permissive`
- ✅ `classify_license("BSL-1.1")` → `CopyleftTier::SourceAvailableNonOsi`
- ✅ Repo with `"GPL-3.0"` license → `overall_risk_score >= 8.0`
- ✅ SPDX expression `"GPL-2.0-only WITH Classpath-exception-2.0"` parses without error
- ✅ LicenseReport round-trips through JSON

---

## Language

**Rust** — License detection, SPDX parsing, copyleft classification, risk scoring.

---

## Out-of-Scope

- Do NOT implement module-level license inference; focus on repo and dependencies
- Do NOT implement automatic license compliance remediation
- Do NOT implement detailed SPDX license conflict resolution (flag for manual review)
