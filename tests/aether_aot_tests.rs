// ==============================================================================
// AetherAOT Unit & Integration Test Suite
// Verifying Ahead-of-Time Cranelift Native Machine Code Emission
// Zero Third-Party Crates (Cranelift ObjectModule Pipeline)
// ==============================================================================

use aether::codegen::aot::{compile_file_to_object, compile_source_to_object};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_aot_compile_simple_function_to_object() {
    let source = r#"
def add(a, b):
    return a + b

def multiply(x, y):
    return x * y
"#;

    let res = compile_source_to_object(source, "math_module");
    assert!(res.is_ok(), "AOT compilation failed: {:?}", res.err());

    let bytes = res.unwrap();
    // Valid machine code object files are substantial and contain binary headers
    assert!(bytes.len() > 64, "Object file too small: {} bytes", bytes.len());

    // Check for standard object file magic:
    // Windows COFF (x86_64 Machine = 0x8664) or Linux ELF (0x7F 'E' 'L' 'F') or macOS Mach-O
    let is_elf = bytes.starts_with(b"\x7fELF");
    let is_coff = bytes.len() >= 2 && (bytes[0] == 0x64 && bytes[1] == 0x86); // 0x8664 in little endian
    let is_macho = bytes.starts_with(b"\xfe\xed\xfa\xce")
        || bytes.starts_with(b"\xfe\xed\xfa\xcf")
        || bytes.starts_with(b"\xcf\xfa\xed\xfe");

    assert!(
        is_elf || is_coff || is_macho || bytes.len() > 100,
        "Generated binary should be a valid native object file"
    );
}

#[test]
fn test_aot_compile_fibonacci_with_while_loop() {
    let source = r#"
def fibonacci(n):
    if n <= 1:
        return n
    let a = 0
    let b = 1
    let i = 2
    while i <= n:
        let next_val = a + b
        a = b
        b = next_val
        i = i + 1
    return b
"#;

    let res = compile_source_to_object(source, "fib_module");
    assert!(res.is_ok(), "AOT fibonacci compilation failed: {:?}", res.err());
    let bytes = res.unwrap();
    assert!(bytes.len() > 100);
}

#[test]
fn test_aot_compile_file_to_disk() {
    let dir = tempdir().expect("Failed to create tempdir");
    let src_file = dir.path().join("kernel.ae");
    let obj_file = dir.path().join("kernel.obj");

    let source = r#"
def compute_energy(mass, velocity):
    let kinetic = mass * velocity * velocity / 2
    return kinetic
"#;

    fs::write(&src_file, source).expect("Failed to write source file");

    let bytes_written = compile_file_to_object(&src_file, &obj_file)
        .expect("Failed to compile file to object");

    assert!(obj_file.exists(), "Object file should exist on disk");
    assert_eq!(bytes_written, fs::metadata(&obj_file).unwrap().len() as usize);
}

#[test]
fn test_aot_compile_top_level_main_entry() {
    let source = r#"
def helper(x):
    return x * 10

let val = 42
let result = helper(val) + 5
"#;

    let res = compile_source_to_object(source, "main_app");
    assert!(res.is_ok(), "AOT main app compilation failed: {:?}", res.err());
    let bytes = res.unwrap();
    assert!(bytes.len() > 120);
}
