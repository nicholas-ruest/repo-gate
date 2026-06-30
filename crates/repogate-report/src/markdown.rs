//! Human-readable Markdown rendering via `minijinja` (ADR-011).

use minijinja::{context, Environment};
use repogate_core::Assessment;

use crate::ReportError;

const TEMPLATE: &str = r#"# RepoGate Assessment Report

## Executive Summary

- Repository: {{ repo.name }}
- URL: {{ repo.url }}
- Schema Version: {{ schema_version }}
- Generated: {{ generated_at }}
- Complete: {{ is_complete }}
- Total LOC: {{ repo.metrics.total_loc }}

## Gating Recommendations

{% if gating_strategy %}{% for assignment in gating_strategy.tier_assignments %}- **{{ assignment.module_name }}**: {{ assignment.tier }} — {{ assignment.rationale }}
{% endfor %}{% endif %}
## License Posture

{% if repo.license %}Primary license: {{ repo.license }}{% else %}No primary license detected.{% endif %}

## Risk Analysis

{% for risk in risks %}- **{{ risk.kind }}** ({{ risk.severity }}): {{ risk.description }}
{% endfor %}
## Modules

{% for module in modules %}### {{ module.name }}
- Path: {{ module.path }}
- Layer: {{ module.layer }}
- LOC: {{ module.loc }}
- Recommended tier: {{ module.recommended_tier }}
{% endfor %}"#;

/// Render the assessment to a Markdown report.
pub fn render_markdown(assessment: &Assessment) -> Result<String, ReportError> {
    let mut env = Environment::new();
    env.add_template("report", TEMPLATE)
        .map_err(|e| ReportError::Render(e.to_string()))?;
    let tmpl = env
        .get_template("report")
        .map_err(|e| ReportError::Render(e.to_string()))?;
    tmpl.render(context! {
        repo => &assessment.repository,
        schema_version => &assessment.schema_version,
        generated_at => &assessment.generated_at,
        is_complete => assessment.is_complete,
        gating_strategy => &assessment.gating_strategy,
        risks => &assessment.risks,
        modules => &assessment.modules,
    })
    .map_err(|e| ReportError::Render(e.to_string()))
}
