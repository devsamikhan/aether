use aether::vm::{run_source, Value};

#[test]
fn test_try_except_zero_division() {
    let code = r#"
result = 0
try:
    bad = 100 / 0
except as e:
    result = 999
result
"#;
    let res = run_source(code).expect("Script execution failed");
    assert_eq!(res, Value::Int(999));
}

#[test]
fn test_try_except_custom_raise() {
    let code = r#"
caught_error = ""
try:
    raise "CustomError: Access Denied"
except as e:
    caught_error = e
caught_error
"#;
    let res = run_source(code).expect("Script execution failed");
    assert_eq!(res, Value::string("CustomError: Access Denied"));
}

#[test]
fn test_try_finally_flow() {
    let code = r#"
trace = []
try:
    push(trace, "step_try")
    raise "fail"
except as e:
    push(trace, "step_except")
finally:
    push(trace, "step_finally")
trace
"#;
    let res = run_source(code).expect("Script execution failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], Value::string("step_try"));
        assert_eq!(items[1], Value::string("step_except"));
        assert_eq!(items[2], Value::string("step_finally"));
    } else {
        panic!("Expected array trace");
    }
}

#[test]
fn test_list_comprehension_basic() {
    let code = r#"
squares = [x * x for x in [1, 2, 3, 4, 5]]
squares
"#;
    let res = run_source(code).expect("List comp failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items, vec![Value::Int(1), Value::Int(4), Value::Int(9), Value::Int(16), Value::Int(25)]);
    } else {
        panic!("Expected array");
    }
}

#[test]
fn test_list_comprehension_with_filter() {
    let code = r#"
evens = [x * 2 for x in [1, 2, 3, 4, 5, 6] if x % 2 == 0]
evens
"#;
    let res = run_source(code).expect("Filtered list comp failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items, vec![Value::Int(4), Value::Int(8), Value::Int(12)]);
    } else {
        panic!("Expected array");
    }
}

#[test]
fn test_dict_comprehension_basic() {
    let code = r#"
cubes = {x: x * x * x for x in [1, 2, 3]}
cubes["2"]
"#;
    let res = run_source(code).expect("Dict comp failed");
    assert_eq!(res, Value::Int(8));
}

#[test]
fn test_dict_comprehension_filter() {
    let code = r#"
filtered = {x: x * 10 for x in [1, 2, 3, 4] if x > 2}
len(filtered)
"#;
    let res = run_source(code).expect("Filtered dict comp failed");
    assert_eq!(res, Value::Int(2));
}

#[test]
fn test_module_import_math() {
    let code = r#"
import math as m
m.sqrt(64)
"#;
    let res = run_source(code).expect("Module import failed");
    assert_eq!(res, Value::Float(8.0));
}

#[test]
fn test_from_module_import() {
    let code = r#"
from math import sqrt
sqrt(49)
"#;
    let res = run_source(code).expect("From import failed");
    assert_eq!(res, Value::Float(7.0));
}

#[test]
fn test_http_serve_and_get() {
    let code = r#"
import sys

# Spawn background HTTP server responding to single request
spawn:
    Http.serve_response(18899, "PONG from Aether Server")

# Give server 50ms to bind
Sys.sleep(0.05)

# Perform HTTP GET request
resp = Http.get("http://127.0.0.1:18899/ping")
resp["body"]
"#;
    let res = run_source(code).expect("HTTP roundtrip failed");
    assert_eq!(res, Value::string("PONG from Aether Server"));
}
