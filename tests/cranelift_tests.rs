use aether::codegen::cranelift_backend::CraneliftCompiler;
use aether::syntax::parse;

#[test]
fn test_cranelift_jit_arithmetic() {
    let code = r#"
let a = 15
let b = 35
let c = (a * 4) + (b / 5) - 7
c
"#;
    let prog = parse(code).expect("Parse failed");
    let mut compiler = CraneliftCompiler::new().expect("Compiler creation failed");
    let res = compiler.compile_and_run(&prog).expect("Execution failed");
    // (15 * 4) + (35 / 5) - 7 = 60 + 7 - 7 = 60
    assert_eq!(res, 60);
}

#[test]
fn test_cranelift_jit_functions_and_recursion() {
    let code = r#"
fn fib(n):
    if n <= 1:
        return n
    else:
        return fib(n - 1) + fib(n - 2)

fib(12)
"#;
    let prog = parse(code).expect("Parse failed");
    let mut compiler = CraneliftCompiler::new().expect("Compiler creation failed");
    let res = compiler.compile_and_run(&prog).expect("Execution failed");
    assert_eq!(res, 144);
}

#[test]
fn test_cranelift_jit_while_loop() {
    let code = r#"
let i = 1
let total = 0
while i <= 100:
    total += i
    i += 1
total
"#;
    let prog = parse(code).expect("Parse failed");
    let mut compiler = CraneliftCompiler::new().expect("Compiler creation failed");
    let res = compiler.compile_and_run(&prog).expect("Execution failed");
    assert_eq!(res, 5050);
}

#[test]
fn test_cranelift_jit_pattern_matching() {
    let code = r#"
fn check_status(code):
    match code:
        200 => 1
        400..499 => 4
        500..599 => 5
        _ => 0

let s1 = check_status(200)
let s2 = check_status(404)
let s3 = check_status(503)
let s4 = check_status(999)

(s1 * 1000) + (s2 * 100) + (s3 * 10) + s4
"#;
    let prog = parse(code).expect("Parse failed");
    let mut compiler = CraneliftCompiler::new().expect("Compiler creation failed");
    let res = compiler.compile_and_run(&prog).expect("Execution failed");
    // 1000 + 400 + 50 + 0 = 1450
    assert_eq!(res, 1450);
}

#[test]
fn test_cranelift_jit_intent_contract_success() {
    let code = r#"
intent safe_divide(a, b):
    require: b > 0
    ensure: result >= 0
    body:
        return a / b

safe_divide(100, 4)
"#;
    let prog = parse(code).expect("Parse failed");
    let mut compiler = CraneliftCompiler::new().expect("Compiler creation failed");
    let res = compiler.compile_and_run(&prog).expect("Execution failed");
    assert_eq!(res, 25);
}

#[test]
fn test_cranelift_jit_for_range_loop() {
    let code = r#"
let sum = 0
for i in range(1, 101):
    sum += i
sum
"#;
    let prog = parse(code).expect("Parse failed");
    let mut compiler = CraneliftCompiler::new().expect("Compiler creation failed");
    let res = compiler.compile_and_run(&prog).expect("Execution failed");
    assert_eq!(res, 5050);
}

#[test]
fn test_cranelift_jit_native_buffer_and_indexing() {
    let code = r#"
let buf = alloc_buffer(10)
for i in range(10):
    buf[i] = (i + 1) * 10

let total = 0
for i in range(10):
    total += buf[i]

free_buffer(buf, 10)
total
"#;
    let prog = parse(code).expect("Parse failed");
    let mut compiler = CraneliftCompiler::new().expect("Compiler creation failed");
    let res = compiler.compile_and_run(&prog).expect("Execution failed");
    // 10 + 20 + 30 + 40 + 50 + 60 + 70 + 80 + 90 + 100 = 550
    assert_eq!(res, 550);
}

#[test]
fn test_cranelift_jit_array_literal() {
    let code = r#"
let arr = [100, 200, 300, 400]
arr[0] + arr[1] + arr[2] + arr[3]
"#;
    let prog = parse(code).expect("Parse failed");
    let mut compiler = CraneliftCompiler::new().expect("Compiler creation failed");
    let res = compiler.compile_and_run(&prog).expect("Execution failed");
    assert_eq!(res, 1000);
}

