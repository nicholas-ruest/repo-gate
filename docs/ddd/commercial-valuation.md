# Bounded Context: CommercialValuation

**Subdomain**: Core Domain
**Crate**: `rg-commercial-valuation`

---

## Purpose

CommercialValuation is the scoring engine at the heart of RepoGate's commercial recommendations. It takes every discovered module and capability and evaluates them across eight commercial dimensions, producing a composite score that GatingStrategy uses to assign tiers.

The scoring model encodes the expertise of open-core commercialization: understanding which capabilities drive enterprise adoption, which create competitive leverage, which are safe to open, and which must be protected. This is proprietary judgment operationalized as domain rules, not a commodity calculation.

License risk from LicenseCompliance is incorporated as a ninth sub-score that can override or constrain the composite.

---

## Ubiquitous Language

| Term | Definition |
|---|---|
| **ValuationReport** | The aggregate root. Contains all module scores for one repository assessment. |
| **ModuleScore** | The scored evaluation of a single `ModuleNode`. Contains scores across all 8 dimensions plus a composite. |
| **CommercialScore** | A value object holding the 8-dimension scores for a single module. Each dimension is a `f32` in `[0.0, 10.0]`. |
| **ScoreWeights** | A configurable value object determining the relative importance of each dimension in the composite. Defaults are set by domain experts; can be overridden per assessment. |
| **CompositeScore** | The weighted sum of all 8 dimension scores, normalised to `[0.0, 10.0]`. |
| **LicenseRiskSubScore** | An optional override or constraint derived from `CopyleftExposure` in LicenseCompliance. Copyleft exposure can cap or reduce the composite. |
| **GatingSignal** | A derived indicator from `CompositeScore` suggesting whether a module is a candidate for gating: `strong_gate_candidate`, `weak_gate_candidate`, `open_candidate`, `undetermined`. |
| **Dimension** | One of the 8 axes on which a module is scored. See below. |
| **ScoringRationale** | A text explanation (produced by Claude) of why a module received a particular score on a given dimension. |

---

## The 8 Scoring Dimensions

| # | Dimension Name | What it Measures |
|---|---|---|
| 1 | `adoption_value` | How important this module is for open-source adoption, community growth, and developer on-ramp. High → keep open. |
| 2 | `enterprise_buyer_value` | How much enterprise buyers are willing to pay for this capability. High → gate. |
| 3 | `commercial_leverage` | Whether this module creates pricing power or locks in commercial customers. High → gate. |
| 4 | `competitive_sensitivity` | How much publishing this module helps competitors. High → protect or gate. |
| 5 | `operational_value` | Whether this module provides operational differentiation (scaling, reliability, observability) that enterprise teams pay for. |
| 6 | `security_sensitivity` | Whether this module handles sensitive operations (auth, encryption, secrets, audit). High → gate or careful review. |
| 7 | `support_burden` | How much ongoing support this module requires. High burden → disincentive to gate (support cost exceeds revenue). |
| 8 | `strategic_importance` | How central this module is to the long-term product roadmap and moat. High → protect. |

---

## Aggregate Root: `ValuationReport`

`ValuationReport` accumulates `ModuleScore` entities as each module is evaluated. It is complete when all modules from the `ArchitectureMap` have been scored.

### State Transitions

```
pending → scoring → incorporating_license_risk → aggregating → complete → failed
```

### Entity: `ModuleScore`

| Field | Type | Notes |
|---|---|---|
| `id` | `ModuleScoreId` | |
| `module_node_id` | `ModuleNodeId` | Reference to the scored module |
| `module_name` | `String` | Denormalised for report convenience |
| `scores` | `CommercialScore` | 8-dimension scores |
| `composite` | `CompositeScore` | Weighted aggregate |
| `license_risk_sub_score` | `Option<LicenseRiskSubScore>` | Nil if no copyleft exposure |
| `effective_composite` | `CompositeScore` | Composite adjusted by license risk |
| `gating_signal` | `GatingSignal` | Derived from `effective_composite` |
| `rationale` | `ScoringRationale` | Per-dimension justification |
| `scored_at` | `DateTime<Utc>` | |

### Value Objects

#### `CommercialScore`
```rust
pub struct CommercialScore {
    pub adoption_value: Score,          // 0.0–10.0
    pub enterprise_buyer_value: Score,
    pub commercial_leverage: Score,
    pub competitive_sensitivity: Score,
    pub operational_value: Score,
    pub security_sensitivity: Score,
    pub support_burden: Score,
    pub strategic_importance: Score,
}
```

#### `Score`
```rust
pub struct Score(f32); // invariant: 0.0 <= inner <= 10.0
impl Score {
    pub fn new(value: f32) -> Result<Score, ScoreRangeError>;
}
```

#### `ScoreWeights`
```rust
pub struct ScoreWeights {
    pub adoption_value: f32,
    pub enterprise_buyer_value: f32,
    pub commercial_leverage: f32,
    pub competitive_sensitivity: f32,
    pub operational_value: f32,
    pub security_sensitivity: f32,
    pub support_burden: f32,  // note: high burden reduces score, so weight is applied negatively
    pub strategic_importance: f32,
}
impl ScoreWeights {
    /// Invariant: all weights >= 0.0; they need not sum to 1.0 (normalised internally)
    pub fn new(...) -> Result<ScoreWeights, WeightError>;
    pub fn default() -> ScoreWeights; // expert-tuned defaults
}
```

#### `CompositeScore`
```rust
pub struct CompositeScore(f32); // 0.0–10.0, derived from CommercialScore + ScoreWeights
```

#### `LicenseRiskSubScore`
```rust
pub struct LicenseRiskSubScore {
    pub copyleft_type: CopyleftType, // from LicenseCompliance
    pub cap: Option<Score>,          // if set, effective_composite cannot exceed this
    pub penalty: Score,              // subtracted from composite before capping
}
```

#### `GatingSignal`
```rust
pub enum GatingSignal {
    StrongGateCandidate,   // effective_composite >= 7.0
    WeakGateCandidate,     // effective_composite in [5.0, 7.0)
    OpenCandidate,         // effective_composite < 5.0 OR adoption_value >= 8.0
    Undetermined,          // insufficient data to score confidently
}
```

---

## Invariants

1. `Score` must be in `[0.0, 10.0]`. Out-of-range values are rejected at construction.
2. `CompositeScore` is always derived from `CommercialScore` and `ScoreWeights` — it cannot be set manually.
3. `effective_composite` is always `<= composite`; applying license risk can only reduce the score, not increase it.
4. `ModuleScore` with `gating_signal: Undetermined` must include a rationale explaining what information was missing.
5. Every `ModuleNode` in the `ArchitectureMap` must have exactly one `ModuleScore` in the `ValuationReport`. No orphaned scores; no unscored modules.
6. `ScoreWeights` must have all values `>= 0.0`; they are normalised internally but not required to sum to any target.
7. The `support_burden` dimension is subtracted from the composite (not added), because high support burden reduces commercial attractiveness of gating.

---

## Domain Events

### `ModuleScored`
Emitted when a `ModuleScore` is added to the report.
```rust
pub struct ModuleScored {
    pub repo_id: RepositoryId,
    pub report_id: ValuationReportId,
    pub score: ModuleScore,
    pub scored_at: DateTime<Utc>,
}
```

### `ValuationComplete`
Emitted when all modules have been scored.
```rust
pub struct ValuationComplete {
    pub repo_id: RepositoryId,
    pub report_id: ValuationReportId,
    pub module_count: u32,
    pub strong_gate_candidates: u32,
    pub open_candidates: u32,
    pub completed_at: DateTime<Utc>,
}
```

### `LicenseRiskApplied`
Emitted when a license risk sub-score modifies a composite.
```rust
pub struct LicenseRiskApplied {
    pub repo_id: RepositoryId,
    pub report_id: ValuationReportId,
    pub module_score_id: ModuleScoreId,
    pub sub_score: LicenseRiskSubScore,
    pub original_composite: CompositeScore,
    pub effective_composite: CompositeScore,
}
```

---

## Repository Interface

```rust
pub trait ValuationReportStore {
    async fn save(&self, report: &ValuationReport) -> Result<(), StoreError>;
    async fn find_by_repo_id(&self, repo_id: RepositoryId) -> Result<Option<ValuationReport>, StoreError>;
    async fn append_score(&self, report_id: ValuationReportId, score: ModuleScore) -> Result<(), StoreError>;
    async fn list_scores(&self, report_id: ValuationReportId) -> Result<Vec<ModuleScore>, StoreError>;
    async fn list_by_gating_signal(
        &self,
        report_id: ValuationReportId,
        signal: GatingSignal,
    ) -> Result<Vec<ModuleScore>, StoreError>;
}
```

---

## Integration Points

| Context | Direction | What crosses the boundary |
|---|---|---|
| FunctionalityDiscovery | Upstream | Subscribes to `InventoryComplete`; maps `FunctionalityItem` list to scoring input |
| ArchitectureMapping | Upstream | Subscribes to `DependencyGraphBuilt`; uses `ModuleNode` list and centrality |
| LicenseCompliance | Upstream | Subscribes to `LicensesScanned` and `CopyleftExposureDetected` for license risk |
| AssessmentOrchestration | Coordinator | Issues `ScoreModules` command; subscribes to `ValuationComplete` |
| GatingStrategy | Downstream | Subscribes to `ValuationComplete`; reads all `ModuleScore`s to assign tiers |
| ReportDelivery | Downstream | Reads `ValuationReport` for the commercial scoring section |

### Anti-Corruption Layer

The scoring inputs from FunctionalityDiscovery and ArchitectureMapping arrive as domain events with clean types. The only ACL concern is the **LLM scoring output**: Claude produces per-dimension scores and rationale as structured JSON. The `ScoringOutputAdapter` (infrastructure layer) parses this output and validates that all scores are in range before constructing `CommercialScore`. Invalid scores cause the module to be marked `Undetermined` rather than silently clamped.
