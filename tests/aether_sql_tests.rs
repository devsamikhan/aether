// ==============================================================================
// AetherSQL Unit & Integration Test Suite
// Verifying Vector-Relational Hybrid SQL Engine over Columnar DataFrames
// ==============================================================================

use aether::vm::dataframe::{NativeDataFrame, TypedColumn};
use aether::vm::sql::execute_sql;
use aether::vm::{run_source, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn create_sample_products_df() -> Arc<Mutex<NativeDataFrame>> {
    let mut cols = HashMap::new();
    cols.insert("id".to_string(), TypedColumn::Int(vec![1, 2, 3, 4]));
    cols.insert("name".to_string(), TypedColumn::String(vec![
        "Laptop".into(), "Phone".into(), "Tablet".into(), "Monitor".into()
    ]));
    cols.insert("price".to_string(), TypedColumn::Float(vec![1200.0, 800.0, 400.0, 300.0]));
    cols.insert("in_stock".to_string(), TypedColumn::Bool(vec![true, true, false, true]));
    // 2D Embeddings: Laptop [0.9, 0.1], Phone [0.8, 0.2], Tablet [0.7, 0.3], Monitor [0.1, 0.9]
    cols.insert("emb".to_string(), TypedColumn::from_values(&[
        Value::array(vec![Value::Float(0.9), Value::Float(0.1)]),
        Value::array(vec![Value::Float(0.8), Value::Float(0.2)]),
        Value::array(vec![Value::Float(0.7), Value::Float(0.3)]),
        Value::array(vec![Value::Float(0.1), Value::Float(0.9)]),
    ]));

    let names = vec!["id".into(), "name".into(), "price".into(), "in_stock".into(), "emb".into()];
    Arc::new(Mutex::new(NativeDataFrame::new(names, cols)))
}

#[test]
fn test_sql_select_filter_order_limit() {
    let df = create_sample_products_df();
    let mut tables = HashMap::new();
    tables.insert("products".to_string(), df);

    let query = "SELECT name, price FROM products WHERE price >= 400 ORDER BY price DESC LIMIT 2";
    let res = execute_sql(query, &tables).expect("SQL execution failed");

    assert_eq!(res.row_count, 2);
    assert_eq!(res.column_names, vec!["name", "price"]);
    let names = res.columns.get("name").unwrap().to_values();
    assert_eq!(names[0], Value::string("Laptop"));
    assert_eq!(names[1], Value::string("Phone"));
}

#[test]
fn test_sql_vector_cosine_similarity_search() {
    let df = create_sample_products_df();
    let mut tables = HashMap::new();
    tables.insert("products".to_string(), df);

    // Search query vector [1.0, 0.0] (tech laptop search)
    let query = "SELECT name, vector_cosine(emb, [1.0, 0.0]) AS score FROM products WHERE in_stock = true ORDER BY score DESC LIMIT 1";
    let res = execute_sql(query, &tables).expect("Vector SQL execution failed");

    assert_eq!(res.row_count, 1);
    let names = res.columns.get("name").unwrap().to_values();
    assert_eq!(names[0], Value::string("Laptop"));

    let scores = res.columns.get("score").unwrap().to_values();
    if let Value::Float(score) = scores[0] {
        assert!(score > 0.95); // High cosine similarity
    } else {
        panic!("Expected float score");
    }
}

#[test]
fn test_sql_vector_l2_and_dot() {
    let df = create_sample_products_df();
    let mut tables = HashMap::new();
    tables.insert("products".to_string(), df);

    let query = "SELECT name, vector_dot(emb, [1.0, 1.0]) AS dot, vector_l2(emb, [0.0, 0.0]) AS l2 FROM products LIMIT 1";
    let res = execute_sql(query, &tables).expect("Vector dot & l2 query failed");

    assert_eq!(res.row_count, 1);
    let dots = res.columns.get("dot").unwrap().to_values();
    // For Laptop [0.9, 0.1] dot [1.0, 1.0] = 1.0
    assert_eq!(dots[0], Value::Float(1.0));
}

#[test]
fn test_sql_computed_columns_and_aliases() {
    let mut cols = HashMap::new();
    cols.insert("item".to_string(), TypedColumn::String(vec!["A".into(), "B".into()]));
    cols.insert("price".to_string(), TypedColumn::Float(vec![10.0, 20.0]));
    cols.insert("qty".to_string(), TypedColumn::Int(vec![5, 3]));
    let df = Arc::new(Mutex::new(NativeDataFrame::new(vec!["item".into(), "price".into(), "qty".into()], cols)));

    let mut tables = HashMap::new();
    tables.insert("sales".to_string(), df);

    let query = "SELECT item, price * qty AS total FROM sales";
    let res = execute_sql(query, &tables).expect("Computed columns query failed");

    assert_eq!(res.row_count, 2);
    let totals = res.columns.get("total").unwrap().to_values();
    assert_eq!(totals[0], Value::Float(50.0));
    assert_eq!(totals[1], Value::Float(60.0));
}

#[test]
fn test_sql_wildcard_select() {
    let df = create_sample_products_df();
    let mut tables = HashMap::new();
    tables.insert("products".to_string(), df);

    let query = "SELECT * FROM products WHERE in_stock = true";
    let res = execute_sql(query, &tables).expect("Wildcard query failed");

    assert_eq!(res.row_count, 3); // Laptop, Phone, Monitor
    assert_eq!(res.column_names.len(), 5);
}

#[test]
fn test_sql_aether_stdlib_script() {
    let code = r#"
from aether_df import DataFrame
from aether_sql import SQLContext, sql

# 1. Create source DataFrame
let items_df = DataFrame({
    "sku": ["SKU-001", "SKU-002", "SKU-003", "SKU-004"],
    "category": ["GPU", "CPU", "RAM", "GPU"],
    "price": [999.0, 450.0, 120.0, 650.0],
    "stock": [5, 12, 50, 0]
})

# 2. Query via SQLContext
let ctx = SQLContext()
ctx.register("inventory", items_df)

let high_end_gpus = ctx.query("SELECT sku, price FROM inventory WHERE category = 'GPU' AND stock > 0 ORDER BY price DESC")
let gpus_shape = high_end_gpus.shape()

[
    gpus_shape[0],
    gpus_shape[1],
    high_end_gpus["sku"][0],
    high_end_gpus["price"][0]
]
"#;

    let res = run_source(code).expect("AetherSQL script execution failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(1)); // 1 GPU with stock > 0 (SKU-001)
        assert_eq!(items[1], Value::Int(2)); // sku, price
        assert_eq!(items[2], Value::string("SKU-001"));
        assert_eq!(items[3], Value::Float(999.0));
    } else {
        panic!("Expected array, got {:?}", res);
    }
}
