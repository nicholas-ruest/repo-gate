# P12 — `repogate-report`: Report Assembly, `minijinja` Templates, Canonical JSON

## Context

**You are implementing report generation: assembling Assessment from pipeline output and rendering JSON + Markdown.**

**Prerequisites:** P11 (synthesis + risk) is complete.

---

## Phase & Dependencies

- **Phase:** Reporting
- **Depends on:** P11

---

## Scope & Deliverables

Implement `repogate-report/src/` for report generation.

### File: `src/assembly.rs`

```rust
pub fn assemble(output: &PipelineOutput, generated_at: &str) -> Assessment {
    Assessment {
        repo_id: output.manifest.repo_id.clone(),
        schema_version: "1.0".to_string(),
        generated_at: generated_at.to_string(),
        is_complete: output.is_complete,
        repository: output.manifest.repository.clone(),
        modules: output.arch_map.modules.iter().map(|m| {
            Module {
                id: m.id.clone(),
                name: m.name.clone(),
                description: None,
                path: m.path.clone(),
                layer: m.layer,
                file_count: m.file_count,
                loc: m.loc,
                commercial_score: None,  // TODO: lookup from valuation
                recommended_tier: None,
                risks: vec![],
            }
        }).collect(),
        gating_strategy: Some(output.strategy.clone()),
        risks: output.risk_profile.risks.clone(),
    }
}
```

### File: `src/json.rs`

```rust
pub fn write_json(assessment: &Assessment, writer: impl std::io::Write) -> Result<(), ReportError> {
    serde_json::to_writer_pretty(writer, assessment)?;
    Ok(())
}

pub fn to_json_bytes(assessment: &Assessment) -> Result<Vec<u8>, ReportError> {
    serde_json::to_vec_pretty(assessment).map_err(|e| ReportError::JsonError(e.to_string()))
}
```

### File: `src/markdown.rs`

```rust
pub fn render_markdown(assessment: &Assessment) -> Result<String, ReportError> {
    let template = r#"
# RepoGate Assessment Report

## Executive Summary

Repository: {{ repo.name }}
Schema Version: {{ schema_version }}

## Gating Recommendations

{% for assignment in gating_strategy.tier_assignments %}
- **{{ assignment.module_name }}**: {{ assignment.tier }}
{% endfor %}

## Risk Analysis

{% for risk in risks %}
- **{{ risk.kind }}** ({{ risk.severity }}): {{ risk.description }}
{% endfor %}

## Modules

{% for module in modules %}
### {{ module.name }}
- Path: {{ module.path }}
- LOC: {{ module.loc }}
{% endfor %}
"#;
    
    use minijinja::Environment;
    let mut env = Environment::new();
    env.add_template("report", template)?;
    
    let tmpl = env.get_template("report")?;
    let rendered = tmpl.render(minijinja::context! {
        repo => &assessment.repository,
        schema_version => &assessment.schema_version,
        gating_strategy => &assessment.gating_strategy,
        risks => &assessment.risks,
        modules => &assessment.modules,
    })?;
    
    Ok(rendered)
}
```

### File: `src/pdf.rs`

```rust
pub fn render_pdf(markdown: &str, output_path: &Path) -> Result<(), ReportError> {
    let mut child = tokio::process::Command::new("pandoc")
        .arg("-f").arg("markdown")
        .arg("-t").arg("pdf")
        .arg("-o").arg(output_path)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ReportError::PandocNotFound
            } else {
                ReportError::PandocError(e.to_string())
            }
        })?;
    
    child.stdin.as_mut().unwrap().write_all(markdown.as_bytes())?;
    drop(child.stdin.take());
    
    child.wait()?;
    Ok(())
}
```

### File: `src/naming.rs`

```rust
pub fn report_stem(repo_url: &str, completed_at: &str) -> String {
    let parts: Vec<&str> = repo_url.trim_end_matches('/').split('/').collect();
    let owner = parts.get(parts.len() - 2).unwrap_or(&"unknown");
    let repo = parts.get(parts.len() - 1).unwrap_or(&"repo");
    
    let slugified_owner = owner.to_lowercase().replace("_", "-");
    let slugified_repo = repo.to_lowercase().replace("_", "-");
    
    format!("repogate-{}-{}-{}", slugified_owner, slugified_repo, completed_at)
}
```

### File: `src/lib.rs`

```rust
pub mod assembly;
pub mod json;
pub mod markdown;
pub mod pdf;
pub mod naming;

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("json error: {0}")]
    JsonError(String),
    
    #[error("pandoc not found")]
    PandocNotFound,
    
    #[error("pandoc error: {0}")]
    PandocError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_stem_github() {
        let stem = naming::report_stem("https://github.com/acme/myproject", "20240101-120000");
        assert!(stem.starts_with("repogate-acme-myproject"));
    }

    #[test]
    fn render_markdown_minimal() {
        let assessment = Assessment {
            // Minimal test assessment
        };
        let md = markdown::render_markdown(&assessment).unwrap();
        assert!(md.contains("Executive Summary"));
    }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-011-assessment-output-formats.md`** — Canonical JSON, minijinja, schema_version, pandoc PDF
- **`docs/ddd/report-delivery.md`** — Report structure, delivery mechanisms

---

## Acceptance Criteria

- ✅ `assemble()` → `is_complete: true` when pipeline complete
- ✅ `render_markdown()` of minimal Assessment contains "Executive Summary" and "Gating Recommendations"
- ✅ `report_stem("https://github.com/acme/myproject", ...)` → `"repogate-acme-myproject-<ts>"`
- ✅ JSON round-trip: write → read → equal
- ✅ `cargo test -p repogate-report` passes

---

## Language

**Rust** — JSON serialization, minijinja templating, PDF subprocess.

---

## Out-of-Scope

- Do NOT implement HTML export
- Do NOT implement report signing or encryption
