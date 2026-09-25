// ==============================================================================
// AetherLive / AetherHotReload Unit & Integration Test Suite
// Verifying Erlang BEAM-Grade Live Code Swapping & Hot State Preservation
// Pure Rust Standard Library (Zero Third-Party Crates)
// ==============================================================================

use aether::vm::compiler::BytecodeCompiler;
use aether::vm::hotreload::{hot_reload, ReloadConfig};
use aether::vm::{run_source, Value, VM};
use std::sync::Arc;

#[test]
fn test_hot_reload_function_swapping_and_state_preservation() {
    let mut vm = VM::new();

    // Initial script: counter starts at 100, calculate adds 10
    let v1_source = r#"
let counter = 100
def calculate(x):
    return x + 10
"#;
    let program1 = aether::syntax::parse(v1_source).unwrap();
    let fn1 = BytecodeCompiler::new("<main>", 0).compile(&program1).unwrap();
    vm.interpret(fn1).unwrap();

    // Verify initial calculation: 5 + 10 = 15
    let res1 = vm.call_global("calculate", vec![Value::Int(5)]).unwrap();
    assert_eq!(res1, Value::Int(15));

    // Live runtime state mutation: user sets counter to 999
    vm.globals.insert("counter".to_string(), Value::Int(999));

    // Hot-reload V2: new code multiplies by 100, adds a new function, has let counter = 0
    let v2_source = r#"
let counter = 0
def calculate(x):
    return x * 100

def double(x):
    return x * 2
"#;
    let report = hot_reload(&mut vm, v2_source, None).expect("Hot reload failed");
    assert!(report.success);
    assert_eq!(report.functions_updated, vec!["calculate"]);
    assert_eq!(report.functions_added, vec!["double"]);
    assert!(report.globals_preserved.contains(&"counter".to_string()));

    // Verify upgraded calculation: 5 * 100 = 500
    let res2 = vm.call_global("calculate", vec![Value::Int(5)]).unwrap();
    assert_eq!(res2, Value::Int(500));

    // Verify newly added function
    let res3 = vm.call_global("double", vec![Value::Int(21)]).unwrap();
    assert_eq!(res3, Value::Int(42));

    // CRITICAL: Verify live state was PRESERVED at 999 (not reset to 0!)
    let live_counter = vm.globals.get("counter").cloned().unwrap();
    assert_eq!(live_counter, Value::Int(999));
}

#[test]
fn test_hot_reload_atomic_safety_on_syntax_and_compile_error() {
    let mut vm = VM::new();

    let working_source = r#"
def secret_sauce():
    return 42
"#;
    let program = aether::syntax::parse(working_source).unwrap();
    let compiled = BytecodeCompiler::new("<main>", 0).compile(&program).unwrap();
    vm.interpret(compiled).unwrap();

    // Verify working state
    let res = vm.call_global("secret_sauce", vec![]).unwrap();
    assert_eq!(res, Value::Int(42));

    // Attempt hot-reload with invalid syntax
    let broken_source = r#"
def broken_syntax(
    return 123 + + +
"#;
    let reload_res = hot_reload(&mut vm, broken_source, None);
    assert!(reload_res.is_err(), "Broken source should fail atomically");

    // Verify VM is 100% UNTOUCHED and secret_sauce still works!
    let res_after = vm.call_global("secret_sauce", vec![]).unwrap();
    assert_eq!(res_after, Value::Int(42));
}

#[test]
fn test_hot_reload_in_place_class_patching() {
    let mut vm = VM::new();

    let v1 = r#"
class PaymentGateway:
    def fee_rate(self):
        return 0.05
"#;
    let p1 = aether::syntax::parse(v1).unwrap();
    let f1 = BytecodeCompiler::new("<main>", 0).compile(&p1).unwrap();
    vm.interpret(f1).unwrap();

    // Get reference to original class
    let original_class_arc = match vm.globals.get("PaymentGateway").unwrap() {
        Value::ClassDef(c) => c.clone(),
        _ => panic!("Expected ClassDef"),
    };

    // Hot-reload V2 with upgraded fee_rate and added discount method
    let v2 = r#"
class PaymentGateway:
    def fee_rate(self):
        return 0.02
    def discount(self):
        return 0.50
"#;
    let report = hot_reload(&mut vm, v2, None).expect("Hot reload failed");
    assert!(report.classes_updated.contains(&"PaymentGateway".to_string()));

    // Verify original class Arc was patched in-place
    let current_class_arc = match vm.globals.get("PaymentGateway").unwrap() {
        Value::ClassDef(c) => c.clone(),
        _ => panic!("Expected ClassDef"),
    };

    // Reference identity must match!
    assert!(Arc::ptr_eq(&original_class_arc, &current_class_arc));

    // Methods in original ClassDef now include new logic
    let methods = original_class_arc.methods.lock();
    assert!(methods.contains_key("fee_rate"));
    assert!(methods.contains_key("discount"));
}

#[test]
fn test_hot_reload_lifecycle_hook_and_versioning() {
    let mut vm = VM::new();

    let v1 = r#"
total = 0
def on_reload():
    global total
    total = total + 1
"#;
    let p1 = aether::syntax::parse(v1).unwrap();
    let f1 = BytecodeCompiler::new("<main>", 0).compile(&p1).unwrap();
    vm.interpret(f1).unwrap();

    // Trigger reload
    let v2 = r#"
total = 0
def on_reload():
    global total
    total = total + 1
"#;
    let config = ReloadConfig {
        preserve_globals: true,
        update_classes_in_place: true,
        call_on_reload: true,
    };
    let report = hot_reload(&mut vm, v2, Some(config)).unwrap();
    assert!(report.version > 0);

    // Verify on_reload was called and incremented total
    let count = vm.globals.get("total").cloned().unwrap();
    assert_eq!(count, Value::Int(1));
}

#[test]
fn test_hot_reload_stdlib_script_execution() {
    let script = r#"
import aether_live

let rep = aether_live.live.reload("def dynamic_worker(x):\n    return x * 3\n")
assert(rep["success"] == true, "Hot reload report should be success")

let v = dynamic_worker(7)
assert(v == 21, "Dynamic worker should be 21")

let ver = aether_live.live.version()
assert(ver > 0, "Version should be positive integer")
"#;

    let res = run_source(script);
    assert!(res.is_ok(), "AetherLive script failed: {:?}", res.err());
}
