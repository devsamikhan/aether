// ==============================================================================
// AETHER 2.0 Final Production Stress, Toolchain & Quality Gate Test Suite
// Verifying System Diagnostics, Project Scaffolding, Stress Loops & Toolchain
// Pure Rust Standard Library (Zero Third-Party Crates)
// ==============================================================================

use aether::toolchain::{perform_self_update, run_doctor, scaffold_project};
use aether::vm::run_source;
use std::fs;

#[test]
fn test_toolchain_diagnostics_doctor() {
    let report = run_doctor();
    assert_eq!(report.version, "1.1.0");
    assert!(!report.checks.is_empty());

    let home_check = report.checks.iter().find(|c| c.name.contains("AETHER_HOME"));
    assert!(home_check.is_some());

    let vm_check = report.checks.iter().find(|c| c.name.contains("Bytecode VM"));
    assert!(vm_check.is_some());
    assert!(vm_check.unwrap().passed);
}

#[test]
fn test_toolchain_project_scaffolding_templates() {
    let temp_dir = std::env::temp_dir().join("aether_test_project_ai");
    if temp_dir.exists() {
        let _ = fs::remove_dir_all(&temp_dir);
    }

    // 1. Scaffold AI Template
    let path = scaffold_project(temp_dir.to_str().unwrap(), "ai").expect("Failed to scaffold project");
    assert!(path.exists());
    assert!(path.join("aether.toml").exists());
    assert!(path.join("src/main.ae").exists());
    assert!(path.join("tests/test_main.ae").exists());

    // Verify generated code executes cleanly
    let main_src = fs::read_to_string(path.join("src/main.ae")).unwrap();
    let res = run_source(&main_src);
    assert!(res.is_ok(), "Generated AI main.ae should execute cleanly: {:?}", res.err());

    let test_src = fs::read_to_string(path.join("tests/test_main.ae")).unwrap();
    let test_res = run_source(&test_src);
    assert!(test_res.is_ok(), "Generated AI test_main.ae should execute cleanly: {:?}", test_res.err());

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    // 2. Scaffold FinTech Template
    let fintech_dir = std::env::temp_dir().join("aether_test_project_fintech");
    if fintech_dir.exists() {
        let _ = fs::remove_dir_all(&fintech_dir);
    }

    let fpath = scaffold_project(fintech_dir.to_str().unwrap(), "fintech").expect("Failed to scaffold fintech project");
    let fmain_src = fs::read_to_string(fpath.join("src/main.ae")).unwrap();
    let fres = run_source(&fmain_src);
    assert!(fres.is_ok(), "Generated FinTech main.ae should execute cleanly: {:?}", fres.err());

    let ftest_src = fs::read_to_string(fpath.join("tests/test_main.ae")).unwrap();
    let ftest_res = run_source(&ftest_src);
    assert!(ftest_res.is_ok(), "Generated FinTech test_main.ae should execute cleanly: {:?}", ftest_res.err());

    // Cleanup
    let _ = fs::remove_dir_all(&fintech_dir);
}

#[test]
fn test_production_stress_recursive_stack_and_concurrency() {
    let script = r#"
def deep_fib(n):
    if n <= 1:
        return n
    return deep_fib(n - 1) + deep_fib(n - 2)

let val = deep_fib(20)
assert(val == 6765, "Fib(20) should be 6765")

# Stress concurrent fiber channel passing
let ch = channel()
spawn {
    let sum = 0
    let i = 0
    while i < 100:
        sum = sum + i
        i = i + 1
    send(ch, sum)
}

let result = recv(ch)
assert(result == 4950, "Sum 0..99 should be 4950")
"#;

    let res = run_source(script);
    assert!(res.is_ok(), "Stress recursive stack & concurrency failed: {:?}", res.err());
}

#[test]
fn test_production_stress_columnar_dataframe_and_graph_scale() {
    let script = r#"
import aether_df
import aether_graph

# 1. DataFrame scale stress
let ids = []
let scores = []
let i = 0
while i < 100:
    push(ids, i)
    push(scores, i * 2.5)
    i = i + 1

let df = aether_df.DataFrame({"id": ids, "score": scores})
let score_col = df["score"]
let total = score_col.sum()
assert(total > 0.0, "Total score should be positive")

# 2. Knowledge Graph scale stress
let g = aether_graph.Graph("stress_graph")
let prev = g.add_node("Node_0")
let j = 1
while j <= 50:
    let curr = g.add_node("Node_" + str(j))
    g.add_edge(prev, curr, "LINK", 1.0)
    prev = curr
    j = j + 1

let path = g.shortest_path(1, 51)
assert(path != nil, "Shortest path should exist")
assert(path["cost"] == 50.0, "Cost across 50 linear edges should be 50.0")
"#;

    let res = run_source(script);
    assert!(res.is_ok(), "Stress dataframe and graph scale failed: {:?}", res.err());
}

#[test]
fn test_toolchain_self_update_simulation() {
    let rep = perform_self_update().expect("Self update simulation should succeed");
    assert_eq!(rep.current_version, "1.1.0");
    assert!(rep.verified);
    assert!(rep.success);
    // Cleanup temporary backup
    let _ = fs::remove_file(rep.backup_path);
}
