# P09 — `repogate-orchestrator`: Functionality Discovery Fan-Out Phase

## Context

**You are implementing the fan-out phase where each module gets analyzed by Claude for deep functionality discovery.**

**Prerequisites:** P08 (architecture mapping) is complete.

---

## Phase & Dependencies

- **Phase:** Analysis pipeline
- **Depends on:** P08

---

## Scope & Deliverables

### File: `src/pipeline/feature_discovery.rs`

```rust
pub async fn run_feature_discovery_phase(
    arch_map: &ArchitectureMap,
    repo_path: &Path,
    session_runner: impl SessionRunner + Clone,
    module_store: &dyn ModuleAssessmentStore,
    budget: &BudgetTracker,
    job_id: &str,
    max_concurrent: usize,
) -> Result<FunctionalityInventory, OrchestratorError> {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));
    let mut handles = vec![];
    
    for module in &arch_map.modules {
        // Skip if already analyzed (crash recovery)
        if module_store.find_by_module_id(&module.id).await.ok().flatten().is_some() {
            continue;
        }
        
        let module_clone = module.clone();
        let sem = semaphore.clone();
        let runner = session_runner.clone();
        let store = module_store.clone();
        let repo = repo_path.to_path_buf();
        let model = select_model(&module.name, Phase::FeatureDiscovery);
        
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.ok()?;
            
            let prompt = format!("Analyze module {} at {}. Discover all capabilities (public, internal, experimental, undocumented, enterprise). Return ModuleAssessment schema.", module_clone.name, module_clone.path);
            
            let result = runner.run(
                ClaudeInvocation {
                    prompt,
                    model,
                    schema_path: Some("module_assessment_schema.json".into()),
                    allowed_tools: vec!["Read".into(), "Glob".into(), "Bash(grep)".into(), "Bash(find)".into()],
                    system_prompt: None,
                    working_dir: Some(repo),
                    session_id: None,
                }
            ).await.ok()?;
            
            let assessment: ModuleAssessment = serde_json::from_str(&result.output).ok()?;
            store.save(assessment).await.ok()?;
            
            Some(result.usage.input_tokens + result.usage.output_tokens)
        });
        
        handles.push(handle);
        
        if budget.is_exceeded() {
            break;
        }
    }
    
    // Collect results
    let mut total_tokens = 0u64;
    for handle in handles {
        if let Ok(Some(tokens)) = handle.await {
            total_tokens += tokens;
        }
    }
    
    Ok(FunctionalityInventory {
        repo_id: job_id.to_string(),
        items: vec![],  // Populated from module assessments
        total_count: arch_map.modules.len(),
        hidden_count: 0,
        enterprise_count: 0,
        api_entry_points: vec![],
    })
}
```

### File: `src/pipeline/llm_adapter.rs`

```rust
pub fn parse_module_assessment(raw: &str) -> Result<ModuleAssessment, SchemaViolationError> {
    serde_json::from_str(raw).map_err(|e| SchemaViolationError(e.to_string()))
}

pub fn map_to_functionality_items(
    assessment: &ModuleAssessment,
    module_path: &str,
) -> Vec<FunctionalityItem> {
    assessment.capabilities.iter().map(|cap| {
        FunctionalityItem {
            name: cap.name.clone(),
            description: cap.description.clone(),
            visibility: if cap.is_enterprise {
                Visibility::Enterprise
            } else if cap.is_undocumented {
                Visibility::Undocumented
            } else {
                Visibility::Public
            },
            source_location: None,
            discovery_method: cap.discovery_method.clone(),
            is_confirmed: cap.source_locations.is_some(),
        }
    }).collect()
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-008-deep-traversal-map-reduce.md`** — Sub-agent-per-module, tool allowlist, concurrency
- **`docs/adr/ADR-003-headless-claude-code-invocation.md`** — Tool allowlist

---

## Acceptance Criteria

- ✅ Mock SessionRunner with canned JSON: saves assessments, skips already-analyzed modules
- ✅ Respects concurrency cap (Semaphore)
- ✅ `is_enterprise: true` → `Visibility::Enterprise`
- ✅ Budget exhaustion stops new sessions, preserves stored assessments
- ✅ `cargo test -p repogate-orchestrator` passes

---

## Language

**Rust** — Async fan-out, Claude invocation, schema adaptation.

---

## Out-of-Scope

- Do NOT implement repomix small-repo path (P17)
- Do NOT implement deep source code tracing; rely on Claude
