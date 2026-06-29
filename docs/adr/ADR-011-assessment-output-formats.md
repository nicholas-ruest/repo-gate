# ADR-011 — Assessment Output Formats

**Status:** Accepted

---

## Context

RepoGate produces assessment outputs consumed by different audiences:
- **API consumers and downstream tools** need machine-readable, stable, schema-validated data.
- **Human reviewers** (founders, legal teams, product managers) need readable narrative output with clear structure.
- **Enterprise customers** may need PDF exports for sharing in document-based workflows.

The output system must serve all three without the formats diverging or requiring separate data pipelines. There must be a single authoritative source of truth from which all formats are derived.

Additionally, output files must be named in a way that makes them identifiable, sortable, and associated with the specific repository and run time.

---

## Decision

**Canonical JSON as the durable artifact:**

The `AssessmentReport` struct (defined in `repogate-core`, schema-generated via `schemars`) serialized to JSON is the canonical output artifact. It is:
- Written to disk and stored in the database after the `reporting` phase.
- The source from which all other formats are rendered.
- Versioned (a `schema_version` field in the root object) to allow future schema evolution with backward-compatible readers.
- Returned by the `GET /assessments/:id` API endpoint as `application/json`.

**Markdown rendering via `minijinja`:**

The human-readable report is a Markdown file rendered from the canonical JSON using `minijinja` (a minimal, correct Jinja2 implementation in Rust). Templates live in `repogate-report/templates/`. The Markdown report includes:
- Executive summary
- Module-by-module table (tier, composite score, key findings)
- Full dimensional scores per module (collapsible in rendered Markdown)
- License and dependency posture
- Legal review flags
- Gating recommendations with rationale

`minijinja` is preferred over `tera` because it is actively maintained, has a smaller surface area, and is compatible with standard Jinja2 template syntax, making templates legible to non-Rust contributors.

**PDF rendering (optional):**

PDF export is implemented by converting the Markdown output to PDF via `pandoc` invoked as a subprocess. PDF generation is optional (off by default, enabled per-job via the API or CLI flag) because `pandoc` is an external binary dependency. When PDF is requested and `pandoc` is not available, the error is surfaced clearly rather than silently skipping.

**File naming convention:**

```
repogate-{repo-slug}-{YYYYMMDD-HHmmss}.json
repogate-{repo-slug}-{YYYYMMDD-HHmmss}.md
repogate-{repo-slug}-{YYYYMMDD-HHmmss}.pdf  (if requested)
```

Where `{repo-slug}` is the repository's owner/name with `/` replaced by `-` and non-alphanumeric characters stripped (e.g., `acme-myproject`). Timestamps are UTC in the job's `completed_at` field.

**API delivery:**

| Endpoint | Response |
|---|---|
| `GET /assessments/:id` | `AssessmentReport` as `application/json` |
| `GET /assessments/:id/report` | Rendered Markdown as `text/markdown` |
| `GET /assessments/:id/report.pdf` | PDF as `application/pdf` (if available) |

---

## Consequences

**Positive:**
- Single source of truth (canonical JSON) prevents format drift.
- `minijinja` templates are editable without Rust knowledge — product and design can iterate on report layout independently.
- File naming convention makes reports sortable by repository and date in any file browser or object store.
- Schema versioning allows the canonical format to evolve without breaking existing stored reports.

**Negative / Trade-offs:**
- PDF via `pandoc` subprocess adds an external binary dependency. In containerized deployments, `pandoc` must be included in the image, which is large (~500 MB with LaTeX support for high-quality PDF). Using a lighter PDF renderer (e.g., `wkhtmltopdf`, `weasyprint`) is a future option if `pandoc` image size is prohibitive.
- `minijinja` template rendering can be slow for very large reports (hundreds of modules). Template caching and lazy rendering of collapsed sections may be needed at scale.
- Markdown is not universally rendered — raw Markdown is less useful in non-GitHub contexts. The API serves it as `text/markdown`; rendering is the client's responsibility.

---

## Alternatives Considered

**`tera` template engine** — An earlier consideration. `minijinja` is preferred because it is more actively maintained and has fewer breaking changes between versions. Both are Jinja2-compatible. Rejected in favor of `minijinja`.

**HTML as the primary human-readable format** — More portable than Markdown for sharing (no renderer required). But HTML is harder to review in a terminal, harder to diff in git, and requires a CSS layer. Markdown is the better fit for the target audience (developers, technical reviewers). PDF covers the "share in a meeting" use case. Rejected as primary.

**JSON + separate schema** — Storing the schema separately from the data. Rejected in favor of embedding `schema_version` in the canonical JSON and generating schemas from Rust types.
