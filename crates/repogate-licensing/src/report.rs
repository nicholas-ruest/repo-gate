//! Assembly of the repository-level [`LicenseReport`].

use repogate_ingestion::DependencyRecord;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::copyleft::{classify_license, copyleft_risk_score};
use crate::detect::LicenseDetection;
use crate::LicensingError;

/// Repository-level license posture and copyleft risk.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LicenseReport {
    pub repo_id: String,
    pub detections: Vec<LicenseDetection>,
    /// `(dependency_name, spdx_expression)` pairs.
    pub dependency_licenses: Vec<(String, String)>,
    pub copyleft_exposure: f32,
    pub missing_licenses: bool,
    pub conflicts: Vec<String>,
    pub overall_risk_score: f32,
}

/// Build a [`LicenseReport`] from license detections and dependency records.
///
/// `overall_risk_score` is the maximum copyleft risk across both the
/// repository's own detected licenses and its dependencies, so a repo licensed
/// under strong copyleft scores high even with no risky dependencies.
pub fn build_report(
    repo_id: &str,
    detections: Vec<LicenseDetection>,
    deps: &[DependencyRecord],
) -> Result<LicenseReport, LicensingError> {
    let mut copyleft_exposure: f32 = 0.0;
    let mut overall_risk: f32 = 0.0;

    for detection in &detections {
        let score = copyleft_risk_score(classify_license(&detection.spdx_expression));
        copyleft_exposure = copyleft_exposure.max(score);
        overall_risk = overall_risk.max(score);
    }

    let mut dependency_licenses = Vec::new();
    for dep in deps {
        if let Some(license) = &dep.spdx_license {
            dependency_licenses.push((dep.name.clone(), license.clone()));
            overall_risk = overall_risk.max(copyleft_risk_score(classify_license(license)));
        }
    }

    let missing_licenses = detections.is_empty() && dependency_licenses.is_empty();

    Ok(LicenseReport {
        repo_id: repo_id.to_string(),
        detections,
        dependency_licenses,
        copyleft_exposure,
        missing_licenses,
        conflicts: Vec::new(),
        overall_risk_score: overall_risk,
    })
}
