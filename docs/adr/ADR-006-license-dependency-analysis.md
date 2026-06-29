# ADR-006 — License and Dependency Analysis Stack

**Status:** Accepted

---

## Context

A core RepoGate output is a legal and licensing posture assessment. This must answer:

1. What is the declared license of the repository?
2. Do the dependency licenses create copyleft obligations that affect commercialization?
3. Are there mixed licenses, missing licenses, or files with no header?
4. What is the SBOM (Software Bill of Materials) across all package ecosystems present in the repo?
5. Which dependencies carry specific risk categories (copyleft, source-available, patent grant issues)?

The analysis must cover multiple package ecosystems: Cargo (`Cargo.toml`), npm (`package.json`), PyPI (`requirements.txt`, `pyproject.toml`), Go (`go.mod`), Maven (`pom.xml`), and others.

The copyleft risk matrix is a key output: a ranked assessment of which licenses in the dependency graph impose source-disclosure or relicensing obligations that could block commercialization.

---

## Decision

**License text matching: `askalono`.**

`askalono` is an SPDX-aware license text matcher. Given the text of a file (LICENSE, COPYING, license headers), it produces an SPDX identifier with a confidence score. It is used in the `repogate-licensing` crate to identify declared licenses from license files and inline headers.

**SPDX expression parsing and validation: `spdx` crate.**

License expressions in package metadata (`Cargo.toml`'s `license` field, `package.json`'s `license` field) are SPDX expressions (e.g., `MIT OR Apache-2.0`, `GPL-2.0-only WITH Classpath-exception-2.0`). The `spdx` crate parses and validates these expressions against the SPDX license list.

**Cargo dependency metadata: `cargo_metadata`.**

The `cargo_metadata` crate invokes `cargo metadata --format-version 1` and parses the JSON output. This produces the full dependency tree (transitive), package metadata (license fields, repository URLs), and feature flags. Used within the `repogate-ingestion` crate to extract Cargo-specific metadata.

**Multi-ecosystem SBOM: `syft` via subprocess.**

`syft` (Anchore) generates SBOMs in SPDX and CycloneDX formats across npm, PyPI, Go, Maven, Cargo, and many other ecosystems. It is invoked as a subprocess by the `repogate-ingestion` crate:
```
syft <repo-path> -o spdx-json
```
The JSON output is parsed into the `RepoManifest` dependency list. `syft` is the most comprehensive multi-ecosystem SBOM tool available and is maintained by a dedicated team.

**Copyleft risk matrix:**

The `repogate-licensing` crate encodes the following license risk tiers as a static lookup table:

| Tier | Licenses | Risk |
|---|---|---|
| Strong copyleft | GPL-2.0, GPL-3.0, AGPL-3.0 | Source disclosure required; blocks proprietary use |
| Weak copyleft | LGPL-2.0, LGPL-2.1, LGPL-3.0, MPL-2.0, EUPL-1.2 | Library exception or file-level copyleft; limited exposure |
| Source-available (not OSI) | BSL-1.1, SSPL-1.0, Elastic-2.0 | Use restrictions; cannot redistribute as SaaS |
| Permissive | MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC | No commercialization restriction |
| Public domain | CC0-1.0, Unlicense | No restriction |
| Unknown / missing | (no SPDX match) | Requires legal review |

---

## Consequences

**Positive:**
- `askalono` handles the common case (full license file text) with high confidence; SPDX expressions cover package metadata.
- `syft` eliminates the need to write per-ecosystem parsers for npm, PyPI, Go, Maven, etc. — one subprocess call produces a unified SBOM.
- The copyleft risk matrix is deterministic Rust code, not model inference — licensing risk outputs are reproducible and auditable.
- `cargo_metadata` provides exact dependency tree data for Rust projects, including feature-gated optional dependencies.

**Negative / Trade-offs:**
- `syft` is an external binary dependency, like `git`. It must be version-pinned in the execution environment.
- `askalono` confidence scores are not 100% — ambiguous or modified license texts may produce incorrect SPDX identifications. Claude Code performs a secondary check on flagged files.
- The copyleft matrix is a static table; new licenses (e.g., future BSL variants) require code changes to classify correctly.
- SPDX expression parsing covers the `license` field in package metadata but not `licenseFile` references or per-file REUSE headers without additional tooling.

---

## Alternatives Considered

**`package-parser` crate** — A Rust-native multi-ecosystem package parser. It is newer and less battle-tested than `syft` across edge cases in real-world repositories. Retained as a possible future native alternative if `syft` subprocess dependency becomes a maintenance concern. Not used for MVP.

**`scancode-toolkit` via subprocess** — Comprehensive license scanner (including file-level headers) used by the Linux Foundation. Requires Python runtime. Rejected due to runtime dependency; `askalono` covers the primary use case natively.

**Manual SPDX parsing** — Writing custom parsers for each ecosystem's metadata format. High implementation cost and ongoing maintenance burden as ecosystem formats evolve. Rejected in favor of `syft`.

**LLM-based license analysis only** — Claude Code could infer license risk from file text. Rejected as the primary mechanism because the copyleft matrix must be deterministic and auditable for legal defensibility. LLM analysis is used as a secondary layer for ambiguous cases only.
