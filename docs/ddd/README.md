# RepoGate — Domain-Driven Design Overview

## Strategic Intent

RepoGate's core insight is that open-core commercialization decisions fail when analysis is shallow. The domain model is built around the idea that **deep codebase understanding must precede any gating recommendation**. The moat is not the recommendation output — any tool can produce a report. The moat is the depth of the analysis that feeds it.

---

## Subdomain Classification

| Subdomain Type | Bounded Contexts |
|---|---|
| **Core Domain** (competitive moat) | FunctionalityDiscovery, CommercialValuation, GatingStrategy, AssessmentOrchestration |
| **Supporting** (enables Core, domain-specific) | RepositoryIngestion, LicenseCompliance, ArchitectureMapping, RiskAnalysis, ReportDelivery |
| **Generic** (could be off-the-shelf) | UserWorkflow |

### Why These Are Core

- **FunctionalityDiscovery** — traversing a codebase to surface hidden, undocumented, or buried capabilities is hard. Most tools don't go past top-level structure.
- **CommercialValuation** — multi-dimension scoring calibrated to open-core economics is proprietary judgment, not a commodity.
- **GatingStrategy** — mapping scored modules to discrete tiers while preserving open-core balance requires domain expertise baked into rules.
- **AssessmentOrchestration** — owning the Claude Code session lifecycle, fan-out per module, token budget enforcement, and recovery is the operational moat.

---

## The 10 Bounded Contexts

```
┌─────────────────────────────────────────────────────────────────────┐
│                        UserWorkflow (Generic)                        │
│  Accepts URL submissions, tracks job status, renders results         │
│  ↓ submits AnalysisRequest (ACL: translates UI concepts to domain)  │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                    ┌────────▼────────┐
                    │  Assessment     │  ← CORE DOMAIN (orchestrator)
                    │  Orchestration  │
                    │                 │
                    │  Owns the job   │
                    │  state machine  │
                    │  and Claude     │
                    │  session mgmt   │
                    └────┬──────┬─────┘
           ┌─────────────┘      └─────────────────────────┐
           │ upstream calls (U/S)                          │
           │                                               │
    ┌──────▼──────────┐                          Supporting │ contexts
    │  Repository     │ ────→  LicenseCompliance ────────►│
    │  Ingestion      │ ────→  ArchitectureMapping ───────►│
    │  (Supporting)   │ ────→  FunctionalityDiscovery ────►│ (Core)
    └─────────────────┘        CommercialValuation ────────►│ (Core)
                               GatingStrategy ──────────────►│ (Core)
                               RiskAnalysis ────────────────►│ (Supporting)
                                                             │
                                                   ┌─────────▼──────────┐
                                                   │  ReportDelivery    │
                                                   │  (Supporting)      │
                                                   └────────────────────┘
```

### Upstream / Downstream Relationships

| Upstream (U) | Downstream (D) | Integration Pattern |
|---|---|---|
| RepositoryIngestion | All analysis contexts | Published Event (`RepositoryCloned`, `ManifestBuilt`) |
| LicenseCompliance | GatingStrategy, RiskAnalysis, ReportDelivery | Published Event (`LicensesScanned`) |
| ArchitectureMapping | FunctionalityDiscovery, CommercialValuation | Published Event (`DependencyGraphBuilt`) |
| FunctionalityDiscovery | CommercialValuation, GatingStrategy | Published Event (`FeaturesDiscovered`) |
| CommercialValuation | GatingStrategy | Published Event (`ModuleScored`) |
| GatingStrategy | RiskAnalysis, ReportDelivery | Published Event (`StrategyGenerated`) |
| RiskAnalysis | ReportDelivery | Published Event (`RiskDetected`) |
| AssessmentOrchestration | All contexts | **Orchestrator** (commands + subscribes to events) |
| UserWorkflow | AssessmentOrchestration | ACL — translates `AnalysisRequest` into `AssessmentJob` |

### Anti-Corruption Layers

- **UserWorkflow → AssessmentOrchestration**: The UserWorkflow ACL prevents UI concepts (form state, session cookies, URL slugs) from polluting `AssessmentJob` or any domain aggregate. The ACL translates an `AnalysisRequest` DTO into a validated `AssessmentJob` command.
- **AssessmentOrchestration → Claude API**: A `ClaudeSession` entity encapsulates the external API surface; no bounded context outside Orchestration speaks directly to the Claude Code API.
- **RepositoryIngestion → VCS providers**: Git clone logic is encapsulated in RepositoryIngestion's infrastructure layer. Other contexts receive a `Repository` aggregate, not a raw git object.

---

## Bounded Context Files

| Context | File | Subdomain |
|---|---|---|
| RepositoryIngestion | [repository-ingestion.md](repository-ingestion.md) | Supporting |
| LicenseCompliance | [license-compliance.md](license-compliance.md) | Supporting |
| FunctionalityDiscovery | [functionality-discovery.md](functionality-discovery.md) | **Core** |
| ArchitectureMapping | [architecture-mapping.md](architecture-mapping.md) | Supporting |
| CommercialValuation | [commercial-valuation.md](commercial-valuation.md) | **Core** |
| GatingStrategy | [gating-strategy.md](gating-strategy.md) | **Core** |
| RiskAnalysis | [risk-analysis.md](risk-analysis.md) | Supporting |
| AssessmentOrchestration | [assessment-orchestration.md](assessment-orchestration.md) | **Core** |
| ReportDelivery | [report-delivery.md](report-delivery.md) | Supporting |
| UserWorkflow | [user-workflow.md](user-workflow.md) | Generic |

---

## Rust Crate Layout (indicative)

```
repo-gate/
  crates/
    rg-repository-ingestion/
    rg-license-compliance/
    rg-functionality-discovery/
    rg-architecture-mapping/
    rg-commercial-valuation/
    rg-gating-strategy/
    rg-risk-analysis/
    rg-assessment-orchestration/
    rg-report-delivery/
    rg-user-workflow/
    rg-domain-events/       # shared event types, no logic
    rg-shared-kernel/       # primitive VOs used across contexts
  web/                      # TypeScript dashboard only
```

Cross-context communication happens through domain events published to an in-process event bus (initially) or a durable queue (production). Each crate depends only on `rg-domain-events` and `rg-shared-kernel` — never on another context's internal types.
