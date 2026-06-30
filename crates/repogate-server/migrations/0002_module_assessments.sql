CREATE TABLE module_assessments (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL,
    module_id TEXT NOT NULL,
    module_name TEXT NOT NULL,
    assessment_json TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(job_id, module_id)
);
