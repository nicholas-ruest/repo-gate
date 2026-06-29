# P04 — `repogate-ingestion`: Dependency Manifest Parsing + `syft` SBOM

## Context

RepoGate is a deep repository assessment platform that analyzes full open-source codebases to determine what should remain open source versus become part of commercial tiers, using Claude Code as the reasoning engine and Rust as the primary implementation language.

**You are implementing exactly ONE build unit: dependency extraction from package manifests and SBOM generation via syft.** Do not start work on other units. Build and tests must be green before this prompt is considered done.

**Prerequisites:** P03 (git clone, file walk) is complete.

---

## Phase & Dependencies

- **Phase:** Ingestion
- **Depends on:** P03

---

## Scope & Deliverables

Extend `repogate-ingestion` with dependency parsing.

### File: `src/deps/cargo.rs` — Cargo Manifest Parsing

```rust
pub struct DependencyRecord {
    pub name: String,
    pub version: String,
    pub ecosystem: Ecosystem,
    pub spdx_license: Option<String>,  // Preserved verbatim; not parsed
    pub is_direct: bool,
    pub is_transitive: bool,
}

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

pub async fn parse_cargo_deps(repo_path: &std::path::Path) -> Result<Vec<DependencyRecord>, IngestionError> {
    // Run: cargo metadata --format-version 1
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
    
    // Extract from metadata.packages[]
    if let Some(packages) = metadata["packages"].as_array() {
        for pkg in packages {
            deps.push(DependencyRecord {
                name: pkg["name"].as_str().unwrap_or("").to_string(),
                version: pkg["version"].as_str().unwrap_or("").to_string(),
                ecosystem: Ecosystem::Cargo,
                spdx_license: pkg["license"].as_str().map(|s| s.to_string()),
                is_direct: true,  // Simplification; parse resolve graph for accuracy
                is_transitive: false,
            });
        }
    }
    
    Ok(deps)
}
```

### File: `src/deps/sbom.rs` — Syft SBOM Parsing

```rust
pub async fn extract_sbom_via_syft(repo_path: &std::path::Path) -> Result<Vec<DependencyRecord>, IngestionError> {
    // Run: syft <repo-path> -o spdx-json --quiet
    let output = tokio::process::Command::new("syft")
        .arg(repo_path)
        .arg("-o")
        .arg("spdx-json")
        .arg("--quiet")
        .output()
        .await;
    
    match output {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(IngestionError::SyftNotFound);
        }
        Err(e) => return Err(IngestionError::Io(e)),
        Ok(out) => {
            if !out.status.success() {
                return Ok(vec![]);  // Graceful fallback
            }
            
            // Parse SPDX JSON output
            let spdx: serde_json::Value = serde_json::from_slice(&out.stdout)?;
            let mut deps = Vec::new();
            
            // Extract from spdx["packages"][]
            if let Some(packages) = spdx["packages"].as_array() {
                for pkg in packages {
                    deps.push(DependencyRecord {
                        name: pkg["name"].as_str().unwrap_or("").to_string(),
                        version: pkg["versionInfo"].as_str().unwrap_or("").to_string(),
                        ecosystem: infer_ecosystem(&pkg["name"].as_str().unwrap_or("")),
                        spdx_license: pkg["licenseConcluded"].as_str().map(|s| s.to_string()),
                        is_direct: false,  // SBOM reports all
                        is_transitive: true,
                    });
                }
            }
            
            Ok(deps)
        }
    }
}

fn infer_ecosystem(name: &str) -> Ecosystem {
    // Heuristic: check package name patterns
    // (Simplified; real impl would use package registry APIs)
    Ecosystem::Unknown
}
```

### File: `src/deps/mod.rs`

```rust
pub mod cargo;
pub mod sbom;

pub use cargo::{DependencyRecord, Ecosystem};

pub async fn extract_dependencies(
    manifest: &RepoManifest,
    repo_path: &std::path::Path,
) -> Result<Vec<DependencyRecord>, IngestionError> {
    let mut all_deps = Vec::new();
    
    // Detect manifest types and extract accordingly
    let has_cargo = manifest.package_files.iter().any(|pf| {
        matches!(pf.file_type, PackageFileType::Cargo)
    });
    
    if has_cargo {
        all_deps.extend(cargo::parse_cargo_deps(repo_path).await?);
    }
    
    // Run syft and merge
    if let Ok(sbom_deps) = sbom::extract_sbom_via_syft(repo_path).await {
        all_deps.extend(sbom_deps);
    }
    
    // Dedup by (name, version, ecosystem)
    all_deps.sort_by(|a, b| (a.name.cmp(&b.name), a.version.cmp(&b.version)).cmp(&(a.name.cmp(&b.name), a.version.cmp(&b.version))));
    all_deps.dedup_by(|a, b| a.name == b.name && a.version == b.version && a.ecosystem == b.ecosystem);
    
    Ok(all_deps)
}
```

### File: `src/lib.rs` — Extend Manifest

```rust
// In RepoManifest:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoManifest {
    pub repo_id: String,
    pub url: String,
    pub total_files: usize,
    pub total_loc: usize,
    pub language_stats: LanguageStats,
    pub root_dirs: Vec<String>,
    pub file_entries: Vec<FileEntry>,
    pub package_files: Vec<PackageFileRef>,
    pub dependencies: Vec<DependencyRecord>,  // NEW
}

#[derive(Debug, thiserror::Error)]
pub enum IngestionError {
    // ... previous variants ...
    #[error("cargo metadata failed")]
    CargoMetadataFailed,
    
    #[error("syft not found")]
    SyftNotFound,
}

pub async fn ingest(
    url: &str,
    dest: &std::path::Path,
) -> Result<RepoManifest, IngestionError> {
    let git = SubprocessGit;
    git.clone(url, dest).await?;
    
    let entries = walk::walk_repository(dest).await?;
    let lang_stats = language::compute_language_stats(&entries);
    let package_files = detect_package_files(&entries);
    
    let mut manifest = RepoManifest {
        repo_id: uuid::Uuid::new_v4().to_string(),
        url: url.to_string(),
        total_files: entries.len(),
        total_loc: 0,
        language_stats: lang_stats,
        root_dirs: vec![],
        file_entries: entries,
        package_files,
        dependencies: vec![],
    };
    
    // Extract dependencies
    manifest.dependencies = deps::extract_dependencies(&manifest, dest).await.unwrap_or_default();
    
    Ok(manifest)
}

fn detect_package_files(entries: &[FileEntry]) -> Vec<PackageFileRef> {
    entries.iter()
        .filter_map(|e| {
            let file_name = e.path.file_name()?.to_string_lossy();
            let file_type = match file_name.as_ref() {
                "Cargo.toml" => PackageFileType::Cargo,
                "package.json" => PackageFileType::Npm,
                "pyproject.toml" => PackageFileType::PyProject,
                "go.mod" => PackageFileType::GoMod,
                "pom.xml" => PackageFileType::Maven,
                "build.gradle" => PackageFileType::Gradle,
                "Gemfile" => PackageFileType::Gemfile,
                _ => return None,
            };
            Some(PackageFileRef {
                path: e.path.clone(),
                file_type,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_record_preserves_license() {
        let record = DependencyRecord {
            name: "serde".to_string(),
            version: "1.0".to_string(),
            ecosystem: Ecosystem::Cargo,
            spdx_license: Some("MIT OR Apache-2.0".to_string()),
            is_direct: true,
            is_transitive: false,
        };
        assert_eq!(record.spdx_license.unwrap(), "MIT OR Apache-2.0");
    }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-006-license-dependency-analysis.md`** — `cargo_metadata`, `syft` subprocess, multi-ecosystem SBOM parsing
- **`docs/ddd/repository-ingestion.md`** — `package_files` structure

---

## Acceptance Criteria

- ✅ `cargo test -p repogate-ingestion` passes
- ✅ Rust repo analysis produces non-empty `Vec<DependencyRecord>` with `Cargo` ecosystem
- ✅ `syft` missing → `Err(SyftNotFound)`, no panic
- ✅ Cargo license string `"MIT OR Apache-2.0"` is preserved verbatim (not parsed)
- ✅ `DependencyRecord` serializes and deserializes via serde
- ✅ Deduplication removes duplicate (name, version, ecosystem) tuples

---

## Language

**Rust** — Manifest parsing, subprocess coordination, dependency extraction.

---

## Out-of-Scope

- Do NOT parse or interpret SPDX license expressions (P05)
- Do NOT implement supply-chain risk assessment
- Do NOT implement transitive dependency graph building
