# Bounded Context: UserWorkflow

**Subdomain**: Generic
**Crate**: `rg-user-workflow`

---

## Purpose

UserWorkflow is the anti-corruption layer between user-facing surfaces (web dashboard, CLI) and the core domain. It accepts repository URL submissions, tracks job progress for display, and surfaces completed assessment results. It is deliberately thin — its only domain logic is validation of user input and translation of user-facing concepts into domain commands.

UI concepts must never leak into the domain. This context ensures that notions like "session cookies", "form state", "URL slugs", "user accounts", and "pagination" stay entirely within this bounded context and never appear in `AssessmentOrchestration` or any analysis context.

---

## Ubiquitous Language

| Term | Definition |
|---|---|
| **AnalysisRequest** | A user's submitted request to assess a repository URL. This is a UserWorkflow concept, not a domain concept. It is translated into an `AssessmentJob` command before crossing the boundary. |
| **JobView** | A read-model projection of an `AssessmentJob`'s status, suitable for display. Contains no internal domain state. |
| **ResultView** | A read-model projection of a completed `AssessmentReport`, structured for UI consumption. |
| **Submission** | The act of a user providing a URL and initiating an assessment. |
| **StatusPoll** | A user querying the current state of an in-progress assessment. |
| **SubmittedUrl** | The raw URL string as entered by the user, before `RepoUrl` validation. |

---

## Aggregate Root: (none — this context is primarily a read-model and ACL)

UserWorkflow does not have a domain aggregate in the traditional sense. Its primary responsibility is:
1. Translating `AnalysisRequest` → `SubmitAssessment` command for AssessmentOrchestration
2. Projecting `AssessmentJob` events into `JobView` read models for display
3. Projecting `AssessmentReport` events into `ResultView` read models

A lightweight `Submission` entity tracks user-submitted requests for idempotency and display.

### Entity: `Submission`

| Field | Type | Notes |
|---|---|---|
| `id` | `SubmissionId` | |
| `submitted_url` | `String` | Raw URL as entered |
| `job_id` | `Option<AssessmentJobId>` | Set once the job is created |
| `submitted_at` | `DateTime<Utc>` | |
| `submitted_by` | `Option<UserId>` | None for unauthenticated CLI use |
| `status` | `SubmissionStatus` | Mirrors job status for display |

### Value Object: `JobView`

```rust
pub struct JobView {
    pub job_id: AssessmentJobId,
    pub repo_url: String,
    pub status: JobStatus,
    pub current_phase: Option<String>,
    pub progress_pct: Option<u8>,    // rough estimate for progress bar
    pub submitted_at: DateTime<Utc>,
    pub elapsed_secs: Option<u64>,
    pub token_usage: Option<u64>,
}
```

### Value Object: `ResultView`

```rust
pub struct ResultView {
    pub report_id: AssessmentReportId,
    pub repo_url: String,
    pub executive_summary: String,
    pub packaging_recommendation: String,
    pub open_core_ratio: f32,
    pub high_risk_count: u32,
    pub formats_available: Vec<ReportFormat>,
    pub download_url: Option<String>,
    pub completed_at: DateTime<Utc>,
}
```

---

## The Anti-Corruption Layer

The ACL lives in the `SubmissionTranslator` domain service:

```rust
pub struct SubmissionTranslator;

impl SubmissionTranslator {
    /// Validates the raw URL and translates it into a domain SubmitAssessment command.
    /// Returns Err if the URL is invalid or unsupported.
    pub fn translate(&self, request: AnalysisRequest) -> Result<SubmitAssessmentCommand, SubmissionError>;
}
```

Key translations:
- `AnalysisRequest.url` → `RepoUrl` (validates scheme, host, path)
- `AnalysisRequest.budget_tokens` (optional UI field) → `TokenBudget`
- UI error messages are generated from domain errors without exposing domain internals

---

## Invariants

1. `SubmissionTranslator` must validate the `RepoUrl` before issuing the `SubmitAssessment` command. Invalid URLs are rejected with a user-friendly error message; no job is created.
2. `JobView` is a projection — it is never the source of truth for job status. The source of truth is the `AssessmentJob` aggregate in AssessmentOrchestration.
3. UserWorkflow may never directly read from another context's aggregate store. It reads only through events and the query projections it maintains.
4. No concept from AssessmentOrchestration (e.g., `PhaseKind`, `ClaudeSession`, `TokenBudget`) may appear in any response sent to a user. The ACL must translate these into user-friendly terms.

---

## Domain Events (published by UserWorkflow)

### `AssessmentRequested`
Emitted when a valid submission is translated and the `SubmitAssessment` command is dispatched.
```rust
pub struct AssessmentRequested {
    pub submission_id: SubmissionId,
    pub job_id: AssessmentJobId,
    pub repo_url: String,
    pub requested_at: DateTime<Utc>,
}
```

---

## Repository Interface

```rust
pub trait SubmissionStore {
    async fn save(&self, submission: &Submission) -> Result<(), StoreError>;
    async fn find_by_id(&self, id: SubmissionId) -> Result<Option<Submission>, StoreError>;
    async fn find_by_job_id(&self, job_id: AssessmentJobId) -> Result<Option<Submission>, StoreError>;
}

pub trait JobViewStore {
    async fn upsert(&self, view: &JobView) -> Result<(), StoreError>;
    async fn find_by_job_id(&self, job_id: AssessmentJobId) -> Result<Option<JobView>, StoreError>;
    async fn list_recent(&self, limit: u32) -> Result<Vec<JobView>, StoreError>;
}
```

---

## Integration Points

| Context | Direction | What crosses the boundary |
|---|---|---|
| AssessmentOrchestration | Downstream | Issues `SubmitAssessment` command; subscribes to `JobQueued`, `PhaseStarted`, `PhaseCompleted`, `JobCompleted`, `JobFailed` to update `JobView` |
| ReportDelivery | Downstream consumer | Subscribes to `ReportAssembled` to build `ResultView`; provides download links |

### Notes on Generic Subdomain Classification

UserWorkflow is classified as Generic because similar "submit job, track status, view result" workflows appear in many systems and can be built with standard patterns or off-the-shelf libraries. The domain-specific logic is entirely in the ACL translator, not in the tracking or display projections.
