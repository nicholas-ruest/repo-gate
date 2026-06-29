# ADR-005 — Git Ingestion (subprocess `git` for MVP → `gix`), Tree Walking with `ignore`, Language Stats via `tokei` + `hyperpolyglot`

**Status:** Accepted

---

## Context

Before Claude Code can analyze a repository, RepoGate must:

1. **Clone or fetch** the repository from a remote URL (GitHub, GitLab, Bitbucket, self-hosted).
2. **Walk the file tree** efficiently, respecting `.gitignore` rules so binary artifacts, `node_modules`, and build outputs are excluded from analysis scope.
3. **Detect file types and languages** to route files to the correct analysis strategy and produce accurate language statistics for the module manifest.
4. **Filter binaries** early — sending binary files to Claude Code wastes tokens and produces noise.

The ingestion phase produces a `RepoManifest` struct (defined in `repogate-core`) that the orchestrator uses to build per-module analysis prompts.

Several Rust crates exist for each of these responsibilities. The choices must balance maturity, correctness, and maintenance health.

---

## Decision

**Git operations: subprocess `git` for the MVP; migrate to `gix` (gitoxide) post-MVP; `git2` as the intermediate fallback.**

The clone step is abstracted behind a `GitProvider` trait in `repogate-ingestion` so the implementation can evolve without touching the rest of the pipeline. The decided ordering is: **subprocess `git` (MVP) → `gix` (post-MVP target) → `git2` (fallback)**.

**MVP — subprocess `git`.** The orchestrator shells out to the system `git` binary via `tokio::process::Command`:

```
git clone --depth=1 --filter=blob:none <url> <dest>
```

This adds zero new library dependencies, and every developer machine and CI runner already has `git` installed. It reliably clones arbitrary public repositories — including the SSH, LFS, and submodule cases that pure-Rust clients still handle imperfectly. `--depth=1 --filter=blob:none` performs a partial clone (commit metadata only, file blobs fetched lazily on demand during the tree walk), which keeps even very large repositories fast to ingest. This is the most robust option for an MVP that must clone unpredictable third-party repos on day one.

**Post-MVP target — `gix` (gitoxide).** `gix` is a pure-Rust implementation of the Git protocol and object model: async-native, no `libgit2` C dependency, reproducible builds. It is the forward-looking choice and removes the runtime dependency on a system `git` binary. It is deferred because, as of mid-2026, `gix` still has rough edges in SSH protocol handling and shallow-clone behaviour that make it risky for an MVP cloning arbitrary repos. Migration is a clean swap behind `GitProvider`; the rest of the ingestion pipeline (`ignore` + `tokei` + `hyperpolyglot`) is already pure Rust and does not change.

**Fallback — `git2`.** Bindings to `libgit2`; mature and widely used, but links a C library (cross-compilation and static-build friction). Retained as the intermediate fallback for any operation `gix` cannot yet perform once we migrate.

**Tree walking: `ignore` crate (the ripgrep engine).**

The `ignore` crate implements `.gitignore`, `.ignore`, and global ignore patterns with the same semantics as ripgrep — the most widely tested gitignore implementation outside of git itself. It handles nested ignore files, negation patterns, and case sensitivity correctly. The walker is parallel by default (powered by `rayon` internally) and returns entries in a deterministic order with configurable threading.

**Language detection: `tokei` (as library) + `hyperpolyglot`.**

`tokei` counts lines of code by language with high accuracy and is well-maintained. Used as a library (`tokei::Languages`), it produces per-language statistics (files, lines, code, comments, blanks) that feed the module manifest.

`hyperpolyglot` is a Rust port of GitHub's Linguist: it uses heuristics, filename patterns, and shebang detection to classify files where `tokei` is ambiguous. The two are used in combination: `tokei` for counts, `hyperpolyglot` for per-file language classification when needed.

**Binary filtering:**

Files are filtered as binary early in the walk using a simple heuristic: if the first 8 KB of a file contains a null byte (`\0`), it is classified as binary and excluded from the analysis manifest. Known binary extensions (`.png`, `.jpg`, `.wasm`, `.pdf`, `.zip`, etc.) are excluded by extension before reading. This happens in the `repogate-ingestion` crate before the manifest is handed to the orchestrator.

---

## Consequences

**Positive:**
- Subprocess `git` for the MVP adds zero library dependencies and is the most robust way to clone arbitrary public repos (SSH, LFS, submodules all handled by the system binary), de-risking the most failure-prone step on day one.
- `--depth=1 --filter=blob:none` partial clone keeps even very large repos fast to ingest while still letting the tree walk resolve file content on demand.
- The `GitProvider` trait makes the `gix` migration a clean, isolated swap with no change to the rest of the pipeline.
- The `ignore` crate's gitignore semantics are tested at ripgrep scale — fewer surprises with unusual ignore patterns.
- `tokei` + `hyperpolyglot` gives both aggregate statistics (for the executive summary) and per-file classification (for routing files to the correct module analysis agent).
- Binary filtering early in the walk avoids wasting orchestrator time and tokens on non-text content.

**Negative / Trade-offs:**
- The MVP depends on a system `git` binary of unknown version. Mitigation: pin a minimum version and assert it at startup; subprocess output parsing is minimal (we rely on exit status, not stdout scraping).
- `gix` is less battle-tested than `git2` for exotic repository configurations (large monorepos, unusual packfile formats, LFS), which is precisely why it is deferred past the MVP rather than adopted first.
- Two language detection crates increase the dependency surface. If one becomes unmaintained, the other provides coverage, but both must be audited in `cargo audit`.
- Shallow / partial clones are the default for analysis — full history is not needed. But some repositories use git history as documentation (changelogs, blame). This is an acceptable loss for the analysis use case.

---

## Alternatives Considered

**`gix` as the day-one primary client** — Tempting for a pure-Rust build, but as of mid-2026 its SSH and shallow-clone behaviour is not yet reliable enough to clone arbitrary third-party repos unattended. Deferred to a post-MVP migration rather than rejected — it remains the long-term target.

**`git2` only (no subprocess, no `gix`)** — `git2` is mature but requires linking `libgit2` (a C library), which complicates cross-compilation and static builds, and offers no advantage over subprocess `git` for a clone-and-walk workload. Retained only as the intermediate fallback.

**Subprocess `git` as the permanent solution** — Rejected as a permanent choice (not as the MVP choice): it couples RepoGate to an external binary of unknown version. Acceptable and pragmatic for the MVP; `gix` removes this coupling once it stabilizes.

**`walkdir` for tree walking** — Does not implement `.gitignore` semantics natively. Would require bolting on ignore logic manually. The `ignore` crate does this correctly out of the box. Rejected.

**`linguist` via Ruby subprocess** — GitHub's original Linguist is the gold standard but requires a Ruby runtime. `hyperpolyglot` is a direct Rust port. Rejected (Ruby runtime dependency).
