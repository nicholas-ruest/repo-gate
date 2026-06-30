//! Functionality discovery: one Claude session per module, with bounded
//! concurrency, crash-recovery skipping, and budget enforcement (ADR-008).

use std::path::Path;

use repogate_core::{ModuleAssessment, Visibility};
use repogate_ingestion::RepoManifest;
use tokio::sync::Semaphore;

use super::arch_mapping::ArchitectureMap;
use super::llm_adapter::{
    map_to_functionality_items, parse_module_assessment, FunctionalityInventory,
};
use crate::claude::{
    run_structured, select_model, ClaudeInvocation, ClaudeModel, Phase, SessionRunner,
};
use crate::job::{BudgetTracker, ModuleAssessmentStore};
use crate::OrchestratorError;

/// Repositories below this LOC threshold use the single-session (repomix) path
/// instead of per-module fan-out (ADR-008).
pub const REPOMIX_LOC_THRESHOLD: usize = 50_000;

/// Whether a repository is small enough for the single-session repomix path.
pub fn should_use_repomix(manifest: &RepoManifest) -> bool {
    manifest.total_loc < REPOMIX_LOC_THRESHOLD
}

/// Small-repo path: pack the whole repo (repomix when available) and run exactly
/// one Claude session over it. Falls back to a lightweight directory context
/// when repomix is not installed, so it always issues a single session.
pub async fn run_single_session_analysis(
    repo_path: &Path,
    session_runner: &impl SessionRunner,
    job_id: &str,
) -> Result<FunctionalityInventory, OrchestratorError> {
    let context = pack_with_repomix(repo_path)
        .await
        .unwrap_or_else(|| format!("Repository at {}", repo_path.display()));

    let prompt = format!(
        "Analyze this entire repository and identify all capabilities (public, \
         internal, experimental, undocumented, enterprise). Return a \
         ModuleAssessment with module_name \"all\".\n\n{context}"
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

    let result = session_runner.run(invocation).await?;
    let items = match parse_module_assessment(&result.output) {
        Ok(assessment) => map_to_functionality_items(&assessment, ""),
        Err(_) => Vec::new(),
    };

    Ok(FunctionalityInventory {
        repo_id: job_id.to_string(),
        total_count: items.len(),
        hidden_count: 0,
        enterprise_count: 0,
        items,
        api_entry_points: Vec::new(),
        degraded_modules: Vec::new(),
        budget_skipped_modules: Vec::new(),
    })
}

/// Pack a repository into a single text blob with repomix; `None` if repomix is
/// absent or fails.
async fn pack_with_repomix(repo_path: &Path) -> Option<String> {
    let output = tokio::process::Command::new("repomix")
        .arg("--output-format")
        .arg("xml")
        .arg(repo_path)
        .output()
        .await
        .ok()?;
    if output.status.success() {
        String::from_utf8(output.stdout).ok()
    } else {
        None
    }
}

fn discovery_tools() -> Vec<String> {
    vec![
        "Read".to_string(),
        "Glob".to_string(),
        "Bash(grep *)".to_string(),
        "Bash(find *)".to_string(),
    ]
}

/// Run the functionality-discovery fan-out over every module in `arch_map`.
///
/// Each not-yet-assessed module is analyzed by one Claude session (model chosen
/// per ADR-012). A semaphore bounds in-flight work to `max_concurrent`. Already
/// stored assessments are skipped (crash recovery); once the budget is exceeded,
/// no further sessions start and prior results are preserved.
#[allow(clippy::too_many_arguments)]
pub async fn run_feature_discovery_phase(
    arch_map: &ArchitectureMap,
    repo_path: &Path,
    session_runner: &impl SessionRunner,
    module_store: &impl ModuleAssessmentStore,
    budget: &BudgetTracker,
    job_id: &str,
    max_concurrent: usize,
) -> Result<FunctionalityInventory, OrchestratorError> {
    let semaphore = Semaphore::new(max_concurrent.max(1));

    let mut items = Vec::new();
    let mut hidden_count = 0usize;
    let mut enterprise_count = 0usize;
    let mut degraded_modules = Vec::new();
    let mut budget_skipped_modules = Vec::new();

    for module in &arch_map.modules {
        if budget.is_exceeded() {
            // Record every remaining un-stored module as budget-skipped (ADR-016 R4).
            if !module_store
                .exists(job_id, &module.id)
                .await
                .unwrap_or(false)
            {
                tracing::warn!(module = %module.id, "module skipped: token budget exhausted");
                budget_skipped_modules.push(module.id.clone());
            }
            continue;
        }
        if module_store
            .exists(job_id, &module.id)
            .await
            .unwrap_or(false)
        {
            continue;
        }

        let permit = semaphore
            .acquire()
            .await
            .map_err(|e| OrchestratorError::SessionFailed(e.to_string()))?;

        let prompt = format!(
            "Analyze module `{}` at `{}`. Discover all capabilities (public, \
             internal, experimental, undocumented, enterprise), API entry points, \
             CLI commands, and SDK exports. Score the eight commercial value \
             dimensions (0-10 each). Return a ModuleAssessment.",
            module.name, module.path
        );
        let invocation = ClaudeInvocation {
            prompt,
            model: select_model(&module.name, Phase::FeatureDiscovery),
            schema_path: None, // set by run_structured
            allowed_tools: discovery_tools(),
            system_prompt: None,
            working_dir: Some(repo_path.to_path_buf()),
            session_id: None,
        };

        // Schema-enforced with retry-then-surface (ADR-016 R1). A module whose
        // session fails after retry is recorded as degraded, not silently dropped.
        let structured = match run_structured::<ModuleAssessment>(session_runner, invocation).await
        {
            Ok(s) => s,
            Err(_) => {
                tracing::warn!(module = %module.id, "module discovery failed schema validation after retry");
                degraded_modules.push(module.id.clone());
                drop(permit);
                continue;
            }
        };

        budget.record_usage(
            structured.usage.input_tokens,
            structured.usage.output_tokens,
            structured.usage.cache_read_input_tokens,
        );

        let assessment = structured.value;
        for item in map_to_functionality_items(&assessment, &module.path) {
            match item.visibility {
                Visibility::Undocumented => hidden_count += 1,
                Visibility::Enterprise => enterprise_count += 1,
                _ => {}
            }
            items.push(item);
        }

        module_store
            .save(job_id, assessment)
            .await
            .map_err(|e| OrchestratorError::SessionFailed(e.to_string()))?;

        drop(permit);
    }

    Ok(FunctionalityInventory {
        repo_id: job_id.to_string(),
        total_count: items.len(),
        hidden_count,
        enterprise_count,
        items,
        api_entry_points: Vec::new(),
        degraded_modules,
        budget_skipped_modules,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::{ClaudeInvocation, SessionResult, UsageStats};
    use crate::job::InMemoryModuleAssessmentStore;
    use crate::pipeline::arch_mapping::{ArchitectureMap, ModuleNode};
    use repogate_core::{CapabilityFinding, DiscoveryMethod, Layer, ModuleAssessment, TokenBudget};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct MockRunner {
        calls: Arc<AtomicUsize>,
        usage: UsageStats,
    }

    impl SessionRunner for MockRunner {
        async fn run(
            &self,
            _invocation: ClaudeInvocation,
        ) -> Result<SessionResult, OrchestratorError> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            let assessment = ModuleAssessment {
                module_name: format!("m{n}"),
                module_path: "src".to_string(),
                capabilities: vec![CapabilityFinding {
                    name: "sso".to_string(),
                    description: "single sign-on".to_string(),
                    is_enterprise: true,
                    is_undocumented: false,
                    discovery_method: DiscoveryMethod::SourceTracing,
                    source_locations: None,
                }],
                commercial_score: None,
                commercial_value_estimate: None,
                estimated_tier: None,
                risks: vec![],
            };
            Ok(SessionResult {
                session_id: "s".to_string(),
                output: serde_json::to_string(&assessment).unwrap(),
                usage: self.usage.clone(),
            })
        }
    }

    fn node(id: &str) -> ModuleNode {
        ModuleNode {
            id: id.to_string(),
            name: id.to_string(),
            path: format!("src/{id}"),
            layer: Layer::Core,
            centrality: 0.5,
            file_count: 1,
            loc: 10,
            has_public_interface: true,
        }
    }

    fn arch_map(ids: &[&str]) -> ArchitectureMap {
        ArchitectureMap {
            modules: ids.iter().map(|i| node(i)).collect(),
            edges: vec![],
            ascii_diagram: String::new(),
        }
    }

    fn budget(total: u64) -> BudgetTracker {
        BudgetTracker::new(TokenBudget {
            total_limit: total,
            per_phase_limit: total,
            per_session_limit: total,
            warn_threshold: 0.8,
        })
    }

    #[tokio::test]
    async fn maps_enterprise_capability_and_saves() {
        let store = InMemoryModuleAssessmentStore::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let runner = MockRunner {
            calls: calls.clone(),
            usage: UsageStats {
                input_tokens: 10,
                output_tokens: 5,
                cache_read_input_tokens: 0,
            },
        };
        let map = arch_map(&["a", "b"]);
        let inv = run_feature_discovery_phase(
            &map,
            Path::new("."),
            &runner,
            &store,
            &budget(1_000_000),
            "job1",
            4,
        )
        .await
        .unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(inv.enterprise_count, 2);
        assert!(inv
            .items
            .iter()
            .all(|i| matches!(i.visibility, repogate_core::Visibility::Enterprise)));
        assert_eq!(store.load_all("job1").await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn skips_already_assessed_modules() {
        let store = InMemoryModuleAssessmentStore::new();
        // Pre-store an assessment for module "a".
        store
            .save(
                "job1",
                ModuleAssessment {
                    module_name: "a".to_string(),
                    module_path: "src/a".to_string(),
                    capabilities: vec![],
                    commercial_score: None,
                    commercial_value_estimate: None,
                    estimated_tier: None,
                    risks: vec![],
                },
            )
            .await
            .unwrap();

        let calls = Arc::new(AtomicUsize::new(0));
        let runner = MockRunner {
            calls: calls.clone(),
            usage: UsageStats::default(),
        };
        let map = arch_map(&["a", "b"]);
        run_feature_discovery_phase(
            &map,
            Path::new("."),
            &runner,
            &store,
            &budget(1_000_000),
            "job1",
            4,
        )
        .await
        .unwrap();

        // Only module "b" should trigger a session.
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn budget_exhaustion_stops_new_sessions() {
        let store = InMemoryModuleAssessmentStore::new();
        let calls = Arc::new(AtomicUsize::new(0));
        // Each session reports 100 tokens; budget of 50 is exceeded after the first.
        let runner = MockRunner {
            calls: calls.clone(),
            usage: UsageStats {
                input_tokens: 100,
                output_tokens: 0,
                cache_read_input_tokens: 0,
            },
        };
        let map = arch_map(&["a", "b", "c"]);
        run_feature_discovery_phase(
            &map,
            Path::new("."),
            &runner,
            &store,
            &budget(50),
            "job1",
            4,
        )
        .await
        .unwrap();

        // First module runs (then budget exceeds); the rest are skipped.
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(store.load_all("job1").await.unwrap().len(), 1);
    }
}
