# Bounded Context: FunctionalityDiscovery

**Subdomain**: Core Domain
**Crate**: `rg-functionality-discovery`

---

## Purpose

FunctionalityDiscovery is the first pillar of RepoGate's competitive moat. Its job is to produce a **complete inventory of everything the repository actually does** — not what its README claims, not what its top-level folder names suggest, but what the code itself implements.

This means surfacing:
- Documented public features (the easy part)
- Internal, private, and experimental capabilities
- Features only visible through tests or examples
- Undocumented enterprise or advanced capabilities
- Hidden workflows, CLI flags, SDK methods, API endpoints
- Capabilities that cross multiple files or modules

The output is a `FunctionalityInventory` that CommercialValuation and GatingStrategy use to make scoring and tier assignment decisions. If this context misses a capability, that capability is invisible to every downstream decision.

The analysis is Claude Code-assisted: AssessmentOrchestration fans out `ClaudeSession` instances (one per module or file cluster), and the structured findings are ingested here as `FunctionalityItem` entities.

---

## Ubiquitous Language

| Term | Definition |
|---|---|
| **FunctionalityInventory** | The aggregate root. The complete catalogue of discovered capabilities for one repository. |
| **FunctionalityItem** | A single discrete capability: a feature, workflow, endpoint, CLI command, SDK method, or behavior that the repository implements. |
| **Visibility** | The access classification of a capability: `public`, `internal`, `experimental`, `undocumented`, or `enterprise`. |
| **DiscoveryMethod** | How a capability was found: `PublicApi`, `TestCoverage`, `ExampleCode`, `CliInspection`, `SourceTracing`, `ConfigAnalysis`, `DocumentationCross`, `LlmInference`. |
| **SourceLocation** | The file path(s) and line range(s) where the capability is implemented or exercised. |
| **ApiSurface** | The set of publicly exported functions, types, endpoints, CLI commands, and SDK methods. |
| **HiddenCapability** | A `FunctionalityItem` with `Visibility::undocumented` or `Visibility::internal` that was not mentioned in any public documentation. |
| **EnterpriseCapability** | A `FunctionalityItem` with `Visibility::enterprise` that appears to target enterprise use cases (multi-tenant, RBAC, audit, SSO, SLA features). |
| **FeatureCluster** | A group of `FunctionalityItem`s that together form a coherent capability (e.g., all items related to "audit logging"). |
| **ApiSurfaceMap** | A structured mapping of all public API entry points — HTTP routes, CLI subcommands, SDK exports — with their signatures and documentation status. |
| **CapabilityTag** | A free-form label used to group capabilities by domain concept (e.g., `auth`, `storage`, `observability`). |

---

## Aggregate Root: `FunctionalityInventory`

`FunctionalityInventory` is built incrementally as `FunctionalityItem` entities are discovered. It reaches `complete` when AssessmentOrchestration signals that all module scans have finished.

### State Transitions

```
pending → discovering → api_surface_mapping → cross_referencing → complete → failed
```

### Entity: `FunctionalityItem`

| Field | Type | Notes |
|---|---|---|
| `id` | `FunctionalityItemId` | |
| `name` | `String` | Short capability name, e.g. "RBAC role assignment" |
| `description` | `String` | One-paragraph description from analysis |
| `visibility` | `Visibility` | `public \| internal \| experimental \| undocumented \| enterprise` |
| `discovery_method` | `DiscoveryMethod` | How it was found |
| `source_locations` | `Vec<SourceLocation>` | At least one required |
| `tags` | `Vec<CapabilityTag>` | |
| `is_hidden` | `bool` | Derived: `visibility == undocumented \| internal` |
| `is_enterprise` | `bool` | Derived: `visibility == enterprise` |
| `api_entry_points` | `Vec<ApiEntryPoint>` | HTTP routes, CLI flags, exported symbols |
| `has_tests` | `bool` | Whether test coverage was found for this capability |
| `has_documentation` | `bool` | Whether public docs reference this capability |

### Value Objects

#### `Visibility`
```rust
pub enum Visibility {
    Public,         // documented, exported, user-facing
    Internal,       // used within the codebase, not exported
    Experimental,   // marked as experimental, unstable, or feature-flagged
    Undocumented,   // implemented but absent from public docs
    Enterprise,     // enterprise-specific: multi-tenant, RBAC, SSO, audit, SLA
}
```

#### `SourceLocation`
```rust
pub struct SourceLocation {
    pub file_path: PathBuf,
    pub start_line: Option<u32>,
    pub end_line: Option<u32>,
    pub symbol: Option<String>, // function, type, or endpoint name
}
```

#### `DiscoveryMethod`
```rust
pub enum DiscoveryMethod {
    PublicApi,           // found in exported interface
    TestCoverage,        // found in test files
    ExampleCode,         // found in examples/ or docs/
    CliInspection,       // found by inspecting CLI argument parsers
    SourceTracing,       // found by tracing call graphs
    ConfigAnalysis,      // found in config schemas or env var declarations
    DocumentationCross,  // cross-referenced from docs to impl
    LlmInference,        // inferred by Claude from code context
}
```

#### `ApiEntryPoint`
```rust
pub struct ApiEntryPoint {
    pub kind: ApiKind,  // HttpRoute | CliCommand | SdkExport | GrpcMethod
    pub path: String,   // e.g. "POST /api/v1/users", "gate analyze", "fn analyze_repo"
    pub is_documented: bool,
}
```

---

## Invariants

1. Every `FunctionalityItem` must have at least one `SourceLocation`. Items derived purely from LLM inference without a code reference are flagged with `discovery_method: LlmInference` and marked as unconfirmed until a source location is attached.
2. `Visibility::Enterprise` requires at least one supporting signal: a feature flag, a tier-check in code, an enterprise-specific config key, or a comment referencing enterprise/pro/commercial.
3. `FunctionalityInventory` may not transition to `complete` while any module scan is still in-flight (coordination via AssessmentOrchestration).
4. Duplicate `FunctionalityItem`s (same name + same source location) are merged, not duplicated.
5. `Visibility` is determined by evidence, not by the LLM's opinion. The LLM provides raw findings; the domain model classifies visibility based on observable signals in the code.
6. `ApiSurface` must be a subset of the `FunctionalityItem` list — every entry point must correspond to at least one item.

---

## Domain Events

### `FeaturesDiscovered`
Emitted when a batch of `FunctionalityItem`s is produced from one module scan.
```rust
pub struct FeaturesDiscovered {
    pub repo_id: RepositoryId,
    pub inventory_id: FunctionalityInventoryId,
    pub module_path: PathBuf,
    pub items: Vec<FunctionalityItem>,
    pub discovered_at: DateTime<Utc>,
}
```

### `HiddenCapabilityFound`
Emitted for each `FunctionalityItem` with `visibility: Undocumented | Internal` that has no public documentation reference.
```rust
pub struct HiddenCapabilityFound {
    pub repo_id: RepositoryId,
    pub inventory_id: FunctionalityInventoryId,
    pub item: FunctionalityItem,
    pub signal: String, // brief description of why it's considered hidden
}
```

### `ApiSurfaceMapped`
Emitted when the full API surface has been catalogued.
```rust
pub struct ApiSurfaceMapped {
    pub repo_id: RepositoryId,
    pub inventory_id: FunctionalityInventoryId,
    pub surface: ApiSurfaceMap,
    pub mapped_at: DateTime<Utc>,
}
```

### `EnterpriseCapabilityFound`
Emitted for each item classified as `Visibility::Enterprise`.
```rust
pub struct EnterpriseCapabilityFound {
    pub repo_id: RepositoryId,
    pub inventory_id: FunctionalityInventoryId,
    pub item: FunctionalityItem,
    pub enterprise_signals: Vec<String>,
}
```

### `InventoryComplete`
Emitted when all module scans have been ingested and the inventory is fully built.
```rust
pub struct InventoryComplete {
    pub repo_id: RepositoryId,
    pub inventory_id: FunctionalityInventoryId,
    pub total_items: u32,
    pub hidden_count: u32,
    pub enterprise_count: u32,
    pub completed_at: DateTime<Utc>,
}
```

---

## Repository Interface

```rust
pub trait FunctionalityInventoryStore {
    async fn save(&self, inventory: &FunctionalityInventory) -> Result<(), StoreError>;
    async fn find_by_repo_id(&self, repo_id: RepositoryId) -> Result<Option<FunctionalityInventory>, StoreError>;
    async fn append_item(&self, inventory_id: FunctionalityInventoryId, item: FunctionalityItem) -> Result<(), StoreError>;
    async fn list_items(&self, inventory_id: FunctionalityInventoryId) -> Result<Vec<FunctionalityItem>, StoreError>;
    async fn list_items_by_visibility(
        &self,
        inventory_id: FunctionalityInventoryId,
        visibility: Visibility,
    ) -> Result<Vec<FunctionalityItem>, StoreError>;
}
```

---

## Integration Points

| Context | Direction | What crosses the boundary |
|---|---|---|
| RepositoryIngestion | Upstream | Subscribes to `ManifestBuilt`; uses file list to determine scan scope |
| ArchitectureMapping | Upstream | Subscribes to `ModuleMapped`; uses module structure to organise per-module discovery |
| AssessmentOrchestration | Coordinator | Issues `DiscoverFeatures` commands per module; subscribes to `InventoryComplete` |
| CommercialValuation | Downstream | Subscribes to `InventoryComplete`; reads `FunctionalityItem` list to score each module |
| GatingStrategy | Downstream | Reads `FunctionalityInventory` to assign tiers per capability |
| ReportDelivery | Downstream | Reads full inventory for report assembly |

### Anti-Corruption Layer

The ACL concern here is the **LLM output boundary**. Claude sessions return unstructured or semi-structured text/JSON. The `LlmOutputAdapter` (in the infrastructure layer of this crate) is responsible for parsing Claude's output and mapping it into `FunctionalityItem` entities with proper `Visibility` classification. This adapter never leaks LLM-specific types into the domain. If the adapter cannot parse a finding, it is either discarded with a warning event or stored as an unconfirmed item pending human review.
