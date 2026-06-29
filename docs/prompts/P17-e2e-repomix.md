# P17 — End-to-End Integration Tests + Repomix Small-Repo Path

## Context

**You are implementing end-to-end tests and the small-repo optimization path using repomix.**

**Prerequisites:** P14 (CLI), P15 (server), P16 (web) are complete.

---

## Phase & Dependencies

- **Phase:** Hardening
- **Depends on:** P14, P15, P16

---

## Scope & Deliverables

### File: `tests/integration/e2e_pipeline.rs`

```rust
#[tokio::test]
async fn test_full_pipeline_with_mock() {
    // Create a mock small local repo (use a git fixture)
    let repo_path = Path::new("tests/fixtures/sample-repo");
    
    let runner = PipelineRunner::new(
        Box::new(MockSessionRunner::with_canned_responses(
            vec![
                "module_assessment_1.json",
                "module_assessment_2.json",
                "synthesis_output.json",
                "risk_output.json",
            ]
        )),
        Box::new(InMemoryCheckpointStore::new()),
        Box::new(InMemoryAssessmentJobStore::new()),
    );
    
    let output = runner.run(
        "https://github.com/example/test-repo",
        20_000_000,  // 20M tokens
        &ScoreWeights::default(),
    ).await.expect("pipeline should complete");
    
    assert!(output.is_complete);
    assert!(!output.arch_map.modules.is_empty());
    assert!(!output.valuation.module_scores.is_empty());
    assert!(output.strategy.tier_assignments.len() > 0);
}

#[tokio::test]
async fn test_crash_recovery() {
    let runner = PipelineRunner::new(
        Box::new(MockSessionRunner::default()),
        Box::new(InMemoryCheckpointStore::new()),
        Box::new(InMemoryAssessmentJobStore::new()),
    );
    
    // Simulate crash after module 2 of 5
    let checkpoint = JobCheckpoint {
        job_id: "test-job".to_string(),
        last_completed_phase: Some(PhaseKind::FeatureDiscovery),
        completed_module_ids: vec!["mod1".to_string(), "mod2".to_string()],
        token_usage_so_far: 500_000,
        partial_results: serde_json::json!({}),
        saved_at: chrono::Utc::now().to_rfc3339(),
    };
    
    let phases_to_run = phases_to_run(&checkpoint);
    
    // Should resume from next phase
    assert!(phases_to_run.contains(&PhaseKind::Scoring));
    assert!(!phases_to_run.contains(&PhaseKind::FeatureDiscovery));
}

#[test]
fn test_repomix_single_session_path() {
    let manifest = RepoManifest {
        total_loc: 30_000,  // < 50k
        // ...
    };
    
    assert!(should_use_repomix(&manifest));
}

fn should_use_repomix(manifest: &RepoManifest) -> bool {
    manifest.total_loc < 50_000
}
```

### File: `src/pipeline/feature_discovery.rs` — Repomix Integration

```rust
pub async fn run_feature_discovery_phase_with_repomix(
    arch_map: &ArchitectureMap,
    repo_path: &Path,
    session_runner: impl SessionRunner,
    module_store: &dyn ModuleAssessmentStore,
) -> Result<FunctionalityInventory, OrchestratorError> {
    let total_loc = compute_total_loc(repo_path);
    
    if total_loc < 50_000 {
        // Single-session repomix path
        run_single_session_analysis(repo_path, session_runner).await
    } else {
        // Standard fan-out path
        run_fan_out_analysis(arch_map, repo_path, session_runner, module_store).await
    }
}

async fn run_single_session_analysis(
    repo_path: &Path,
    session_runner: impl SessionRunner,
) -> Result<FunctionalityInventory, OrchestratorError> {
    // Run: repomix --output-format xml <path>
    let output = tokio::process::Command::new("repomix")
        .arg("--output-format").arg("xml")
        .arg(repo_path)
        .output()
        .await;
    
    match output {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Fallback to fan-out if repomix not installed
            return Err(OrchestratorError("repomix not found, falling back".into()));
        }
        Err(e) => return Err(OrchestratorError(format!("repomix error: {}", e))),
        Ok(out) => {
            if !out.status.success() {
                return Err(OrchestratorError("repomix failed".into()));
            }
            
            let xml_content = String::from_utf8(out.stdout)?;
            
            // Single Claude session over full repo
            let prompt = format!(
                "Analyze this repository XML output. Identify all capabilities.\nReturn ModuleAssessment schema with module_name: 'all'.\n\n{}",
                xml_content
            );
            
            let result = session_runner.run(
                ClaudeInvocation {
                    prompt,
                    model: ClaudeModel::Sonnet,
                    schema_path: Some("module_assessment_schema.json".into()),
                    allowed_tools: vec![],
                    system_prompt: None,
                    working_dir: Some(repo_path.to_path_buf()),
                    session_id: None,
                }
            ).await?;
            
            let assessment: ModuleAssessment = serde_json::from_str(&result.output)?;
            
            Ok(FunctionalityInventory {
                repo_id: uuid::Uuid::new_v4().to_string(),
                items: map_to_functionality_items(&assessment, ""),
                total_count: 1,
                hidden_count: 0,
                enterprise_count: 0,
                api_entry_points: vec![],
            })
        }
    }
}

fn compute_total_loc(repo_path: &Path) -> usize {
    // Use tokei to aggregate LOC
    0  // Placeholder
}
```

### File: `.github/workflows/e2e.yml`

```yaml
name: E2E Tests

on:
  push:
    branches: [main]

jobs:
  e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      
      - name: Build CLI
        run: cargo build -p repogate-cli --release
      
      - name: Run E2E with mock sessions
        env:
          REPOGATE_MOCK_SESSIONS: "true"
        run: |
          ./target/release/repogate analyze \
            https://github.com/BurntSushi/toml \
            --budget 0.50 \
            --yes \
            --output json \
            --output-file assessment.json
      
      - name: Validate JSON output
        run: |
          jq '.schema_version' assessment.json | grep -q '"1.0"'
      
      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: assessment
          path: assessment.json
```

### Directory: `tests/fixtures/`

- `canned_module_assessment.json` — Sample ModuleAssessment response
- `canned_synthesis_output.json` — Sample SynthesisOutput response
- `canned_risk_output.json` — Sample RiskAnalysisOutput response
- `dev.db` — SQLite fixture (created by P01)

---

## Source Documents to Read

- **`docs/adr/ADR-008-deep-traversal-map-reduce.md`** — Repomix small-repo path (<50k LOC single-session)
- **`docs/adr/ADR-009-multi-phase-pipeline-crash-recovery.md`** — Crash recovery testing
- **`docs/adr/ADR-013-token-budget-enforcement.md`** — Partial results on budget exhaustion

---

## Acceptance Criteria

- ✅ `cargo test -p repogate-orchestrator --test e2e_pipeline` passes (0 live API calls)
- ✅ Crash recovery: resume after crash at module 2 → only modules 3–5 analyzed (3 sessions)
- ✅ Repomix path: <50k LOC manifest → exactly 1 Claude session
- ✅ `repogate analyze <small-repo> --budget 0.50 --yes` with `REPOGATE_MOCK_SESSIONS=true` → exit 0 + JSON
- ✅ All CI jobs pass on clean checkout

---

## Language

**Rust** (tests), **YAML** (workflow).

---

## Out-of-Scope

- Do NOT implement real live API e2e tests (use mocks)
- Do NOT implement performance benchmarking
