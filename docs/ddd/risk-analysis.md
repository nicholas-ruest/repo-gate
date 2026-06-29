# Bounded Context: RiskAnalysis

**Subdomain**: Supporting
**Crate**: `rg-risk-analysis`

---

## Purpose

RiskAnalysis aggregates risk signals from every upstream context and produces a `RiskProfile` that surfaces what could go wrong with the proposed gating strategy. It does not score commercial value or assign tiers — it identifies threats to those decisions.

Risks span multiple categories: over-gating that kills adoption, license conflicts that create legal exposure, competitive risks from publishing certain capabilities, security-sensitive code that should never be in the open, and community backlash from gating features that users expect to remain free.

RiskAnalysis is the "second opinion" layer. GatingStrategy may produce a tier assignment, but RiskAnalysis tells stakeholders what that assignment risks.

---

## Ubiquitous Language

| Term | Definition |
|---|---|
| **RiskProfile** | The aggregate root. The complete catalogue of identified risks for one repository assessment. |
| **RiskItem** | A single identified risk: what could go wrong, how severe it is, what category it falls into, and which module or capability it affects. |
| **Severity** | A three-level risk severity: `low`, `medium`, `high`. High severity risks block or constrain the gating recommendation. |
| **RiskCategory** | The type of risk. See the taxonomy below. |
| **OverGating** | The risk of gating too much, causing community fragmentation, adoption failure, or open-source credibility damage. |
| **UnderGating** | The risk of leaving commercially valuable capabilities open, reducing revenue potential. |
| **CommunityBacklash** | The risk of negative community response to gating decisions that users perceive as betraying open-source trust. |
| **LicenseConflictRisk** | A legal risk arising from incompatible licenses in the dependency graph or between the repository and its intended commercial license. |
| **CompetitiveExposure** | The risk that publishing a module gives competitors a meaningful capability they could not easily build themselves. |
| **SecurityExposureRisk** | The risk that an open module contains security-sensitive logic (cryptographic primitives, auth checks, secret handling) that should not be published. |
| **AccidentalOpenSource** | The risk that commercially valuable logic is inadvertently left in the open tier due to incomplete analysis. |
| **MitigationSuggestion** | A concrete, actionable recommendation for reducing a specific risk. |

---

## Aggregate Root: `RiskProfile`

`RiskProfile` accumulates `RiskItem` entities from multiple upstream event sources. It is considered complete once AssessmentOrchestration signals the risk analysis phase is finished.

### State Transitions

```
pending → collecting_signals → classifying → complete → failed
```

### Entity: `RiskItem`

| Field | Type | Notes |
|---|---|---|
| `id` | `RiskItemId` | |
| `category` | `RiskCategory` | |
| `severity` | `Severity` | |
| `title` | `String` | Short risk name |
| `description` | `String` | Full description of the risk and its consequences |
| `affected_modules` | `Vec<ModuleNodeId>` | Which modules this risk applies to |
| `affected_tiers` | `Vec<GatingTier>` | Which tier assignments this risk challenges |
| `source_event` | `RiskSourceEvent` | Which upstream event triggered this risk |
| `mitigation` | `Option<MitigationSuggestion>` | Suggested action to reduce the risk |
| `is_blocking` | `bool` | True if this risk should prevent the strategy from proceeding |

### Value Objects

#### `Severity`
```rust
pub enum Severity {
    Low,    // informational; does not materially change the recommendation
    Medium, // should be addressed; may affect tier assignment
    High,   // must be addressed before proceeding; may block the strategy
}
```

#### `RiskCategory`
```rust
pub enum RiskCategory {
    OverGating,            // too much is gated; adoption at risk
    UnderGating,           // valuable capability left open
    CommunityBacklash,     // community trust or expectation violation
    LicenseConflict,       // incompatible or problematic licenses
    CopyleftExposure,      // copyleft license constraints on commercial packaging
    CompetitiveExposure,   // publishing gives competitors meaningful advantage
    SecurityExposure,      // security-sensitive code in open tier
    AccidentalOpenSource,  // commercial value accidentally left open
    BoundaryInstability,   // gating boundary cuts across tightly coupled modules
    LegalUncertainty,      // license or ownership issues require legal review
    MissingLicense,        // no license for module that is proposed to be gated
}
```

#### `MitigationSuggestion`
```rust
pub struct MitigationSuggestion {
    pub action: String,       // e.g., "Move this module to EnterpriseTier"
    pub rationale: String,    // why this action reduces the risk
    pub effort: MitigationEffort, // Low | Medium | High
}
```

#### `RiskSourceEvent`
```rust
pub enum RiskSourceEvent {
    OverGatingRiskFlagged,
    CopyleftExposureDetected,
    LicenseConflictFound,
    StrategyGenerated,
    BoundaryDefined,
    SecuritySensitiveModuleInOpenTier,  // derived by this context
    HighCompetitiveSensitivityInOpenTier, // derived by this context
    MissingLicenseOnGatedModule,
}
```

---

## Invariants

1. Every `RiskItem` must reference at least one `affected_modules` entry or a repository-level concern (scope = entire repository).
2. `Severity::High` items with `is_blocking: true` are surfaced in the report's executive summary and must include a `MitigationSuggestion`.
3. `RiskCategory::SecurityExposure` is automatically `Severity::High` when the affected module handles cryptographic operations, secrets, authentication, or authorisation.
4. `RiskCategory::MissingLicense` is automatically `Severity::High` when the affected module is assigned `GatingTier::ProTier` or above.
5. `RiskProfile` may not transition to `complete` with zero `RiskItem`s unless the repository has no gated modules (i.e., all are `Open`). Even minimal assessments must produce at least a coverage confirmation.
6. Duplicate `RiskItem`s (same category + same affected modules) are merged by taking the higher severity.

---

## Domain Events

### `RiskDetected`
Emitted per `RiskItem` as it is identified.
```rust
pub struct RiskDetected {
    pub repo_id: RepositoryId,
    pub profile_id: RiskProfileId,
    pub risk: RiskItem,
    pub detected_at: DateTime<Utc>,
}
```

### `SeverityClassified`
Emitted when a risk's severity is determined (may differ from initial estimate after cross-referencing).
```rust
pub struct SeverityClassified {
    pub repo_id: RepositoryId,
    pub profile_id: RiskProfileId,
    pub risk_id: RiskItemId,
    pub severity: Severity,
    pub classified_at: DateTime<Utc>,
}
```

### `BlockingRiskFound`
Emitted when a `Severity::High`, `is_blocking: true` risk is identified.
```rust
pub struct BlockingRiskFound {
    pub repo_id: RepositoryId,
    pub profile_id: RiskProfileId,
    pub risk: RiskItem,
}
```

### `RiskProfileComplete`
Emitted when the full risk profile is assembled.
```rust
pub struct RiskProfileComplete {
    pub repo_id: RepositoryId,
    pub profile_id: RiskProfileId,
    pub total_risks: u32,
    pub high_severity_count: u32,
    pub blocking_count: u32,
    pub completed_at: DateTime<Utc>,
}
```

---

## Repository Interface

```rust
pub trait RiskProfileStore {
    async fn save(&self, profile: &RiskProfile) -> Result<(), StoreError>;
    async fn find_by_repo_id(&self, repo_id: RepositoryId) -> Result<Option<RiskProfile>, StoreError>;
    async fn append_risk(&self, profile_id: RiskProfileId, risk: RiskItem) -> Result<(), StoreError>;
    async fn list_risks(&self, profile_id: RiskProfileId) -> Result<Vec<RiskItem>, StoreError>;
    async fn list_by_severity(&self, profile_id: RiskProfileId, severity: Severity) -> Result<Vec<RiskItem>, StoreError>;
    async fn list_blocking(&self, profile_id: RiskProfileId) -> Result<Vec<RiskItem>, StoreError>;
}
```

---

## Integration Points

| Context | Direction | What crosses the boundary |
|---|---|---|
| LicenseCompliance | Upstream | Subscribes to `CopyleftExposureDetected`, `LicenseConflictFound`, `MissingLicenseFlagged` |
| GatingStrategy | Upstream | Subscribes to `StrategyGenerated`, `BoundaryDefined`, `OverGatingRiskFlagged`, `LicenseConstrainedTierAssigned` |
| CommercialValuation | Upstream | Reads `ModuleScore` to detect high-competitive-sensitivity modules in open tiers |
| ArchitectureMapping | Upstream | Reads `DependencyGraph` to detect boundary instability risks |
| AssessmentOrchestration | Coordinator | Issues `AnalyzeRisks` command; subscribes to `RiskProfileComplete` |
| ReportDelivery | Downstream | Reads `RiskProfile` for the risk section of the report |

### Anti-Corruption Layer

RiskAnalysis consumes typed domain events from multiple contexts. No ACL is required, but there is an internal **risk correlation engine** that cross-references signals from different contexts to detect compound risks (e.g., a module that is both copyleft-exposed AND high-competitive-sensitivity AND assigned to EnterpriseTier). This correlation logic is internal to RiskAnalysis and is not exposed to other contexts.
