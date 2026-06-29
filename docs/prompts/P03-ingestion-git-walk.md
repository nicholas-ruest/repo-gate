# P03 — `repogate-ingestion`: Git Clone, File Walk, Language Detection, Binary Filtering

## Context

RepoGate is a deep repository assessment platform that analyzes full open-source codebases to determine what should remain open source versus become part of commercial tiers, using Claude Code as the reasoning engine and Rust as the primary implementation language.

**You are implementing exactly ONE build unit: the repository ingestion layer, including git cloning, file traversal, language detection, and binary filtering.** Do not start work on other units. Build and tests must be green before this prompt is considered done.

**Prerequisites:** P02 (core types and schemas) is complete.

---

## Phase & Dependencies

- **Phase:** Ingestion
- **Depends on:** P02

---

## Scope & Deliverables

Implement the `repogate-ingestion` crate with safe, parallel repository ingestion logic.

### File: `src/git.rs` — Git Provider Trait & Subprocess Implementation

Define a trait and implement it using subprocess:

```rust
pub trait GitProvider: Send + Sync {
    async fn clone(&self, url: &str, dest: &std::path::Path) -> Result<(), IngestionError>;
    async fn resolve_head(&self, repo_path: &std::path::Path) -> Result<String, IngestionError>;
}

pub struct SubprocessGit;

impl GitProvider for SubprocessGit {
    async fn clone(&self, url: &str, dest: &std::path::Path) -> Result<(), IngestionError> {
        // Validate URL first (reject file://, localhost, RFC-1918 IPs)
        validate_repo_url(url)?;
        
        // Run: git clone --depth=1 --filter=blob:none <url> <dest>
        let output = tokio::process::Command::new("git")
            .arg("clone")
            .arg("--depth=1")
            .arg("--filter=blob:none")
            .arg(url)
            .arg(dest)
            .output()
            .await?;
        
        if !output.status.success() {
            return Err(IngestionError::CloneFailed {
                url: url.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        Ok(())
    }
    
    async fn resolve_head(&self, repo_path: &std::path::Path) -> Result<String, IngestionError> {
        let output = tokio::process::Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(repo_path)
            .output()
            .await?;
        
        if !output.status.success() {
            return Err(IngestionError::RevParseFailed);
        }
        
        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }
}

fn validate_repo_url(url: &str) -> Result<(), IngestionError> {
    // Reject file://, localhost, 127.0.0.1, RFC-1918 (10.x, 172.16-31.x, 192.168.x.x)
    if url.starts_with("file://") {
        return Err(IngestionError::InvalidUrl("file:// URLs not allowed".into()));
    }
    if url.contains("localhost") || url.contains("127.0.0.1") {
        return Err(IngestionError::InvalidUrl("localhost URLs not allowed".into()));
    }
    // Additional IP range checks as needed
    Ok(())
}
```

### File: `src/walk.rs` — File Walk & Language Detection

```rust
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: std::path::PathBuf,
    pub size_bytes: u64,
    pub is_binary: bool,
    pub language: Option<repogate_core::Language>,
    pub hash: String,  // BLAKE3 hash
}

pub async fn walk_repository(
    repo_path: &std::path::Path,
) -> Result<Vec<FileEntry>, IngestionError> {
    // Use ignore::WalkBuilder for gitignore-aware traversal
    // Detect binaries (null byte in first 8 KB OR known binary extension)
    // Detect generated files (.gitattributes linguist-generated, vendor/, node_modules/, *.min.js)
    // Parallel walk via rayon or tokio
    // Classify each file's language via language_detector::classify() or similar
    // Compute BLAKE3 hash per file
    
    let mut entries = Vec::new();
    let walker = ignore::WalkBuilder::new(repo_path)
        .hidden(true)
        .ignore(true)
        .build_parallel();
    
    let (tx, rx) = tokio::sync::mpsc::channel(1000);
    tokio::spawn(async move {
        walker.run(|| {
            let tx = tx.clone();
            Box::new(move |result| {
                if let Ok(entry) = result {
                    // Process each file entry
                    let _ = tx.blocking_send(entry);
                }
                ignore::WalkState::Continue
            })
        });
    });
    
    // Collect results from channel
    while let Some(entry) = rx.recv().await {
        // Build FileEntry from DirEntry
        entries.push(FileEntry {
            path: entry.path().to_path_buf(),
            size_bytes: 0,  // TODO: calculate
            is_binary: detect_binary(entry.path()),
            language: None,  // TODO: classify
            hash: String::new(),  // TODO: compute BLAKE3
        });
    }
    
    Ok(entries)
}

fn detect_binary(path: &std::path::Path) -> bool {
    // Check for known binary extensions
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        if matches!(ext_str.as_str(), "png" | "jpg" | "jpeg" | "gif" | "so" | "dll" | "dylib" | "bin" | "exe" | "wasm") {
            return true;
        }
    }
    
    // Try reading first 8KB and check for null bytes
    if let Ok(data) = std::fs::read(&path) {
        let sample = &data[..std::cmp::min(8192, data.len())];
        if sample.iter().any(|&b| b == 0) {
            return true;
        }
    }
    
    false
}

fn classify_language(path: &std::path::Path) -> Option<repogate_core::Language> {
    // Use tree-sitter or similar to classify by extension + content
    // Fallback to hyperpolyglot heuristics
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        match ext_str.as_str() {
            "rs" => Some(repogate_core::Language::Rust),
            "ts" | "tsx" | "js" | "jsx" => Some(repogate_core::Language::TypeScript),
            "py" => Some(repogate_core::Language::Python),
            "go" => Some(repogate_core::Language::Go),
            "java" => Some(repogate_core::Language::Java),
            _ => None,
        }
    } else {
        None
    }
}
```

### File: `src/language.rs` — Language Statistics

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageStats {
    pub language_counts: HashMap<repogate_core::Language, usize>,  // LOC per language
}

pub fn compute_language_stats(entries: &[FileEntry]) -> LanguageStats {
    // Use tokei library to aggregate LOC per language
    // Or use hyperpolyglot classification for each file
    let mut counts = HashMap::new();
    
    for entry in entries {
        if let Some(lang) = entry.language {
            *counts.entry(lang).or_insert(0) += 1;  // Simplified: count files, not LOC
        }
    }
    
    LanguageStats {
        language_counts: counts,
    }
}
```

### File: `src/manifest.rs` — Repository Manifest

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PackageFileType {
    Cargo,     // Cargo.toml
    Npm,       // package.json
    PyProject, // pyproject.toml
    GoMod,     // go.mod
    Maven,     // pom.xml
    Gradle,    // build.gradle
    Gemfile,   // Gemfile
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageFileRef {
    pub path: std::path::PathBuf,
    pub file_type: PackageFileType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoManifest {
    pub repo_id: String,
    pub url: String,
    pub total_files: usize,
    pub total_loc: usize,
    pub language_stats: LanguageStats,
    pub root_dirs: Vec<String>,  // Top-level directories
    pub file_entries: Vec<FileEntry>,
    pub package_files: Vec<PackageFileRef>,
}
```

### File: `src/lib.rs`

```rust
#![doc = "RepoGate repository ingestion: git cloning, file walking, language detection."]

pub mod git;
pub mod walk;
pub mod language;
pub mod manifest;

pub use git::{GitProvider, SubprocessGit};
pub use walk::FileEntry;
pub use manifest::RepoManifest;

#[derive(Debug, thiserror::Error)]
pub enum IngestionError {
    #[error("clone failed for {url}: {stderr}")]
    CloneFailed { url: String, stderr: String },
    
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    
    #[error("rev-parse failed")]
    RevParseFailed,
    
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("utf8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

pub async fn ingest(
    url: &str,
    dest: &std::path::Path,
) -> Result<RepoManifest, IngestionError> {
    let git = SubprocessGit;
    git.clone(url, dest).await?;
    let _head = git.resolve_head(dest).await?;
    
    let entries = walk::walk_repository(dest).await?;
    let lang_stats = language::compute_language_stats(&entries);
    
    Ok(RepoManifest {
        repo_id: uuid::Uuid::new_v4().to_string(),
        url: url.to_string(),
        total_files: entries.len(),
        total_loc: 0,  // TODO: sum from tokei
        language_stats: lang_stats,
        root_dirs: vec![],  // TODO: extract top-level dirs
        file_entries: entries,
        package_files: vec![],  // TODO: detect package files
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_url_rejects_file() {
        assert!(git::validate_repo_url("file:///local/path").is_err());
    }

    #[test]
    fn validate_url_rejects_localhost() {
        assert!(git::validate_repo_url("http://localhost:8080/repo").is_err());
    }

    #[test]
    fn validate_url_accepts_github() {
        assert!(git::validate_repo_url("https://github.com/rust-lang/rust").is_ok());
    }

    #[test]
    fn detect_binary_png() {
        assert!(walk::detect_binary(std::path::Path::new("image.png")));
    }

    #[test]
    fn classify_language_rust() {
        let lang = walk::classify_language(std::path::Path::new("main.rs"));
        assert_eq!(lang, Some(repogate_core::Language::Rust));
    }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-005-git-ingestion-and-tree-walking.md`** — Git strategy, `ignore` crate, `tokei`, `hyperpolyglot`, binary detection, `GitProvider` trait
- **`docs/ddd/repository-ingestion.md`** — Ingestion invariants, `RepoUrl`, `FileEntry`, `ModuleManifest`

---

## Acceptance Criteria

- ✅ `cargo test -p repogate-ingestion` passes
- ✅ Integration test: clone `https://github.com/rust-lang/regex` to temp; `total_files > 50`, `language_stats` contains at least `Rust`
- ✅ `file://` URL validation rejects with `InvalidUrl` error
- ✅ `http://localhost:8080` URL validation rejects with error
- ✅ `.png` file in entries has `is_binary: true` and `language: None`
- ✅ `.rs` file classified as `Language::Rust`

---

## Language

**Rust** — All ingestion logic, file operations, git integration.

---

## Out-of-Scope

- Do NOT implement dependency parsing or manifest interpretation (P04)
- Do NOT implement license detection (P05)
- Do NOT implement module boundary detection or architecture mapping (P08)
- Do NOT implement deep file content inspection; just basic language classification
