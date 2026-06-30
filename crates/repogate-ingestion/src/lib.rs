#![doc = "RepoGate repository ingestion: git cloning, file walking, language detection."]

pub mod git;
pub mod language;
pub mod manifest;
pub mod walk;

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
}

/// Clone `url` into `dest`, walk the tree, and assemble a [`RepoManifest`].
pub async fn ingest(url: &str, dest: &std::path::Path) -> Result<RepoManifest, IngestionError> {
    let git = SubprocessGit;
    git.clone(url, dest).await?;
    let _head = git.resolve_head(dest).await?;
    build_manifest(url, dest).await
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
