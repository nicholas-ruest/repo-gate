# RepoGate

**RepoGate** is a deep repository assessment platform that analyzes complete open-source codebases to determine what functionality should remain open source, what should be commercialized into paid tiers, and what requires legal or licensing review before packaging.

## What It Does

RepoGate ingests a public repository URL, performs deep codebase analysis using Claude Code as the reasoning engine, and generates a comprehensive gating assessment report. Rather than surface-level README scanning, RepoGate traverses the full codebase—inspecting source files, tests, examples, configuration, deployment assets, and documentation—to identify all functionality before recommending what stays open and what becomes part of a commercial tier.

**Simple UX**: `repogate analyze <repo-url>` → JSON and Markdown reports with gating recommendations across 7 tiers.

## How It Works

1. **Ingest**: Clone and analyze the repository structure, dependency manifests, and license files.
2. **License Scan**: Identify licenses, detect conflicts, and flag files needing legal review using `askalano`, `spdx`, and `syft` (for supply chain dependencies).
3. **Module Discovery**: Break the repository into functional modules (core runtime, APIs, SDKs, CLIs, connectors, integrations, dashboards, etc.).
4. **Deep Analysis**: Spawn a Claude Code sub-agent per module to inspect implementation details, uncover hidden capabilities, assess enterprise value, and identify undocumented features.
5. **Scoring**: Evaluate each module against commercial value criteria (adoption impact, enterprise buyer value, competitive sensitivity, support burden, strategic importance, gating suitability).
6. **Gating Strategy**: Recommend boundaries between open-source core and commercial tiers, with risk analysis to prevent over-gating or community backlash.
7. **Report**: Output a structured JSON assessment plus a human-readable Markdown report with executive summary, functionality inventory, architecture map, and recommendations.

## Tech Stack

**Rust (Primary)**
- **git (subprocess) → gix** — Repository cloning: subprocess `git clone --depth=1 --filter=blob:none` for the MVP, migrating to the pure-Rust `gix` post-MVP
- **ignore** — .gitignore-aware directory traversal
- **tokei** — Code metrics and line-of-code analysis
- **askalono** — License detection and classification
- **spdx** — SPDX license expression parsing and compliance checking
- **sqlx** — Async SQL for assessment storage and versioning
- **axum** — High-performance async web server for the API
- **clap** — Command-line argument parsing (CLI)
- **tera** — Template rendering for report generation

**External**
- **syft** — Supply chain dependency scanning (subprocess)
- **Claude Code CLI** (`claude --bare -p`) — Headless codebase analysis via stream-json output and --json-schema

**Frontend**
- **Next.js** — React dashboard for viewing and sharing assessment reports

## Installation

### Prerequisites

- **Rust toolchain** (1.70+): [Install Rust](https://rustup.rs/)
- **Claude CLI** (authenticated): [Install Claude Code](https://claude.ai/docs/claude-code)
- **syft** binary: Install via [Anchore syft releases](https://github.com/anchore/syft/releases) or package manager

### Build

```bash
git clone https://github.com/yourusername/repo-gate.git
cd repo-gate
cargo build --release
```

The compiled binary is available at `target/release/repogate`.

## Quick Start

```bash
# Analyze a public open-source repository
repogate analyze https://github.com/rust-lang/rust

# Output includes:
# - assessment.json (structured gating recommendation)
# - assessment.md (human-readable report)
```

## Output

### Gating Tiers

RepoGate recommends one of seven tiers for each module:

1. **open** — Remains in the open-source community edition; foundational to adoption.
2. **source_available** — Source is published but binaries/services are restricted; useful for preventing direct commercial forks.
3. **pro_tier** — Professional/team features; supports individuals and small organizations.
4. **enterprise_tier** — Enterprise features; multi-tenant, SSO, advanced compliance, SLA support.
5. **managed_cloud** — Proprietary managed service tier; deployment, scaling, monitoring, operational burden.
6. **legal_review** — Licensing or legal concerns prevent clear recommendation; requires manual review.
7. **not_recommended** — Over-gating risk; community backlash likely if closed; recommend keeping open.

### Report Sections

- **Executive Summary** — High-level assessment, recommended open-core boundary, and strategic positioning.
- **Functionality Inventory** — Comprehensive list of observed features, including hidden, undocumented, and enterprise capabilities.
- **Repository Architecture** — Module breakdown with data flow and dependency graph.
- **Module Analysis** — Per-module scoring (commercial value, adoption value, risk, gating suitability).
- **Licensing Posture** — License compliance, mixed-license risks, third-party concerns.
- **Commercial Value Scoring** — Quantitative assessment of business impact for each tier.
- **Risk Analysis** — Over-gating, community adoption, competitive exposure, security.
- **Final Recommendations** — Clear boundary definition between tiers and product packaging strategy.

## For Whom

RepoGate is designed for:

- **Open-source maintainers** deciding on commercialization strategy
- **Infrastructure and platform teams** evaluating repository value
- **Founders and CTOs** commercializing open-source software
- **Product leaders** defining open-core strategy and tier boundaries
- **Enterprise software teams** managing complex open-source ecosystems
- **Legal and licensing reviewers** assessing compliance risk before gating

## Documentation

- **[Architecture Overview](docs/architecture.md)** — System design, pipeline, and Claude Code integration
- **[Getting Started](docs/getting-started.md)** — Prerequisites, build, run, and interpretation guide
- **[Architecture Decision Records](docs/adr/)** — Design rationale and trade-offs (ADR index)
- **[Domain Model](docs/ddd/)** — Bounded contexts and domain entity reference

## Cost

RepoGate uses Claude Code (programmatic `claude` CLI) for deep repository analysis. Analysis cost depends on repository size and complexity. Expect:

- Small repos (< 10k lines): ~$0.10–$0.50
- Medium repos (10k–100k lines): ~$1–$5
- Large repos (100k+ lines): ~$5–$20+

Pricing follows Claude's standard token rates (January 2026). See [Anthropic Pricing](https://anthropic.com/pricing) for current rates.

## License

RepoGate is open source. See [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome. Please read [CLAUDE.md](CLAUDE.md) for development guidelines and agent coordination patterns.

---

**Questions?** Check the [Getting Started](docs/getting-started.md) guide or file an issue.
