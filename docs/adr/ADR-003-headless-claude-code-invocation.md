# ADR-003 — Headless Claude Code Invocation via the `claude` CLI

**Status:** Accepted

---

## Context

Having decided to use Claude Code as the analysis engine (ADR-002), the orchestrator must have a concrete protocol for driving it headlessly from Rust. Claude Code exposes a CLI (`claude`) that supports non-interactive invocation with structured output, session continuity, and tool restrictions.

The key requirements for the invocation protocol are:

1. **Non-interactive** — the orchestrator drives the session programmatically, not via a human at a terminal.
2. **Structured output** — responses must conform to a JSON Schema contract, not free-text.
3. **Streaming** — large responses (module assessments) should stream as they are produced, not block until complete.
4. **Tool restriction** — the Claude Code agent must be constrained to read-only file-system operations within the cloned repository; it must not have network access, write access, or shell execution beyond `grep`/`find`.
5. **Session continuity** — a multi-turn analysis of a large module must be resumable without re-establishing context from scratch.
6. **Large-file handling** — the 10 MB stdin cap means large files must be passed by path, not piped.

---

## Decision

The `repogate-orchestrator` crate invokes Claude Code using `tokio::process::Command` with the following canonical invocation shape:

```
claude \
  --bare \
  -p "<prompt>" \
  --output-format stream-json \
  --json-schema <path-to-schema.json> \
  --allowedTools "Read,Glob,Bash(grep *),Bash(find *)" \
  --append-system-prompt "<phase-specific-system-prompt>"
```

**Key flag decisions:**

| Flag | Rationale |
|---|---|
| `--bare` | Suppresses ANSI/interactive decoration; produces clean newline-delimited JSON on stdout. |
| `-p "<prompt>"` | Single-turn entry point; the prompt is constructed by the orchestrator from the module manifest. |
| `--output-format stream-json` | Emits newline-delimited JSON events as they are produced. The orchestrator reads them via `BufReader` on the child's stdout. |
| `--json-schema <path>` | Schema is written to a temp file by the orchestrator before invocation; enforces structured output at the model level (see ADR-007). |
| `--allowedTools "Read,Glob,Bash(grep *),Bash(find *)"` | Whitelist: read-only file operations only. No network, no writes, no arbitrary shell. |
| `--append-system-prompt "..."` | Injects the phase-specific system context (e.g., "You are analyzing module X of repository Y. Focus on...") without replacing the base Claude Code system prompt. |

**Session management:**

The first invocation captures the `session_id` from the `system`/`init` event emitted at the start of the stream. Subsequent turns in the same analysis job use `--continue` (resume the most-recent session) or `--resume <session_id>` (resume a specific session by ID). This allows multi-turn traversal of large modules without re-reading already-discovered context.

**Large-file handling:**

Files larger than ~1 MB are never injected into the prompt text. Instead, the orchestrator passes the absolute path within the cloned repository. Claude Code's `Read` tool fetches the file content directly, bypassing the stdin cap.

**Event parsing:**

The orchestrator parses newline-delimited JSON events from the child process stdout. Relevant event types:
- `system` / `init` — captures `session_id`
- `assistant` — contains model response content (may include tool calls)
- `tool_result` — confirms tool execution by the Claude Code runtime
- `result` — final structured output; validated against the JSON Schema by the orchestrator before storing

**Error handling:**

Non-zero exit codes, `error` events, or schema validation failures are propagated as `OrchestratorError` variants. The job state machine transitions to `failed` with the partial output preserved (see ADR-009).

---

## Consequences

**Positive:**
- Pure Rust orchestration: no Node.js runtime, no TypeScript sidecar. The subprocess boundary is clean and well-understood.
- Tool allowlist enforces a security boundary: the analysis agent cannot modify the repository, make network calls, or execute arbitrary commands.
- `--resume` enables crash recovery at the session level without re-running prior turns.
- Streaming output means the orchestrator can begin persisting partial results before the model finishes responding.

**Negative / Trade-offs:**
- The `claude` CLI must be installed and version-pinned in the execution environment. A breaking CLI change (flag rename, output-format change) can break the orchestrator without a compile-time signal.
- The subprocess boundary adds latency overhead (process spawn, pipe setup) relative to an in-process SDK call.
- The 10 MB stdin cap is a hard constraint; the orchestrator must always pass large files by path, never by content injection.
- `--bare` mode is less documented than interactive mode; behavior on edge cases (very long outputs, tool errors) must be validated in integration tests.

---

## Alternatives Considered

**Self-hosted MCP server** — An MCP server exposes tools to the model over a protocol connection rather than via subprocess flags. This is a cleaner long-term architecture but requires running a persistent server process and a more complex integration. Rejected for MVP; may be revisited.

**Direct API with manual tool loop** — The orchestrator would implement the tool-execution loop itself, calling `messages.create` with tool definitions and executing `Read`/`Glob`/`grep` calls in Rust. This replicates Claude Code's loop, bypasses the `--json-schema` enforcement, and requires maintaining prompt construction for tool results manually. Rejected as higher complexity for lower capability.

**Python subprocess driver** — A Python script could drive the `claude` CLI and be called from Rust. This adds a Python runtime dependency and an extra subprocess layer. No benefit over direct Rust subprocess. Rejected.
