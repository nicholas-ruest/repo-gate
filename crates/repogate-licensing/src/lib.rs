#![doc = "RepoGate license detection and copyleft analysis."]

pub mod copyleft;
pub mod detect;
pub mod report;
pub mod spdx;

pub use copyleft::{classify_license, copyleft_risk_score, CopyleftTier};
pub use detect::{DetectionMethod, LicenseDetection};
pub use report::LicenseReport;

/// Errors produced during license analysis.
#[derive(Debug, thiserror::Error)]
pub enum LicensingError {
    #[error("SPDX parse failed: {0}")]
    SpdxParseFailed(String),

    #[error("license detection failed")]
    DetectionFailed,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Analyze a cloned repository's license posture.
pub async fn analyze(
    manifest: &repogate_ingestion::RepoManifest,
    repo_path: &std::path::Path,
) -> Result<LicenseReport, LicensingError> {
    let detections = detect::detect_licenses(repo_path).await?;
    report::build_report(&manifest.repo_id, detections, &manifest.dependencies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use detect::{DetectionMethod, LicenseDetection};
    use repogate_ingestion::{DependencyRecord, Ecosystem};

    fn detection(spdx: &str) -> LicenseDetection {
        LicenseDetection {
            file_path: "LICENSE".to_string(),
            spdx_expression: spdx.to_string(),
            confidence: 0.95,
            detection_method: DetectionMethod::LicenseFile,
            needs_review: false,
        }
    }

    #[test]
    fn classify_agpl() {
        assert_eq!(classify_license("AGPL-3.0"), CopyleftTier::StrongCopyleft);
    }

    #[test]
    fn classify_gpl3() {
        assert_eq!(classify_license("GPL-3.0"), CopyleftTier::StrongCopyleft);
    }

    #[test]
    fn classify_mit() {
        assert_eq!(classify_license("MIT"), CopyleftTier::Permissive);
    }

    #[test]
    fn classify_bsl() {
        assert_eq!(
            classify_license("BSL-1.1"),
            CopyleftTier::SourceAvailableNonOsi
        );
    }

    #[test]
    fn risk_score_strong_copyleft() {
        assert!(copyleft_risk_score(CopyleftTier::StrongCopyleft) >= 8.0);
    }

    #[test]
    fn gpl3_repo_overall_risk_high() {
        let report = report::build_report("repo-1", vec![detection("GPL-3.0")], &[]).unwrap();
        assert!(report.overall_risk_score >= 8.0);
    }

    #[test]
    fn dependency_gpl_drives_risk() {
        let dep = DependencyRecord {
            name: "somecrate".to_string(),
            version: "1.0".to_string(),
            ecosystem: Ecosystem::Cargo,
            spdx_license: Some("GPL-3.0".to_string()),
            is_direct: true,
            is_transitive: false,
        };
        let report = report::build_report("repo-1", vec![], &[dep]).unwrap();
        assert!(report.overall_risk_score >= 8.0);
        assert_eq!(report.dependency_licenses.len(), 1);
    }

    #[test]
    fn spdx_with_exception_parses() {
        assert!(spdx::parse_and_normalize("GPL-2.0-only WITH Classpath-exception-2.0").is_ok());
        let ids =
            spdx::extract_base_identifiers("GPL-2.0-only WITH Classpath-exception-2.0").unwrap();
        assert_eq!(ids, vec!["GPL-2.0-only".to_string()]);
    }

    #[test]
    fn spdx_compound_parses() {
        let ids = spdx::extract_base_identifiers("MIT OR Apache-2.0").unwrap();
        assert!(ids.contains(&"MIT".to_string()));
        assert!(ids.contains(&"Apache-2.0".to_string()));
    }

    #[test]
    fn license_report_json_round_trip() {
        let report = report::build_report("repo-1", vec![detection("MIT")], &[]).unwrap();
        let json = serde_json::to_string(&report).unwrap();
        let back: LicenseReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.repo_id, "repo-1");
        assert_eq!(back.detections.len(), 1);
    }

    #[test]
    fn identify_mit_text() {
        let text = "Permission is hereby granted, free of charge, to any person obtaining a copy";
        assert_eq!(
            detect::identify_license_text(text).map(|(id, _)| id),
            Some("MIT".to_string())
        );
    }
}
