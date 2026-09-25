use aether::vm::{run_source, Value};

#[test]
fn test_function_creation_def_and_fn() {
    let code = r#"
def add(a, b):
    return a + b

fn multiply(a, b) {
    return a * b
}

r1 = add(3, 7)
r2 = multiply(4, 5)
[r1, r2]
"#;
    let result = run_source(code).expect("def and fn creation failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(10));
        assert_eq!(items[1], Value::Int(20));
    } else {
        panic!("Expected array of results");
    }
}

#[test]
fn test_default_arguments() {
    let code = r#"
def greet(name, greeting="Hello", punctuation="!"):
    return greeting + " " + name + punctuation

r1 = greet("Alice")
r2 = greet("Bob", "Hi")
r3 = greet("Charlie", "Hey", "?")
[r1, r2, r3]
"#;
    let result = run_source(code).expect("Default arguments failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::string("Hello Alice!"));
        assert_eq!(items[1], Value::string("Hi Bob!"));
        assert_eq!(items[2], Value::string("Hey Charlie?"));
    } else {
        panic!("Expected array of strings");
    }
}

#[test]
fn test_keyword_arguments() {
    let code = r#"
def format_user(first, last, role="user", active=true):
    return first + " " + last + " (" + role + ") active=" + str(active)

r1 = format_user(last="Khan", first="Sami")
r2 = format_user("John", "Doe", active=false, role="admin")
[r1, r2]
"#;
    let result = run_source(code).expect("Keyword arguments failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::string("Sami Khan (user) active=true"));
        assert_eq!(items[1], Value::string("John Doe (admin) active=false"));
    } else {
        panic!("Expected array of strings");
    }
}

#[test]
fn test_varargs_and_kwargs() {
    let code = r#"
def stats(primary, *extras, **metadata):
    total = primary
    for x in extras:
        total = total + x
    return [total, len(extras), metadata["tag"]]

res = stats(10, 20, 30, 40, tag="summary", author="aether")
res
"#;
    let result = run_source(code).expect("Varargs and kwargs failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(100));
        assert_eq!(items[1], Value::Int(3));
        assert_eq!(items[2], Value::string("summary"));
    } else {
        panic!("Expected array from stats");
    }
}

#[test]
fn test_callsite_unpacking() {
    let code = r#"
def add_all(a, b, c):
    return a + b + c

args = [10, 20, 30]
res1 = add_all(*args)

def config(host, port):
    return host + ":" + str(port)

opts = {"host": "localhost", "port": 8080}
res2 = config(**opts)

[res1, res2]
"#;
    let result = run_source(code).expect("Callsite unpacking failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(60));
        assert_eq!(items[1], Value::string("localhost:8080"));
    } else {
        panic!("Expected array of results");
    }
}

#[test]
fn test_multiple_return_values_and_unpacking() {
    let code = r#"
def get_dimensions():
    return 1920, 1080

w, h = get_dimensions()
x, y = 100, 200
[w, h, x, y]
"#;
    let result = run_source(code).expect("Multiple return values and unpacking failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(1920));
        assert_eq!(items[1], Value::Int(1080));
        assert_eq!(items[2], Value::Int(100));
        assert_eq!(items[3], Value::Int(200));
    } else {
        panic!("Expected array of ints");
    }
}

#[test]
fn test_lambdas_and_closures() {
    let code = r#"
double = lambda x: x * 2

def make_multiplier(factor):
    return lambda n: n * factor

triple = make_multiplier(3)

[double(7), triple(8)]
"#;
    let result = run_source(code).expect("Lambdas and closures failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(14));
        assert_eq!(items[1], Value::Int(24));
    } else {
        panic!("Expected array of results");
    }
}

#[test]
fn test_scope_and_global_statement() {
    let code = r#"
total = 0

def increment(amount):
    global total
    total = total + amount

increment(10)
increment(25)
total
"#;
    let result = run_source(code).expect("Scope and global statement failed");
    assert_eq!(result, Value::Int(35));
}

#[test]
fn test_recursion_with_def() {
    let code = r#"
def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n - 1)

def fib(n):
    if n <= 0:
        return 0
    elif n == 1:
        return 1
    return fib(n - 1) + fib(n - 2)

[factorial(5), factorial(6), fib(7)]
"#;
    let result = run_source(code).expect("Recursion with def failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(120));
        assert_eq!(items[1], Value::Int(720));
        assert_eq!(items[2], Value::Int(13));
    } else {
        panic!("Expected array of numbers");
    }
}

#[test]
fn test_nested_function_unpacking_and_scope() {
    let code = r#"
def compute_stats(pairs):
    acc = 0
    for p in pairs:
        x, y = p
        acc = acc + (x * y)
    return acc

res = compute_stats([[2, 3], [4, 5], [6, 7]])
res
"#;
    let result = run_source(code).expect("Nested unpacking inside function failed");
    assert_eq!(result, Value::Int(6 + 20 + 42)); // 68
}

#[test]
fn test_higher_order_functions_and_lambdas() {
    let code = r#"
def apply_op(f, a, b):
    return f(a, b)

sum_val = apply_op(lambda x, y: x + y, 10, 20)
prod_val = apply_op(lambda x, y: x * y, 10, 20)

def custom_map(items, func):
    result = []
    for it in items:
        result.push(func(it))
    return result

squares = custom_map([1, 2, 3, 4], lambda x: x * x)

[sum_val, prod_val, squares]
"#;
    let result = run_source(code).expect("Higher order functions and lambdas failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(30));
        assert_eq!(items[1], Value::Int(200));
        if let Value::Array(sq) = &items[2] {
            let sq_items = sq.lock().clone();
            assert_eq!(sq_items, vec![Value::Int(1), Value::Int(4), Value::Int(9), Value::Int(16)]);
        } else {
            panic!("Expected array of squares");
        }
    } else {
        panic!("Expected array of results");
    }
}

