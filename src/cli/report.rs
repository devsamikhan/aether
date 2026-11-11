use super::diagnostics::format_error_with_source;
use aether::syntax::ast::Statement;
use aether::vm::{TestRecord, Value};
use std::collections::{BTreeMap, HashSet};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ScriptMetrics {
    pub source_lines: usize,
    pub statements: usize,
    pub functions: usize,
    pub classes: usize,
    pub structs: usize,
    pub enums: usize,
    pub imports: usize,
}

#[derive(Debug, Clone)]
pub struct StructuredTable {
    pub name: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct ReportContext {
    pub file_path: String,
    pub elapsed_ms: f64,
    pub metrics: ScriptMetrics,
    pub return_value: String,
    pub user_globals: BTreeMap<String, String>,
    pub structured_tables: Vec<StructuredTable>,
    pub tests: Vec<TestRecord>,
}

/// Executes an AETHER script, collects real VM state & AST metrics, and writes a dynamic HTML report.
pub fn run_report_command(file_path: &str, out_html: Option<&str>) -> Result<(), String> {
    let source = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read '{}': {}", file_path, e))?;

    let target_path = out_html
        .map(str::to_string)
        .unwrap_or_else(|| default_report_path(file_path));

    println!("Generating dynamic HTML report for '{}'...", file_path);
    let ctx = execute_and_collect(file_path, &source)?;
    let html = render_dynamic_html_report(&source, &ctx);

    std::fs::write(&target_path, html)
        .map_err(|e| format!("Failed to write HTML report to '{}': {}", target_path, e))?;
    println!("Report generated successfully: {}", target_path);
    Ok(())
}

fn default_report_path(file_path: &str) -> String {
    if let Some(stem) = file_path.strip_suffix(".ae") {
        format!("{}.html", stem)
    } else if let Some(stem) = file_path.strip_suffix(".aether") {
        format!("{}.html", stem)
    } else {
        format!("{}.html", file_path)
    }
}

pub fn execute_and_collect(file_path: &str, source: &str) -> Result<ReportContext, String> {
    let raw_prog = aether::syntax::parse(source).map_err(|(e, span)| {
        aether::syntax::diagnostics::render_diagnostic_snippet(
            source, span.line, span.col, &e, None,
        )
    })?;

    let metrics = collect_ast_metrics(source, &raw_prog.statements);
    let (prog, _) = aether::vm::optimizer::optimize_program(&raw_prog);
    let compiler = aether::vm::BytecodeCompiler::new("<main>", 0);
    let compiled_fn = compiler.compile(&prog)?;

    let mut vm = aether::vm::VM::new();
    vm.test_runner_mode = true;
    let builtin_keys: HashSet<String> = vm.globals.keys().cloned().collect();

    let start = Instant::now();
    let result_val = vm
        .interpret(compiled_fn)
        .map_err(|e| format_error_with_source(source, &e))?;
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

    let mut user_globals = BTreeMap::new();
    let mut structured_tables = Vec::new();

    for (k, v) in &vm.globals {
        if builtin_keys.contains(k) || k.starts_with("__") {
            continue;
        }
        if matches!(
            v,
            Value::Function(_) | Value::Closure { .. } | Value::Native(_, _)
        ) {
            continue;
        }
        if let Some(table) = extract_table_from_value(k, v) {
            structured_tables.push(table);
        }
        user_globals.insert(k.clone(), sanitize_value_preview(v));
    }
    structured_tables.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(ReportContext {
        file_path: file_path.to_string(),
        elapsed_ms,
        metrics,
        return_value: sanitize_value_preview(&result_val),
        user_globals,
        structured_tables,
        tests: vm.test_results,
    })
}

fn collect_ast_metrics(source: &str, stmts: &[Statement]) -> ScriptMetrics {
    let mut m = ScriptMetrics {
        source_lines: source.lines().count(),
        statements: stmts.len(),
        functions: 0,
        classes: 0,
        structs: 0,
        enums: 0,
        imports: 0,
    };
    for stmt in stmts {
        count_stmt(stmt, &mut m);
    }
    m
}

fn count_stmt(stmt: &Statement, m: &mut ScriptMetrics) {
    match stmt {
        Statement::FnDef { .. } | Statement::IntentDef { .. } => m.functions += 1,
        Statement::ClassDef { .. } => m.classes += 1,
        Statement::StructDef { .. } => m.structs += 1,
        Statement::EnumDef { .. } => m.enums += 1,
        Statement::Import { .. } | Statement::FromImport { .. } => m.imports += 1,
        Statement::Decorated { stmt: inner, .. } => count_stmt(inner, m),
        _ => {}
    }
}

fn sanitize_value_preview(val: &Value) -> String {
    match val {
        Value::Secret(_) => "***REDACTED***".to_string(),
        other => {
            let s = format!("{}", other);
            if s.len() > 280 {
                format!("{}...", &s[..277])
            } else {
                s
            }
        }
    }
}

fn extract_table_from_value(name: &str, val: &Value) -> Option<StructuredTable> {
    match val {
        Value::Array(arr) => {
            let guard = arr.lock();
            if guard.is_empty() {
                return None;
            }
            let mut col_set = Vec::new();
            let mut map_rows = Vec::new();
            for item in guard.iter().take(100) {
                if let Value::Map(m) = item {
                    let row_guard = m.lock();
                    for k in row_guard.keys() {
                        if !col_set.contains(k) {
                            col_set.push(k.clone());
                        }
                    }
                    map_rows.push(row_guard.clone());
                } else {
                    return None;
                }
            }
            col_set.sort();
            if col_set.is_empty() {
                return None;
            }
            let rows = map_rows
                .into_iter()
                .map(|row| {
                    col_set
                        .iter()
                        .map(|c| {
                            row.get(c)
                                .map(sanitize_value_preview)
                                .unwrap_or_else(|| "-".to_string())
                        })
                        .collect()
                })
                .collect();
            Some(StructuredTable {
                name: name.to_string(),
                columns: col_set,
                rows,
            })
        }
        Value::Map(m) => {
            let guard = m.lock();
            if guard.is_empty() {
                return None;
            }
            let mut keys: Vec<_> = guard.keys().cloned().collect();
            keys.sort();
            let rows = keys
                .into_iter()
                .take(100)
                .map(|k| {
                    let v_str = guard
                        .get(&k)
                        .map(sanitize_value_preview)
                        .unwrap_or_else(|| "nil".to_string());
                    vec![k, v_str]
                })
                .collect();
            Some(StructuredTable {
                name: name.to_string(),
                columns: vec!["Key".to_string(), "Value".to_string()],
                rows,
            })
        }
        _ => None,
    }
}

pub fn render_dynamic_html_report(source: &str, ctx: &ReportContext) -> String {
    let tables_html = if ctx.structured_tables.is_empty() {
        "<p class=\"muted\">No structured table or map outputs were bound in global scope.</p>"
            .to_string()
    } else {
        ctx.structured_tables
            .iter()
            .map(|t| {
                let ths: String = t
                    .columns
                    .iter()
                    .map(|c| format!("<th>{}</th>", html_escape(c)))
                    .collect();
                let trs: String = t
                    .rows
                    .iter()
                    .map(|r| {
                        let tds: String = r
                            .iter()
                            .map(|cell| format!("<td>{}</td>", html_escape(cell)))
                            .collect();
                        format!("<tr>{}</tr>", tds)
                    })
                    .collect();
                format!(
                    "<div class=\"section\"><h3>Global Table: <code>{}</code> ({} rows)</h3><table><thead><tr>{}</tr></thead><tbody>{}</tbody></table></div>",
                    html_escape(&t.name),
                    t.rows.len(),
                    ths,
                    trs
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let globals_rows = if ctx.user_globals.is_empty() {
        "<tr><td colspan=\"2\" class=\"muted\">No user-defined globals captured.</td></tr>"
            .to_string()
    } else {
        ctx.user_globals
            .iter()
            .map(|(k, v)| {
                format!(
                    "<tr><td><code>{}</code></td><td><code>{}</code></td></tr>",
                    html_escape(k),
                    html_escape(v)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let tests_html = if ctx.tests.is_empty() {
        String::new()
    } else {
        let rows: String = ctx
            .tests
            .iter()
            .map(|rec| {
                let badge = if rec.passed {
                    "<span class=\"badge pass\">PASS</span>"
                } else {
                    "<span class=\"badge fail\">FAIL</span>"
                };
                format!(
                    "<tr><td>{}</td><td>{}</td><td>{} us</td><td>{}</td></tr>",
                    html_escape(&rec.name),
                    badge,
                    rec.duration_us,
                    html_escape(rec.error.as_deref().unwrap_or("-"))
                )
            })
            .collect();
        format!(
            "<div class=\"section\"><h2>Recorded Test Blocks</h2><table><thead><tr><th>Test Name</th><th>Status</th><th>Duration</th><th>Detail</th></tr></thead><tbody>{}</tbody></table></div>",
            rows
        )
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>AETHER Execution Report - {title}</title>
  <style>
    body {{ font-family: system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif; background: #0f172a; color: #e2e8f0; margin: 0; padding: 2rem; line-height: 1.5; }}
    .container {{ max-width: 1120px; margin: 0 auto; }}
    .card {{ background: #1e293b; border: 1px solid #334155; border-radius: 12px; padding: 1.5rem; margin-bottom: 1.5rem; }}
    .grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 1rem; margin-top: 1rem; }}
    .metric {{ background: #0f172a; border: 1px solid #334155; border-radius: 8px; padding: 0.9rem; }}
    .metric-label {{ font-size: 0.78rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.04em; }}
    .metric-val {{ font-size: 1.25rem; font-weight: 700; color: #38bdf8; margin-top: 0.25rem; }}
    h1, h2, h3 {{ color: #f8fafc; margin-top: 0; }}
    table {{ width: 100%; border-collapse: collapse; margin-top: 0.75rem; font-size: 0.9rem; }}
    th, td {{ text-align: left; padding: 0.6rem 0.75rem; border-bottom: 1px solid #334155; vertical-align: top; }}
    th {{ background: #0f172a; color: #38bdf8; }}
    code, pre {{ font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }}
    pre {{ background: #020617; border: 1px solid #334155; border-radius: 8px; padding: 1rem; overflow-x: auto; font-size: 0.85rem; }}
    .badge {{ display: inline-block; padding: 0.15rem 0.5rem; border-radius: 999px; font-size: 0.75rem; font-weight: 700; }}
    .badge.pass {{ background: rgba(34, 197, 94, 0.2); color: #4ade80; }}
    .badge.fail {{ background: rgba(239, 68, 68, 0.2); color: #f87171; }}
    .muted {{ color: #94a3b8; }}
    .section {{ margin-top: 1.5rem; }}
  </style>
</head>
<body>
  <div class="container">
    <div class="card">
      <h1>AETHER Execution Report</h1>
      <p class="muted">Script: <code>{file_path}</code> | Execution Time: <strong>{elapsed:.2} ms</strong></p>
      <div class="grid">
        <div class="metric"><div class="metric-label">Source Lines</div><div class="metric-val">{lines}</div></div>
        <div class="metric"><div class="metric-label">Top Statements</div><div class="metric-val">{stmts}</div></div>
        <div class="metric"><div class="metric-label">Functions / Intents</div><div class="metric-val">{funcs}</div></div>
        <div class="metric"><div class="metric-label">Classes / Structs</div><div class="metric-val">{types}</div></div>
        <div class="metric"><div class="metric-label">User Globals</div><div class="metric-val">{globals_count}</div></div>
      </div>
      <div class="section">
        <h3>Final Expression Value</h3>
        <pre>{return_value}</pre>
      </div>
    </div>

    <div class="card">
      <h2>Runtime State &amp; Output Bindings</h2>
      <table>
        <thead><tr><th>Symbol</th><th>Observed Value</th></tr></thead>
        <tbody>{globals_rows}</tbody>
      </table>
      {tables_html}
    </div>

    {tests_html}

    <div class="card">
      <details>
        <summary><strong>Source Script ({lines} lines)</strong></summary>
        <pre>{source_escaped}</pre>
      </details>
    </div>
  </div>
</body>
</html>
"#,
        title = html_escape(&ctx.file_path),
        file_path = html_escape(&ctx.file_path),
        elapsed = ctx.elapsed_ms,
        lines = ctx.metrics.source_lines,
        stmts = ctx.metrics.statements,
        funcs = ctx.metrics.functions,
        types = ctx.metrics.classes + ctx.metrics.structs,
        globals_count = ctx.user_globals.len(),
        return_value = html_escape(&ctx.return_value),
        globals_rows = globals_rows,
        tables_html = tables_html,
        tests_html = tests_html,
        source_escaped = html_escape(source),
    )
}

fn html_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_report_reflects_script_values_without_hardcoded_data() {
        let src = r#"
let title_xss = "<script>alert(1)</script>"
let metrics = {"accuracy": 0.94, "loss": 0.12}
let samples = [{"id": 1, "score": 88}, {"id": 2, "score": 95}]
metrics["accuracy"]
"#;
        let ctx = execute_and_collect("demo_eval.ae", src).unwrap();
        let html = render_dynamic_html_report(src, &ctx);

        assert!(html.contains("demo_eval.ae"));
        assert!(html.contains("0.94"));
        assert!(html.contains("88"));
        assert!(html.contains("95"));
        assert!(
            !html.contains("<script>alert(1)</script>"),
            "XSS must be escaped"
        );
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(
            !html.contains("MRTX1133"),
            "Report must not contain hardcoded KRAS fixtures"
        );
    }
}
