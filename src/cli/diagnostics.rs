use aether::syntax::diagnostics::render_diagnostic_snippet;

/// Formats a parse, compile, or VM runtime error together with a source caret snippet
/// whenever a line (and optional column) location can be extracted from the message.
pub fn format_error_with_source(source: &str, err: &str) -> String {
    if let Some((line, col, headline)) = extract_line_col(err) {
        let mut out = render_diagnostic_snippet(source, line, col.max(1), &headline, None);
        if let Some((_, trace)) = err.split_once("\nStack trace:") {
            out.push_str("\nStack trace:");
            out.push_str(trace);
        }
        return out;
    }
    err.to_string()
}

fn extract_line_col(err: &str) -> Option<(usize, usize, String)> {
    let first_line = err.lines().next()?.trim();

    // Match "[line L:C in fn] message" or "[line L in fn] message"
    if let Some(after_prefix) = first_line.strip_prefix("[line ") {
        if let Some((loc_part, msg_part)) = after_prefix.split_once("] ") {
            let coords = loc_part.split_whitespace().next().unwrap_or("");
            if let Some((l_str, c_str)) = coords.split_once(':') {
                if let (Ok(l), Ok(c)) = (l_str.parse::<usize>(), c_str.parse::<usize>()) {
                    return Some((l, c, msg_part.to_string()));
                }
            } else if let Ok(l) = coords.parse::<usize>() {
                return Some((l, 1, msg_part.to_string()));
            }
        }
    }

    // Match "L:C: message" parser format
    let mut parts = first_line.splitn(3, ':');
    if let (Some(l_str), Some(c_str), Some(msg)) = (parts.next(), parts.next(), parts.next()) {
        if let (Ok(l), Ok(c)) = (l_str.trim().parse::<usize>(), c_str.trim().parse::<usize>()) {
            return Some((l, c, msg.trim().to_string()));
        }
    }

    None
}
