# Bounded Context: ReportDelivery

**Subdomain**: Supporting
**Crate**: `rg-report-delivery`

---

## Purpose

ReportDelivery assembles outputs from all analysis contexts into the canonical `AssessmentReport` and delivers it through the requested channels (CLI output, web API, file export). It is the last step in the pipeline and the only context that knows the full shape of the final output.

This context is intentionally thin on domain logic. Its job is faithful assembly and formatting, not analysis. It reads completed aggregates from other contexts, combines them into a canonical structure, and serialises to the requested formats (JSON as source of truth, Markdown via `tera`/`minijinja`, optional PDF).

---

## Ubiquitous Language

| Term | Definition |
|---|---|
| **AssessmentReport** | The aggregate root. The complete, structured output of one repository assessment. |
| **ReportSection** | A named section (executive summary, functionality inventory, architecture map, scoring, gating strategy, risk analysis, etc.). |
| **DeliveryChannel** | How the output is delivered: `Cli`, `WebApi`, `FileExport`. |
| **ReportFormat** | The serialisation format: `Json`, `Markdown`, `Pdf`. |
| **ReportTemplate** | A `tera`/`minijinja` template used to render a section as Markdown. |
| **CanonicalJson** | The JSON serialisation that is the authoritative source of truth. All other formats are derived from this. |
| **ExecutiveSummary** | The opening section: key findings, recommended packaging model, highest-severity risks, and overall open-core recommendation. |
| **DeliveryAck** | Acknowledgement that output has been successfully delivered through a channel. |

---

## Aggregate Root: `AssessmentReport`

`AssessmentReport` is assembled from all the upstream aggregates produced by prior phases. It becomes immutable once it reaches `assembled` state.

### State Transitions

```
pending → assembling → assembled → delivering → delivered → failed
```

### Entity: `ReportSection`

| Field | Type | Notes |
|---|---|---|
| `id` | `ReportSectionId` | |
| `report_id` | `AssessmentReportId` | |
| `kind` | `SectionKind` | |
| `title` | `String` | |
| `content_json` | `serde_json::Value` | Structured section data |
| `render_order` | `u8` | Position in output |
| `is_complete` | `bool` | False if source data was partial (from partial recovery) |

#### `SectionKind`
```rust
pub enum SectionKind {
    ExecutiveSummary,
    FunctionalityInventory,
    ArchitectureMap,
    ModuleScoring,
    OpenCoreBoundary,
    GatingStrategy,
    LegalLicensing,
    RiskAnalysis,
    PackagingRecommendation,
    HiddenCapabilities,
    EnterpriseCapabilities,
}
```

### Value Objects

#### `ReportFormat`
```rust
pub enum ReportFormat {
    Json,
    Markdown,
    Pdf,  // rendered from Markdown; optional dependency
}
```

#### `DeliveryChannel`
```rust
pub enum DeliveryChannel {
    Cli,           // stdout or file path
    WebApi,        // HTTP response, streamed or buffered
    FileExport,    // written to disk at a specified path
}
```

#### `DeliveryAck`
```rust
pub struct DeliveryAck {
    pub channel: DeliveryChannel,
    pub format: ReportFormat,
    pub destination: Option<String>, // file path or URL
    pub delivered_at: DateTime<Utc>,
    pub size_bytes: u64,
}
```

---

## Invariants

1. `AssessmentReport` may not transition to `assembled` unless all required `SectionKind` variants are present. Optional sections (PDF-only extras) are exempt.
2. `CanonicalJson` is always produced first. Markdown and PDF are derived from it; they cannot be produced without it.
3. An `AssessmentReport` is immutable once `assembled`. No field may change after this state. A correction requires a new report.
4. `ReportSection.is_complete = false` is allowed only when `AssessmentJob` used partial recovery and a phase produced incomplete data. The output surfaces this as a coverage caveat in the `ExecutiveSummary`.
5. The `ExecutiveSummary` section must always be present and must reference the `OpenCoreBoundary` and at least the top three risks by severity.
6. Delivery to `DeliveryChannel::WebApi` is streamed (sections delivered as they are assembled) when the client requests streaming. Buffered delivery is the fallback.

---

## Domain Events

### `ReportAssembled`
Emitted when all sections are complete and the canonical JSON is ready.
```rust
pub struct ReportAssembled {
    pub job_id: AssessmentJobId,
    pub report_id: AssessmentReportId,
    pub section_count: u8,
    pub has_partial_sections: bool,
    pub assembled_at: DateTime<Utc>,
}
```

### `ReportDelivered`
Emitted per successful delivery.
```rust
pub struct ReportDelivered {
    pub job_id: AssessmentJobId,
    pub report_id: AssessmentReportId,
    pub ack: DeliveryAck,
}
```

### `ReportDeliveryFailed`
Emitted when a delivery attempt fails (e.g., disk full, API timeout).
```rust
pub struct ReportDeliveryFailed {
    pub job_id: AssessmentJobId,
    pub report_id: AssessmentReportId,
    pub channel: DeliveryChannel,
    pub reason: String,
    pub failed_at: DateTime<Utc>,
}
```

---

## Repository Interface

```rust
pub trait AssessmentReportStore {
    async fn save(&self, report: &AssessmentReport) -> Result<(), StoreError>;
    async fn find_by_job_id(&self, job_id: AssessmentJobId) -> Result<Option<AssessmentReport>, StoreError>;
    async fn find_by_id(&self, id: AssessmentReportId) -> Result<Option<AssessmentReport>, StoreError>;
    async fn append_section(&self, report_id: AssessmentReportId, section: ReportSection) -> Result<(), StoreError>;
    async fn save_delivery_ack(&self, report_id: AssessmentReportId, ack: DeliveryAck) -> Result<(), StoreError>;
}
```

---

## Integration Points

| Context | Direction | What crosses the boundary |
|---|---|---|
| AssessmentOrchestration | Upstream | Issues `AssembleReport` command; subscribes to `ReportAssembled`, `ReportDelivered` |
| FunctionalityDiscovery | Upstream | Reads `FunctionalityInventory` for the inventory section |
| ArchitectureMapping | Upstream | Reads `ArchitectureMap` for the architecture section |
| CommercialValuation | Upstream | Reads `ValuationReport` for the scoring section |
| GatingStrategy | Upstream | Reads `GatingStrategy` for the strategy and boundary sections |
| LicenseCompliance | Upstream | Reads `LicenseReport` for the legal section |
| RiskAnalysis | Upstream | Reads `RiskProfile` for the risk section |
| UserWorkflow | Downstream consumer | Subscribes to `ReportAssembled` to build `ResultView`; provides download links |

### Anti-Corruption Layer

The assembly step reads from multiple contexts' stores. The `ReportAssembler` domain service translates the internal types of each context into `ReportSection.content_json`. This translation layer ensures that changes to an upstream context's internal model do not require changes to the output schema as long as the translation adapter is updated. Each upstream context contributes one or more sections through its own `SectionAdapter` in the infrastructure layer.
