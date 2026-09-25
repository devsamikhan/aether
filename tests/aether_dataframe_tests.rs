// ==============================================================================
// AetherDataFrame Tests — Vectorized Columnar Analytics & Relational Engine
// ==============================================================================

use aether::vm::dataframe::{NativeDataFrame, TypedColumn};
use aether::vm::{run_source, Value};
use std::collections::HashMap;

#[test]
fn test_dataframe_native_columnar_memory_and_types() {
    let col_int = TypedColumn::from_values(&[Value::Int(10), Value::Int(20), Value::Int(30)]);
    assert_eq!(col_int.type_name(), "int64");
    assert_eq!(col_int.sum(), Value::Int(60));
    assert_eq!(col_int.mean(), Value::Float(20.0));
    assert_eq!(col_int.min(), Value::Int(10));
    assert_eq!(col_int.max(), Value::Int(30));

    let col_float = TypedColumn::from_values(&[Value::Float(1.5), Value::Float(2.5), Value::Float(6.0)]);
    assert_eq!(col_float.type_name(), "float64");
    assert_eq!(col_float.sum(), Value::Float(10.0));

    let col_str = TypedColumn::from_values(&[Value::string("alpha"), Value::string("beta")]);
    assert_eq!(col_str.type_name(), "string");
    assert_eq!(col_str.len(), 2);
}

#[test]
fn test_dataframe_sorting_and_filtering() {
    let mut map = HashMap::new();
    map.insert("name".to_string(), Value::array(vec![Value::string("Alice"), Value::string("Bob"), Value::string("Charlie")]));
    map.insert("score".to_string(), Value::array(vec![Value::Int(88), Value::Int(95), Value::Int(72)]));

    let df = NativeDataFrame::from_map(map);
    assert_eq!(df.row_count, 3);

    // Filter score >= 80: Alice and Bob match
    let filtered = df.filter(&[true, true, false]);
    assert_eq!(filtered.row_count, 2);

    // Sort descending by score: Bob (95), Alice (88), Charlie (72)
    let sorted = df.sort_by("score", true).expect("Sort failed");
    let top_name = sorted.columns.get("name").unwrap().get(0).unwrap();
    assert_eq!(top_name.to_string(), "Bob");
}

#[test]
fn test_dataframe_group_by_and_aggregations() {
    let mut map = HashMap::new();
    map.insert(
        "dept".to_string(),
        Value::array(vec![
            Value::string("Eng"),
            Value::string("Eng"),
            Value::string("Sales"),
            Value::string("Sales"),
        ]),
    );
    map.insert(
        "salary".to_string(),
        Value::array(vec![
            Value::Float(100.0),
            Value::Float(150.0),
            Value::Float(80.0),
            Value::Float(120.0),
        ]),
    );

    let df = NativeDataFrame::from_map(map);

    let mut aggs = HashMap::new();
    aggs.insert("salary".to_string(), "mean".to_string());

    let grouped = df.group_by("dept", &aggs).expect("Group by failed");
    assert_eq!(grouped.row_count, 2); // Eng and Sales
    assert!(grouped.column_names.contains(&"salary_mean".to_string()));
}

#[test]
fn test_dataframe_relational_joins() {
    // Left: Users
    let mut left_map = HashMap::new();
    left_map.insert("id".to_string(), Value::array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));
    left_map.insert("name".to_string(), Value::array(vec![Value::string("Alice"), Value::string("Bob"), Value::string("Carol")]));
    let left_df = NativeDataFrame::from_map(left_map);

    // Right: Transactions
    let mut right_map = HashMap::new();
    right_map.insert("id".to_string(), Value::array(vec![Value::Int(1), Value::Int(2)]));
    right_map.insert("amount".to_string(), Value::array(vec![Value::Float(500.0), Value::Float(750.0)]));
    let right_df = NativeDataFrame::from_map(right_map);

    let joined = left_df.join(&right_df, "id", "inner").expect("Join failed");
    assert_eq!(joined.row_count, 2); // Only IDs 1 and 2
    assert!(joined.column_names.contains(&"amount".to_string()));
}

#[test]
fn test_dataframe_csv_roundtrip_parsing() {
    let csv_data = "id,name,active,rate\n1,Alpha,true,99.5\n2,Beta,false,84.0\n3,Gamma,true,105.25\n";
    let df = NativeDataFrame::from_csv(csv_data).expect("Failed to parse CSV");
    assert_eq!(df.row_count, 3);
    assert_eq!(df.column_names.len(), 4);

    let exported = df.to_csv();
    assert!(exported.contains("Alpha"));
    assert!(exported.contains("105.25"));
}

#[test]
fn test_dataframe_aether_stdlib_script() {
    let code = r#"
from aether_df import DataFrame, Series

# 1. Create DataFrame from column maps
df = DataFrame({
    "product": ["Widget A", "Widget B", "Widget C", "Widget D"],
    "price": [25.0, 50.0, 75.0, 100.0],
    "quantity": [10, 5, 2, 8]
})

# 2. Vectorized column computation
# revenue = price * quantity
let p = df["price"]
let q = df["quantity"]
let rev = p * q
df["revenue"] = rev

# 3. Filter rows where revenue > 300
# Widget A: 25*10=250 (false)
# Widget B: 50*5=250 (false)
# Widget C: 75*2=150 (false)
# Widget D: 100*8=800 (true)
let mask = df["revenue"] > 300.0
let high_rev = df.filter(mask)

[
    df.shape()[0],
    df.shape()[1],
    high_rev.shape()[0]
]
"#;
    let res = run_source(code).expect("Aether DataFrame script failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(4)); // 4 rows
        assert_eq!(items[1], Value::Int(4)); // 4 columns (product, price, quantity, revenue)
        assert_eq!(items[2], Value::Int(1)); // 1 high revenue row (Widget D)
    } else {
        panic!("Expected array, got {:?}", res);
    }
}
