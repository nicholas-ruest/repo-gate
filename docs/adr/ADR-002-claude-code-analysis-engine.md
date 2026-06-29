# ADR-002 — Claude Code as the Analysis and Reasoning Engine

**Status:** Accepted

---

## Context

RepoGate's core value proposition is deep, not surface-level, repository analysis. The initial spec explicitly calls out the failure mode of LLM-based reviews that stop at the README, folder names, and obvious files. Achieving genuine depth requires an agent that can:

1. Traverse an arbitrary directory tree using filesystem tools (`Read`, `Glob`, `find`, `grep`).
2. Follow references across files — a function defined in `src/core/engine.rs` called from `src/cli/mod.rs` and tested in `tests/integration/engine_test.rs` must be understood as a single capability, not three disconnected fragments.
3. Reason about intent from tests, examples, and undocumented internal modules, not just public API surfaces.
4. Produce structured output from that reasoning in a machine-readable schema.

A plain Anthropic API call (`messages.create`) with file contents pasted into the prompt cannot achieve this. The context window has a fixed upper bound; a large repository (millions of tokens of source) cannot be pasted in. More importantly, the model cannot choose which files to read next based on what it has already seen — there is no tool-calling loop.

Claude Code is the Anthropic-built agentic execution environment that provides exactly this capability: it exposes file-system tools to the model in a structured tool-use loop, allows multi-turn sessions, and supports schema-enforced structured output.

---

## Decision

Claude Code is the analysis and reasoning engine for RepoGate. All deep codebase traversal, architectural reasoning, module assessment, and final synthesis are performed by Claude Code agents invoked by the `repogate-orchestrator` crate.

Claude Code is used **instead of** direct Anthropic API calls for analysis tasks. Direct API calls are not used for any analysis phase.

The reasoning:
- Claude Code's tool-use loop lets the model decide what to read next — the model is an active participant in discovery, not a passive recipient of pre-selected context.
- File-system tools (`Read`, `Glob`, `Bash(grep ...)`, `Bash(find ...)`) allow traversal of repositories that are orders of magnitude larger than any context window.
- The `--json-schema` flag enforces structured output at the model level, not via post-hoc parsing (see ADR-007).
- Multi-turn sessions (`--continue`/`--resume`) allow an analysis job to span many model interactions without losing context within a session.

Claude Code is invoked headlessly as a subprocess. See ADR-003 for the invocation protocol.

---

## Consequences

**Positive:**
- Genuine deep traversal: the model reads only what it needs, in the order it chooses, discovering hidden capabilities that a static pre-selection would miss.
- Schema-enforced output eliminates a class of parsing failures.
- Multi-turn session continuity means a single logical "analysis of module X" can span many file reads without the orchestrator manually accumulating context.
- The agentic approach naturally supports the map-reduce pattern described in ADR-008: one sub-agent per module, each with its own isolated context.

**Negative / Trade-offs:**
- RepoGate is operationally dependent on the `claude` CLI binary being available in the execution environment. Version pinning is required to avoid breaking schema or flag changes.
- Analysis cost is driven by model token consumption, not just compute time. See ADR-013 for budget enforcement.
- The tool-use loop is non-deterministic in traversal order; two runs on the same repo may visit files in different sequences (though the final structured output is schema-validated).
- Claude Code does not have an official Rust SDK. The orchestrator drives it via subprocess (ADR-003).

---

## Alternatives Considered

**Direct Anthropic API (`messages.create` with tool_use)** — Would require the orchestrator to implement the tool-execution loop itself, manage file reads, and inject content into the prompt. This replicates Claude Code's core loop at significant complexity cost, and still hits context-window limits on large repos. Rejected.

**OpenAI / other LLM providers** — RepoGate is designed around Claude Code's specific capabilities (schema-enforced output, file-system tool integration, `--resume` sessions). Provider lock-in is intentional here: depth of integration > provider neutrality. Rejected for MVP.

**Static analysis only (tree-sitter, semgrep, etc.)** — Cannot reason about business value, commercialization risk, or undocumented intent. These tools are used as inputs to Claude Code (see ADR-008) but not as the primary reasoning layer. Rejected as the primary engine.
