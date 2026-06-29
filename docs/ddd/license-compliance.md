# Bounded Context: LicenseCompliance

**Subdomain**: Supporting
**Crate**: `rg-license-compliance`

---

## Purpose

LicenseCompliance answers one question: **can this code be commercially packaged, and what are the legal risks if so?** It detects license files, parses SPDX expressions, cross-references dependency licenses (via `syft` or `cargo_metadata`), and flags copyleft exposure, license conflicts, and missing attribution requirements.

The output is a `LicenseReport` aggregate that downstream contexts (GatingStrategy, RiskAnalysis) rely on to know which modules are legally safe to gate, which are restricted by copyleft, and which have unknown or missing licenses.

This context is **intentionally narrow**: it produces legal facts, not commercial recommendations. The interpretation of those facts (what to do about copyleft exposure) is the job of GatingStrategy and RiskAnalysis.

---

## Ubiquitous Language

| Term | Definition |
|---|---|
| **LicenseReport** | The aggregate root. Summarises the license posture of one repository. |
| **SpdxExpression** | A structured license identifier or compound expression per the SPDX specification (e.g., `MIT`, `Apache-2.0 OR MIT`, `GPL-2.0-only WITH Classpath-exception-2.0`). |
| **LicenseConfidence** | A 0.0–1.0 score indicating how certain the system is that a detected license matches an SPDX identifier (produced by `askalono`). |
| **LicenseDetection** | A single license finding: a file path, an `SpdxExpression`, and a `LicenseConfidence`. |
| **DependencyLicense** | The license declared by a transitive or direct dependency (from manifest files). |
| **CopyleftExposure** | A value object capturing which SPDX licenses in the dependency graph are copyleft (GPL, AGPL, EUPL, etc.), and whether they require source disclosure. |
| **LicenseConflict** | Two or more licenses in the dependency graph that are mutually incompatible. |
| **MissingLicense** | A source file or package manifest that declares no license and has no detectable license header. |
| **ProprietaryIndicator** | Text patterns (`"All rights reserved"`, `"Proprietary"`) detected in source files that are not covered by an SPDX license. |
| **Copyleft** | Any license that requires derivative works to be distributed under the same or a compatible open license. |
| **SPDX** | Software Package Data Exchange — the standard for communicating software license information. |

---

## Aggregate Root: `LicenseReport`

`LicenseReport` is built incrementally as license detection runs over the file tree and dependency manifests. It is considered complete only when all files have been scanned and all dependencies resolved.

### State Transitions

```
pending → scanning_files → scanning_deps → complete → failed
```

### Entities (owned by LicenseReport)

#### `LicenseDetection`

| Field | Type | Notes |
|---|---|---|
| `file_path` | `PathBuf` | |
| `spdx_expression` | `Option<SpdxExpression>` | None if no license detected |
| `confidence` | `LicenseConfidence` | |
| `detection_method` | `DetectionMethod` | `FileMatch`, `HeaderScan`, `ManifestDeclaration` |
| `is_proprietary_indicator` | `bool` | True if proprietary text found |

#### `DependencyLicense`

| Field | Type | Notes |
|---|---|---|
| `package_name` | `String` | |
| `package_version` | `String` | |
| `ecosystem` | `Ecosystem` | `Cargo`, `Npm`, `PyPI`, `Maven`, etc. |
| `declared_license` | `Option<SpdxExpression>` | None = unknown |
| `is_copyleft` | `bool` | |
| `copyleft_type` | `Option<CopyleftType>` | `StrongCopyleft`, `WeakCopyleft`, `NetworkCopyleft` |

### Value Objects

#### `SpdxExpression`
- Wraps a parsed SPDX expression tree. Validated using the `spdx` crate.
- `fn parse(raw: &str) -> Result<SpdxExpression, SpdxParseError>`
- `fn is_copyleft(&self) -> bool`
- `fn is_permissive(&self) -> bool`
- `fn is_compatible_with(&self, other: &SpdxExpression) -> bool`

#### `LicenseConfidence`
- Wraps an `f32` in range `[0.0, 1.0]`.
- Threshold for "confident detection" is `>= 0.90`. Below this, the detection is flagged as uncertain.

#### `CopyleftExposure`
- `strong_copyleft: Vec<SpdxExpression>` — GPL-2.0, GPL-3.0, AGPL-3.0, etc.
- `weak_copyleft: Vec<SpdxExpression>` — LGPL, MPL, EUPL, etc.
- `network_copyleft: Vec<SpdxExpression>` — AGPL-3.0 when used as a network service
- `requires_source_disclosure: bool`
- `requires_network_disclosure: bool`

---

## Invariants

1. A `LicenseReport` may only transition to `complete` when all `FileEntry` items from the corresponding `Repository` have been processed (by path count match).
2. `LicenseConfidence` must be in `[0.0, 1.0]`; values outside this range are rejected at construction.
3. A `DependencyLicense` with `declared_license: None` is stored but always surfaces in the report as a missing-license finding — it cannot be silently discarded.
4. A `CopyleftExposure` with `strong_copyleft` non-empty sets `requires_source_disclosure = true` automatically.
5. `SpdxExpression::is_compatible_with` is symmetric — if A is incompatible with B, then B is incompatible with A.
6. `LicenseConflict` is only raised when two licenses are present in the **same binary/package** boundary (transitive conflicts in different deployable units are flagged as warnings, not errors).

---

## Domain Events

### `LicensesScanned`
Emitted when file-level license detection is complete for all source files.
```rust
pub struct LicensesScanned {
    pub repo_id: RepositoryId,
    pub report_id: LicenseReportId,
    pub detections: Vec<LicenseDetection>,
    pub missing_count: u32,
    pub scanned_at: DateTime<Utc>,
}
```

### `DependencyLicensesResolved`
Emitted when all dependency manifests have been processed.
```rust
pub struct DependencyLicensesResolved {
    pub repo_id: RepositoryId,
    pub report_id: LicenseReportId,
    pub dependency_licenses: Vec<DependencyLicense>,
    pub resolved_at: DateTime<Utc>,
}
```

### `CopyleftExposureDetected`
Emitted when at least one copyleft dependency or source file is found.
```rust
pub struct CopyleftExposureDetected {
    pub repo_id: RepositoryId,
    pub report_id: LicenseReportId,
    pub exposure: CopyleftExposure,
    pub affecting_packages: Vec<String>,
}
```

### `MissingLicenseFlagged`
Emitted per file or package that has no detectable license.
```rust
pub struct MissingLicenseFlagged {
    pub repo_id: RepositoryId,
    pub report_id: LicenseReportId,
    pub target: MissingLicenseTarget, // FileEntry | DependencyPackage
    pub flagged_at: DateTime<Utc>,
}
```

### `LicenseConflictFound`
Emitted when two incompatible licenses coexist in the same deployment unit.
```rust
pub struct LicenseConflictFound {
    pub repo_id: RepositoryId,
    pub report_id: LicenseReportId,
    pub conflicting_licenses: (SpdxExpression, SpdxExpression),
    pub scope: String, // e.g. package name where conflict occurs
}
```

---

## Repository Interface

```rust
pub trait LicenseReportStore {
    async fn save(&self, report: &LicenseReport) -> Result<(), StoreError>;
    async fn find_by_repo_id(&self, repo_id: RepositoryId) -> Result<Option<LicenseReport>, StoreError>;
    async fn append_detection(&self, report_id: LicenseReportId, detection: LicenseDetection) -> Result<(), StoreError>;
    async fn append_dependency(&self, report_id: LicenseReportId, dep: DependencyLicense) -> Result<(), StoreError>;
}
```

---

## Integration Points

| Context | Direction | What crosses the boundary |
|---|---|---|
| RepositoryIngestion | Upstream | Subscribes to `ManifestBuilt`; receives `FileEntry` list |
| AssessmentOrchestration | Downstream consumer | Issues `ScanLicenses` command; subscribes to `LicensesScanned` |
| GatingStrategy | Downstream consumer | Reads `LicenseReport` to block or constrain tier assignment |
| RiskAnalysis | Downstream consumer | Subscribes to `CopyleftExposureDetected`, `LicenseConflictFound` |
| ReportDelivery | Downstream consumer | Reads `LicenseReport` for inclusion in final report |

### Anti-Corruption Layer

No ACL is required between LicenseCompliance and its upstream (RepositoryIngestion), because both speak in shared-kernel file-path types. The only translation is from a raw `syft`/`cargo_metadata` JSON output into the internal `DependencyLicense` entity — this is encapsulated in an infrastructure adapter and never leaks into the domain.
