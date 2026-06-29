# P13 — `sqlx` Schema, Migrations, Store Implementations

## Context

**You are implementing durable persistence: SQL migrations and sqlx-backed store implementations.**

**Prerequisites:** P07 (state machine), P12 (report assembly) are complete.

---

## Phase & Dependencies

- **Phase:** Persistence
- **Depends on:** P07, P12

---

## Scope & Deliverables

Implement sqlx stores in `repogate-server/src/db/`.

### Directory: `repogate-server/migrations/`

**`0001_jobs.sql`**
```sql
CREATE TABLE jobs (
    id TEXT PRIMARY KEY,
    repo_url TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    budget_limit INTEGER NOT NULL,
    token_usage INTEGER NOT NULL DEFAULT 0,
    error_message TEXT
);
```

**`0002_module_assessments.sql`**
```sql
CREATE TABLE module_assessments (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES jobs(id),
    module_id TEXT NOT NULL,
    module_name TEXT NOT NULL,
    assessment_json TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(job_id, module_id)
);
```

**`0003_checkpoints.sql`**
```sql
CREATE TABLE checkpoints (
    job_id TEXT PRIMARY KEY REFERENCES jobs(id),
    last_completed_phase TEXT,
    completed_modules TEXT NOT NULL,  -- JSON array
    token_usage INTEGER NOT NULL,
    partial_results TEXT,
    saved_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**`0004_reports.sql`**
```sql
CREATE TABLE reports (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL UNIQUE REFERENCES jobs(id),
    assessment_json TEXT NOT NULL,
    markdown_content TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**`0005_cache.sql`**
```sql
CREATE TABLE analysis_cache (
    repo_url TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    cached_assessment TEXT NOT NULL,
    ttl_days INTEGER NOT NULL DEFAULT 30,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (repo_url, commit_sha)
);
```

### File: `src/db/job_store.rs`

```rust
pub struct SqlxAssessmentJobStore {
    pool: sqlx::AnyPool,
}

#[async_trait]
impl AssessmentJobStore for SqlxAssessmentJobStore {
    async fn save(&self, job: AssessmentJob) -> Result<(), StoreError> {
        sqlx::query!(
            r#"
            INSERT INTO jobs (id, repo_url, status, budget_limit, token_usage)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(id) DO UPDATE SET status = ?3, updated_at = CURRENT_TIMESTAMP
            "#,
            job.id,
            job.repo_url,
            format!("{:?}", job.status),  // Simplified
            job.budget_limit,
            job.token_usage,
        ).execute(&self.pool).await?;
        Ok(())
    }
    
    async fn find_by_id(&self, job_id: &str) -> Result<Option<AssessmentJob>, StoreError> {
        let row = sqlx::query!("SELECT * FROM jobs WHERE id = ?1", job_id)
            .fetch_optional(&self.pool)
            .await?;
        
        Ok(row.map(|r| AssessmentJob {
            id: r.id,
            repo_url: r.repo_url,
            status: JobStatus::Queued,  // Parse from r.status
            // ... other fields
        }))
    }
    
    async fn find_by_status(&self, status: JobStatus) -> Result<Vec<AssessmentJob>, StoreError> {
        let status_str = format!("{:?}", status);
        let rows = sqlx::query!("SELECT * FROM jobs WHERE status = ?1", status_str)
            .fetch_all(&self.pool)
            .await?;
        
        Ok(rows.into_iter().map(|_| AssessmentJob { /* ... */ }).collect())
    }
    
    async fn find_concurrent_for_repo(&self, repo_url: &str) -> Result<Vec<AssessmentJob>, StoreError> {
        // Find jobs for same repo running concurrently
        Ok(vec![])
    }
}
```

### File: `src/db/pool.rs`

```rust
pub async fn create_pool(database_url: &str) -> Result<sqlx::AnyPool, sqlx::Error> {
    let pool = sqlx::AnyPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    
    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;
    
    Ok(pool)
}
```

### File: `src/db/cache.rs`

```rust
pub struct AnalysisCacheStore {
    pool: sqlx::AnyPool,
}

impl AnalysisCacheStore {
    pub async fn get(&self, repo_url: &str, commit_sha: &str) -> Result<Option<Assessment>, StoreError> {
        let row = sqlx::query!("SELECT cached_assessment FROM analysis_cache WHERE repo_url = ?1 AND commit_sha = ?2 AND created_at > datetime('now', '-' || ttl_days || ' days')", repo_url, commit_sha)
            .fetch_optional(&self.pool)
            .await?;
        
        Ok(row.and_then(|r| serde_json::from_str(&r.cached_assessment).ok()))
    }
    
    pub async fn set(&self, repo_url: &str, commit_sha: &str, assessment: &Assessment, ttl_days: i32) -> Result<(), StoreError> {
        let json = serde_json::to_string(assessment)?;
        sqlx::query!("INSERT INTO analysis_cache (repo_url, commit_sha, cached_assessment, ttl_days) VALUES (?1, ?2, ?3, ?4)", repo_url, commit_sha, json, ttl_days)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    
    pub async fn invalidate(&self, repo_url: &str) -> Result<(), StoreError> {
        sqlx::query!("DELETE FROM analysis_cache WHERE repo_url = ?1", repo_url)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
```

### File: `scripts/prepare-sqlx.sh`

```bash
#!/bin/bash
cd repogate-server
cargo sqlx prepare -- --lib
```

---

## Source Documents to Read

- **`docs/adr/ADR-014-persistence-sqlx-sqlite-postgres.md`** — sqlx, SQLite dev/Postgres prod, compile-time queries, offline mode

---

## Acceptance Criteria

- ✅ `cargo build -p repogate-server` with sqlx offline mode (`sqlx-data.json` committed)
- ✅ Migrations run cleanly on fresh SQLite DB
- ✅ SqlxAssessmentJobStore save→find_by_id round-trips
- ✅ Cache set→get returns stored assessment; past-TTL → None
- ✅ `cargo test -p repogate-server` passes (in-memory SQLite)

---

## Language

**Rust** — SQL, sqlx query macros, async database operations.

---

## Out-of-Scope

- Do NOT implement connection pooling configuration beyond defaults
- Do NOT implement complex SQL queries
