# ADR-008 — Deep-Not-Surface Traversal: Sub-Agent-Per-Module and Map-Reduce Synthesis

**Status:** Accepted

---

## Context

RepoGate's central design requirement is avoiding surface-level analysis. A single Claude Code session given a large repository cannot deeply analyze the entire codebase in one pass: context windows are finite, and a single session forced to process hundreds of files will necessarily skim or drop content.

The naive approach — dump as much of the repository as possible into a single prompt — fails for repositories above ~50k LOC:
- Context window limits are hit, causing truncation.
- The model's attention is spread too thin across too many files simultaneously.
- Hidden features in less-prominent modules receive less analysis than top-level README content.

The solution is a map-reduce pattern: decompose the repository into modules (map phase), analyze each module independently with its own Claude Code session and focused context (module analysis phase), then synthesize the structured module-level outputs into a final assessment (reduce phase). The reduce/synthesis pass consumes only schema-validated JSON summaries — not raw code — keeping its context clean.

---

## Decision

**Phase 1 — Module manifest construction (Rust):**

`repogate-ingestion` walks the file tree and groups files into logical modules using heuristics:
- Top-level directory boundaries (e.g., `src/`, `cli/`, `tests/`, `examples/`).
- Language/type clustering (all Rust files in `src/engine/` form a module).
- Manifest files (Cargo workspace members, `package.json` workspaces) define explicit module boundaries when present.

The result is a `ModuleManifest`: a list of modules, each with a name, file list, language breakdown, and estimated token count.

**Phase 2 — Sub-agent-per-module analysis (Claude Code):**

For each module in the manifest, `repogate-orchestrator` spawns one Claude Code invocation (using the protocol from ADR-003) with:
- A module-scoped prompt: "Analyze the following module of repository X. Files: [list]. Focus on: functionality inventory, hidden capabilities, enterprise value indicators, security sensitivity."
- `--allowedTools "Read,Glob,Bash(grep *),Bash(find *)"` — the agent can read files in the cloned repo freely.
- `--json-schema <ModuleAssessment schema>` — output is constrained to the `ModuleAssessment` struct.

Each module session is independent: its context contains only the files relevant to that module. The agent can read deeply within the module without context pressure from unrelated parts of the repository.

Module sessions run concurrently, bounded by the configured `max_concurrent_sessions` (default: 4, tunable per deployment to manage cost).

**Phase 3 — Synthesis (Claude Code):**

Once all module assessments are complete and validated, a single synthesis Claude Code session receives:
- The full list of `ModuleAssessment` JSON objects (not raw code).
- The `LicenseAnalysis` output.
- A synthesis prompt: "Given these per-module assessments, produce the final open-core gating recommendation and executive summary."
- `--json-schema <SynthesisOutput schema>`.

Because the synthesis pass receives only structured JSON summaries (not source files), its context is compact even for repositories with hundreds of modules.

**Size-based strategy selection:**

| Repository size | Strategy |
|---|---|
| < 50k LOC | Repomix: flatten the full repo into a single optimized prompt for single-session analysis |
| 50k–500k LOC | Sub-agent-per-module map-reduce (default) |
| > 500k LOC | Tree-Sitter knowledge-graph pre-processing to extract entity relationships before module decomposition |

---

## Consequences

**Positive:**
- Each module agent operates with a focused context — deep analysis of `src/engine/` is not diluted by the content of `docs/` or `examples/`.
- Concurrency: module sessions run in parallel, reducing wall-clock analysis time.
- The synthesis pass is cheap: it processes JSON structs, not raw code. Context usage is predictable.
- The `ModuleAssessment` schema acts as a quality gate: a module agent that fails to produce a valid assessment is retried or flagged, not silently dropped.
- The map phase is deterministic Rust code, not model inference — module boundaries are reproducible.

**Negative / Trade-offs:**
- Cross-module dependencies (a function in `src/core/` called from `src/cli/` and `src/sdk/`) may be partially missed if the two modules are analyzed by separate agents. The synthesis pass must be prompted to identify cross-module interactions.
- Spawning one session per module multiplies API cost proportionally to module count. Budget enforcement (ADR-013) is essential.
- Repomix (for small repos) introduces a different code path that must be tested separately.
- Tree-Sitter knowledge-graph pre-processing (for very large repos) is complex and not part of MVP; very large repos will use the standard map-reduce path until that capability is built.

---

## Alternatives Considered

**Single session, full-repo context** — Pass all repository content to one Claude Code session. Fails for repos > context window size; attention dilution degrades quality on large repos. Rejected for the primary path; retained as the Repomix path for small repos.

**Static analysis only (tree-sitter, semgrep) as primary** — Cannot reason about business value or commercialization intent. Used as pre-processing input (module boundaries, entity extraction) but not as the reasoning layer. Rejected as primary.

**Hierarchical session tree (sessions spawning sub-sessions)** — More granular decomposition (directory → file → function). Increases API cost and orchestration complexity significantly. May be explored for very large repos post-MVP.
