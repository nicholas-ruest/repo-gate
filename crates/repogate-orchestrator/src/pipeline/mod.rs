//! Analysis-pipeline phases driven by the orchestrator.

pub mod arch_mapping;
pub mod feature_discovery;
pub mod llm_adapter;
pub mod risk_analysis;
pub mod runner;
pub mod synthesis;

pub use arch_mapping::{
    detect_modules_heuristic, generate_ascii_diagram, run_architecture_mapping_phase,
    ArchitectureMap, ModuleNode,
};
pub use feature_discovery::run_feature_discovery_phase;
pub use llm_adapter::{
    map_to_functionality_items, parse_module_assessment, FunctionalityInventory, FunctionalityItem,
};
pub use risk_analysis::{run_risk_analysis_phase, RiskAnalysisOutput, RiskFinding, RiskProfile};
pub use runner::{PipelineOutput, PipelineRunner};
pub use synthesis::run_synthesis_phase;
