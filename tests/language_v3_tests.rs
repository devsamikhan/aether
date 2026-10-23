use aether::syntax::lexer::Lexer;
use aether::syntax::parser::Parser;
use aether::types::checker::TypeChecker;
use aether::vm::run_source as execute_source;
use aether::vm::optimizer::{count_peephole_opportunities, optimize_program};
use aether::vm::value::Value;

#[test]
fn test_enum_adt_and_pattern_matching_with_guards() {
    let src = r#"
enum Shape<T> {
    Circle(r),
    Rect(w, h),
    Point
}

fn describe(s) {
    return match s {
        Shape::Circle(r) if r > 10 => f"large circle {r}",
        Shape::Circle(r) => f"circle {r}",
        Shape::Rect(w, h) if w == h => f"square {w}",
        Shape::Rect(w, h) => f"rect {w}x{h}",
        Shape::Point => "point",
        _ => "unknown"
    }
}

let a = describe(Shape::Circle(15))
let b = describe(Shape::Circle(4))
let c = describe(Shape::Rect(7, 7))
let d = describe(Shape::Point)
f"{a} | {b} | {c} | {d}"
"#;
    let res = execute_source(src).expect("Execution failed");
    assert_eq!(res, Value::string("large circle 15 | circle 4 | square 7 | point"));
}

#[test]
fn test_trait_and_impl_blocks_on_structs() {
    let src = r#"
trait Area {
    fn area(self) -> Int
    fn scale(self, factor: Int) -> Int
}

struct Box2D {
    w: Int,
    h: Int
}

impl Area for Box2D {
    fn area(self) -> Int {
        return self.w * self.h
    }
    fn scale(self, factor: Int) -> Int {
        return (self.w * self.h) * factor
    }
}

let b = Box2D(6, 7)
b.area() + b.scale(2)
"#;
    let res = execute_source(src).expect("Execution failed");
    assert_eq!(res, Value::Int(42 + 84));
}

#[test]
fn test_structural_destructuring_and_match_patterns() {
    let src = r#"
let (x, y) = (10, 20)
let [head, second, ..rest] = [1, 2, 3, 4, 5]
let { name, role: user_role } = {"name": "Aether", "role": "Compiler"}

let status_code = 201
let label = match status_code {
    200 | 201 | 204 => "success",
    400 | 404 => "client_err",
    _ => "other"
}

let arr_match = match [10, 20, 30, 40] {
    [a, b, ..tail] => a + b + len(tail),
    _ => 0
}

f"{x + y}:{head}:{second}:{len(rest)}:{name}:{user_role}:{label}:{arr_match}"
"#;
    let res = execute_source(src).expect("Execution failed");
    assert_eq!(res, Value::string("30:1:2:3:Aether:Compiler:success:32"));
}

#[test]
fn test_ergonomic_operators_coalesce_optional_try_range_slice() {
    let src = r#"
let a = nil ?? 42
let b = 99 ?? 42

let user = {"profile": {"city": "Lahore"}, "scores": [10, 20, 30]}
let missing = nil
let c1 = user?.profile?.city
let c2 = missing?.profile?.city ?? "Unknown"
let s1 = user.scores?[1]
let s2 = missing?[0] ?? -1

fn parse_positive(n) {
    if n > 0 {
        return Ok(n * 2)
    }
    return Err("negative")
}

fn compute(x) {
    let v = parse_positive(x)?
    return Ok(v + 1)
}

let ok_res = match compute(5) {
    Ok(val) => val,
    Err(_) => -1
}
let err_res = match compute(-3) {
    Ok(val) => val,
    Err(msg) => msg
}

let r_excl = 1..5
let r_incl = 1..=5
let nums = [0, 1, 2, 3, 4, 5]
let sub = nums[1:4]
let rev = nums[::-1]
let str_rev = "AETHER"[::-1]

f"{a},{b},{c1},{c2},{s1},{s2},{ok_res},{err_res},{len(r_excl)},{len(r_incl)},{len(sub)},{rev[0]},{str_rev}"
"#;
    let res = execute_source(src).expect("Execution failed");
    assert_eq!(
        res,
        Value::string("42,99,Lahore,Unknown,20,-1,11,negative,4,5,3,5,REHTEA")
    );
}

#[test]
fn test_mutable_closures_and_generators() {
    let src = r#"
fn make_counter(start) {
    let count = start
    return || {
        count += 1
        return count
    }
}

let c = make_counter(10)
let c1 = c()
let c2 = c()
let c3 = c()

fn fib_gen(limit) {
    let a = 0
    let b = 1
    for i in 0..limit {
        yield a
        let nxt = a + b
        a = b
        b = nxt
    }
}

let g = fib_gen(6)
let first = next(g)
let second = next(g)
let sum = 0
for val in fib_gen(6) {
    sum += val
}

f"{c1},{c2},{c3}:{first},{second}:{sum}"
"#;
    let res = execute_source(src).expect("Execution failed");
    assert_eq!(res, Value::string("11,12,13:0,1:12"));
}

#[test]
fn test_decorators_memoize_and_custom_chains() {
    let src = r#"
let calls = 0

@memoize
fn fib(n) {
    calls += 1
    if n <= 1 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}

let f10 = fib(10)
let calls_after_first = calls
let f10_again = fib(10)
let calls_after_second = calls

fn double_result(f) {
    return |x| f(x) * 2
}

@double_result
fn base_val(x) {
    return x + 3
}

let d = base_val(7)
f"{f10}:{calls_after_first == calls_after_second}:{d}"
"#;
    let res = execute_source(src).expect("Execution failed");
    assert_eq!(res, Value::string("55:true:20"));
}

#[test]
fn test_generics_unions_type_aliases_and_intent_contracts() {
    let src = r#"
type Score = Int
type FlexId = Int | String

fn identity<T>(val: T) -> T {
    return val
}

intent SafeTransfer(amount: Score, balance: Score) {
    require: amount > 0
    require: balance >= amount
    ensure: result >= 0
    do: {
        return balance - amount
    }
}

let remaining = SafeTransfer(40, 100)
let caught = "none"
try {
    SafeTransfer(-5, 100)
} catch as err {
    caught = "contract_caught"
}

f"{identity(remaining)}:{caught}"
"#;
    let res = execute_source(src).expect("Execution failed");
    assert_eq!(res, Value::string("60:contract_caught"));
}

#[test]
fn test_static_type_checker_trait_validation() {
    let src = r#"
type UserId = Int | String

trait Serializable {
    fn serialize(self) -> String
    fn version(self) -> Int
}

struct Packet {
    id: Int
}

impl Serializable for Packet {
    fn serialize(self) -> String {
        return "pkt"
    }
}
"#;
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().unwrap();

    let mut checker = TypeChecker::new();
    let err = checker.check_program(&program).expect_err("Expected missing trait method error");
    assert!(err[0].contains("missing required method 'version'"));
}

#[test]
fn test_multipass_optimizer_and_peephole() {
    let src = r#"
let x = (10 + 20) * 3
let s = "AETHER" + "_V3"
let y = nil ?? 99
if false {
    println("dead branch")
}
"#;
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let raw_program = parser.parse_program().unwrap();

    let (program, stats) = optimize_program(&raw_program);
    assert!(stats.constants_folded >= 3);
    assert!(stats.dead_statements_removed >= 1);

    let compiler = aether::vm::compiler::BytecodeCompiler::new("<main>", 0);
    let func = compiler.compile(&program).unwrap();
    let _peep = count_peephole_opportunities(&func.chunk);
}

#[test]
fn test_multifile_module_loader_and_export() {
    let temp_dir = tempfile::tempdir().unwrap();
    let helper_path = temp_dir.path().join("math_utils_v3.ae");
    std::fs::write(
        &helper_path,
        r#"
export let PI_APPROX = 314
export fn triple(x) {
    return x * 3
}
"#,
    )
    .unwrap();

    let helper_str = helper_path.to_str().unwrap().replace('\\', "/");
    let main_src = format!(
        r#"
import "{}" as mu
from "{}" import triple

let a = mu.PI_APPROX
let b = triple(14)
f"{{a}}:{{b}}"
"#,
        helper_str, helper_str
    );

    let res = execute_source(&main_src).expect("Module execution failed");
    assert_eq!(res, Value::string("314:42"));
}
