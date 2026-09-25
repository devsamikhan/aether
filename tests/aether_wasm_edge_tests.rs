// ==============================================================================
// AetherWasm & AetherEdge Tests — Universal WebAssembly & Sandboxed Edge Runtime
// ==============================================================================

use aether::codegen::wasm::WasmCompiler;
use aether::syntax::parse;
use aether::vm::wasm_runtime::{WasmInstance, WasmModule};
use aether::vm::{run_source, Value};

#[test]
fn test_wasm_binary_generation_and_w3c_compliance() {
    let source = r#"
def add(a, b):
    return a + b

def multiply(x, y):
    return x * y
"#;
    let program = parse(source).expect("Failed to parse Aether source");
    let mut compiler = WasmCompiler::new();
    let wasm_bytes = compiler.compile_program(&program).expect("Failed to compile to WASM");

    // 1. Verify W3C Magic Header: \0asm (0x00, 0x61, 0x73, 0x6D)
    assert!(wasm_bytes.len() >= 8);
    assert_eq!(&wasm_bytes[0..4], &[0x00, 0x61, 0x73, 0x6D]);

    // 2. Verify W3C Version 1: 0x01, 0x00, 0x00, 0x00
    assert_eq!(&wasm_bytes[4..8], &[0x01, 0x00, 0x00, 0x00]);

    // 3. Verify Parsing back into Module
    let module = WasmModule::parse(&wasm_bytes).expect("Failed to parse emitted WASM binary");
    assert_eq!(module.functions.len(), 2);
    assert!(module.exports.contains_key("add"));
    assert!(module.exports.contains_key("multiply"));
    assert!(module.exports.contains_key("memory"));
}

#[test]
fn test_wasm_edge_runtime_arithmetic_and_locals() {
    let source = r#"
def compute_formula(x, y, z):
    let prod = x * y
    let diff = prod - z
    return diff + 10
"#;
    let program = parse(source).expect("Failed to parse Aether source");
    let mut compiler = WasmCompiler::new();
    let wasm_bytes = compiler.compile_program(&program).expect("Failed to compile to WASM");

    let module = WasmModule::parse(&wasm_bytes).expect("Failed to parse WASM");
    let mut instance = WasmInstance::new(module);

    // compute_formula(5, 6, 7) = (5 * 6) - 7 + 10 = 30 - 7 + 10 = 33
    let result = instance.invoke("compute_formula", &[5, 6, 7], 100_000).expect("WASM invoke failed");
    assert_eq!(result, 33);
}

#[test]
fn test_wasm_edge_recursive_computation() {
    let source = r#"
def fib(n):
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)
"#;
    let program = parse(source).expect("Failed to parse Aether source");
    let mut compiler = WasmCompiler::new();
    let wasm_bytes = compiler.compile_program(&program).expect("Failed to compile to WASM");

    let module = WasmModule::parse(&wasm_bytes).expect("Failed to parse WASM");
    let mut instance = WasmInstance::new(module);

    // fib(10) = 55
    let result = instance.invoke("fib", &[10], 1_000_000).expect("WASM invoke failed");
    assert_eq!(result, 55);

    // fib(7) = 13
    let result_7 = instance.invoke("fib", &[7], 1_000_000).expect("WASM invoke failed");
    assert_eq!(result_7, 13);
}

#[test]
fn test_wasm_edge_loop_and_conditionals() {
    let source = r#"
def power_mod(base, exp, modulus):
    let result = 1
    let b = base % modulus
    let e = exp
    while e > 0:
        if e % 2 == 1:
            result = (result * b) % modulus
        e = e / 2
        b = (b * b) % modulus
    return result
"#;
    let program = parse(source).expect("Failed to parse Aether source");
    let mut compiler = WasmCompiler::new();
    let wasm_bytes = compiler.compile_program(&program).expect("Failed to compile to WASM");

    let module = WasmModule::parse(&wasm_bytes).expect("Failed to parse WASM");
    let mut instance = WasmInstance::new(module);

    // 7^13 mod 100 = 7
    let result = instance.invoke("power_mod", &[7, 13, 100], 100_000).expect("WASM invoke failed");
    assert_eq!(result, 7);

    // 3^7 mod 13: 2187 mod 13 = 3
    let result_2 = instance.invoke("power_mod", &[3, 7, 13], 100_000).expect("WASM invoke failed");
    assert_eq!(result_2, 3);
}

#[test]
fn test_wasm_edge_linear_memory_and_strings() {
    let source = r#"
def get_banner_ptr():
    return "AETHER_EDGE_PROTOCOL_V1"
"#;
    let program = parse(source).expect("Failed to parse Aether source");
    let mut compiler = WasmCompiler::new();
    let wasm_bytes = compiler.compile_program(&program).expect("Failed to compile to WASM");

    let module = WasmModule::parse(&wasm_bytes).expect("Failed to parse WASM");
    let mut instance = WasmInstance::new(module);

    let offset = instance.invoke("get_banner_ptr", &[], 100_000).expect("WASM invoke failed") as usize;
    assert!(offset >= 1024);

    let banner = String::from_utf8_lossy(&instance.memory[offset..offset + 23]).to_string();
    assert_eq!(banner, "AETHER_EDGE_PROTOCOL_V1");
}

#[test]
fn test_aether_edge_serverless_worker() {
    let code = r#"
from aether_wasm import WasmModule, EdgeWorker

# 1. Compile Aether function into sandboxed WASM module
src = """
def handler(request_id):
    let balance = request_id * 2
    return balance + 50
"""

wasm_mod = WasmModule.compile_source(src)
worker = EdgeWorker(wasm_mod, "/api/v1/compute")

# 2. Dispatch simulated Cloudflare / Edge serverless request
req = {
    "handler": "handler",
    "args": [125]
}

res = worker.handle_request(req)

[
    res["status"],
    res["route"],
    res["result"],
    res["engine"]
]
"#;
    let res = run_source(code).expect("AetherEdge serverless worker test failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0].to_string(), "200");
        assert_eq!(items[1].to_string(), "/api/v1/compute");
        // handler(125) = (125 * 2) + 50 = 300
        assert_eq!(items[2].to_string(), "300");
        assert_eq!(items[3].to_string(), "AetherEdge-v1.0");
    } else {
        panic!("Expected Array, got {:?}", res);
    }
}
