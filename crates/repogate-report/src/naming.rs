//! Report file-naming convention: `repogate-{owner}-{repo}-{timestamp}` (ADR-011).

/// Build the report file stem from a repository URL and a completion timestamp.
pub fn report_stem(repo_url: &str, completed_at: &str) -> String {
    let parts: Vec<&str> = repo_url
        .trim_end_matches('/')
        .split('/')
        .filter(|p| !p.is_empty())
        .collect();

    let owner = parts
        .len()
        .checked_sub(2)
        .and_then(|i| parts.get(i))
        .copied()
        .unwrap_or("unknown");
    let repo = parts.last().copied().unwrap_or("repo");

    format!(
        "repogate-{}-{}-{}",
        slugify(owner),
        slugify(repo.trim_end_matches(".git")),
        completed_at
    )
}

fn slugify(s: &str) -> String {
    s.to_lowercase().replace('_', "-")
}
