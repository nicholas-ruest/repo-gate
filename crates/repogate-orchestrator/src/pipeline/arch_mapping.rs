//! Architecture mapping: deterministic module-boundary detection followed by a
//! Sonnet manifest-summarization pass (ADR-008).

use std::path::Path;

use repogate_core::Layer;
use repogate_ingestion::RepoManifest;
use serde::{Deserialize, Serialize};

use crate::claude::{ClaudeInvocation, ClaudeModel, SessionRunner};
use crate::OrchestratorError;

/// A functional module identified in the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleNode {
    pub id: String,
    pub name: String,
    pub path: String,
    pub layer: Layer,
    pub centrality: f32,
    pub file_count: usize,
    pub loc: usize,
    pub has_public_interface: bool,
}

/// The module dependency graph plus a renderable ASCII diagram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureMap {
    pub modules: Vec<ModuleNode>,
    pub edges: Vec<(String, String)>,
    pub ascii_diagram: String,
}

/// Run the architecture-mapping phase.
///
/// Deterministic heuristics produce candidate modules; a Sonnet session may
/// refine them. If the model output cannot be parsed as modules, the heuristic
/// candidates are used directly, so the phase always yields a valid map.
pub async fn run_architecture_mapping_phase(
    manifest: &RepoManifest,
    repo_path: &Path,
    session_runner: &impl SessionRunner,
) -> Result<ArchitectureMap, OrchestratorError> {
    let candidates = detect_modules_heuristic(manifest, repo_path);

    let prompt = format!(
        "Identify the functional modules and their roles for this repository. \
         Candidate modules: {}. Return a JSON array of ModuleNode objects.",
        candidates
            .iter()
            .map(|m| m.name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let invocation = ClaudeInvocation {
        prompt,
        model: ClaudeModel::Sonnet,
        schema_path: None,
        allowed_tools: vec![],
        system_prompt: None,
        working_dir: Some(repo_path.to_path_buf()),
        session_id: None,
    };

    let modules = match session_runner.run(invocation).await {
        Ok(result) => serde_json::from_str::<Vec<ModuleNode>>(&result.output).unwrap_or(candidates),
        Err(_) => candidates,
    };

    let edges = compute_dependencies(&modules);
    let ascii_diagram = generate_ascii_diagram(&modules, &edges);

    Ok(ArchitectureMap {
        modules,
        edges,
        ascii_diagram,
    })
}

/// Detect candidate modules using deterministic heuristics: Cargo workspace
/// members when present, otherwise top-level source directories.
pub fn detect_modules_heuristic(manifest: &RepoManifest, repo_path: &Path) -> Vec<ModuleNode> {
    let workspace = detect_workspace_members(repo_path);
    if !workspace.is_empty() {
        return workspace;
    }
    detect_top_level_dirs(manifest, repo_path)
}

fn detect_workspace_members(repo_path: &Path) -> Vec<ModuleNode> {
    let cargo_toml = repo_path.join("Cargo.toml");
    let Ok(content) = std::fs::read_to_string(&cargo_toml) else {
        return Vec::new();
    };
    let Ok(value) = content.parse::<toml::Value>() else {
        return Vec::new();
    };

    let members = value
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array());

    let Some(members) = members else {
        return Vec::new();
    };

    members
        .iter()
        .filter_map(|m| m.as_str())
        .map(|member| {
            let name = member.rsplit('/').next().unwrap_or(member).to_string();
            ModuleNode {
                id: name.clone(),
                name,
                path: member.to_string(),
                layer: Layer::Core,
                centrality: 0.5,
                file_count: 0,
                loc: 0,
                has_public_interface: true,
            }
        })
        .collect()
}

fn detect_top_level_dirs(_manifest: &RepoManifest, repo_path: &Path) -> Vec<ModuleNode> {
    const DIRS: &[(&str, Layer)] = &[
        ("src", Layer::Core),
        ("cli", Layer::Cli),
        ("lib", Layer::Core),
        ("tests", Layer::Test),
        ("examples", Layer::Documentation),
        ("docs", Layer::Documentation),
    ];

    DIRS.iter()
        .filter(|(dir, _)| repo_path.join(dir).is_dir())
        .map(|(dir, layer)| ModuleNode {
            id: dir.to_string(),
            name: dir.to_string(),
            path: dir.to_string(),
            layer: *layer,
            centrality: 0.5,
            file_count: 0,
            loc: 0,
            has_public_interface: true,
        })
        .collect()
}

fn compute_dependencies(_modules: &[ModuleNode]) -> Vec<(String, String)> {
    // Detailed import-graph analysis is out of scope for the MVP (ADR-008).
    Vec::new()
}

/// Render a simple text-art tree of the modules.
pub fn generate_ascii_diagram(modules: &[ModuleNode], _edges: &[(String, String)]) -> String {
    let mut out = String::from("repository\n");
    for (i, m) in modules.iter().enumerate() {
        let branch = if i + 1 == modules.len() {
            "└─"
        } else {
            "├─"
        };
        out.push_str(&format!("  {branch} {} ({:?})\n", m.name, m.layer));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::{SessionResult, UsageStats};

    struct MockRunner {
        output: String,
    }

    impl SessionRunner for MockRunner {
        async fn run(
            &self,
            _invocation: ClaudeInvocation,
        ) -> Result<SessionResult, OrchestratorError> {
            Ok(SessionResult {
                session_id: "mock".to_string(),
                output: self.output.clone(),
                usage: UsageStats::default(),
            })
        }
    }

    async fn manifest_for(path: &Path) -> RepoManifest {
        repogate_ingestion::build_manifest("https://example.com/x", path)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn heuristic_top_level_dirs() {
        let dir = tempfile::tempdir().unwrap();
        for sub in ["src", "cli", "tests"] {
            std::fs::create_dir(dir.path().join(sub)).unwrap();
            std::fs::write(dir.path().join(sub).join("f.rs"), "// x\n").unwrap();
        }
        let manifest = manifest_for(dir.path()).await;
        let modules = detect_modules_heuristic(&manifest, dir.path());
        assert_eq!(modules.len(), 3);
    }

    #[tokio::test]
    async fn heuristic_cargo_workspace_members() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/a\", \"crates/b\", \"crates/c\"]\n",
        )
        .unwrap();
        let manifest = manifest_for(dir.path()).await;
        let modules = detect_modules_heuristic(&manifest, dir.path());
        assert_eq!(modules.len(), 3);
        assert!(modules.iter().any(|m| m.name == "a"));
    }

    #[tokio::test]
    async fn phase_parses_model_output() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "// x\n").unwrap();
        let manifest = manifest_for(dir.path()).await;

        let modules = vec![ModuleNode {
            id: "core".to_string(),
            name: "core".to_string(),
            path: "src".to_string(),
            layer: Layer::Core,
            centrality: 0.9,
            file_count: 1,
            loc: 1,
            has_public_interface: true,
        }];
        let runner = MockRunner {
            output: serde_json::to_string(&modules).unwrap(),
        };

        let map = run_architecture_mapping_phase(&manifest, dir.path(), &runner)
            .await
            .unwrap();
        assert_eq!(map.modules.len(), 1);
        assert_eq!(map.modules[0].name, "core");
        // ArchitectureMap serializes to valid JSON.
        let json = serde_json::to_string(&map).unwrap();
        let _: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(!map.ascii_diagram.is_empty());
    }

    #[tokio::test]
    async fn phase_falls_back_to_heuristic_on_bad_output() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "// x\n").unwrap();
        let manifest = manifest_for(dir.path()).await;

        let runner = MockRunner {
            output: "not json".to_string(),
        };
        let map = run_architecture_mapping_phase(&manifest, dir.path(), &runner)
            .await
            .unwrap();
        assert!(!map.modules.is_empty()); // fell back to heuristic candidates
    }
}
