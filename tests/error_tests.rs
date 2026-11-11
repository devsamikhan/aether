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

#[test]
fn test_vm_runtime_error_includes_line_and_stack_trace() {
    let source = r#"
fn inner() {
    let x = 1 / 0
    return x
}

fn outer() {
    return inner()
}

outer()
"#;
    let err = aether::vm::run_source(source).expect_err("Expected division by zero runtime error");
    assert!(
        err.contains("ZeroDivisionError"),
        "Missing error kind: {}",
        err
    );
    assert!(
        err.contains("[line 3:"),
        "Expected line 3 in headline, got: {}",
        err
    );
    assert!(
        err.contains("Stack trace:"),
        "Expected Stack trace section, got: {}",
        err
    );
    assert!(
        err.contains("at inner (line 3:"),
        "Expected inner frame on line 3: {}",
        err
    );
    assert!(
        err.contains("at outer (line 8:"),
        "Expected outer frame on line 8: {}",
        err
    );
    assert!(
        err.contains("at <main> (line 11:"),
        "Expected main frame on line 11: {}",
        err
    );
}

#[test]
fn test_vm_try_catch_still_recovers_without_aborting() {
    let source = r#"
let caught = ""
try {
    let x = 1 / 0
} catch as err {
    caught = str(err)
}
caught
"#;
    let val = aether::vm::run_source(source).expect("try/catch should recover cleanly");
    assert!(
        val.to_string().contains("ZeroDivisionError"),
        "Expected caught ZeroDivisionError, got: {}",
        val
    );
}
