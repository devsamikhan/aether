use aether::vm::{run_source, Value};

fn run_code(code: &str) -> Result<Value, String> {
    run_source(code)
}

#[test]
fn test_lists_comprehensive() {
    let code = r#"
nums = [10, 20, 30]

# Indexing & negative indexing
first = nums[0]
last = nums[-1]

# Mutation
nums[1] = 99
modified = nums[1]

# Methods
nums.append(40)
appended_len = len(nums)

popped = nums.pop()

# 'in' operator
has_10 = 10 in nums
has_500 = 500 in nums

# List concatenation & repetition
c = [1, 2] + [3, 4]
rep = [7] * 3

# Iteration
sum = 0
for n in [1, 2, 3, 4]:
    sum = sum + n

[first, last, modified, appended_len, popped, has_10, has_500, len(c), len(rep), sum]
"#;
    let result = run_code(code).expect("List test failed");
    if let Value::Array(arr) = result {
        let items = arr.lock();
        assert_eq!(items[0], Value::Int(10));
        assert_eq!(items[1], Value::Int(30));
        assert_eq!(items[2], Value::Int(99));
        assert_eq!(items[3], Value::Int(4));
        assert_eq!(items[4], Value::Int(40));
        assert_eq!(items[5], Value::Bool(true));
        assert_eq!(items[6], Value::Bool(false));
        assert_eq!(items[7], Value::Int(4));
        assert_eq!(items[8], Value::Int(3));
        assert_eq!(items[9], Value::Int(10));
    } else {
        panic!("Expected array result");
    }
}

#[test]
fn test_tuples_immutability_and_operations() {
    let code = r#"
# Tuples creation
t_empty = ()
t_single = (42,)
t_multi = (1, 2, 3, 4, 5)

# Grouping without comma remains math
math_grouped = (10 + 20) * 2

# Indexing
first = t_multi[0]
last = t_multi[-1]

# Length
l_multi = len(t_multi)
l_single = len(t_single)

# 'in' operator
in_tuple = 3 in t_multi
not_in_tuple = 99 in t_multi

# Methods
count_2 = t_multi.count(2)
idx_3 = t_multi.index(3)

# Iteration
acc = 0
for x in (10, 20, 30):
    acc = acc + x

[first, last, l_multi, l_single, in_tuple, not_in_tuple, count_2, idx_3, acc, math_grouped]
"#;
    let result = run_code(code).expect("Tuple test failed");
    if let Value::Array(arr) = result {
        let items = arr.lock();
        assert_eq!(items[0], Value::Int(1));
        assert_eq!(items[1], Value::Int(5));
        assert_eq!(items[2], Value::Int(5));
        assert_eq!(items[3], Value::Int(1));
        assert_eq!(items[4], Value::Bool(true));
        assert_eq!(items[5], Value::Bool(false));
        assert_eq!(items[6], Value::Int(1));
        assert_eq!(items[7], Value::Int(2));
        assert_eq!(items[8], Value::Int(60));
        assert_eq!(items[9], Value::Int(60));
    } else {
        panic!("Expected array result");
    }
}

#[test]
fn test_sets_uniqueness_and_operations() {
    let code = r#"
# Set literal with unique elements
s = {1, 2, 3, 2, 1}
initial_len = len(s)

# Add method
s.add(4)
s.add(3) # Duplicate should not increase len
len_after_add = len(s)

# Contains / 'in' operator
has_2 = 2 in s
has_99 = 99 in s

# Remove
s.remove(1)
len_after_remove = len(s)

# Set from constructor with deduplication
from_list = set([10, 20, 20, 30, 10])
len_from_list = len(from_list)

# Iteration
sum = 0
for item in {5, 10, 15}:
    sum = sum + item

[initial_len, len_after_add, has_2, has_99, len_after_remove, len_from_list, sum]
"#;
    let result = run_code(code).expect("Set test failed");
    if let Value::Array(arr) = result {
        let items = arr.lock();
        assert_eq!(items[0], Value::Int(3)); // unique: {1, 2, 3}
        assert_eq!(items[1], Value::Int(4)); // after add(4)
        assert_eq!(items[2], Value::Bool(true));
        assert_eq!(items[3], Value::Bool(false));
        assert_eq!(items[4], Value::Int(3)); // after remove(1)
        assert_eq!(items[5], Value::Int(3)); // unique: 10, 20, 30
        assert_eq!(items[6], Value::Int(30)); // 5 + 10 + 15
    } else {
        panic!("Expected array result");
    }
}

#[test]
fn test_dictionaries_key_value_and_methods() {
    let code = r#"
# Dictionary literal
user = {"name": "Aether", "speed": 100}

# Access
name = user["name"]
speed = user["speed"]

# Mutation & new key
user["speed"] = 200
user["version"] = 1.1
updated_speed = user["speed"]

# Get method with default
role = user.get("role", "Architect")
missing = user.get("unknown")

# Keys & Values
k = user.keys()
v = user.values()

# 'in' operator for keys
has_name = "name" in user
has_password = "password" in user

# Iteration over keys
keys_accum = ""
for key in {"x": 1, "y": 2}:
    keys_accum = keys_accum + key

[name, updated_speed, role, missing, len(k), len(v), has_name, has_password, keys_accum]
"#;
    let result = run_code(code).expect("Dict test failed");
    if let Value::Array(arr) = result {
        let items = arr.lock();
        assert_eq!(items[0], Value::string("Aether"));
        assert_eq!(items[1], Value::Int(200));
        assert_eq!(items[2], Value::string("Architect"));
        assert_eq!(items[3], Value::Nil);
        assert_eq!(items[4], Value::Int(3)); // name, speed, version
        assert_eq!(items[5], Value::Int(3));
        assert_eq!(items[6], Value::Bool(true));
        assert_eq!(items[7], Value::Bool(false));
        assert_eq!(items[8], Value::string("xy"));
    } else {
        panic!("Expected array result");
    }
}

#[test]
fn test_strings_comprehensive_methods() {
    let code = r#"
text = "Aether Language"

# Indexing & negative indexing
c0 = text[0]
c_last = text[-1]

# 'in' operator (substring search)
has_ae = "Aeth" in text
has_py = "Python" in text

# Methods
upper_text = text.upper()
lower_text = text.lower()
starts = text.startswith("Aeth")
ends = text.endswith("age")

# Trim & Split
padded = "  spaced  "
trimmed = padded.trim()
words = text.split(" ")

# String repetition
stars = "*" * 5

# Iteration over chars
char_count = 0
for ch in "hello":
    char_count = char_count + 1

[c0, c_last, has_ae, has_py, upper_text, lower_text, starts, ends, trimmed, len(words), stars, char_count]
"#;
    let result = run_code(code).expect("String test failed");
    if let Value::Array(arr) = result {
        let items = arr.lock();
        assert_eq!(items[0], Value::string("A"));
        assert_eq!(items[1], Value::string("e"));
        assert_eq!(items[2], Value::Bool(true));
        assert_eq!(items[3], Value::Bool(false));
        assert_eq!(items[4], Value::string("AETHER LANGUAGE"));
        assert_eq!(items[5], Value::string("aether language"));
        assert_eq!(items[6], Value::Bool(true));
        assert_eq!(items[7], Value::Bool(true));
        assert_eq!(items[8], Value::string("spaced"));
        assert_eq!(items[9], Value::Int(2));
        assert_eq!(items[10], Value::string("*****"));
        assert_eq!(items[11], Value::Int(5));
    } else {
        panic!("Expected array result");
    }
}

#[test]
fn test_nested_heterogeneous_structures() {
    let code = r#"
data = {
    "tags": ["compiler", "vm", "aot"],
    "meta": (1, "stable", True),
    "unique_ids": {101, 102, 103}
}

tag_count = len(data["tags"])
first_tag = data["tags"][0]
status = data["meta"][1]
has_id = 102 in data["unique_ids"]

[tag_count, first_tag, status, has_id]
"#;
    let result = run_code(code).expect("Nested data structures test failed");
    if let Value::Array(arr) = result {
        let items = arr.lock();
        assert_eq!(items[0], Value::Int(3));
        assert_eq!(items[1], Value::string("compiler"));
        assert_eq!(items[2], Value::string("stable"));
        assert_eq!(items[3], Value::Bool(true));
    } else {
        panic!("Expected array result");
    }
}
