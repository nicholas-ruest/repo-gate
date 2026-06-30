CREATE TABLE checkpoints (
    job_id TEXT PRIMARY KEY,
    checkpoint_json TEXT NOT NULL,
    saved_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
