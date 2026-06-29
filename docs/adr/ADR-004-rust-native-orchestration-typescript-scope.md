# ADR-004 — Rust-Native Orchestration; TypeScript Scoped to the Web Dashboard Only

**Status:** Accepted

---

## Context

During early planning, a "TypeScript driver sidecar" pattern was considered: a Node.js process would drive Claude Code (since TypeScript SDKs for Claude Code exist and are well-documented), and the Rust orchestrator would call into it over a local socket or pipe. This pattern appears attractive because the `@anthropic-ai/claude-code` TypeScript SDK offers typed interfaces for session management and streaming events.

However, the project has already committed to Rust as the primary language (ADR-001) and to subprocess invocation of the `claude` CLI (ADR-003). The question is whether Rust can orchestrate Claude Code directly without a TypeScript intermediary.

The answer is yes. The `claude` CLI's `--output-format stream-json` and `--json-schema` flags provide everything needed:

- Structured output: enforced at the model level via `--json-schema`, parsed in Rust as `serde_json::Value`.
- Streaming: newline-delimited JSON events read via `tokio::io::BufReader` on the child stdout.
- Session management: `session_id` from the init event, `--resume` flag for continuity.
- Tool allowlist: `--allowedTools` flag.

There is no Rust SDK for Claude Code that is officially supported by Anthropic. Community crates exist (`claude-code-sdk-rs`, `anthropic-ai-sdk`) but they wrap the same subprocess interface. Using them would add an unversioned dependency on an unofficial crate for no additional capability.

The web dashboard has legitimately different constraints: it is a browser-rendered UI, it benefits from React's component model, and Next.js is the natural choice for SSR + static export + polling. TypeScript is the right tool for this layer.

---

## Decision

**Orchestration is implemented entirely in Rust.** The `repogate-orchestrator` crate drives `claude` CLI subprocesses directly using `tokio::process::Command`. No TypeScript sidecar, no Node.js process, no inter-process RPC between Rust and JavaScript for the analysis pipeline.

**The `@anthropic-ai/claude-code` TypeScript SDK and any community Rust wrappers are not used.** The subprocess interface is sufficient and avoids unofficial dependency risk.

**TypeScript is used only for the Next.js web dashboard** (`repogate-web/`). The web app communicates with the Rust `repogate-server` over HTTP (see ADR-015). It has no direct dependency on the orchestrator internals.

The boundary is:
```
repogate-server (Rust/axum) ←HTTP/JSON→ repogate-web (TypeScript/Next.js)
```

No Rust crate imports TypeScript. No TypeScript module imports Rust. The contract is the HTTP API schema.

---

## Consequences

**Positive:**
- Single language for all analysis and server logic: Rust. No context-switching between Rust and TypeScript in the hot path.
- No dependency on unofficial crates for core orchestration functionality.
- TypeScript scope is clearly bounded: the web UI is the only place it appears. Contributors can understand the system without needing to know both Rust and TypeScript orchestration patterns.
- The HTTP boundary between `repogate-server` and `repogate-web` is a clean, testable contract.

**Negative / Trade-offs:**
- The TypeScript Claude Code SDK offers typed stream event interfaces that Rust must replicate manually using `serde` derive macros. This is a one-time implementation cost.
- If Anthropic changes the `claude` CLI's output format or flag interface, Rust parsing code must be updated. An SDK would abstract this — but an unofficial Rust SDK provides no guarantee of keeping pace either.
- Contributors who know the TypeScript SDK well will need to re-learn the Rust subprocess interface.

---

## Alternatives Considered

**TypeScript driver sidecar (originally proposed)** — A Node.js process drives Claude Code using the official TypeScript SDK; the Rust orchestrator calls it over a Unix socket or HTTP. This adds a process boundary, a second language runtime in the hot path, IPC complexity, and two separate failure modes. The TypeScript SDK's advantages do not justify this complexity when `--output-format stream-json` is available. **Rejected; this decision supersedes the sidecar proposal.**

**Unofficial `claude-code-sdk-rs` crate** — Provides typed Rust bindings for the subprocess interface. Useful as a reference implementation but introduces an unversioned, community-maintained dependency on a critical integration point. Rejected in favor of direct `tokio::process::Command` with `serde` parsing.

**Go for the orchestrator** — Go's subprocess handling (`exec.Command`) is comparable to Rust's. But the project is already committed to Rust for other crates (ingestion, licensing, scoring), and splitting the orchestrator into a second language increases operational complexity. Rejected.
