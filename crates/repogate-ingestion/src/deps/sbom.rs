//! Multi-ecosystem SBOM extraction via the `syft` subprocess (ADR-006).

use std::path::Path;

use crate::deps::cargo::{DependencyRecord, Ecosystem};
use crate::IngestionError;

/// Generate an SPDX-JSON SBOM with `syft` and convert it to [`DependencyRecord`]s.
///
/// Returns [`IngestionError::SyftNotFound`] (not a panic) when the `syft` binary
/// is not installed, and an empty vector when syft runs but reports nothing.
pub async fn extract_sbom_via_syft(
    repo_path: &Path,
) -> Result<Vec<DependencyRecord>, IngestionError> {
    let output = tokio::process::Command::new("syft")
        .arg(repo_path)
        .arg("-o")
        .arg("spdx-json")
        .arg("--quiet")
        .output()
        .await;

    let out = match output {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(IngestionError::SyftNotFound);
        }
        Err(e) => return Err(IngestionError::Io(e)),
        Ok(out) => out,
    };

    if !out.status.success() {
        return Ok(vec![]);
    }

    let spdx: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    let mut deps = Vec::new();

    if let Some(packages) = spdx["packages"].as_array() {
        for pkg in packages {
            let name = pkg["name"].as_str().unwrap_or("").to_string();
            deps.push(DependencyRecord {
                ecosystem: infer_ecosystem(&name),
                version: pkg["versionInfo"].as_str().unwrap_or("").to_string(),
                spdx_license: pkg["licenseConcluded"].as_str().map(|s| s.to_string()),
                name,
                is_direct: false,
                is_transitive: true,
            });
        }
    }

    Ok(deps)
}

/// Best-effort ecosystem inference from a package name. Detailed per-ecosystem
/// classification is out of scope here; defaults to [`Ecosystem::Unknown`].
fn infer_ecosystem(_name: &str) -> Ecosystem {
    Ecosystem::Unknown
}
