pub mod codes;
pub mod format;
pub mod recovery;

use codes::ErrorCode;

/// Severity classification of compiler diagnostic alerts.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

/// Character text range source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

/// Suggestion correction text block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub replacement: String,
    pub message: String,
}

/// Unified compiler diagnostic report structure.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: ErrorCode,
    pub severity: Severity,
    pub message: String,
    pub span: SourceSpan,
    pub notes: Vec<String>,
    pub suggestions: Vec<Suggestion>,
}

impl Diagnostic {
    pub fn new(code: ErrorCode, severity: Severity, message: &str, span: SourceSpan) -> Self {
        Self {
            code,
            severity,
            message: message.to_string(),
            span,
            notes: Vec::new(),
            suggestions: Vec::new(),
        }
    }
}
