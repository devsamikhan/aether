// ==============================================================================
// AETHER Rich Compiler Diagnostics ("Did You Mean?" Fuzzy Hints, Source Caret
// Snippets), Canonical Code Formatter (`Formatter`), & Static Linter (`Linter`)
// Pure Rust Standard Library — Zero External Crates
// ==============================================================================

use std::collections::HashSet;
use super::ast::*;

/// Computes Levenshtein edit distance between two strings.
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0usize; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = usize::from(a_chars[i - 1] != b_chars[j - 1]);
            curr[j] = (prev[j] + 1)
                .min(curr[j - 1] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

/// Finds the closest matching identifier from `candidates` within a small edit distance.
pub fn suggest_closest_symbol<'a, I>(target: &str, candidates: I) -> Option<String>
where
    I: IntoIterator<Item = &'a str>,
{
    if target.is_empty() {
        return None;
    }
    let max_dist = match target.len() {
        1..=2 => 1,
        3..=5 => 2,
        _ => 3,
    };

    let mut best: Option<(usize, String)> = None;
    for cand in candidates {
        if cand == target || cand.starts_with("__") {
            continue;
        }
        let dist = levenshtein_distance(target, cand);
        if dist <= max_dist {
            match &best {
                Some((best_d, best_s)) => {
                    if dist < *best_d || (dist == *best_d && cand < best_s.as_str()) {
                        best = Some((dist, cand.to_string()));
                    }
                }
                None => best = Some((dist, cand.to_string())),
            }
        }
    }

    best.map(|(_, s)| s)
}

/// Renders a Rust/Elm-style diagnostic snippet with source line, caret underline, and hint.
pub fn render_diagnostic_snippet(
    source: &str,
    line: usize,
    col: usize,
    message: &str,
    hint: Option<&str>,
) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let line_idx = line.saturating_sub(1);
    let mut out = format!("error: {}\n", message);
    out.push_str(&format!("  --> line {}:{}\n", line.max(1), col.max(1)));
    out.push_str("   |\n");

    if line_idx < lines.len() {
        let src_line = lines[line_idx];
        out.push_str(&format!("{:>2} | {}\n", line.max(1), src_line));
        let pad = " ".repeat(col.saturating_sub(1));
        out.push_str(&format!("   | {}^^^\n", pad));
    }

    if let Some(h) = hint {
        out.push_str(&format!("   = help: {}\n", h));
    }
    out
}

// ==============================================================================
// Built-in Static Linter (`Linter.lint`)
// ==============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintWarning {
    pub code: String,
    pub message: String,
    pub line: usize,
}

pub fn lint_source(source: &str) -> Result<Vec<LintWarning>, String> {
    let program = super::parse(source).map_err(|(e, sp)| format!("{}:{}: {}", sp.line, sp.col, e))?;
    Ok(lint_program(&program))
}

pub fn lint_program(program: &Program) -> Vec<LintWarning> {
    let mut warnings = Vec::new();
    let mut declared: Vec<(String, usize)> = Vec::new();
    let mut used: HashSet<String> = HashSet::new();
    let mut outer_scopes: HashSet<String> = HashSet::new();

    lint_statement_list(
        &program.statements,
        &mut declared,
        &mut used,
        &mut outer_scopes,
        &mut warnings,
    );

    for (name, line) in declared {
        if !name.starts_with('_') && !used.contains(&name) {
            warnings.push(LintWarning {
                code: "W001_UNUSED_VAR".to_string(),
                message: format!("Variable '{}' is declared but never used (prefix with '_' to silence)", name),
                line,
            });
        }
    }

    warnings
}

fn lint_statement_list(
    stmts: &[Statement],
    declared: &mut Vec<(String, usize)>,
    used: &mut HashSet<String>,
    outer_scopes: &mut HashSet<String>,
    warnings: &mut Vec<LintWarning>,
) {
    let mut terminated = false;
    for stmt in stmts {
        if terminated {
            warnings.push(LintWarning {
                code: "W002_UNREACHABLE_CODE".to_string(),
                message: "Unreachable statement after control-flow terminator".to_string(),
                line: stmt.span().line,
            });
            break;
        }

        match stmt {
            Statement::Let {
                name,
                initializer,
                span,
                ..
            } => {
                if let Some(init) = initializer {
                    collect_used_in_expr(init, used);
                }
                if outer_scopes.contains(name) && !name.starts_with('_') {
                    warnings.push(LintWarning {
                        code: "W003_SHADOWED_VAR".to_string(),
                        message: format!("Variable '{}' shadows an existing binding in an outer scope", name),
                        line: span.line,
                    });
                }
                declared.push((name.clone(), span.line));
            }
            Statement::LetDestructure { initializer, .. } => {
                collect_used_in_expr(initializer, used);
            }
            Statement::Assign { target, value, .. } => {
                collect_used_in_expr(target, used);
                collect_used_in_expr(value, used);
            }
            Statement::FnDef {
                name,
                params,
                body,
                ..
            } => {
                used.insert(name.clone());
                let mut fn_outer = outer_scopes.clone();
                for (n, _) in declared.iter() {
                    fn_outer.insert(n.clone());
                }
                for p in params {
                    fn_outer.insert(p.name.clone());
                }
                lint_statement_list(&body.statements, declared, used, &mut fn_outer, warnings);
                if let Some(ref res) = body.result {
                    collect_used_in_expr(res, used);
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                collect_used_in_expr(condition, used);
                lint_statement_list(&body.statements, declared, used, outer_scopes, warnings);
            }
            Statement::For { iter, body, .. } => {
                collect_used_in_expr(iter, used);
                lint_statement_list(&body.statements, declared, used, outer_scopes, warnings);
            }
            Statement::TryCatch {
                try_block,
                handlers,
                finally_block,
                ..
            } => {
                lint_statement_list(&try_block.statements, declared, used, outer_scopes, warnings);
                for h in handlers {
                    let is_empty = h.body.statements.is_empty()
                        || h.body
                            .statements
                            .iter()
                            .all(|s| matches!(s, Statement::Pass(_)));
                    if is_empty {
                        warnings.push(LintWarning {
                            code: "W004_EMPTY_CATCH".to_string(),
                            message: "Empty catch/except block silently swallows exceptions".to_string(),
                            line: h.span.line,
                        });
                    }
                    lint_statement_list(&h.body.statements, declared, used, outer_scopes, warnings);
                }
                if let Some(fb) = finally_block {
                    lint_statement_list(&fb.statements, declared, used, outer_scopes, warnings);
                }
            }
            Statement::Return { value, .. } => {
                if let Some(v) = value {
                    collect_used_in_expr(v, used);
                }
                terminated = true;
            }
            Statement::Break(_) | Statement::Continue(_) => {
                terminated = true;
            }
            Statement::Raise { expr, .. } => {
                if let Some(e) = expr {
                    collect_used_in_expr(e, used);
                }
                terminated = true;
            }
            Statement::Yield { value: Some(v), .. } | Statement::Defer { expr: v, .. } => {
                collect_used_in_expr(v, used);
            }
            Statement::Expr(e) => collect_used_in_expr(e, used),
            _ => {}
        }
    }
}

fn collect_used_in_expr(expr: &Expr, used: &mut HashSet<String>) {
    match expr {
        Expr::Ident(name, _) => {
            used.insert(name.clone());
        }
        Expr::Unary(_, inner, _)
        | Expr::Try(inner, _)
        | Expr::OptionalMember(inner, _, _)
        | Expr::Member(inner, _, _) => collect_used_in_expr(inner, used),
        Expr::Binary(l, _, r, _)
        | Expr::Pipe(l, r, _)
        | Expr::Index(l, r, _)
        | Expr::OptionalIndex(l, r, _) => {
            collect_used_in_expr(l, used);
            collect_used_in_expr(r, used);
        }
        Expr::Call(callee, args, _) => {
            collect_used_in_expr(callee, used);
            for a in args {
                collect_used_in_expr(a, used);
            }
        }
        Expr::Array(items, _) | Expr::Tuple(items, _) | Expr::Set(items, _) | Expr::FString(items, _) => {
            for item in items {
                collect_used_in_expr(item, used);
            }
        }
        Expr::Map(entries, _) => {
            for (k, v) in entries {
                collect_used_in_expr(k, used);
                collect_used_in_expr(v, used);
            }
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            collect_used_in_expr(condition, used);
            for s in &then_branch.statements {
                if let Statement::Expr(e) = s {
                    collect_used_in_expr(e, used);
                }
            }
            if let Some(ref r) = then_branch.result {
                collect_used_in_expr(r, used);
            }
            if let Some(eb) = else_branch {
                for s in &eb.statements {
                    if let Statement::Expr(e) = s {
                        collect_used_in_expr(e, used);
                    }
                }
                if let Some(ref r) = eb.result {
                    collect_used_in_expr(r, used);
                }
            }
        }
        Expr::Lambda(_, body, _) => {
            for s in &body.statements {
                if let Statement::Expr(e) = s {
                    collect_used_in_expr(e, used);
                }
            }
            if let Some(ref r) = body.result {
                collect_used_in_expr(r, used);
            }
        }
        Expr::Block(body, _) | Expr::Comptime(body, _) => {
            for s in &body.statements {
                if let Statement::Expr(e) = s {
                    collect_used_in_expr(e, used);
                }
            }
            if let Some(ref r) = body.result {
                collect_used_in_expr(r, used);
            }
        }
        Expr::Await(inner, _) => collect_used_in_expr(inner, used),
        _ => {}
    }
}

// ==============================================================================
// Built-in Canonical Code Formatter (`Formatter.format`)
// ==============================================================================

pub fn format_source(source: &str) -> String {
    let mut out = String::new();
    let mut indent_level = 0usize;

    for raw_line in source.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            out.push('\n');
            continue;
        }

        if trimmed.starts_with('}') || trimmed.starts_with(']') || trimmed.starts_with(')') {
            indent_level = indent_level.saturating_sub(1);
        }

        let indent = "    ".repeat(indent_level);
        let normalized = normalize_line_spacing(trimmed);
        out.push_str(&indent);
        out.push_str(&normalized);
        out.push('\n');

        if trimmed.ends_with('{') {
            indent_level += 1;
        }
    }

    out
}

fn normalize_line_spacing(line: &str) -> String {
    if line.starts_with('#') {
        return line.to_string();
    }
    let mut out = String::with_capacity(line.len() + 8);
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut in_str: Option<char> = None;

    while i < chars.len() {
        let c = chars[i];
        if let Some(q) = in_str {
            out.push(c);
            if c == '\\' && i + 1 < chars.len() {
                i += 1;
                out.push(chars[i]);
            } else if c == q {
                in_str = None;
            }
            i += 1;
            continue;
        }
        if c == '"' || c == '\'' {
            in_str = Some(c);
            out.push(c);
            i += 1;
            continue;
        }
        if c == ',' {
            out.push(',');
            if i + 1 < chars.len() && chars[i + 1] != ' ' {
                out.push(' ');
            }
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }

    // Collapse multiple consecutive spaces outside strings
    out
}
