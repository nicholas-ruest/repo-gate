//! Adapts Claude `ModuleAssessment` output into domain functionality items.

use repogate_core::{DiscoveryMethod, ModuleAssessment, SourceLocation, Visibility};
use serde::{Deserialize, Serialize};

use crate::OrchestratorError;

/// A single discovered capability, normalized for the functionality inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionalityItem {
    pub name: String,
    pub description: String,
    pub visibility: Visibility,
    pub source_location: Option<SourceLocation>,
    pub discovery_method: DiscoveryMethod,
    /// True when the finding is backed by a concrete source location.
    pub is_confirmed: bool,
}

/// The repository-wide inventory of discovered functionality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionalityInventory {
    pub repo_id: String,
    pub items: Vec<FunctionalityItem>,
    pub total_count: usize,
    pub hidden_count: usize,
    pub enterprise_count: usize,
    pub api_entry_points: Vec<String>,
}

/// Parse a schema-enforced `ModuleAssessment` from raw model output.
pub fn parse_module_assessment(raw: &str) -> Result<ModuleAssessment, OrchestratorError> {
    serde_json::from_str(raw)
        .map_err(|e| OrchestratorError::SchemaViolation(format!("module assessment: {e}")))
}

/// Map a module assessment's capabilities into [`FunctionalityItem`]s.
pub fn map_to_functionality_items(
    assessment: &ModuleAssessment,
    _module_path: &str,
) -> Vec<FunctionalityItem> {
    assessment
        .capabilities
        .iter()
        .map(|cap| {
            let visibility = if cap.is_enterprise {
                Visibility::Enterprise
            } else if cap.is_undocumented {
                Visibility::Undocumented
            } else {
                Visibility::Public
            };
            let source_location = cap
                .source_locations
                .as_ref()
                .and_then(|locs| locs.first().cloned());
            FunctionalityItem {
                name: cap.name.clone(),
                description: cap.description.clone(),
                visibility,
                is_confirmed: source_location.is_some(),
                source_location,
                discovery_method: cap.discovery_method.clone(),
            }
        })
        .collect()
}
