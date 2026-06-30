//! Cargo dependency extraction via `cargo metadata`.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::IngestionError;

/// A single dependency discovered in a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyRecord {
    pub name: String,
    pub version: String,
    pub ecosystem: Ecosystem,
    /// SPDX license string, preserved verbatim — parsing happens in `repogate-licensing` (P05).
    pub spdx_license: Option<String>,
    pub is_direct: bool,
    pub is_transitive: bool,
}

/// The package ecosystem a dependency belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ecosystem {
    Cargo,
    Npm,
    PyPi,
    Go,
    Maven,
    Gradle,
    Ruby,
    Unknown,
}

/// Parse Cargo dependencies by shelling out to `cargo metadata`.
pub async fn parse_cargo_deps(repo_path: &Path) -> Result<Vec<DependencyRecord>, IngestionError> {
    let output = tokio::process::Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(repo_path)
        .output()
        .await?;

    if !output.status.success() {
        return Err(IngestionError::CargoMetadataFailed);
    }

    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let mut deps = Vec::new();

    if let Some(packages) = metadata["packages"].as_array() {
        for pkg in packages {
            deps.push(DependencyRecord {
                name: pkg["name"].as_str().unwrap_or("").to_string(),
                version: pkg["version"].as_str().unwrap_or("").to_string(),
                ecosystem: Ecosystem::Cargo,
                spdx_license: pkg["license"].as_str().map(|s| s.to_string()),
                is_direct: true,
                is_transitive: false,
            });
        }
    }

    Ok(deps)
}
