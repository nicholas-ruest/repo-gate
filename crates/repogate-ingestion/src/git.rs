//! Git provider trait and a subprocess-backed implementation.
//!
//! For the MVP we shell out to the system `git` binary (ADR-005): it reliably
//! clones arbitrary public repositories and needs no extra library dependency.

use std::net::Ipv4Addr;
use std::path::Path;

use crate::IngestionError;

/// Abstraction over git operations so the implementation can migrate from
/// subprocess `git` to `gix` post-MVP without touching the rest of the pipeline.
#[allow(async_fn_in_trait)]
pub trait GitProvider: Send + Sync {
    /// Clone `url` into `dest` (shallow, blobless).
    async fn clone(&self, url: &str, dest: &Path) -> Result<(), IngestionError>;

    /// Resolve the cloned repository's `HEAD` to a full commit SHA.
    async fn resolve_head(&self, repo_path: &Path) -> Result<String, IngestionError>;
}

/// Subprocess-backed [`GitProvider`] using the system `git` binary.
pub struct SubprocessGit;

impl GitProvider for SubprocessGit {
    async fn clone(&self, url: &str, dest: &Path) -> Result<(), IngestionError> {
        validate_repo_url(url)?;

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

    async fn resolve_head(&self, repo_path: &Path) -> Result<String, IngestionError> {
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

/// Validate a remote repository URL before cloning.
///
/// Rejects local and private-network targets (`file://`, `localhost`,
/// loopback, and RFC-1918 ranges) so a submitted URL cannot be used to reach
/// internal services — the `RepoUrl` security invariant from the
/// RepositoryIngestion bounded context.
pub fn validate_repo_url(url: &str) -> Result<(), IngestionError> {
    let lowered = url.to_ascii_lowercase();

    if lowered.starts_with("file://") {
        return Err(IngestionError::InvalidUrl(
            "file:// URLs are not allowed".into(),
        ));
    }
    if lowered.contains("localhost") {
        return Err(IngestionError::InvalidUrl(
            "localhost URLs are not allowed".into(),
        ));
    }

    // Extract the host portion (between scheme and the first '/', ':', or '@').
    let host = extract_host(&lowered);
    if let Some(host) = host {
        if let Ok(ip) = host.parse::<Ipv4Addr>() {
            if ip.is_loopback() || ip.is_private() || ip.is_link_local() || ip.is_unspecified() {
                return Err(IngestionError::InvalidUrl(format!(
                    "private/loopback address not allowed: {ip}"
                )));
            }
        }
    }

    Ok(())
}

/// Best-effort extraction of the host from a URL for IP-range checks.
fn extract_host(url: &str) -> Option<String> {
    let after_scheme = url.split("://").nth(1).unwrap_or(url);
    // Strip any userinfo (`user@host`).
    let after_userinfo = after_scheme.rsplit('@').next().unwrap_or(after_scheme);
    // Host ends at the first '/', ':' (port), or '?'.
    let host: String = after_userinfo
        .chars()
        .take_while(|c| *c != '/' && *c != ':' && *c != '?')
        .collect();
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}
