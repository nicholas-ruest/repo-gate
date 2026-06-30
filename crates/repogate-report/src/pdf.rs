//! Optional PDF rendering via the `pandoc` subprocess (ADR-011).

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::ReportError;

/// Render Markdown to a PDF at `output_path` using `pandoc`.
///
/// Returns [`ReportError::PandocNotFound`] (not a panic) when `pandoc` is not
/// installed; PDF output is opt-in.
pub fn render_pdf(markdown: &str, output_path: &Path) -> Result<(), ReportError> {
    let mut child = Command::new("pandoc")
        .arg("-f")
        .arg("markdown")
        .arg("-t")
        .arg("pdf")
        .arg("-o")
        .arg(output_path)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ReportError::PandocNotFound
            } else {
                ReportError::PandocError(e.to_string())
            }
        })?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| ReportError::PandocError("no stdin".to_string()))?;
        stdin.write_all(markdown.as_bytes())?;
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(ReportError::PandocError(format!(
            "pandoc exited with status {}",
            status.code().unwrap_or(-1)
        )));
    }
    Ok(())
}
