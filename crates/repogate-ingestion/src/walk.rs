//! Gitignore-aware file traversal with binary filtering, language
//! classification, and per-file BLAKE3 hashing.

use std::path::{Path, PathBuf};

use repogate_core::Language;
use serde::{Deserialize, Serialize};

use crate::IngestionError;

/// One file discovered during the walk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub is_binary: bool,
    pub language: Option<Language>,
    /// BLAKE3 content hash (hex). Empty for binary/unhashed files.
    pub hash: String,
    /// True when the file looks vendored or machine-generated.
    pub is_generated: bool,
}

const KNOWN_BINARY_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "ico", "webp", "pdf", "zip", "tar", "gz", "bz2", "xz",
    "7z", "exe", "dll", "so", "dylib", "bin", "class", "jar", "wasm", "o", "a", "lib", "pyc",
    "woff", "woff2", "ttf", "eot", "mp3", "mp4", "mov", "avi", "wav",
];

/// Walk `repo_path` honoring `.gitignore`, returning a [`FileEntry`] per file.
pub async fn walk_repository(repo_path: &Path) -> Result<Vec<FileEntry>, IngestionError> {
    let root = repo_path.to_path_buf();
    // The `ignore` walker is synchronous; run it on a blocking thread so we do
    // not stall the async runtime.
    tokio::task::spawn_blocking(move || walk_blocking(&root))
        .await
        .map_err(|e| IngestionError::Walk(e.to_string()))?
}

fn walk_blocking(repo_path: &Path) -> Result<Vec<FileEntry>, IngestionError> {
    let mut entries = Vec::new();

    let walker = ignore::WalkBuilder::new(repo_path)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .build();

    for result in walker {
        let dir_entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };
        // Skip directories and anything without a regular-file type.
        if !dir_entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let path = dir_entry.path();
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let is_binary = detect_binary(path);
        let is_generated = detect_generated(path);
        let language = if is_binary {
            None
        } else {
            classify_language(path)
        };
        let hash = if is_binary {
            String::new()
        } else {
            hash_file(path).unwrap_or_default()
        };

        entries.push(FileEntry {
            path: path.to_path_buf(),
            size_bytes: metadata.len(),
            is_binary,
            language,
            hash,
            is_generated,
        });
    }

    Ok(entries)
}

/// Classify a file as binary by extension or by a null byte in the first 8 KB.
pub(crate) fn detect_binary(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        if KNOWN_BINARY_EXTENSIONS.contains(&ext.as_str()) {
            return true;
        }
    }

    if let Ok(data) = std::fs::read(path) {
        let sample = &data[..std::cmp::min(8192, data.len())];
        if sample.contains(&0) {
            return true;
        }
    }

    false
}

/// Heuristic detection of vendored / generated files.
pub(crate) fn detect_generated(path: &Path) -> bool {
    let p = path.to_string_lossy();
    p.contains("/vendor/")
        || p.contains("/node_modules/")
        || p.ends_with(".min.js")
        || p.ends_with(".min.css")
}

/// Classify a file's programming language by extension.
pub(crate) fn classify_language(path: &Path) -> Option<Language> {
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    let lang = match ext.as_str() {
        "rs" => Language::Rust,
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => Language::TypeScript,
        "py" | "pyi" => Language::Python,
        "go" => Language::Go,
        "java" => Language::Java,
        _ => return None,
    };
    Some(lang)
}

/// Compute the number of text lines in a file (best effort).
pub(crate) fn count_lines(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

fn hash_file(path: &Path) -> Option<String> {
    let data = std::fs::read(path).ok()?;
    Some(blake3::hash(&data).to_hex().to_string())
}
