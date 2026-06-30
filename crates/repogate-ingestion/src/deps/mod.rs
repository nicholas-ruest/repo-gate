//! Dependency extraction across ecosystems: Cargo via `cargo metadata`, and
//! everything else via the `syft` SBOM subprocess.

pub mod cargo;
pub mod sbom;

pub use cargo::{DependencyRecord, Ecosystem};

use std::path::Path;

use crate::manifest::{PackageFileType, RepoManifest};
use crate::IngestionError;

/// Extract dependencies for a cloned repository, combining Cargo metadata and
/// the syft SBOM, then de-duplicating by (name, version, ecosystem).
pub async fn extract_dependencies(
    manifest: &RepoManifest,
    repo_path: &Path,
) -> Result<Vec<DependencyRecord>, IngestionError> {
    let mut all_deps = Vec::new();

    let has_cargo = manifest
        .package_files
        .iter()
        .any(|pf| matches!(pf.file_type, PackageFileType::Cargo));

    if has_cargo {
        all_deps.extend(cargo::parse_cargo_deps(repo_path).await?);
    }

    // syft covers the remaining ecosystems; a missing binary is non-fatal here.
    match sbom::extract_sbom_via_syft(repo_path).await {
        Ok(sbom_deps) => all_deps.extend(sbom_deps),
        Err(IngestionError::SyftNotFound) => {}
        Err(e) => return Err(e),
    }

    dedup_dependencies(&mut all_deps);
    Ok(all_deps)
}

/// Sort and remove duplicate (name, version, ecosystem) records in place.
pub(crate) fn dedup_dependencies(deps: &mut Vec<DependencyRecord>) {
    deps.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then(a.version.cmp(&b.version))
            .then((a.ecosystem as u8).cmp(&(b.ecosystem as u8)))
    });
    deps.dedup_by(|a, b| a.name == b.name && a.version == b.version && a.ecosystem == b.ecosystem);
}
