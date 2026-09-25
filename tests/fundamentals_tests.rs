use aether::vm::{run_source, Value};

#[test]
fn test_variables_zero_ceremony_and_mutation() {
    let code = r#"
# Global zero-ceremony variable declaration
x = 100
x = 200
x += 50
x -= 25
x *= 2
x /= 5
x %= 50

fn compute():
    # Local zero-ceremony variable declaration
    a = 10
    b = 20
    a += 5
    b %= 7
    return a + b + x

compute()
"#;
    let result = run_source(code).expect("Variable test failed");
    // x = 100 -> 200 -> 250 -> 225 -> 450 -> 90 -> 40
    // compute: a = 15, b = 20 % 7 = 6, total = 15 + 6 + 40 = 61
    assert_eq!(result, Value::Int(61));
}

#[test]
fn test_data_types_strings_and_operations() {
    let code = r#"
# Single and double quote strings
s1 = 'hello'
s2 = "world"
joined = s1 + " " + s2

# String indexing
first_char = joined[0]

# Mixed concatenation
mixed_right = "Score: " + 100
mixed_left = 404 + " Not Found"

# String repetition
shout = "Na" * 4 + " Batman"
shout_rev = 3 * "ha"

[joined, first_char, mixed_right, mixed_left, shout, shout_rev]
"#;
    let result = run_source(code).expect("String test failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::string("hello world"));
        assert_eq!(items[1], Value::string("h"));
        assert_eq!(items[2], Value::string("Score: 100"));
        assert_eq!(items[3], Value::string("404 Not Found"));
        assert_eq!(items[4], Value::string("NaNaNaNa Batman"));
        assert_eq!(items[5], Value::string("hahaha"));
    } else {
        panic!("Expected array of string results");
    }
}

#[test]
fn test_multiline_docstrings() {
    let code = r#"
"""
This is a Python-style multiline docstring
spanning multiple lines cleanly.
"""
doc = """Hello
World"""
doc
"#;
    let result = run_source(code).expect("Multiline docstring test failed");
    assert_eq!(result, Value::string("Hello\nWorld"));
}

#[test]
fn test_boolean_and_none_keywords() {
    let code = r#"
t1 = true
t2 = True
f1 = false
f2 = False
n1 = nil
n2 = None

[t1 == t2, f1 == f2, n1 == n2, t1 != f1]
"#;
    let result = run_source(code).expect("Boolean/None keywords test failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Bool(true));
        assert_eq!(items[1], Value::Bool(true));
        assert_eq!(items[2], Value::Bool(true));
        assert_eq!(items[3], Value::Bool(true));
    } else {
        panic!("Expected array");
    }
}

#[test]
fn test_type_conversions() {
    let code = r#"
# int() conversions
i1 = int("42")
i2 = int(3.9)
i3 = int(True)
i4 = int(False)

# float() conversions
f1 = float("3.14")
f2 = float(10)
f3 = float(True)

# str() conversions
s1 = str(123)
s2 = str(True)
s3 = str(None)

# bool() conversions
b1 = bool(1)
b2 = bool(0)
b3 = bool("")
b4 = bool("Aether")
b5 = bool(None)

# type() and type_of()
t1 = type(42)
t2 = type(3.14)
t3 = type("hello")
t4 = type(True)
t5 = type(None)
t6 = type_of([1, 2])

[i1, i2, i3, i4, f1, f2, f3, s1, s2, s3, b1, b2, b3, b4, b5, t1, t2, t3, t4, t5, t6]
"#;
    let result = run_source(code).expect("Type conversion test failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(42));
        assert_eq!(items[1], Value::Int(3));
        assert_eq!(items[2], Value::Int(1));
        assert_eq!(items[3], Value::Int(0));

        assert_eq!(items[4], Value::Float(3.14));
        assert_eq!(items[5], Value::Float(10.0));
        assert_eq!(items[6], Value::Float(1.0));

        assert_eq!(items[7], Value::string("123"));
        assert_eq!(items[8], Value::string("true"));
        assert_eq!(items[9], Value::string("nil"));

        assert_eq!(items[10], Value::Bool(true));
        assert_eq!(items[11], Value::Bool(false));
        assert_eq!(items[12], Value::Bool(false));
        assert_eq!(items[13], Value::Bool(true));
        assert_eq!(items[14], Value::Bool(false));

        assert_eq!(items[15], Value::string("int"));
        assert_eq!(items[16], Value::string("float"));
        assert_eq!(items[17], Value::string("string"));
        assert_eq!(items[18], Value::string("bool"));
        assert_eq!(items[19], Value::string("nil"));
        assert_eq!(items[20], Value::string("array"));
    } else {
        panic!("Expected array of conversion results");
    }
}

#[test]
fn test_operators_arithmetic_comparison_logical() {
    let code = r#"
# Arithmetic
add = 10 + 5
sub = 10 - 5
mul = 10 * 5
div = 10 / 2
mod = 10 % 3
pow1 = 2 ^ 3
pow2 = 2 ** 3

# Comparison
c1 = (10 == 10)
c2 = (10 != 5)
c3 = (5 < 10)
c4 = (5 <= 5)
c5 = (10 > 5)
c6 = (10 >= 10)

# Logical words (and, or, not)
l1 = True and True
l2 = True and False
l3 = False or True
l4 = not False

# Logical symbols (&&, ||, !)
s1 = true && true
s2 = false || true
s3 = !false

[add, sub, mul, div, mod, pow1, pow2, c1 && c2 && c3 && c4 && c5 && c6, l1 && !l2 && l3 && l4, s1 && s2 && s3]
"#;
    let result = run_source(code).expect("Operators test failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(15));
        assert_eq!(items[1], Value::Int(5));
        assert_eq!(items[2], Value::Int(50));
        assert_eq!(items[3], Value::Int(5));
        assert_eq!(items[4], Value::Int(1));
        assert_eq!(items[5], Value::Int(8));
        assert_eq!(items[6], Value::Int(8));
        assert_eq!(items[7], Value::Bool(true));
        assert_eq!(items[8], Value::Bool(true));
        assert_eq!(items[9], Value::Bool(true));
    } else {
        panic!("Expected array");
    }
}

#[test]
fn test_comments_syntax() {
    let code = r#"
# Python-style single line comment
// C-style single line comment
/*
  C-style multi-line block comment
*/
val = 42 # inline comment
val
"#;
    let result = run_source(code).expect("Comments test failed");
    assert_eq!(result, Value::Int(42));
}
