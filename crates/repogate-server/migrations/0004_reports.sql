CREATE TABLE reports (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL UNIQUE,
    assessment_json TEXT NOT NULL,
    markdown_content TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
