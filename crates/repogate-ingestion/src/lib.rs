#![doc = "RepoGate repository ingestion: git cloning, file walking, language detection."]

pub mod deps;
pub mod git;
pub mod language;
pub mod manifest;
pub mod walk;

pub use deps::{DependencyRecord, Ecosystem};
pub use git::{GitProvider, SubprocessGit};
pub use language::LanguageStats;
pub use manifest::{PackageFileRef, PackageFileType, RepoManifest};
pub use walk::FileEntry;

/// Errors produced while ingesting a repository.
#[derive(Debug, thiserror::Error)]
pub enum IngestionError {
    #[error("clone failed for {url}: {stderr}")]
    CloneFailed { url: String, stderr: String },

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("git rev-parse HEAD failed")]
    RevParseFailed,

    #[error("file walk failed: {0}")]
    Walk(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("utf8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("json parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("cargo metadata failed")]
    CargoMetadataFailed,

    #[error("syft not found")]
    SyftNotFound,
}

/// Clone `url` into `dest`, walk the tree, and assemble a [`RepoManifest`].
pub async fn ingest(url: &str, dest: &std::path::Path) -> Result<RepoManifest, IngestionError> {
    let git = SubprocessGit;
    git.clone(url, dest).await?;
    let _head = git.resolve_head(dest).await?;
    let mut manifest = build_manifest(url, dest).await?;
    manifest.dependencies = deps::extract_dependencies(&manifest, dest)
        .await
        .unwrap_or_default();
    Ok(manifest)
}

/// Build a manifest from an already-cloned repository at `repo_path`.
///
/// Split out from [`ingest`] so it can be exercised without network access.
pub async fn build_manifest(
    url: &str,
    repo_path: &std::path::Path,
) -> Result<RepoManifest, IngestionError> {
    let entries = walk::walk_repository(repo_path).await?;
    let language_stats = language::compute_language_stats(&entries);
    let total_loc = language_stats.total_loc();
    let root_dirs = manifest::extract_root_dirs(repo_path, &entries);
    let package_files = manifest::detect_package_files(&entries);

    Ok(RepoManifest {
        repo_id: uuid::Uuid::new_v4().to_string(),
        url: url.to_string(),
        total_files: entries.len(),
        total_loc,
        language_stats,
        root_dirs,
        file_entries: entries,
        package_files,
        dependencies: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn validate_url_rejects_file() {
        assert!(git::validate_repo_url("file:///local/path").is_err());
    }

    #[test]
    fn validate_url_rejects_localhost() {
        assert!(git::validate_repo_url("http://localhost:8080/repo").is_err());
    }

    #[test]
    fn validate_url_rejects_private_ip() {
        assert!(git::validate_repo_url("http://192.168.1.10/repo.git").is_err());
        assert!(git::validate_repo_url("https://10.0.0.5/x").is_err());
        assert!(git::validate_repo_url("http://127.0.0.1/x").is_err());
    }

    #[test]
    fn validate_url_accepts_github() {
        assert!(git::validate_repo_url("https://github.com/rust-lang/rust").is_ok());
    }

    #[test]
    fn detect_binary_png() {
        assert!(walk::detect_binary(Path::new("image.png")));
    }

    #[test]
    fn classify_language_rust() {
        assert_eq!(
            walk::classify_language(Path::new("main.rs")),
            Some(repogate_core::Language::Rust)
        );
    }

    #[tokio::test]
    async fn build_manifest_over_temp_tree() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir(root.join("src")).unwrap();
        std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(root.join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
        std::fs::write(root.join("logo.png"), [0u8, 1, 2, 3]).unwrap();

        let manifest = build_manifest("https://example.com/x", root).await.unwrap();

        assert_eq!(manifest.url, "https://example.com/x");
        assert!(manifest.total_files >= 3);
        assert!(manifest
            .language_stats
            .language_counts
            .contains_key(&repogate_core::Language::Rust));
        let png = manifest
            .file_entries
            .iter()
            .find(|e| e.path.ends_with("logo.png"))
            .expect("png entry present");
        assert!(png.is_binary);
        assert!(png.language.is_none());
        assert!(manifest
            .package_files
            .iter()
            .any(|p| p.file_type == PackageFileType::Cargo));
        assert!(manifest.root_dirs.iter().any(|d| d == "src"));
    }

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
        assert_eq!(record.spdx_license.as_deref(), Some("MIT OR Apache-2.0"));
    }

    #[test]
    fn dependency_record_serde_round_trip() {
        let record = DependencyRecord {
            name: "tokio".to_string(),
            version: "1.40.0".to_string(),
            ecosystem: Ecosystem::Cargo,
            spdx_license: Some("MIT".to_string()),
            is_direct: true,
            is_transitive: false,
        };
        let json = serde_json::to_string(&record).unwrap();
        let back: DependencyRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record, back);
    }

    #[test]
    fn dedup_removes_duplicate_tuples() {
        let mk = |n: &str, v: &str| DependencyRecord {
            name: n.to_string(),
            version: v.to_string(),
            ecosystem: Ecosystem::Cargo,
            spdx_license: None,
            is_direct: true,
            is_transitive: false,
        };
        let mut deps = vec![mk("a", "1.0"), mk("a", "1.0"), mk("b", "2.0")];
        deps::dedup_dependencies(&mut deps);
        assert_eq!(deps.len(), 2);
    }

    #[tokio::test]
    async fn syft_missing_returns_not_found_or_ok() {
        // In environments without syft this must be Err(SyftNotFound), never a panic.
        let dir = tempfile::tempdir().unwrap();
        match deps::sbom::extract_sbom_via_syft(dir.path()).await {
            Err(IngestionError::SyftNotFound) => {}
            Ok(_) => {} // syft is installed; acceptable
            Err(other) => panic!("unexpected error: {other}"),
        }
    }

    #[tokio::test]
    async fn cargo_metadata_extracts_cargo_deps() {
        // A minimal, dependency-free crate still lists its own package, so
        // parse_cargo_deps returns at least one Cargo-ecosystem record offline.
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
        )
        .unwrap();
        std::fs::create_dir(root.join("src")).unwrap();
        std::fs::write(root.join("src/lib.rs"), "// fixture\n").unwrap();

        let deps = deps::cargo::parse_cargo_deps(root).await.unwrap();
        assert!(!deps.is_empty());
        assert!(deps.iter().all(|d| d.ecosystem == Ecosystem::Cargo));
    }

    #[tokio::test]
    #[ignore = "requires network access to clone a public repository"]
    async fn live_clone_regex() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("regex");
        let manifest = ingest("https://github.com/rust-lang/regex", &dest)
            .await
            .unwrap();
        assert!(manifest.total_files > 50);
        assert!(manifest
            .language_stats
            .language_counts
            .contains_key(&repogate_core::Language::Rust));
    }
}
