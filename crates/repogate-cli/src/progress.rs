//! Progress reporting to stderr.

/// Reports pipeline phase progress.
pub trait ProgressReporter: Send {
    fn report(&self, phase: &str, message: &str);
}

/// Writes progress lines to stderr as `[phase] message`.
pub struct StderrProgressReporter;

impl ProgressReporter for StderrProgressReporter {
    fn report(&self, phase: &str, message: &str) {
        eprintln!("[{phase}] {message}");
    }
}
