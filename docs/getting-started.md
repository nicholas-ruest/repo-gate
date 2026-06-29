# Getting Started with RepoGate

This guide walks you through installation, building RepoGate, running your first analysis, and interpreting the results.

## Prerequisites

Before building RepoGate, ensure you have:

### 1. Rust Toolchain

Install Rust (1.70 or later):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustc --version  # Verify installation
```

### 2. Claude CLI

Install the Claude Code CLI and authenticate:

```bash
# Install Claude CLI (or update to the latest version)
# Follow platform-specific instructions at https://claude.ai/docs/claude-code

# Authenticate with your Anthropic account
claude auth login

# Verify authentication
claude status
```

The CLI must be authenticated because RepoGate uses `claude --bare -p` (programmatic mode) to spawn sub-agents for deep analysis.

### 3. Syft Binary

Install the Syft dependency scanner for supply chain analysis:

**macOS:**
```bash
brew install syft
```

**Linux:**
```bash
curl -sSfL https://raw.githubusercontent.com/anchore/syft/main/install.sh | sh -s -- -b /usr/local/bin
```

**Or download directly:**
Visit [Anchore Syft Releases](https://github.com/anchore/syft/releases) and download the appropriate binary for your platform.

**Verify:**
```bash
syft --version
```

## Build

Clone the RepoGate repository and build the release binary:

```bash
git clone https://github.com/yourusername/repo-gate.git
cd repo-gate
cargo build --release
```

The compiled binary is available at:
```
target/release/repogate
```

Optionally, install globally:
```bash
cargo install --path .
repogate --help  # Should work from any directory
```

## Run Your First Analysis

### Analyze a Sample Repository

Let's analyze a well-known open-source project to see RepoGate in action:

```bash
repogate analyze https://github.com/tokio-rs/tokio
```

This command:
1. Clones the Tokio repository into a temporary directory
2. Scans licenses and supply chain
3. Discovers functional modules
4. Spawns Claude Code sub-agents to inspect each module
5. Scores commercial value for each module
6. Generates gating recommendations
7. Outputs two files:
   - `assessment.json` — Machine-readable gating assessment
   - `assessment.md` — Human-readable report

### Command Options

```bash
# Specify output directory (default: current directory)
repogate analyze --output ./assessments https://github.com/tokio-rs/tokio

# Set maximum analysis cost budget (default: $20)
repogate analyze --budget 10 https://github.com/tokio-rs/tokio

# Dry-run: show what would be analyzed without running Claude Code
repogate analyze --dry-run https://github.com/tokio-rs/tokio

# Verbose output for debugging
repogate analyze --verbose https://github.com/tokio-rs/tokio
```

## Understanding the Output

### The Assessment Files

After analysis completes, you'll find:

- **`assessment.json`** — Structured data consumable by scripts and dashboards
- **`assessment.md`** — Human-readable report for review, sharing, and decision-making

### The 7 Gating Tiers

RepoGate recommends one of seven tiers for each module:

#### 1. **open**
Remains in the open-source community edition. These are foundational capabilities essential to adoption. Users expect them to be free and unrestricted. Over-gating these risks community backlash.

**Examples:** Core runtime, basic APIs, standard SDKs, CLI.

#### 2. **source_available**
Source code is published on GitHub, but binaries, Docker images, or commercial services are restricted. Prevents free commercial forks while maintaining transparency and community contribution.

**Examples:** Enterprise features that need source visibility (for compliance audits), reference implementations.

#### 3. **pro_tier**
Professional and team features. Targets small organizations, consultants, and growing teams. Typically includes advanced monitoring, reporting, API quotas, or integrations.

**Examples:** Analytics dashboards, advanced configuration, team collaboration features.

#### 4. **enterprise_tier**
Enterprise-grade features including multi-tenancy, Single Sign-On (SSO), advanced compliance (HIPAA, SOC 2), audit logging, SLA support, and premium features.

**Examples:** Multi-user management, role-based access control, compliance frameworks, dedicated support.

#### 5. **managed_cloud**
Proprietary managed service tier. The company operates infrastructure, handles deployment, scaling, monitoring, and operational burden. Users consume as a service, not a self-hosted product.

**Examples:** Hosted versions with uptime SLAs, backup and recovery, auto-scaling, geographic distribution.

#### 6. **legal_review**
Licensing or legal ambiguities prevent a clear recommendation. Requires manual review by legal counsel before gating decisions are finalized.

**Reasons:** Mixed licenses, ambiguous copyright, third-party code concerns, compliance edge cases.

#### 7. **not_recommended**
Gating is not recommended. Either the functionality is too fundamental to the project, community expectations would create backlash, or licensing constraints prevent gating. Keep this in the open-source tier.

**Examples:** Core dependencies, essential build tools, foundational APIs that are re-used across layers.

### Report Sections

#### Executive Summary
High-level strategic positioning: what the repository does, recommended open-core boundary, and key insights. Start here for a quick understanding.

#### Functionality Inventory
Comprehensive list of all observed capabilities, including:
- Publicly documented features
- Hidden or under-documented capabilities
- Enterprise-oriented features
- Security-sensitive functionality
- Commercially valuable logic

#### Repository Architecture
Module breakdown showing:
- Functional areas (APIs, SDKs, CLIs, connectors, etc.)
- Module dependencies and data flow
- Size and complexity metrics (lines of code, cyclomatic complexity)
- Simple ASCII or visual diagram of module relationships

#### Module-by-Module Analysis
For each identified module:
- **Name & Purpose** — What does it do?
- **Capabilities** — Specific features discovered
- **Metrics** — Size (LOC), complexity, test coverage
- **Commercial Value Score** — 0–100 rating
- **Adoption Impact** — How essential is this to users?
- **Enterprise Value** — Does enterprise need it?
- **Competitive Sensitivity** — Risk if exposed to competitors?
- **Gating Suitability** — Can it be cleanly separated?
- **Recommended Tier** — Final recommendation
- **Rationale** — Why this tier was chosen
- **Risks** — Potential issues with the recommendation

#### Licensing Posture
- **License Detection** — Identified licenses (MIT, Apache 2.0, GPL, etc.)
- **Mixed-License Risk** — Any incompatibilities?
- **Copyleft Exposure** — GPL/AGPL implications if modified?
- **Third-Party Concerns** — Vendored code, unclear copyright
- **Supply Chain Scan** — Notable dependency risks

#### Commercial Value Scoring
Quantitative breakdown of scoring factors:
- **Adoption Impact** (0–20 points) — How essential to users?
- **Enterprise Value** (0–20 points) — Would enterprise pay for this?
- **Commercial Leverage** (0–20 points) — Unique, defensible logic?
- **Competitive Sensitivity** (0–15 points) — Risk if competitors have it?
- **Operational Value** (0–10 points) — Support/maintenance burden?
- **Strategic Importance** (0–10 points) — Aligns with product vision?
- **Gating Suitability** (0–5 points) — Cleanly separable?

**Total Score:** Sum of all factors (0–100). Higher scores suggest suitability for paid tiers.

#### Risk Analysis
- **Over-Gating Risk** — Could over-gating hurt adoption or trigger backlash?
- **Licensing Risks** — Can modules legally be gated given their licenses?
- **Hidden Enterprise Functionality** — Did analysis uncover unexpected valuable features?
- **Accidental Exposure** — Any commercial logic currently open that should be gated?
- **Community Expectations** — What will the community expect to remain free?

#### Final Recommendations
Clear guidance:
- **Recommended Open-Source Tier** — What stays community-edition
- **Recommended Paid Tiers** — What moves to pro, enterprise, managed cloud
- **Boundary Definition** — How to split the codebase (module extraction, API gating, feature flags)
- **Licensing Actions** — Any license or legal steps needed before gating
- **Phasing Strategy** — Suggest rollout timeline (e.g., move pro features in v2.0)
- **Revenue Potential** — Estimated addressable market and tier positioning

### Example Report Section

```markdown
## Module Analysis: Advanced Monitoring Dashboard

**Path:** `src/dashboards/monitoring/`

**Purpose:** Real-time system health, metrics visualization, alerting integration.

**Capabilities:**
- Live metric streaming (WebSocket)
- Custom dashboard builder
- Multi-database support (Prometheus, InfluxDB)
- Alert routing to Slack/PagerDuty
- Historical data retention and replay

**Metrics:**
- Lines of code: 8,234
- Test coverage: 78%
- Cyclomatic complexity: 3.2 (moderate)

**Scoring:**
- Adoption Impact: 8/20 (useful but not essential)
- Enterprise Value: 18/20 (enterprises expect rich dashboards)
- Commercial Leverage: 15/20 (differentiator vs. open competitors)
- Competitive Sensitivity: 12/15 (competitors have similar features)
- Operational Value: 6/10 (moderate support burden)
- Strategic Importance: 14/20 (core to product positioning)
- Gating Suitability: 4/5 (cleanly separable)

**Total Score: 77/100**

**Recommended Tier:** pro_tier

**Rationale:** Dashboards are valuable but not foundational to adoption. Gating this module creates a clear tiering distinction (community get metrics via API, pro tier gets UI). Enterprise may upgrade to enterprise_tier for RBAC and audit logging (see separate module analysis).

**Risks:** Low community backlash risk; many open-source projects gate dashboards.

**Implementation:** Feature flag in source; enterprise version includes compiled, pre-optimized dashboard bundle.
```

## Interpreting Results: Decision Guide

### For Open-Source Maintainers

1. **Identify the open-source tier** → Modules recommended as `open` and `not_recommended`
2. **Choose your paid boundaries** → Pick which `pro_tier`, `enterprise_tier`, and `managed_cloud` modules to include
3. **Address legal concerns** → Resolve any `legal_review` modules with counsel
4. **Plan the split** → Decide on feature flags, separate repositories, or build-time options
5. **Phase the rollout** → Gradual introduction of paid tiers reduces community friction

### For Product Leaders

1. **Validate commercial viability** → Are there enough high-scoring modules to support pricing?
2. **Prioritize tiers** → Which tier (pro, enterprise, managed) is most valuable?
3. **Competitive positioning** → How does this compare to competitors' offerings?
4. **Risk mitigation** → Address over-gating warnings and licensing risks upfront

### For Legal Teams

1. **Flag modules requiring review** → Start with `legal_review` recommendations
2. **License compliance** → Ensure gating doesn't violate open-source licenses
3. **Contributor agreements** → Confirm ownership of code being moved to commercial tiers
4. **Security review** → Prioritize security-sensitive modules for manual inspection

## Running the Web Dashboard

RepoGate includes an optional Next.js web dashboard for viewing and sharing assessment reports.

### Start the Server

```bash
cargo run --release --package repogate-server -- --listen 0.0.0.0:8080
```

### Access the Dashboard

Open your browser to:
```
http://localhost:8080
```

**Features:**
- View recent assessments
- Search and filter by repository, tier, or risk level
- Share reports via URL (with optional password protection)
- Bulk export assessments as JSON/CSV
- Compare multiple assessments side-by-side
- Custom filtering and scoring adjustments

### Configuration

The server reads from `config.toml`:

```toml
[server]
listen = "0.0.0.0:8080"
database_url = "sqlite://assessments.db"  # or PostgreSQL

[analysis]
max_budget = 20  # $ per analysis
default_model = "opus"  # or "sonnet" for cost savings

[claude]
timeout_seconds = 600
```

## Cost and Billing

RepoGate uses Claude Code via the programmatic CLI (`claude --bare -p`) to spawn sub-agents for deep analysis.

### Billing Model (June 2026)

- **Input tokens:** $3 / 1M tokens (Claude Opus 4.8)
- **Output tokens:** $15 / 1M tokens (Claude Opus 4.8)
- **Sonnet 4.6:** $3 / 1M input, $15 / 1M output (discounted)

### Typical Costs

- **Small repo** (< 10k LOC): ~$0.10–$0.50
- **Medium repo** (10k–100k LOC): ~$1–$5
- **Large repo** (100k+ LOC): ~$5–$20+

Costs depend on repository complexity, number of modules, and whether Opus (deep analysis) or Sonnet (standard analysis) is used.

### Budgeting

Set a per-analysis cost ceiling:

```bash
repogate analyze --budget 10 https://github.com/yourusername/yourrepo
```

If analysis would exceed the budget, RepoGate pauses and asks for confirmation.

### Cost Optimization

- Use **Sonnet 4.6** for routine scanning (lower cost, good for most cases)
- Use **Opus 4.8** only for complex, high-value decisions
- Run **dry-run** mode (`--dry-run`) first to estimate costs without spending
- Cache repository data locally to avoid re-scanning

## Next Steps

- **Read** [Architecture Overview](architecture.md) for system design details
- **Check** [Architecture Decision Records](adr/) for design rationale
- **Review** [Domain Model](ddd/) for data structure details
- **Contribute** — See [CLAUDE.md](../CLAUDE.md) for development guidelines

## Troubleshooting

### "Claude CLI not found"

Ensure the Claude CLI is installed and in your `$PATH`:

```bash
which claude
claude --version
```

### "Syft not installed"

Install Syft:

```bash
brew install syft  # macOS
# or follow instructions at https://github.com/anchore/syft
```

### Analysis stops or times out

Large repositories may take 10+ minutes. Increase the timeout:

```bash
repogate analyze --timeout 900 https://github.com/large/repo
```

### High costs for large repositories

Use Sonnet instead of Opus:

```bash
# Set in config.toml
[analysis]
default_model = "sonnet"
```

Or run in dry-run mode first:

```bash
repogate analyze --dry-run --verbose https://github.com/repo
# Review estimated cost before proceeding
```

### "Cannot clone repository"

Ensure the URL is public and reachable:

```bash
git ls-remote https://github.com/username/repo
```

If private, use a GitHub Personal Access Token:

```bash
export GITHUB_TOKEN=your_token_here
repogate analyze https://github.com/private/repo
```

## Questions?

- **[README](../README.md)** — Project overview
- **[Architecture](architecture.md)** — System design
- **[GitHub Issues](https://github.com/yourusername/repo-gate/issues)** — Bug reports and feature requests
