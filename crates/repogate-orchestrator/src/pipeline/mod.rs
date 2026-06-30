//! Analysis-pipeline phases driven by the orchestrator.

pub mod arch_mapping;

pub use arch_mapping::{
    detect_modules_heuristic, generate_ascii_diagram, run_architecture_mapping_phase,
    ArchitectureMap, ModuleNode,
};
