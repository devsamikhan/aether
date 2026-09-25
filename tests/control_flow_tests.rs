use aether::vm::{run_source, Value};

#[test]
fn test_if_elif_else_chains() {
    let code = r#"
fn grade(score):
    if score >= 90:
        return "A"
    elif score >= 80:
        return "B"
    elif score >= 70:
        return "C"
    elif score >= 60:
        return "D"
    else:
        return "F"

g1 = grade(95)
g2 = grade(82)
g3 = grade(74)
g4 = grade(61)
g5 = grade(45)

[g1, g2, g3, g4, g5]
"#;
    let result = run_source(code).expect("If-Elif-Else test failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::string("A"));
        assert_eq!(items[1], Value::string("B"));
        assert_eq!(items[2], Value::string("C"));
        assert_eq!(items[3], Value::string("D"));
        assert_eq!(items[4], Value::string("F"));
    } else {
        panic!("Expected array of grades");
    }
}

#[test]
fn test_nested_conditions() {
    let code = r#"
fn classify(age, has_license):
    status = ""
    if age >= 18:
        if has_license:
            status = "Can Drive"
        else:
            status = "Need License"
    else:
        if age >= 16:
            status = "Learner"
        else:
            status = "Too Young"
    return status

c1 = classify(25, true)
c2 = classify(20, false)
c3 = classify(16, false)
c4 = classify(12, false)

[c1, c2, c3, c4]
"#;
    let result = run_source(code).expect("Nested condition test failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::string("Can Drive"));
        assert_eq!(items[1], Value::string("Need License"));
        assert_eq!(items[2], Value::string("Learner"));
        assert_eq!(items[3], Value::string("Too Young"));
    } else {
        panic!("Expected array");
    }
}

#[test]
fn test_match_case_statements() {
    let code = r#"
fn handle_status(code):
    match code:
        case 200:
            "OK"
        case 400:
            "Bad Request"
        case 404:
            "Not Found"
        case 500..599:
            "Server Error"
        case _:
            "Unknown Status"

res1 = handle_status(200)
res2 = handle_status(404)
res3 = handle_status(503)
res4 = handle_status(999)

[res1, res2, res3, res4]
"#;
    let result = run_source(code).expect("Match-Case test failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::string("OK"));
        assert_eq!(items[1], Value::string("Not Found"));
        assert_eq!(items[2], Value::string("Server Error"));
        assert_eq!(items[3], Value::string("Unknown Status"));
    } else {
        panic!("Expected array");
    }
}

#[test]
fn test_while_loop_break_and_continue() {
    let code = r#"
# Test break in while loop
i = 0
sum_break = 0
while i < 100:
    if i == 5:
        break
    sum_break += i
    i += 1

# Test continue in while loop
j = 0
sum_continue = 0
while j < 10:
    j += 1
    if j % 2 == 0:
        continue
    sum_continue += j

[sum_break, sum_continue]
"#;
    let result = run_source(code).expect("While loop break/continue failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        // sum_break: 0 + 1 + 2 + 3 + 4 = 10
        assert_eq!(items[0], Value::Int(10));
        // sum_continue: odd numbers 1, 3, 5, 7, 9 = 25
        assert_eq!(items[1], Value::Int(25));
    } else {
        panic!("Expected array");
    }
}

#[test]
fn test_for_loop_break_and_continue() {
    let code = r#"
# 1. For loop over range with break
sum_range = 0
for x in range(1, 20):
    if x > 5:
        break
    sum_range += x

# 2. For loop over array with continue
sum_evens = 0
numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
for n in numbers:
    if n % 2 != 0:
        continue
    sum_evens += n

# 3. For loop over string
collected = ""
for ch in "Aether":
    collected = collected + ch + "-"

[sum_range, sum_evens, collected]
"#;
    let result = run_source(code).expect("For loop break/continue failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        // sum_range: 1 + 2 + 3 + 4 + 5 = 15
        assert_eq!(items[0], Value::Int(15));
        // sum_evens: 2 + 4 + 6 + 8 + 10 = 30
        assert_eq!(items[1], Value::Int(30));
        // collected: "A-e-t-h-e-r-"
        assert_eq!(items[2], Value::string("A-e-t-h-e-r-"));
    } else {
        panic!("Expected array");
    }
}

#[test]
fn test_pass_statement() {
    let code = r#"
x = 10
y = 0

if x > 5:
    pass
else:
    y = 99

if x < 0:
    y = 50
else:
    pass

for i in range(3):
    pass

y
"#;
    let result = run_source(code).expect("Pass statement test failed");
    assert_eq!(result, Value::Int(0));
}
