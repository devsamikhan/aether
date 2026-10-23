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
