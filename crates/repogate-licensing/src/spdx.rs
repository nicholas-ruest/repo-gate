//! SPDX expression parsing and validation via the `spdx` crate.

use spdx::Expression;

use crate::LicensingError;

/// Parse and validate an SPDX expression, returning a normalized string form.
///
/// Handles compound expressions (`MIT OR Apache-2.0`) and exceptions
/// (`GPL-2.0-only WITH Classpath-exception-2.0`).
pub fn parse_and_normalize(expr_str: &str) -> Result<String, LicensingError> {
    Expression::parse(expr_str).map_err(|e| LicensingError::SpdxParseFailed(e.to_string()))?;
    Ok(expr_str.trim().to_string())
}

/// Extract the base license identifiers from an SPDX expression, dropping the
/// `AND`/`OR`/`WITH` operators and any `WITH` exception, e.g.
/// `GPL-2.0-only WITH Classpath-exception-2.0` → `["GPL-2.0-only"]`.
pub fn extract_base_identifiers(expr_str: &str) -> Result<Vec<String>, LicensingError> {
    Expression::parse(expr_str).map_err(|e| LicensingError::SpdxParseFailed(e.to_string()))?;

    let mut result = Vec::new();
    let mut after_with = false;
    for tok in expr_str
        .split(|c: char| c.is_whitespace() || c == '(' || c == ')')
        .filter(|t| !t.is_empty())
    {
        match tok.to_ascii_uppercase().as_str() {
            "WITH" => after_with = true,
            "AND" | "OR" => {}
            _ if after_with => after_with = false,
            _ => result.push(tok.to_string()),
        }
    }
    Ok(result)
}
