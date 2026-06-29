# Bounded Context: GatingStrategy

**Subdomain**: Core Domain
**Crate**: `rg-gating-strategy`

---

## Purpose

GatingStrategy is the decision layer that converts scores into strategy. It takes `ValuationReport` composite scores, license compliance findings, and architecture layer data, and maps each module to a discrete commercial tier. It then defines the `OpenCoreBoundary` — the precise line between what stays open and what becomes paid.

This is the context that answers the central question: *what goes in each tier?* It does not produce reports or enforce legal constraints — it produces a `GatingStrategy` aggregate that downstream contexts (RiskAnalysis, ReportDelivery) use to generate the final recommendation.

The strategy is opinionated but configurable. Default tier thresholds encode open-core best practices, but operators can adjust weights and thresholds for their specific market context.

---

## Ubiquitous Language

| Term | Definition |
|---|---|
| **GatingStrategy** | The aggregate root. The complete tier assignment plan for one repository. |
| **GatingTier** | A discrete commercial classification for a module or capability: `open`, `source_available`, `pro_tier`, `enterprise_tier`, `managed_cloud`, `legal_review`, `not_recommended`. |
| **OpenCoreBoundary** | The value object defining which modules are in the open-source core and which are gated. This is the central deliverable. |
| **TierAssignment** | The mapping of one module or capability to a `GatingTier`, along with the rationale. |
| **PackagingRecommendation** | A higher-level recommendation for how to package the repository (community edition + pro + enterprise, source-available model, etc.). |
| **TierThreshold** | The composite score range that maps to a given tier. Configurable. |
| **OpenCoreRatio** | The proportion of code/capability that ends up in the open tier vs. gated tiers. A health check: if > 80% is gated, over-gating risk is elevated. |
| **BoundaryStability** | How clear and maintainable the open/gated boundary is. High stability means the boundary follows natural module lines; low stability means it cuts across modules. |
| **LicenseConstrainedTier** | A tier assignment that was forced by a license finding (e.g., a module cannot be gated because it is AGPL). |

---

## Aggregate Root: `GatingStrategy`

`GatingStrategy` is built from `TierAssignment`s and defines the `OpenCoreBoundary` once all modules are assigned.

### State Transitions

```
pending → assigning_tiers → defining_boundary → recommending_packaging → complete → failed
```

### Entity: `TierAssignment`

| Field | Type | Notes |
|---|---|---|
| `id` | `TierAssignmentId` | |
| `module_node_id` | `ModuleNodeId` | |
| `module_name` | `String` | Denormalised |
| `assigned_tier` | `GatingTier` | |
| `effective_composite` | `CompositeScore` | The score that drove this assignment |
| `license_constrained` | `bool` | True if tier was forced by license findings |
| `rationale` | `String` | Why this tier was assigned |
| `tier_features` | `Vec<String>` | Which product features this module enables in its tier |

### Value Objects

#### `GatingTier`
```rust
pub enum GatingTier {
    Open,             // stays in open-source core; no gating
    SourceAvailable,  // code visible, usage restricted by license (e.g., BUSL, SSPLesque)
    ProTier,          // paid individual/small team tier
    EnterpriseTier,   // paid enterprise tier (RBAC, SSO, audit, SLA)
    ManagedCloud,     // only available as cloud-hosted managed service
    LegalReview,      // cannot be assigned until legal review is complete
    NotRecommended,   // should not be gated; either for adoption or legal reasons
}
```

`GatingTier` has an ordering that reflects commercial intensity:
`Open < SourceAvailable < ProTier < EnterpriseTier < ManagedCloud`
(`LegalReview` and `NotRecommended` are out-of-band).

#### `OpenCoreBoundary`
```rust
pub struct OpenCoreBoundary {
    pub open_modules: Vec<ModuleNodeId>,
    pub gated_modules: Vec<ModuleNodeId>,  // all non-Open tiers
    pub open_core_ratio: f32,              // open_loc / total_loc
    pub boundary_stability: BoundaryStability,
    pub cuts_across_modules: bool,         // true if any single module is split between tiers
    pub summary: String,                   // human-readable boundary description
}
```

#### `TierThreshold`
```rust
pub struct TierThreshold {
    pub tier: GatingTier,
    pub min_composite: f32,  // inclusive lower bound
    pub max_composite: f32,  // exclusive upper bound
}
// Default mapping (adjustable):
// [0.0, 3.5)  → NotRecommended or Open (adoption_value drives the split)
// [3.5, 5.5)  → SourceAvailable or Open
// [5.5, 7.0)  → ProTier
// [7.0, 8.5)  → EnterpriseTier
// [8.5, 10.0] → ManagedCloud
```

#### `BoundaryStability`
```rust
pub enum BoundaryStability {
    High,    // boundary follows natural module/crate/package lines
    Medium,  // some boundary crossing at file level
    Low,     // boundary cuts within files or tightly coupled modules
}
```

#### `PackagingRecommendation`
```rust
pub struct PackagingRecommendation {
    pub model: PackagingModel,  // OpenCoreClassic | SourceAvailableFirst | ManagedCloudFirst | HybridCommunity
    pub tiers: Vec<TierDefinition>,
    pub open_source_edition_scope: String,
    pub commercial_edition_scope: String,
    pub rationale: String,
}
```

---

## Invariants

1. Every `ModuleNode` in the `ArchitectureMap` must receive exactly one `TierAssignment`.
2. A module with a strong-copyleft license (`CopyleftType::StrongCopyleft`) must be assigned `GatingTier::Open` or `GatingTier::LegalReview`. It may never be assigned `ProTier`, `EnterpriseTier`, or `ManagedCloud` without explicit override and recorded justification.
3. `OpenCoreRatio` is always derived from actual `TierAssignment` data; it may not be set manually.
4. If `OpenCoreRatio < 0.20`, the strategy engine raises a `OverGatingRisk` warning before completing.
5. `Layer::Core` modules with `centrality > threshold` and assigned `GatingTier::EnterpriseTier` trigger a `HighCentralityGatingWarning` — gating a central module risks breaking the open-source experience.
6. `OpenCoreBoundary.cuts_across_modules` is `true` only when a single logical module is split between `GatingTier::Open` and a paid tier. This is considered an architectural smell and is flagged.
7. `TierThreshold` ranges must be contiguous and non-overlapping. Validation at construction.

---

## Domain Events

### `TierAssigned`
Emitted per module when a tier assignment is determined.
```rust
pub struct TierAssigned {
    pub repo_id: RepositoryId,
    pub strategy_id: GatingStrategyId,
    pub assignment: TierAssignment,
    pub assigned_at: DateTime<Utc>,
}
```

### `BoundaryDefined`
Emitted when the `OpenCoreBoundary` is computed.
```rust
pub struct BoundaryDefined {
    pub repo_id: RepositoryId,
    pub strategy_id: GatingStrategyId,
    pub boundary: OpenCoreBoundary,
    pub defined_at: DateTime<Utc>,
}
```

### `StrategyGenerated`
Emitted when the full strategy including `PackagingRecommendation` is complete.
```rust
pub struct StrategyGenerated {
    pub repo_id: RepositoryId,
    pub strategy_id: GatingStrategyId,
    pub packaging: PackagingRecommendation,
    pub generated_at: DateTime<Utc>,
}
```

### `OverGatingRiskFlagged`
Emitted when `OpenCoreRatio < 0.20`.
```rust
pub struct OverGatingRiskFlagged {
    pub repo_id: RepositoryId,
    pub strategy_id: GatingStrategyId,
    pub open_core_ratio: f32,
    pub gated_module_count: u32,
}
```

### `LicenseConstrainedTierAssigned`
Emitted when a tier is forced by a license finding rather than by scoring.
```rust
pub struct LicenseConstrainedTierAssigned {
    pub repo_id: RepositoryId,
    pub strategy_id: GatingStrategyId,
    pub module_node_id: ModuleNodeId,
    pub forced_tier: GatingTier,
    pub license_reason: String,
}
```

---

## Repository Interface

```rust
pub trait GatingStrategyStore {
    async fn save(&self, strategy: &GatingStrategy) -> Result<(), StoreError>;
    async fn find_by_repo_id(&self, repo_id: RepositoryId) -> Result<Option<GatingStrategy>, StoreError>;
    async fn append_assignment(&self, strategy_id: GatingStrategyId, assignment: TierAssignment) -> Result<(), StoreError>;
    async fn list_assignments(&self, strategy_id: GatingStrategyId) -> Result<Vec<TierAssignment>, StoreError>;
    async fn list_by_tier(&self, strategy_id: GatingStrategyId, tier: GatingTier) -> Result<Vec<TierAssignment>, StoreError>;
}
```

---

## Integration Points

| Context | Direction | What crosses the boundary |
|---|---|---|
| CommercialValuation | Upstream | Subscribes to `ValuationComplete`; reads `ModuleScore` list |
| LicenseCompliance | Upstream | Subscribes to `LicensesScanned`; uses license findings to constrain tier assignment |
| ArchitectureMapping | Upstream | Reads `ModuleNode` centrality and layer to inform boundary stability |
| FunctionalityDiscovery | Upstream | Reads `FunctionalityItem` list to populate `tier_features` |
| AssessmentOrchestration | Coordinator | Issues `GenerateStrategy` command; subscribes to `StrategyGenerated` |
| RiskAnalysis | Downstream | Subscribes to `StrategyGenerated`, `OverGatingRiskFlagged`, `BoundaryDefined` |
| ReportDelivery | Downstream | Reads `GatingStrategy` for the strategy section of the report |

### Anti-Corruption Layer

No formal ACL between GatingStrategy and its upstreams — all inputs arrive as typed domain events. The one translation that matters is from `TierThreshold` configuration (which may come from operator settings, stored as JSON) into the internal `TierThreshold` value objects; this is handled by an infrastructure configuration adapter.
