# P01 — Cargo Workspace Skeleton + CI

## Context

RepoGate is a deep repository assessment platform that analyzes full open-source codebases to determine what should remain open source versus become part of commercial tiers, using Claude Code as the reasoning engine and Rust as the primary implementation language.

**You are implementing exactly ONE build unit: the Cargo workspace skeleton and CI setup.** Do not start work on other units. Build and tests must be green before this prompt is considered done.

**Prerequisites:** None — this is the foundation.

---

## Phase & Dependencies

- **Phase:** Foundations
- **Depends on:** Nothing

---

## Scope & Deliverables

Your task is to set up the Cargo workspace and CI/CD infrastructure for the entire RepoGate project.

### Cargo Workspace Root

Create `/workspaces/repo-gate/Cargo.toml` as the workspace manifest declaring all 8 member crates:

```toml
[workspace]
members = [
    "crates/repogate-core",
    "crates/repogate-ingestion",
    "crates/repogate-licensing",
    "crates/repogate-orchestrator",
    "crates/repogate-scoring",
    "crates/repogate-report",
    "crates/repogate-cli",
    "crates/repogate-server",
]
resolver = "2"

[workspace.dependencies]
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
schemars = { version = "0.8", features = ["derive"] }
sqlx = { version = "0.8", features = ["runtime-tokio-native-tls", "sqlite", "postgres"] }
axum = "0.7"
clap = { version = "4.5", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1.0"
thiserror = "1.0"
```

### Member Crate Initialization

For each of the 8 crates, create a minimal directory structure:

- `crates/repogate-{name}/`
  - `Cargo.toml` (package manifest, importing workspace deps)
  - `src/lib.rs` or `src/main.rs` with one passing placeholder test
  - `.gitkeep` or similar

**Example `crates/repogate-core/Cargo.toml`:**

```toml
[package]
name = "repogate-core"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
schemars = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["full"] }
```

**Example `crates/repogate-core/src/lib.rs`:**

```rust
#![doc = "RepoGate core types and schemas."]

pub mod error;
pub mod types;

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder() {
        assert!(true);
    }
}
```

### Root-Level Configuration

- **`.gitignore`** — Standard Rust patterns (target/, *.swp, .DS_Store, Cargo.lock, etc.)
- **`rust-toolchain.toml`** — Pin stable toolchain:
  ```toml
  [toolchain]
  channel = "stable"
  ```
- **`tests/fixtures/dev.db`** — Empty SQLite database file (0 bytes or minimal schema) for compile-time `sqlx` checks in CI. Create the file with `touch tests/fixtures/dev.db` and add a `.gitkeep` if needed.

### GitHub Actions CI Workflow

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --workspace --verbose
      - run: cargo test --workspace --verbose
      - run: cargo clippy --workspace -- -D warnings
      - run: cargo fmt --check
```

---

## Source Documents to Read

- **`docs/adr/ADR-001-rust-primary-language.md`** — Workspace structure, crate list, dependency strategy
- **`docs/adr/ADR-014-persistence-sqlx-sqlite-postgres.md`** — Reference to dev.db for compile-time checks

---

## Acceptance Criteria

- ✅ `cargo build --workspace` completes with zero errors and zero warnings
- ✅ `cargo test --workspace` runs and all placeholder tests pass
- ✅ `cargo clippy --workspace -- -D warnings` passes (no lint failures)
- ✅ `cargo fmt --check` passes (code is formatted)
- ✅ All 8 crates are present in `crates/` with minimal valid `Cargo.toml` and `src/` structure
- ✅ `.github/workflows/ci.yml` is syntactically valid (can validate with `yamllint` or GitHub's workflow parser)
- ✅ `tests/fixtures/dev.db` file exists

---

## Language

**Rust** — All crate manifests and build configuration.

---

## Out-of-Scope

- Do NOT implement any domain logic, types, or business code in this prompt
- Do NOT add external crates beyond those listed in `workspace.dependencies`
- Do NOT configure logging, tracing, or database connection pools yet
- Do NOT write integration tests or complex test fixtures
