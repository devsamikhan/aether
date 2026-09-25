use aether::vm::{run_source, Value};

#[test]
fn test_aether_assertions() {
    let code = r#"
from aether_test import assert_eq, assert_ne, assert_true, assert_false, assert_almost_eq, assert_raises

# 1. Successful assertions
assert_eq(1 + 1, 2)
assert_ne("hello", "world")
assert_true(10 > 5)
assert_false(5 > 10)
assert_almost_eq(3.14159, 3.14160, 0.001)

# 2. assert_raises
fn buggy_fn():
    raise "CustomError: something went wrong"

assert_raises(buggy_fn, "CustomError")

true
"#;
    let res = run_source(code).expect("Aether assertions failed");
    assert_eq!(res, Value::Bool(true));
}

#[test]
fn test_aether_test_suite_runner() {
    let code = r#"
from aether_test import TestSuite, assert_eq

suite = TestSuite("Math and Logic Verification")

fn test_addition():
    assert_eq(2 + 2, 4)

fn test_multiplication():
    assert_eq(3 * 3, 9)

suite.test("addition test", test_addition)
suite.test("multiplication test", test_multiplication)

results = suite.run()
[results["passed"], results["failed"], results["total"]]
"#;
    let res = run_source(code).expect("TestSuite execution failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(2)); // passed
        assert_eq!(items[1], Value::Int(0)); // failed
        assert_eq!(items[2], Value::Int(2)); // total
    } else {
        panic!("Expected Array result, got {:?}", res);
    }
}

#[test]
fn test_aether_benchmark_harness() {
    let code = r#"
from aether_test import benchmark

fn compute():
    let x = 10 * 20 + 30
    return x

bench_stats = benchmark("Simple Arithmetic", compute, 50)
[bench_stats["name"], bench_stats["iterations"]]
"#;
    let res = run_source(code).expect("Benchmark execution failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::string("Simple Arithmetic"));
        assert_eq!(items[1], Value::Int(50));
    } else {
        panic!("Expected Array result, got {:?}", res);
    }
}
