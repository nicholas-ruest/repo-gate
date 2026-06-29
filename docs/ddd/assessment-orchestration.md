# Bounded Context: AssessmentOrchestration

**Subdomain**: Core Domain (deepest and most novel)
**Crate**: `rg-assessment-orchestration`

---

## Purpose

AssessmentOrchestration is the operational heart of RepoGate. It owns the entire assessment lifecycle: from a submitted URL to a completed report. It sequences every phase, manages Claude Code session lifecycles, enforces token budgets, coordinates fan-out across modules, handles partial recovery, and drives all other contexts through commands and event subscriptions.

No other context knows about "the assessment". Every other bounded context knows only about its own job (scan licenses, score modules, etc.). AssessmentOrchestration is the only context that understands the end-to-end pipeline.

This is where the operational moat lives. Claude Code session management, sub-agent fan-out per module, context injection strategies, token budget enforcement, and partial recovery from failed phases are non-trivial engineering problems. Getting them right at scale is the execution challenge.

---

## Ubiquitous Language

| Term | Definition |
|---|---|
| **AssessmentJob** | The aggregate root. Represents one complete assessment of one repository, from submission to final report. Its state machine drives the entire pipeline. |
| **AnalysisPhase** | A single stage in the assessment pipeline (ingestion, license scan, architecture mapping, feature discovery, scoring, strategy, risk, report). Each phase is an entity with its own status. |
| **ClaudeSession** | An entity representing one active Claude Code session. Encapsulates the session ID, token usage, context state, and current task. Multiple sessions may be active simultaneously (fan-out). |
| **TokenBudget** | A value object defining the maximum allowed token consumption for a job (total) and per-phase limits. |
| **SessionId** | A value object wrapping the unique identifier of a Claude Code session. |
| **JobStatus** | The current state of an `AssessmentJob` in its state machine. |
| **PhaseStatus** | The current state of an `AnalysisPhase`: `pending`, `in_progress`, `completed`, `failed`, `skipped`. |
| **FanOut** | The pattern of spawning multiple parallel `ClaudeSession`s, one per module, to analyse modules concurrently. |
| **ContextInjection** | The process of loading relevant prior phase outputs into a new `ClaudeSession`'s context window before it begins analysis. |
| **PartialRecovery** | The ability to resume an `AssessmentJob` from its last completed `AnalysisPhase` after a failure, without restarting from scratch. |
| **BudgetExceeded** | The condition in which token consumption surpasses the `TokenBudget` limit. The job is paused or degraded gracefully. |
| **PhaseGate** | A mandatory check between phases that validates completeness of the prior phase's output before advancing. |
| **ModuleScanTask** | A single unit of fan-out work: one `ClaudeSession` assigned to analyse one `ModuleNode`. |
| **CommandDispatch** | The act of sending a command (e.g., `ScanLicenses`, `DiscoverFeatures`) to a downstream context from Orchestration. |
| **JobCheckpoint** | A persisted snapshot of an `AssessmentJob`'s progress used for partial recovery. |

---

## Aggregate Root: `AssessmentJob`

`AssessmentJob` is a process manager (also called a saga). It coordinates a long-running, multi-context workflow through a durable state machine. It does not perform analysis itself — it issues commands and reacts to domain events from other contexts.

### State Machine

```
queued
  │
  ▼
ingesting          (RepositoryIngestion running)
  │
  ▼
analyzing          (LicenseCompliance + ArchitectureMapping running in parallel;
  │                 then FunctionalityDiscovery fan-out running)
  ▼
scoring            (CommercialValuation running)
  │
  ▼
strategizing       (GatingStrategy running)
  │
  ▼
risk_analyzing     (RiskAnalysis running)
  │
  ▼
reporting          (ReportDelivery assembling report)
  │
  ▼
complete           (terminal: all phases done, report delivered)
  │
  (or at any point)
  ▼
failed             (terminal: unrecoverable error; checkpoint saved for recovery)
  │
  ▼
recovering         (resuming from last checkpoint)
```

### Entity: `AnalysisPhase`

| Field | Type | Notes |
|---|---|---|
| `id` | `AnalysisPhaseId` | |
| `job_id` | `AssessmentJobId` | |
| `phase_kind` | `PhaseKind` | `Ingestion \| LicenseScan \| ArchitectureMapping \| FeatureDiscovery \| Scoring \| Strategy \| RiskAnalysis \| ReportAssembly` |
| `status` | `PhaseStatus` | |
| `started_at` | `Option<DateTime<Utc>>` | |
| `completed_at` | `Option<DateTime<Utc>>` | |
| `tokens_used` | `u64` | Cumulative for this phase |
| `session_ids` | `Vec<SessionId>` | All Claude sessions used in this phase |
| `error` | `Option<PhaseError>` | Set on failure |
| `retry_count` | `u8` | Number of retry attempts |

### Entity: `ClaudeSession`

| Field | Type | Notes |
|---|---|---|
| `id` | `SessionId` | |
| `job_id` | `AssessmentJobId` | |
| `phase_id` | `AnalysisPhaseId` | |
| `module_node_id` | `Option<ModuleNodeId>` | Set for fan-out sessions; None for job-level sessions |
| `status` | `SessionStatus` | `active \| completed \| failed \| cancelled` |
| `tokens_input` | `u64` | |
| `tokens_output` | `u64` | |
| `context_injected` | `bool` | Whether prior phase context was loaded |
| `created_at` | `DateTime<Utc>` | |
| `completed_at` | `Option<DateTime<Utc>>` | |

### Value Objects

#### `JobStatus`
```rust
pub enum JobStatus {
    Queued,
    Ingesting,
    Analyzing,
    Scoring,
    Strategizing,
    RiskAnalyzing,
    Reporting,
    Complete,
    Failed,
    Recovering,
}
```

#### `TokenBudget`
```rust
pub struct TokenBudget {
    pub total_limit: u64,         // max tokens for the entire job
    pub per_phase_limit: u64,     // max tokens per phase
    pub per_session_limit: u64,   // max tokens per individual Claude session
    pub warn_threshold: f32,      // fraction at which to emit BudgetWarning (e.g., 0.80)
}

impl TokenBudget {
    pub fn is_exceeded(&self, used: u64) -> bool;
    pub fn is_warn_threshold_crossed(&self, used: u64) -> bool;
    pub fn remaining(&self, used: u64) -> u64;
}
```

#### `SessionId`
```rust
pub struct SessionId(Uuid);
```

#### `PhaseKind`
```rust
pub enum PhaseKind {
    Ingestion,
    LicenseScan,
    ArchitectureMapping,
    FeatureDiscovery,    // the fan-out phase; spawns N sessions, one per module
    Scoring,
    Strategy,
    RiskAnalysis,
    ReportAssembly,
}
```

#### `JobCheckpoint`
```rust
pub struct JobCheckpoint {
    pub job_id: AssessmentJobId,
    pub last_completed_phase: PhaseKind,
    pub completed_phase_ids: Vec<AnalysisPhaseId>,
    pub token_usage_so_far: u64,
    pub partial_results: serde_json::Value, // serialised intermediate outputs
    pub saved_at: DateTime<Utc>,
}
```

---

## Phase Sequencing and Fan-Out Logic

### Phase 1: Ingestion
- Issue `CloneRepository` to RepositoryIngestion.
- Wait for `ManifestBuilt`.
- Open exactly one `ClaudeSession` for repository-level context gathering.

### Phase 2: Analysis (parallel sub-phases)
- Issue `ScanLicenses` to LicenseCompliance (parallel).
- Issue `MapArchitecture` to ArchitectureMapping (parallel).
- Both run concurrently. Wait for `LicensesScanned` AND `DependencyGraphBuilt`.
- Phase gate: both must complete before proceeding.

### Phase 3: Feature Discovery (fan-out)
- For each `ModuleNode` in the `ArchitectureMap`:
  - Spawn one `ClaudeSession` with the module's files as context.
  - Inject prior phase summaries (manifest, architecture map) into the session context.
  - Issue `DiscoverFeatures(module_id)` within that session.
  - Track `tokens_used` per session against `TokenBudget.per_session_limit`.
- Collect `FeaturesDiscovered` events.
- Maximum concurrent sessions: configurable (default 8, respects rate limits).
- Wait for all `FeaturesDiscovered` events for all modules.

### Phase 4: Scoring
- Issue `ScoreModules` to CommercialValuation.
- Wait for `ValuationComplete`.

### Phase 5: Strategy
- Issue `GenerateStrategy` to GatingStrategy.
- Wait for `StrategyGenerated`.

### Phase 6: Risk Analysis
- Issue `AnalyzeRisks` to RiskAnalysis.
- Wait for `RiskProfileComplete`.

### Phase 7: Report Assembly
- Issue `AssembleReport` to ReportDelivery.
- Wait for `ReportAssembled`.
- Transition `AssessmentJob` to `complete`.

---

## Phase Gate Invariants

Between each phase, the following checks run before advancing:

| Phase Gate | Check |
|---|---|
| Ingestion → Analysis | `Repository.state == indexed`, `ModuleManifest` present |
| Analysis → Feature Discovery | `LicensesScanned` and `DependencyGraphBuilt` both received |
| Feature Discovery → Scoring | All `ModuleNode`s have at least one `FeaturesDiscovered` event |
| Scoring → Strategy | `ValuationComplete` received with count matching `ModuleNode` count |
| Strategy → Risk | `StrategyGenerated` received |
| Risk → Report | `RiskProfileComplete` received |

---

## Invariants

1. `AssessmentJob` is the only context that may issue commands to other bounded contexts. No context may call another directly — all cross-context communication goes through domain events or Orchestration commands.
2. `TokenBudget.total_limit` is checked after every `ClaudeSession` completes. If exceeded, the job transitions to `failed` with `reason: BudgetExceeded`, and a `JobCheckpoint` is saved.
3. `ClaudeSession` entities may not outlive their parent `AnalysisPhase`. When a phase fails, all active sessions in that phase are cancelled.
4. Fan-out concurrency must respect the Claude API rate limit. The orchestrator enforces a configurable session concurrency ceiling.
5. `PartialRecovery` may only resume from a `JobCheckpoint` that corresponds to a fully completed phase. Resuming mid-phase restarts the phase from scratch.
6. A `PhaseGate` failure transitions the job to `failed` immediately, regardless of how many phases have completed.
7. `AssessmentJob.repo_id` is immutable after the job is created. A new URL always creates a new job.
8. Two `AssessmentJob`s for the same `(RepoUrl, CommitSha)` may not run concurrently; the second is queued until the first completes or fails.

---

## Domain Events

### `JobQueued`
```rust
pub struct JobQueued {
    pub job_id: AssessmentJobId,
    pub repo_url: RepoUrl,
    pub submitted_at: DateTime<Utc>,
    pub token_budget: TokenBudget,
}
```

### `PhaseStarted`
```rust
pub struct PhaseStarted {
    pub job_id: AssessmentJobId,
    pub phase_id: AnalysisPhaseId,
    pub phase_kind: PhaseKind,
    pub started_at: DateTime<Utc>,
}
```

### `PhaseCompleted`
```rust
pub struct PhaseCompleted {
    pub job_id: AssessmentJobId,
    pub phase_id: AnalysisPhaseId,
    pub phase_kind: PhaseKind,
    pub tokens_used: u64,
    pub completed_at: DateTime<Utc>,
}
```

### `PhaseGateFailed`
```rust
pub struct PhaseGateFailed {
    pub job_id: AssessmentJobId,
    pub phase_kind: PhaseKind,
    pub reason: PhaseGateFailureReason,
    pub failed_at: DateTime<Utc>,
}
```

### `JobFailed`
```rust
pub struct JobFailed {
    pub job_id: AssessmentJobId,
    pub reason: JobFailureReason,
    pub last_completed_phase: Option<PhaseKind>,
    pub failed_at: DateTime<Utc>,
    pub checkpoint_saved: bool,
}
```

### `JobCompleted`
```rust
pub struct JobCompleted {
    pub job_id: AssessmentJobId,
    pub total_tokens_used: u64,
    pub duration_secs: u64,
    pub report_id: AssessmentReportId,
    pub completed_at: DateTime<Utc>,
}
```

### `BudgetExceeded`
```rust
pub struct BudgetExceeded {
    pub job_id: AssessmentJobId,
    pub phase_kind: PhaseKind,
    pub budget: TokenBudget,
    pub actual_used: u64,
    pub exceeded_at: DateTime<Utc>,
}
```

### `BudgetWarning`
Emitted when token usage crosses the `warn_threshold`.
```rust
pub struct BudgetWarning {
    pub job_id: AssessmentJobId,
    pub used: u64,
    pub limit: u64,
    pub fraction: f32,
}
```

### `SessionSpawned`
```rust
pub struct SessionSpawned {
    pub job_id: AssessmentJobId,
    pub phase_id: AnalysisPhaseId,
    pub session: ClaudeSession,
}
```

### `SessionCompleted`
```rust
pub struct SessionCompleted {
    pub job_id: AssessmentJobId,
    pub session_id: SessionId,
    pub tokens_used: u64,
    pub completed_at: DateTime<Utc>,
}
```

---

## Repository Interface

```rust
pub trait AssessmentJobStore {
    async fn save(&self, job: &AssessmentJob) -> Result<(), StoreError>;
    async fn find_by_id(&self, id: AssessmentJobId) -> Result<Option<AssessmentJob>, StoreError>;
    async fn find_by_status(&self, status: JobStatus) -> Result<Vec<AssessmentJob>, StoreError>;
    async fn find_concurrent_for_repo(
        &self,
        url: &RepoUrl,
        sha: &CommitSha,
    ) -> Result<Option<AssessmentJob>, StoreError>;
    async fn save_checkpoint(&self, checkpoint: &JobCheckpoint) -> Result<(), StoreError>;
    async fn load_checkpoint(&self, job_id: AssessmentJobId) -> Result<Option<JobCheckpoint>, StoreError>;
}

pub trait ClaudeSessionStore {
    async fn save(&self, session: &ClaudeSession) -> Result<(), StoreError>;
    async fn find_active_for_job(&self, job_id: AssessmentJobId) -> Result<Vec<ClaudeSession>, StoreError>;
    async fn update_token_usage(&self, session_id: SessionId, input: u64, output: u64) -> Result<(), StoreError>;
    async fn cancel_all_for_phase(&self, phase_id: AnalysisPhaseId) -> Result<u32, StoreError>;
}
```

---

## Integration Points

| Context | Direction | What crosses the boundary |
|---|---|---|
| UserWorkflow | Upstream | Receives `SubmitAssessment` command; emits `JobQueued` |
| RepositoryIngestion | Downstream command | Issues `CloneRepository`; subscribes to `ManifestBuilt`, `IngestionFailed` |
| LicenseCompliance | Downstream command | Issues `ScanLicenses`; subscribes to `LicensesScanned` |
| ArchitectureMapping | Downstream command | Issues `MapArchitecture`; subscribes to `DependencyGraphBuilt` |
| FunctionalityDiscovery | Downstream command | Issues `DiscoverFeatures` per module; subscribes to `InventoryComplete` |
| CommercialValuation | Downstream command | Issues `ScoreModules`; subscribes to `ValuationComplete` |
| GatingStrategy | Downstream command | Issues `GenerateStrategy`; subscribes to `StrategyGenerated` |
| RiskAnalysis | Downstream command | Issues `AnalyzeRisks`; subscribes to `RiskProfileComplete` |
| ReportDelivery | Downstream command | Issues `AssembleReport`; subscribes to `ReportAssembled`, `ReportDelivered` |

### Anti-Corruption Layer

The most critical ACL is the **Claude API boundary**. The `ClaudeApiAdapter` (infrastructure) translates between:
- Domain commands (`DiscoverFeatures(module_id, context)`) → Claude API requests (with appropriate system prompt, file content, and prior context injected)
- Claude API responses (streamed tokens, tool calls, structured JSON) → Domain events (`FeaturesDiscovered`, `ModuleScored`, etc.)

This adapter is the single point of integration with the external LLM. No domain type ever holds a raw Claude API response object. The adapter also enforces `per_session_limit` token tracking and emits `BudgetWarning` / `BudgetExceeded` events when thresholds are crossed.

A second ACL is the **event bus adapter**: this context subscribes to events from all other contexts through a typed event router that maps raw serialised events back to strongly-typed domain event structs. Deserialisation failures are treated as infrastructure errors, not domain failures.
