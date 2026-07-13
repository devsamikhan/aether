use super::{Diagnostic, Severity};

/// Formats a Diagnostic structure into a compiler caret pointing block.
pub fn format_diagnostic(diag: &Diagnostic, source_code: &str, use_color: bool) -> String {
    let mut out = String::new();
    let severity_str = match diag.severity {
        Severity::Error => {
            if use_color {
                "\x1b[31merror\x1b[0m"
            } else {
                "error"
            }
        }
        Severity::Warning => {
            if use_color {
                "\x1b[33mwarning\x1b[0m"
            } else {
                "warning"
            }
        }
        Severity::Note => "note",
        Severity::Help => "help",
    };

    out.push_str(&format!(
        "{}[{}]: {}\n",
        severity_str,
        diag.code.as_str(),
        diag.message
    ));
    out.push_str(&format!(
        "  --> {}:{}:{}\n",
        diag.span.file, diag.span.line, diag.span.column
    ));

    let lines: Vec<&str> = source_code.lines().collect();
    if diag.span.line > 0 && diag.span.line <= lines.len() {
        let snippet = lines[diag.span.line - 1];
        out.push_str("   |\n");
        out.push_str(&format!("{:2} | {}\n", diag.span.line, snippet));

        let mut caret_line = String::new();
        // Spans columns
        let start_col = if diag.span.column > 0 {
            diag.span.column - 1
        } else {
            0
        };
        for _ in 0..start_col {
            caret_line.push(' ');
        }
        let length = if diag.span.length > 0 {
            diag.span.length
        } else {
            1
        };
        for _ in 0..length {
            caret_line.push('^');
        }

        let caret_str = if use_color {
            format!("\x1b[31m{}\x1b[0m", caret_line)
        } else {
            caret_line
        };
        out.push_str(&format!("   | {}\n", caret_str));
    }

    for note in &diag.notes {
        out.push_str(&format!("   = note: {}\n", note));
    }
    for sug in &diag.suggestions {
        out.push_str(&format!(
            "   = help: {} (Suggestion: `{}`)\n",
            sug.message, sug.replacement
        ));
    }

    out
}

/// Serializes the diagnostic to a simple JSON string representation.
pub fn format_json(diag: &Diagnostic) -> String {
    format!(
        "{{\"code\":\"{}\",\"severity\":\"{:?}\",\"message\":\"{}\",\"file\":\"{}\",\"line\":{},\"column\":{},\"length\":{}}}",
        diag.code.as_str(),
        diag.severity,
        diag.message.replace('\"', "\\\""),
        diag.span.file,
        diag.span.line,
        diag.span.column,
        diag.span.length
    )
}
