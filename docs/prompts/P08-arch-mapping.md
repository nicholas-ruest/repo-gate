# P08 — `repogate-orchestrator`: Architecture Mapping Phase

## Context

**You are implementing the architecture mapping phase: deterministic module boundary detection and manifest summarization via Claude.**

**Prerequisites:** P07 (state machine) is complete.

---

## Phase & Dependencies

- **Phase:** Analysis pipeline
- **Depends on:** P07

---

## Scope & Deliverables

### File: `src/pipeline/arch_mapping.rs`

```rust
pub struct ModuleNode {
    pub id: String,
    pub name: String,
    pub path: String,
    pub layer: repogate_core::Layer,
    pub centrality: f32,  // 0.0–1.0
    pub file_count: usize,
    pub loc: usize,
    pub has_public_interface: bool,
}

pub struct ArchitectureMap {
    pub modules: Vec<ModuleNode>,
    pub edges: Vec<(String, String)>,  // (from_id, to_id)
    pub ascii_diagram: String,
}

pub async fn run_architecture_mapping_phase(
    manifest: &RepoManifest,
    repo_path: &Path,
    session_runner: impl SessionRunner,
) -> Result<ArchitectureMap, OrchestratorError> {
    // Deterministic heuristics: top-level dirs, Cargo workspaces, npm workspaces, language clusters, size caps
    let mut module_candidates = detect_modules_heuristic(manifest, repo_path);
    
    // Claude manifest summarization (Sonnet)
    let prompt = format!("Analyze module boundaries. Return ModuleNode schema for: {:?}", module_candidates);
    let result = session_runner.run(
        ClaudeInvocation {
            prompt,
            model: ClaudeModel::Sonnet,
            schema_path: Some("module_summary_schema.json".into()),
            allowed_tools: vec![],
            system_prompt: None,
            working_dir: Some(repo_path.to_path_buf()),
            session_id: None,
        }
    ).await?;
    
    let modules: Vec<ModuleNode> = serde_json::from_str(&result.output)?;
    let edges = compute_dependencies(&modules, repo_path)?;
    let ascii_diagram = generate_ascii_diagram(&modules, &edges);
    
    Ok(ArchitectureMap { modules, edges, ascii_diagram })
}

fn detect_modules_heuristic(manifest: &RepoManifest, repo_path: &Path) -> Vec<ModuleNode> {
    let mut candidates = Vec::new();
    
    // Top-level dir grouping
    for dir in &["src", "cli", "lib", "tests", "examples", "docs"] {
        let path = repo_path.join(dir);
        if path.exists() {
            candidates.push(ModuleNode {
                id: dir.to_string(),
                name: dir.to_string(),
                path: dir.to_string(),
                layer: match *dir {
                    "src" => repogate_core::Layer::Core,
                    "cli" => repogate_core::Layer::Cli,
                    "lib" => repogate_core::Layer::Core,
                    "tests" => repogate_core::Layer::Test,
                    "examples" => repogate_core::Layer::Documentation,
                    "docs" => repogate_core::Layer::Documentation,
                    _ => repogate_core::Layer::Core,
                },
                centrality: 0.5,
                file_count: 0,
                loc: 0,
                has_public_interface: true,
            });
        }
    }
    
    candidates
}

fn compute_dependencies(modules: &[ModuleNode], _repo_path: &Path) -> Result<Vec<(String, String)>, OrchestratorError> {
    // Simplified: analyze imports to find edges
    Ok(vec![])
}

fn generate_ascii_diagram(modules: &[ModuleNode], _edges: &[(String, String)]) -> String {
    // Generate simple text-art tree
    modules.iter()
        .map(|m| format!("  ├─ {}", m.name))
        .collect::<Vec<_>>()
        .join("\n")
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-008-deep-traversal-map-reduce.md`** — Boundary heuristics, Sonnet summarization
- **`docs/ddd/architecture-mapping.md`** — ModuleNode, DependencyEdge, Layer, Centrality

---

## Acceptance Criteria

- ✅ Heuristic: repo with `src/`, `cli/`, `tests/` → 3 module candidates
- ✅ Cargo workspace with 3 members → 3 modules
- ✅ ArchitectureMap serializes to valid JSON
- ✅ ASCII diagram generates without panic
- ✅ `cargo test -p repogate-orchestrator` passes

---

## Language

**Rust** — Heuristics, Claude integration, graph generation.

---

## Out-of-Scope

- Do NOT implement complex dependency graph analysis
- Do NOT call live Claude; use mock SessionRunner in tests
