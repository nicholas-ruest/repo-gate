//! Copyleft classification and risk scoring (ADR-006 risk matrix).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Copyleft exposure tier for a license.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum CopyleftTier {
    /// GPL / AGPL family — strong, viral copyleft.
    StrongCopyleft,
    /// LGPL / MPL / EUPL — file- or library-level copyleft.
    WeakCopyleft,
    /// BSL / SSPL / Elastic — source-available, not OSI-approved.
    SourceAvailableNonOsi,
    /// MIT / Apache / BSD / ISC — permissive.
    Permissive,
    /// Public-domain dedications.
    PublicDomain,
    /// No SPDX match.
    Unknown,
}

/// Classify an SPDX license identifier into a [`CopyleftTier`].
pub fn classify_license(spdx_id: &str) -> CopyleftTier {
    // Normalize common `-only` / `-or-later` suffixes to the base id.
    let base = spdx_id
        .trim()
        .trim_end_matches("-or-later")
        .trim_end_matches("-only");

    match base {
        "AGPL-3.0" | "GPL-3.0" | "GPL-2.0" => CopyleftTier::StrongCopyleft,
        "LGPL-2.1" | "LGPL-3.0" | "MPL-2.0" | "EUPL-1.2" => CopyleftTier::WeakCopyleft,
        "BSL-1.1" | "SSPL-1.0" | "Elastic-2.0" => CopyleftTier::SourceAvailableNonOsi,
        "MIT" | "Apache-2.0" | "BSD-2-Clause" | "BSD-3-Clause" | "ISC" => CopyleftTier::Permissive,
        "Unlicense" | "CC0-1.0" => CopyleftTier::PublicDomain,
        _ => CopyleftTier::Unknown,
    }
}

/// Map a [`CopyleftTier`] to a 0.0–10.0 risk score for the commercial-gating model.
pub fn copyleft_risk_score(tier: CopyleftTier) -> f32 {
    match tier {
        CopyleftTier::StrongCopyleft => 9.0,
        CopyleftTier::WeakCopyleft => 4.0,
        CopyleftTier::SourceAvailableNonOsi => 3.0,
        CopyleftTier::Permissive => 0.0,
        CopyleftTier::PublicDomain => 0.0,
        CopyleftTier::Unknown => 2.0,
    }
}
