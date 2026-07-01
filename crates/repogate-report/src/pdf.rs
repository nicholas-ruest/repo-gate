//! Optional PDF rendering (ADR-011).
//!
//! The report Markdown is converted to styled HTML in-process (via
//! `pulldown-cmark`) and then rendered to PDF by the `weasyprint` subprocess.
//! WeasyPrint renders HTML/CSS directly, so PDF output needs only its runtime —
//! no LaTeX toolchain.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use pulldown_cmark::{html, Options, Parser};

use crate::ReportError;

/// Render Markdown to a PDF at `output_path` using WeasyPrint.
///
/// Returns [`ReportError::PdfEngineNotFound`] (not a panic) when `weasyprint` is
/// not installed; PDF output is opt-in.
pub fn render_pdf(markdown: &str, output_path: &Path) -> Result<(), ReportError> {
    let html_doc = markdown_to_html_doc(markdown);

    // `weasyprint - <output>` reads the HTML document from stdin.
    let mut child = Command::new("weasyprint")
        .arg("-")
        .arg(output_path)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ReportError::PdfEngineNotFound
            } else {
                ReportError::PdfEngineError(e.to_string())
            }
        })?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| ReportError::PdfEngineError("no stdin".to_string()))?;
        stdin.write_all(html_doc.as_bytes())?;
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(ReportError::PdfEngineError(format!(
            "weasyprint exited with status {}",
            status.code().unwrap_or(-1)
        )));
    }
    Ok(())
}

/// Convert report Markdown into a self-contained HTML document with a print
/// stylesheet suitable for WeasyPrint.
fn markdown_to_html_doc(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let mut body = String::new();
    html::push_html(&mut body, Parser::new_ext(markdown, options));

    format!("<!doctype html><html><head><meta charset=\"utf-8\"><style>{STYLE}</style></head><body>{body}</body></html>")
}

/// Print stylesheet for the PDF. Uses only fonts WeasyPrint can resolve without
/// extra configuration.
const STYLE: &str = "\
@page { size: A4; margin: 18mm 16mm; }
body { font-family: 'DejaVu Sans', Arial, sans-serif; color: #1e1b2e; line-height: 1.55; font-size: 11pt; }
h1 { font-size: 22pt; color: #6d28d9; border-bottom: 3px solid #8b5cf6; padding-bottom: 6px; }
h2 { font-size: 15pt; color: #5b21b6; border-left: 4px solid #8b5cf6; padding-left: 10px; margin-top: 22px; }
h3 { font-size: 12pt; color: #4c1d95; margin-top: 16px; }
code { background: #f3f0fb; color: #6d28d9; padding: 1px 5px; border-radius: 4px; font-family: 'DejaVu Sans Mono', monospace; font-size: 9.5pt; }
table { border-collapse: collapse; width: 100%; margin: 10px 0; }
th, td { border: 1px solid #ddd6f3; padding: 5px 9px; text-align: left; font-size: 10pt; }
th { background: #f3f0fb; color: #4c1d95; }
ul { padding-left: 20px; }
";
