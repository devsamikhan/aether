use super::{Diagnostic, Severity};

/// ErrorReporter collecting diagnostics during compiler execution phases, preventing spam.
pub struct ErrorReporter {
    pub diagnostics: Vec<Diagnostic>,
    pub max_errors: usize,
}

impl ErrorReporter {
    pub fn new(max_errors: usize) -> Self {
        Self {
            diagnostics: Vec::new(),
            max_errors,
        }
    }

    /// Report a diagnostic. Returns true if recorded, false if suppressed by limits.
    pub fn report(&mut self, diag: Diagnostic) -> bool {
        if self.diagnostics.len() >= self.max_errors {
            return false;
        }
        self.diagnostics.push(diag);
        true
    }

    /// Verify if any diagnostic reported is of Severity::Error.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}
