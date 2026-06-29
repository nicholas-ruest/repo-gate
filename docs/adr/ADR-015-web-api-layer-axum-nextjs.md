# ADR-015 — Web and API Layer: `axum` HTTP Server and Next.js Dashboard

**Status:** Accepted

---

## Context

RepoGate's UX goal is simple: paste a repository URL, get a gating recommendation. This requires:
1. A **REST API** that accepts repository URLs, creates analysis jobs, and returns job status and reports.
2. A **web dashboard** that provides a browser-based UI for the paste-a-link workflow, displays real-time job progress, and renders the tabbed assessment report.

The API server must be part of the Rust codebase (consistent with ADR-001) and serve the web dashboard's static files in production (no separate static file server required).

The web dashboard is the one place TypeScript is justified: browser UI development with React and Next.js is a well-established TypeScript domain with no comparable Rust alternative for the frontend.

---

## Decision

**HTTP server: `axum`.**

`axum` is the Tokio-native async HTTP framework from the Tokio team. It integrates directly with Tokio's runtime and the `tower` middleware ecosystem. It is chosen over `actix-web` (which uses its own actor runtime, adding complexity) and `warp` (less active maintenance).

The `repogate-server` crate implements the following API:

| Method | Endpoint | Description |
|---|---|---|
| `POST` | `/assessments` | Create a new assessment job. Body: `{ repo_url, budget_usd, model_override?, weights? }`. Returns `{ job_id, estimated_cost }`. |
| `GET` | `/assessments/:id` | Return the canonical `AssessmentReport` as `application/json` (or job status if not complete). |
| `GET` | `/assessments/:id/report` | Return the rendered Markdown report as `text/markdown`. |
| `GET` | `/assessments/:id/report.pdf` | Return the PDF report as `application/pdf` (if available). |
| `GET` | `/assessments/:id/status` | Return the current job phase and progress percentage (for UI polling). |
| `DELETE` | `/assessments/:id` | Cancel a queued or in-progress job. |

**Web dashboard: Next.js (TypeScript).**

The web dashboard (`repogate-web/`) is a Next.js application. It is the only TypeScript artifact in the project (consistent with ADR-004). Responsibilities:
- Paste-a-link form: accept a repository URL, set a budget, submit to `POST /assessments`.
- Job status poller: poll `GET /assessments/:id/status` every 3 seconds while the job is in progress; display the current phase and module-completion count.
- Report viewer: render the tabbed assessment report from the canonical JSON (`GET /assessments/:id`). Tabs: Executive Summary, Module Map, Gating Recommendations, License Posture, Full Inventory.

**Static build served by `axum` in production:**

Next.js is configured for static export (`output: 'export'`). The `repogate-server` crate serves the exported static files from a configurable directory using `tower-http`'s `ServeDir` service. In production, a single `repogate-server` binary serves both the API and the web dashboard — no separate nginx/CDN is required for single-node deployments.

**Development mode:**

In development, the Next.js dev server runs on port 3000 and proxies API requests to `repogate-server` on port 8080 (configured via `next.config.js` rewrites). The Rust server and Next.js dev server run independently.

**Authentication:**

MVP uses API key authentication for the REST API (key passed in `Authorization: Bearer <key>` header). The web dashboard sends the API key in localStorage (dev-grade security, not production-hardened). Production authentication (OAuth, SSO) is deferred post-MVP.

---

## Consequences

**Positive:**
- `axum` + Tokio: the HTTP server shares the same async runtime as the orchestrator and database layer — no thread-pool bridging.
- Single binary serves API + static web dashboard: simple deployment, no nginx dependency.
- The REST API is the clean boundary between Rust and TypeScript (consistent with ADR-004). The Next.js app is a consumer of the API, not a component of the analysis pipeline.
- Next.js static export produces portable HTML/CSS/JS that can be served from any static host if the single-binary approach is not desired.
- The polling model (3-second interval on job status) is simple to implement and requires no WebSocket infrastructure for MVP. WebSocket or SSE push can replace polling post-MVP if real-time UX becomes a priority.

**Negative / Trade-offs:**
- Polling adds latency to UI updates (up to 3 seconds behind actual job progress) and generates steady API traffic during long-running jobs.
- Serving Next.js static files from `axum` means the Rust binary must be redeployed when the web dashboard changes. Decoupling them (separate CDN for the frontend) is a future option.
- `next export` does not support Next.js server-side features (API routes, server components with data fetching). The dashboard is a pure client-side application that fetches data from the Rust API. This limits some Next.js capabilities but is sufficient for the MVP use case.
- API key in localStorage is not production-secure. Authentication must be hardened before public deployment.

---

## Alternatives Considered

**`actix-web`** — Mature, high-performance Rust HTTP framework. Uses its own actor-based runtime, which adds complexity when integrating with Tokio-based components (orchestrator, sqlx). Rejected in favor of `axum`'s native Tokio integration.

**`warp`** — Functional-style Rust HTTP framework. Less actively maintained than `axum` as of 2025. Rejected.

**React without Next.js** — A plain React SPA (CRA or Vite). Next.js is chosen for its built-in TypeScript support, static export, and routing. The overhead is minimal for the dashboard's use case. Acceptable alternative if Next.js becomes a maintenance concern.

**WebSockets for real-time progress** — More responsive than polling but requires persistent connection handling in `axum` and state management in the frontend. The polling model is sufficient for MVP job durations (minutes). Deferred.

**Server-side rendering of the report** — Rendering the Markdown report on the server and serving HTML. Adds Rust template rendering to the HTTP handler. The tabbed report viewer benefits from client-side interactivity (tab switching, collapsible sections), which is better served by a React component. Rejected for the report viewer; the API serves raw JSON/Markdown and the dashboard renders it.
