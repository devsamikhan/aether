use aether::syntax::{parse, Lexer, TokenKind};
use aether::vm::{run_source, Value};

#[test]
fn test_lexer_indentation_and_tokens() {
    let code = r#"
fn add(a, b):
    let result = a + b
    return result
"#;
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().expect("Lexing failed");

    assert!(tokens.iter().any(|t| t.kind == TokenKind::Fn));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Indent));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Dedent));
    assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
}

#[test]
fn test_pipeline_token_and_parsing() {
    let code = r#"
let numbers = [1, 2, 3]
let count = numbers |> len()
"#;
    let program = parse(code).expect("Parsing pipeline failed");
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn test_vm_basic_arithmetic() {
    let code = r#"
let a = 10 + 20 * 3
let b = (100 - 40) / 2
a + b
"#;
    let result = run_source(code).expect("VM run failed");
    // a = 70, b = 30 -> 100
    assert_eq!(result, Value::Int(100));
}

#[test]
fn test_vm_variables_and_mutation() {
    let code = r#"
let mut count = 10
count += 5
count *= 2
count
"#;
    let result = run_source(code).expect("VM run failed");
    assert_eq!(result, Value::Int(30));
}

#[test]
fn test_vm_function_and_recursion() {
    let code = r#"
fn fib(n):
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

fib(10)
"#;
    let result = run_source(code).expect("VM run failed");
    assert_eq!(result, Value::Int(55));
}

#[test]
fn test_vm_arrays_and_indexing() {
    let code = r#"
let arr = [10, 20, 30]
push(arr, 40)
let val = arr[1] + arr[3]
val
"#;
    let result = run_source(code).expect("VM run failed");
    assert_eq!(result, Value::Int(60));
}

#[test]
fn test_vm_map_and_lookup() {
    let code = r#"
let user = {"name": "Aether", "version": 2}
user["version"]
"#;
    let result = run_source(code).expect("VM run failed");
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_vm_pipeline_operator() {
    let code = r#"
fn double(x):
    return x * 2

fn add_ten(x):
    return x + 10

5 |> double() |> add_ten()
"#;
    let result = run_source(code).expect("VM run failed");
    // (5 * 2) + 10 = 20
    assert_eq!(result, Value::Int(20));
}

#[test]
fn test_vm_channels_concurrency() {
    let code = r#"
let ch = channel()
send(ch, 42)
let received = recv(ch)
received
"#;
    let result = run_source(code).expect("VM run failed");
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_vm_fibonacci_benchmark() {
    let code = r#"
fn fib(n):
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

let start = clock()
let res = fib(22)
let elapsed = clock() - start
println("Fib(22) computed in AETHER 2.0 VM:", res, "Elapsed sec:", elapsed)
res
"#;
    let result = run_source(code).expect("VM run failed");
    assert_eq!(result, Value::Int(17711));
}

#[test]
fn test_vm_intent_contract() {
    let code = r#"
intent square(x):
    require: x > 0
    ensure: result > 0
    body:
        return x * x

square(8)
"#;
    let result = run_source(code).expect("VM run failed");
    assert_eq!(result, Value::Int(64));
}

#[test]
fn test_stdlib_math() {
    let code = r#"
let s = Math.sqrt(144)
let a = Math.abs(-50)
let m = Math.max(10, 99)
s + a + m
"#;
    let result = run_source(code).expect("VM run failed");
    // 12.0 + 50 + 99 = 161.0
    assert_eq!(result, Value::Float(161.0));
}

#[test]
fn test_stdlib_strings() {
    let code = r#"
let text = "   aether,rust,python   "
let clean = Strings.trim(text)
let parts = Strings.split(clean, ",")
let upper_joined = Strings.upper(Strings.join(parts, " -> "))
upper_joined
"#;
    let result = run_source(code).expect("VM run failed");
    assert_eq!(result, Value::string("AETHER -> RUST -> PYTHON"));
}

#[test]
fn test_stdlib_json() {
    let code = r#"
let json_str = "{\"name\":\"Aether\",\"version\":2,\"features\":[\"fast\",\"simple\"]}"
let parsed = Json.parse(json_str)
let name = parsed["name"]
let ver = parsed["version"]
let first_feat = parsed["features"][0]
let encoded = Json.stringify(parsed)
name + " " + str(ver) + " is " + first_feat
"#;
    let result = run_source(code).expect("VM run failed");
    assert_eq!(result, Value::string("Aether 2 is fast"));
}

#[test]
fn test_stdlib_file_io() {
    let code = r#"
let test_file = "test_temp_output.txt"
File.write(test_file, "Line 1\nLine 2\nLine 3")
let exists = File.exists(test_file)
let content = File.read(test_file)
let lines_count = len(File.lines(test_file))
File.delete(test_file)
lines_count
"#;
    let result = run_source(code).expect("VM run failed");
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_ast_static_type_inference() {
    use aether::syntax::parse;
    use aether::types::inference::InferenceContext;
    use aether::types::{Type, TypeEnv};

    let prog = parse("10 + 20 * 3").expect("Parse failed");
    if let aether::syntax::ast::Statement::Expr(ref expr) = prog.statements[0] {
        let mut ctx = InferenceContext::new();
        let env = TypeEnv::new();
        let (inferred_type, _) = ctx.infer_syntax_expr(expr, &env).expect("Inference failed");
        assert_eq!(inferred_type, Type::Int);
    } else {
        panic!("Expected Expr statement");
    }

    let float_prog = parse("3.14 * 2.0").expect("Parse failed");
    if let aether::syntax::ast::Statement::Expr(ref expr) = float_prog.statements[0] {
        let mut ctx = InferenceContext::new();
        let env = TypeEnv::new();
        let (inferred_type, _) = ctx.infer_syntax_expr(expr, &env).expect("Inference failed");
        assert_eq!(inferred_type, Type::Float);
    }
}

#[test]
fn test_vm_structs() {
    let code = r#"
struct Point:
    x
    y

let p = Point(15, 35)
p.x + p.y
"#;
    let result = run_source(code).expect("VM run failed");
    assert_eq!(result, Value::Int(50));
}

#[test]
fn test_vm_struct_mutation() {
    let code = r#"
struct Account:
    holder
    balance

let acc = Account("Sami", 1000)
acc.balance += 500
acc.balance
"#;
    let result = run_source(code).expect("VM run failed");
    assert_eq!(result, Value::Int(1500));
}

#[test]
fn test_vm_pattern_matching_literals() {
    let code = r#"
fn status_text(code):
    match code:
        200 => "OK"
        404 => "Not Found"
        500 => "Error"
        _ => "Unknown"

status_text(200) + " - " + status_text(404) + " - " + status_text(999)
"#;
    let result = run_source(code).expect("VM run failed");
    assert_eq!(result, Value::string("OK - Not Found - Unknown"));
}

#[test]
fn test_vm_pattern_matching_ranges() {
    let code = r#"
fn grade(score):
    match score:
        90..100 => "A"
        80..89 => "B"
        70..79 => "C"
        _ => "F"

grade(95) + grade(82) + grade(40)
"#;
    let result = run_source(code).expect("VM run failed");
    assert_eq!(result, Value::string("ABF"));
}
