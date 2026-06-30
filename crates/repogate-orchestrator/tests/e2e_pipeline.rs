//! End-to-end pipeline integration tests using mock Claude sessions (no live API).

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use repogate_core::{
    CapabilityFinding, DiscoveryMethod, ModuleAssessment, ScoreWeights, TokenBudget,
};
use repogate_orchestrator::claude::{ClaudeInvocation, SessionResult, SessionRunner, UsageStats};
use repogate_orchestrator::job::{
    phases_to_run, BudgetTracker, InMemoryModuleAssessmentStore, JobCheckpoint,
    ModuleAssessmentStore, PhaseKind,
};
use repogate_orchestrator::pipeline::{
    run_feature_discovery_phase, run_single_session_analysis, should_use_repomix, ArchitectureMap,
    ModuleNode, PipelineRunner,
};
use repogate_orchestrator::OrchestratorError;

/// Mock runner returning a fixed canned output and counting invocations.
struct MockRunner {
    output: String,
    calls: Arc<AtomicUsize>,
}

impl MockRunner {
    fn new(output: &str) -> Self {
        Self {
            output: output.to_string(),
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl SessionRunner for MockRunner {
    async fn run(&self, _invocation: ClaudeInvocation) -> Result<SessionResult, OrchestratorError> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        // Vary module_name so per-module saves don't collide.
        let assessment = ModuleAssessment {
            module_name: format!("m{n}"),
            module_path: "src".to_string(),
            capabilities: vec![CapabilityFinding {
                name: "feature".to_string(),
                description: "a capability".to_string(),
                is_enterprise: false,
                is_undocumented: false,
                discovery_method: DiscoveryMethod::SourceTracing,
                source_locations: None,
            }],
            commercial_value_estimate: Some(6.0),
            estimated_tier: None,
            risks: vec![],
        };
        let output = if self.output.is_empty() {
            serde_json::to_string(&assessment).unwrap()
        } else {
            self.output.clone()
        };
        Ok(SessionResult {
            session_id: "mock".to_string(),
            output,
            usage: UsageStats::default(),
        })
    }
}

fn budget() -> TokenBudget {
    TokenBudget {
        total_limit: 20_000_000,
        per_phase_limit: 20_000_000,
        per_session_limit: 20_000_000,
        warn_threshold: 0.8,
    }
}

fn arch_map(ids: &[&str]) -> ArchitectureMap {
    ArchitectureMap {
        modules: ids
            .iter()
            .map(|id| ModuleNode {
                id: id.to_string(),
                name: id.to_string(),
                path: format!("src/{id}"),
                layer: repogate_core::Layer::Core,
                centrality: 0.5,
                file_count: 1,
                loc: 10,
                has_public_interface: true,
            })
            .collect(),
        edges: vec![],
        ascii_diagram: String::new(),
    }
}

#[tokio::test]
async fn full_pipeline_with_mock_completes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/lib.rs"), "fn x() {}\n").unwrap();

    let pipeline = PipelineRunner::new(MockRunner::new("{}"), budget());
    let output = pipeline
        .run(
            "https://github.com/example/test-repo",
            dir.path(),
            &ScoreWeights::default(),
        )
        .await
        .expect("pipeline should complete");

    assert!(output.is_complete);
    assert!(!output.arch_map.modules.is_empty());
    assert!(!output.valuation.module_scores.is_empty());
    assert!(!output.strategy.tier_assignments.is_empty());
}

#[test]
fn crash_recovery_resumes_after_feature_discovery() {
    let checkpoint = JobCheckpoint {
        job_id: "test-job".to_string(),
        last_completed_phase: Some(PhaseKind::FeatureDiscovery),
        completed_module_ids: vec!["mod1".to_string(), "mod2".to_string()],
        token_usage_so_far: 500_000,
        partial_results: serde_json::json!({}),
        saved_at: "2026-06-30T00:00:00Z".to_string(),
    };

    let remaining = phases_to_run(&checkpoint);
    assert!(remaining.contains(&PhaseKind::Scoring));
    assert!(!remaining.contains(&PhaseKind::FeatureDiscovery));
}

#[tokio::test]
async fn crash_recovery_reanalyzes_only_remaining_modules() {
    // Five modules; the first two already have stored assessments (crash after
    // module 2). Discovery should issue exactly 3 sessions.
    let store = InMemoryModuleAssessmentStore::new();
    for id in ["mod1", "mod2"] {
        store
            .save(
                "job1",
                ModuleAssessment {
                    module_name: id.to_string(),
                    module_path: format!("src/{id}"),
                    capabilities: vec![],
                    commercial_value_estimate: None,
                    estimated_tier: None,
                    risks: vec![],
                },
            )
            .await
            .unwrap();
    }

    let runner = MockRunner::new("");
    let calls = runner.calls.clone();
    let map = arch_map(&["mod1", "mod2", "mod3", "mod4", "mod5"]);
    let budget = BudgetTracker::new(budget());

    run_feature_discovery_phase(&map, Path::new("."), &runner, &store, &budget, "job1", 4)
        .await
        .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn repomix_path_runs_exactly_one_session() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/lib.rs"), "fn x() {}\n").unwrap();

    let runner = MockRunner::new("");
    let calls = runner.calls.clone();
    let inv = run_single_session_analysis(dir.path(), &runner, "job1")
        .await
        .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(inv.repo_id, "job1");
}

#[tokio::test]
async fn small_repo_uses_repomix_threshold() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/lib.rs"), "fn x() {}\n").unwrap();
    let manifest = repogate_ingestion::build_manifest("https://example.com/x", dir.path())
        .await
        .unwrap();
    assert!(should_use_repomix(&manifest));
}
