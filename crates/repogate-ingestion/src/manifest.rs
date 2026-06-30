//! The repository manifest assembled from the ingestion walk.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::deps::DependencyRecord;
use crate::language::LanguageStats;
use crate::walk::FileEntry;

/// A recognised dependency-manifest file type. Parsing happens in `repogate-licensing`/P04.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageFileType {
    Cargo,
    Npm,
    PyProject,
    GoMod,
    Maven,
    Gradle,
    Gemfile,
    Unknown,
}

impl PackageFileType {
    /// Classify a path by its file name. Returns `None` if it is not a package manifest.
    pub fn from_file_name(name: &str) -> Option<Self> {
        let ty = match name {
            "Cargo.toml" => Self::Cargo,
            "package.json" => Self::Npm,
            "pyproject.toml" | "requirements.txt" => Self::PyProject,
            "go.mod" => Self::GoMod,
            "pom.xml" => Self::Maven,
            "build.gradle" | "build.gradle.kts" => Self::Gradle,
            "Gemfile" => Self::Gemfile,
            _ => return None,
        };
        Some(ty)
    }
}

/// A reference to a dependency-manifest file discovered in the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageFileRef {
    pub path: PathBuf,
    pub file_type: PackageFileType,
}

/// The full manifest produced by ingesting a repository.
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
    pub dependencies: Vec<DependencyRecord>,
}

/// Extract the unique top-level directory names relative to `repo_root`.
pub(crate) fn extract_root_dirs(repo_root: &Path, entries: &[FileEntry]) -> Vec<String> {
    let mut dirs = BTreeSet::new();
    for entry in entries {
        if let Ok(rel) = entry.path.strip_prefix(repo_root) {
            if let Some(first) = rel.components().next() {
                let name = first.as_os_str().to_string_lossy().to_string();
                // Only directories: a top-level file's first component is the file itself.
                if rel.components().count() > 1 {
                    dirs.insert(name);
                }
            }
        }
    }
    dirs.into_iter().collect()
}

/// Detect dependency-manifest files among the walked entries.
pub(crate) fn detect_package_files(entries: &[FileEntry]) -> Vec<PackageFileRef> {
    entries
        .iter()
        .filter_map(|entry| {
            let name = entry.path.file_name()?.to_string_lossy();
            PackageFileType::from_file_name(name.as_ref()).map(|file_type| PackageFileRef {
                path: entry.path.clone(),
                file_type,
            })
        })
        .collect()
}
