use aether::vm::value::Value;

#[test]
fn test_functional_collection_pipeline_methods() {
    let source = r#"
let nums = [5, 2, 8, 1, 9, 2, 4, 8]

let unique_sorted = nums.unique().sort()
let mapped = unique_sorted.map(|x| x * 10)
let filtered = mapped.filter(|x| x >= 40)
let total = filtered.reduce(0, |acc, x| acc + x)

let found = unique_sorted.find(|x| x > 4)
let has_nine = unique_sorted.any(|x| x == 9)
let all_pos = unique_sorted.all(|x| x > 0)

let nested = [[1, 2], [3, 4], [5]]
let flat = nested.flatten()
let taken = flat.skip(1).take(3)
let joined = taken.join("-")

(total, found, has_nine, all_pos, joined, flat.sum())
"#;

    let result = aether::vm::run_source(source).expect("Failed to run functional collection pipeline test");
    assert_eq!(
        result,
        Value::tuple(vec![
            Value::Int(260), // 40 + 50 + 80 + 90
            Value::Int(5),
            Value::Bool(true),
            Value::Bool(true),
            Value::string("2-3-4"),
            Value::Int(15),
        ])
    );
}

#[test]
fn test_comptime_evaluation_and_macro() {
    let source = r#"
macro square_plus_one(n) {
    n * n + 1
}

let FACTORIAL_TABLE = comptime {
    let mut table = []
    let mut acc = 1
    for i in 1..=6 {
        acc = acc * i
        table.push(acc)
    }
    table
}

(FACTORIAL_TABLE[0], FACTORIAL_TABLE[5], square_plus_one(7))
"#;

    let result = aether::vm::run_source(source).expect("Failed to run comptime & macro test");
    assert_eq!(
        result,
        Value::tuple(vec![
            Value::Int(1),
            Value::Int(720),
            Value::Int(50),
        ])
    );
}

#[test]
fn test_async_await_and_structured_tasks() {
    let source = r#"
async fn compute_double(x) {
    return x * 2
}

let t1 = Task.spawn(compute_double, 21)
let t2 = Task.spawn(|n| n + 100, 50)
let t3 = Task.resolve(999)

let r1 = await t1
let all_res = Task.all([t1, t2, t3])
let race_res = Task.race([t3, t1])

(r1, all_res[0], all_res[1], all_res[2], race_res)
"#;

    let result = aether::vm::run_source(source).expect("Failed to run async/await & Task test");
    assert_eq!(
        result,
        Value::tuple(vec![
            Value::Int(42),
            Value::Int(42),
            Value::Int(150),
            Value::Int(999),
            Value::Int(999),
        ])
    );
}

#[test]
fn test_lifo_defer_and_with_context_manager() {
    let source = r#"
let trace = []

class Conn {
    fn __init__(self, name) {
        self.name = name
    }
    fn __enter__(self) {
        trace.push("enter:" + self.name)
        return self
    }
    fn __exit__(self) {
        trace.push("exit:" + self.name)
    }
}

fn run_transaction() {
    defer trace.push("defer:1")
    defer trace.push("defer:2")
    with Conn("db") as c {
        trace.push("use:" + c.name)
    }
    trace.push("body:done")
    return "ok"
}

let status = run_transaction()
(status, trace.join(","))
"#;

    let result = aether::vm::run_source(source).expect("Failed to run defer & with context manager test");
    assert_eq!(
        result,
        Value::tuple(vec![
            Value::string("ok"),
            Value::string("enter:db,use:db,exit:db,body:done,defer:2,defer:1"),
        ])
    );
}

#[test]
fn test_native_regex_module() {
    let source = r##"
let text = "Order #1042 costs 250 USD, Order #2099 costs 480 USD"
let is_valid = Regex.is_match("^Order #[0-9]+", text)
let first = Regex.find("#([0-9]+)", text)
let all_nums = Regex.find_all("[0-9]+", text)
let redacted = Regex.replace_all("#[0-9]+", text, "#REDACTED")
let parts = Regex.split(",\\s*", "alpha, beta,  gamma")

(is_valid, first, len(all_nums), redacted, parts.join("|"))
"##;

    let result = aether::vm::run_source(source).expect("Failed to run Regex module test");
    assert_eq!(
        result,
        Value::tuple(vec![
            Value::Bool(true),
            Value::string("#1042"),
            Value::Int(4),
            Value::string("Order #REDACTED costs 250 USD, Order #REDACTED costs 480 USD"),
            Value::string("alpha|beta|gamma"),
        ])
    );
}

#[test]
fn test_fuzzy_did_you_mean_diagnostics() {
    let source = r#"
let total_balance = 5000
print(total_balanc)
"#;

    let err = aether::vm::run_source(source).expect_err("Expected undefined variable error");
    assert!(
        err.contains("Did you mean 'total_balance'?"),
        "Expected fuzzy suggestion in error, got: {}",
        err
    );
}

#[test]
fn test_builtin_formatter_and_linter() {
    let source = r#"
let unformatted = "fn calc(x){\nlet unused_local = 10\nreturn x+1\n}\n"
let formatted = Formatter.format(unformatted)
let warnings = Linter.lint(unformatted)
let suggestion = Diagnostics.suggest("prnt", ["print", "println", "len"])

(formatted.contains("    let unused_local = 10"), len(warnings), warnings[0]["code"], suggestion)
"#;

    let result = aether::vm::run_source(source).expect("Failed to run Formatter & Linter test");
    assert_eq!(
        result,
        Value::tuple(vec![
            Value::Bool(true),
            Value::Int(1),
            Value::string("W001_UNUSED_VAR"),
            Value::string("print"),
        ])
    );
}

#[test]
fn test_multiline_fluent_chaining_and_pipelines() {
    let source = r#"
fn add_five(n) {
    return n + 5
}

fn double_it(n) {
    return n * 2
}

let chain_res = [1, 2, 3, 4, 5, 6]
    .map(|x| x * 10)
    .filter(|x| x >= 30)
    .sum()

let pipe_res = 10
    |> add_five()
    |> double_it()

let maybe_val = nil
    ?? 42

(chain_res, pipe_res, maybe_val)
"#;

    let result = aether::vm::run_source(source).expect("Failed to run multi-line fluent chaining test");
    assert_eq!(
        result,
        Value::tuple(vec![
            Value::Int(180), // 30 + 40 + 50 + 60
            Value::Int(30),  // (10 + 5) * 2
            Value::Int(42),
        ])
    );
}

#[test]
fn test_language_level_unit_tests_and_property_fuzzing() {
    let source = r#"
test "arithmetic assertions" {
    assert_eq(10 + 20, 30)
    assert_ne(10, 20)
}

test "exception assertions" {
    assert_throws(|| {
        raise "CustomFailure: invalid state"
    }, "CustomFailure")
}

let fuzz_ok = Fuzz.check(|x, y| {
    return (x + y) == (y + x)
}, 50)

let summary = Test.summary()
(summary["total"], summary["passed"], summary["failed"], fuzz_ok)
"#;

    let result = aether::vm::run_source(source).expect("Failed to run language-level test & fuzzing");
    assert_eq!(
        result,
        Value::tuple(vec![
            Value::Int(2),
            Value::Int(2),
            Value::Int(0),
            Value::Bool(true),
        ])
    );

    // Also verify Fuzz.check catches falsified properties with a counterexample
    let bad_fuzz = r#"
Fuzz.check(|x| x < 50, 50)
"#;
    let err = aether::vm::run_source(bad_fuzz).expect_err("Expected FuzzFailure");
    assert!(err.contains("FuzzFailure"), "Expected FuzzFailure, got: {}", err);
}

#[test]
fn test_custom_iterators_and_struct_operator_overloading() {
    let source = r#"
struct Vec2 {
    x,
    y
}

impl Vec2 {
    fn __add__(self, other) {
        return Vec2(self.x + other.x, self.y + other.y)
    }
    fn __sub__(self, other) {
        return Vec2(self.x - other.x, self.y - other.y)
    }
    fn __mul__(self, scalar) {
        return Vec2(self.x * scalar, self.y * scalar)
    }
    fn __neg__(self) {
        return Vec2(-self.x, -self.y)
    }
    fn __eq__(self, other) {
        return self.x == other.x and self.y == other.y
    }
    fn __lt__(self, other) {
        return (self.x + self.y) < (other.x + other.y)
    }
    fn __getitem__(self, idx) {
        if idx == 0 { return self.x }
        return self.y
    }
    fn __len__(self) {
        return 2
    }
    fn __str__(self) {
        return f"({self.x}, {self.y})"
    }
}

let v1 = Vec2(3, 4)
let v2 = Vec2(1, 2)
let v3 = (v1 + v2 - Vec2(0, 1)) * 2
let neg_v2 = -v2

struct RangeCounter {
    curr,
    stop
}

impl RangeCounter {
    fn __next__(self) {
        if self.curr >= self.stop {
            return nil
        }
        let val = self.curr
        self.curr = self.curr + 1
        return val
    }
}

let mut loop_sum = 0
for n in RangeCounter(1, 5) {
    loop_sum = loop_sum + n
}

let comp_items = [x * 10 for x in RangeCounter(1, 4)]
let mapped_items = RangeCounter(1, 4).map(|x| x + 100)

(
    v3 == Vec2(8, 10),
    neg_v2 == Vec2(-1, -2),
    v2 < v1,
    v3[0],
    len(v3),
    str(v1),
    loop_sum,
    comp_items.join(","),
    mapped_items.join(",")
)
"#;

    let result = aether::vm::run_source(source).expect("Failed to run struct operator overloading & custom iterator test");
    assert_eq!(
        result,
        Value::tuple(vec![
            Value::Bool(true),
            Value::Bool(true),
            Value::Bool(true),
            Value::Int(8),
            Value::Int(2),
            Value::string("(3, 4)"),
            Value::Int(10), // 1 + 2 + 3 + 4
            Value::string("10,20,30"),
            Value::string("101,102,103"),
        ])
    );
}

#[test]
fn test_bytecode_disassembler_and_stats() {
    let source = r#"
fn fib_step(a, b) {
    return a + b
}

let dis = Bytecode.disassemble(fib_step)
let st = Bytecode.stats(fib_step)

(
    dis.contains("Function: fib_step"),
    dis.contains("Add"),
    dis.contains("Return"),
    st["name"],
    st["arity"],
    st["code_bytes"] > 0
)
"#;

    let result = aether::vm::run_source(source).expect("Failed to run Bytecode disassembler test");
    assert_eq!(
        result,
        Value::tuple(vec![
            Value::Bool(true),
            Value::Bool(true),
            Value::Bool(true),
            Value::string("fib_step"),
            Value::Int(2),
            Value::Bool(true),
        ])
    );
}
