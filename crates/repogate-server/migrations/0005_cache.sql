CREATE TABLE analysis_cache (
    repo_url TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    cached_assessment TEXT NOT NULL,
    ttl_days INTEGER NOT NULL DEFAULT 30,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (repo_url, commit_sha)
);
