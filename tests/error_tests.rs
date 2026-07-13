use aether::error::codes::ErrorCode;
use aether::error::format::{format_diagnostic, format_json};
use aether::error::recovery::ErrorReporter;
use aether::error::{Diagnostic, Severity, SourceSpan};

#[test]
fn test_diagnostic_formatting() {
    let span = SourceSpan {
        file: "src/main.aether".to_string(),
        line: 1,
        column: 8,
        length: 3,
    };
    let diag = Diagnostic::new(
        ErrorCode::TypeMismatch,
        Severity::Error,
        "type mismatch",
        span,
    );
    let source = "let x: Int = \"hello\";";
    let formatted = format_diagnostic(&diag, source, false);

    assert!(formatted.contains("error[E0200]"));
    assert!(formatted.contains("let x: Int = \"hello\";"));
    assert!(formatted.contains("       ^^^"));
}

#[test]
fn test_diagnostic_json() {
    let span = SourceSpan {
        file: "main.aether".to_string(),
        line: 1,
        column: 1,
        length: 1,
    };
    let diag = Diagnostic::new(
        ErrorCode::InvalidCharacter,
        Severity::Error,
        "invalid character",
        span,
    );
    let json = format_json(&diag);
    assert!(json.contains("\"code\":\"E0001\""));
    assert!(json.contains("\"severity\":\"Error\""));
}

#[test]
fn test_reporter_limit() {
    let mut reporter = ErrorReporter::new(2);
    let span = SourceSpan {
        file: "main.aether".to_string(),
        line: 1,
        column: 1,
        length: 1,
    };
    let d1 = Diagnostic::new(
        ErrorCode::InvalidCharacter,
        Severity::Error,
        "error 1",
        span.clone(),
    );
    let d2 = Diagnostic::new(
        ErrorCode::InvalidCharacter,
        Severity::Error,
        "error 2",
        span.clone(),
    );
    let d3 = Diagnostic::new(
        ErrorCode::InvalidCharacter,
        Severity::Error,
        "error 3",
        span.clone(),
    );

    assert!(reporter.report(d1));
    assert!(reporter.report(d2));
    assert!(!reporter.report(d3)); // Suppressed by limit
    assert_eq!(reporter.diagnostics.len(), 2);
}
