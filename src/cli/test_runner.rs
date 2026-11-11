use super::diagnostics::format_error_with_source;
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;

#[derive(Debug, Default)]
pub struct TestRunSummary {
    pub files_run: usize,
    pub files_failed: usize,
    pub assertions_passed: usize,
    pub assertions_failed: usize,
    pub elapsed_ms: f64,
}

/// Executes `.ae` test files via the real AETHER bytecode compiler & VM.
pub fn run_test_command(target: Option<&str>, all: bool) -> Result<(), String> {
    let files = collect_test_files(target, all)?;
    if files.is_empty() {
        return Err(
            "No AETHER test files found (pass a `<file.ae>`, a directory, or create `.ae` tests in `aether_tests/`).".to_string(),
        );
    }

    let summary = execute_test_files(&files);
    println!("\n================================================================================");
    println!(
        "AETHER Test Summary: {} files ({} failed), {} test cases passed, {} failed in {:.2} ms",
        summary.files_run,
        summary.files_failed,
        summary.assertions_passed,
        summary.assertions_failed,
        summary.elapsed_ms
    );
    println!("================================================================================");

    if summary.files_failed > 0 || summary.assertions_failed > 0 {
        Err(format!(
            "{} test case(s) and {} file(s) failed",
            summary.assertions_failed, summary.files_failed
        ))
    } else {
        Ok(())
    }
}

fn collect_test_files(target: Option<&str>, all: bool) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();

    if let Some(t) = target {
        let path = PathBuf::from(t);
        if path.is_file() {
            files.push(path);
            return Ok(files);
        }
        if path.is_dir() {
            collect_ae_in_dir(&path, &mut files);
            files.sort();
            return Ok(files);
        }
        return Err(format!("Test target '{}' does not exist", t));
    }

    let default_dirs = ["aether_tests", "tests"];
    for dir in default_dirs {
        let p = Path::new(dir);
        if p.is_dir() {
            collect_ae_in_dir(p, &mut files);
        }
    }

    // Also include curated root-level *_test.ae / test_*.ae files if present
    if let Ok(entries) = std::fs::read_dir(".") {
        for entry in entries.flatten() {
            let p = entry.path();
            if is_ae_file(&p) {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("test_") || name.ends_with("_test.ae") {
                        files.push(p);
                    }
                }
            }
        }
    }

    if all {
        let builtin_suite = PathBuf::from("examples/20_aethertest_suite_and_benchmark.ae");
        if builtin_suite.is_file() && !files.contains(&builtin_suite) {
            files.push(builtin_suite);
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn collect_ae_in_dir(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in WalkDir::new(dir).follow_links(false).into_iter().flatten() {
        let p = entry.path();
        if p.is_file() && is_ae_file(p) {
            out.push(p.to_path_buf());
        }
    }
}

fn is_ae_file(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("ae")
}

pub fn execute_test_files(files: &[PathBuf]) -> TestRunSummary {
    let suite_start = Instant::now();
    let mut summary = TestRunSummary::default();

    for path in files {
        summary.files_run += 1;
        println!("=== AETHER Test Runner: {} ===", path.display());
        match run_single_test_file(path) {
            Ok((passed, failed)) => {
                summary.assertions_passed += passed;
                summary.assertions_failed += failed;
                if failed > 0 {
                    summary.files_failed += 1;
                }
            }
            Err(err) => {
                eprintln!("{}", err);
                summary.files_failed += 1;
                summary.assertions_failed += 1;
            }
        }
    }

    summary.elapsed_ms = suite_start.elapsed().as_secs_f64() * 1000.0;
    summary
}

fn run_single_test_file(path: &Path) -> Result<(usize, usize), String> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read test file '{}': {}", path.display(), e))?;

    let raw_prog = match aether::syntax::parse(&src) {
        Ok(p) => p,
        Err((e, span)) => {
            let rendered = aether::syntax::diagnostics::render_diagnostic_snippet(
                &src, span.line, span.col, &e, None,
            );
            return Err(rendered);
        }
    };

    let (prog, _) = aether::vm::optimizer::optimize_program(&raw_prog);
    let compiler = aether::vm::BytecodeCompiler::new("<main>", 0);
    let func = compiler
        .compile(&prog)
        .map_err(|e| format!("Compiler Error in '{}': {}", path.display(), e))?;

    let mut vm = aether::vm::VM::new();
    vm.test_runner_mode = true;
    let exec_res = vm.interpret(func);

    let total = vm.test_results.len();
    let mut passed = 0usize;
    let mut failed = 0usize;

    for rec in &vm.test_results {
        if rec.passed {
            passed += 1;
            println!("  [PASS] {} ({} us)", rec.name, rec.duration_us);
        } else {
            failed += 1;
            println!(
                "  [FAIL] {} ({} us)\n      -> {}",
                rec.name,
                rec.duration_us,
                rec.error.as_deref().unwrap_or("Unknown failure")
            );
        }
    }

    if let Err(e) = exec_res {
        eprintln!("  [ERROR] Uncaught runtime error in '{}':", path.display());
        eprintln!("{}", format_error_with_source(&src, &e));
        return Ok((passed, failed.max(1)));
    }

    // Files containing top-level assertions without explicit `test "..."` blocks still count as 1 pass when Ok.
    if total == 0 {
        passed = 1;
        println!("  [PASS] {} (top-level script checks)", path.display());
    }

    Ok((passed, failed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_runner_passes_valid_ae_test_file() {
        let mut f = NamedTempFile::with_suffix(".ae").unwrap();
        writeln!(f, "test \"math works\" {{\n    assert_eq(2 + 2, 4)\n}}\n").unwrap();
        let res = run_test_command(Some(f.path().to_str().unwrap()), false);
        assert!(res.is_ok(), "Expected passing test file, got: {:?}", res);
    }

    #[test]
    fn test_runner_returns_err_on_failed_assertion() {
        let mut f = NamedTempFile::with_suffix(".ae").unwrap();
        writeln!(
            f,
            "test \"intentional failure\" {{\n    assert_eq(2 + 2, 5)\n}}\n"
        )
        .unwrap();
        let res = run_test_command(Some(f.path().to_str().unwrap()), false);
        assert!(res.is_err(), "Expected Err for failing test assertion");
    }

    #[test]
    fn test_runner_returns_err_on_syntax_error() {
        let mut f = NamedTempFile::with_suffix(".ae").unwrap();
        writeln!(f, "fn broken( {{\n").unwrap();
        let res = run_test_command(Some(f.path().to_str().unwrap()), false);
        assert!(res.is_err(), "Expected Err for syntax error in test file");
    }
}
