//! Aggregate language statistics over a set of [`FileEntry`] values.

use std::collections::HashMap;

use repogate_core::Language;
use serde::{Deserialize, Serialize};

use crate::walk::FileEntry;

/// Lines of code per detected language across the repository.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageStats {
    pub language_counts: HashMap<Language, usize>,
}

impl LanguageStats {
    /// Total lines of code across all languages.
    pub fn total_loc(&self) -> usize {
        self.language_counts.values().sum()
    }
}

/// Aggregate lines-of-code per language from the walked file entries.
///
/// Binary and unclassified files contribute nothing.
pub fn compute_language_stats(entries: &[FileEntry]) -> LanguageStats {
    let mut counts: HashMap<Language, usize> = HashMap::new();

    for entry in entries {
        if entry.is_binary || entry.is_generated {
            continue;
        }
        if let Some(lang) = entry.language.clone() {
            *counts.entry(lang).or_insert(0) += entry.loc;
        }
    }

    LanguageStats {
        language_counts: counts,
    }
}
