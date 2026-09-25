use aether::vm::{run_source, Value};

#[test]
fn test_aetherdb_in_memory_crud() {
    let code = r#"
db_handle = db.open(":memory:")
col = "users"

# 1. Insert documents
u1 = db.insert(db_handle, col, {"name": "Alice", "role": "admin", "age": 30})
u2 = db.insert(db_handle, col, {"name": "Bob", "role": "user", "age": 25})
u3 = db.insert(db_handle, col, {"name": "Charlie", "role": "user", "age": 35})

# 2. Count
total = db.count(db_handle, col)

# 3. Find one
found_alice = db.find_one(db_handle, col, {"name": "Alice"})

# 4. Update
updated_count = db.update(db_handle, col, {"name": "Bob"}, {"age": 26})
found_bob = db.find_one(db_handle, col, {"name": "Bob"})

# 5. Delete
deleted_count = db.delete(db_handle, col, {"name": "Charlie"})
remaining = db.count(db_handle, col)

[total, found_alice["role"], updated_count, found_bob["age"], deleted_count, remaining]
"#;
    let res = run_source(code).expect("AetherDB in-memory CRUD failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(3)); // total
        assert_eq!(items[1], Value::string("admin")); // found_alice["role"]
        assert_eq!(items[2], Value::Int(1)); // updated_count
        assert_eq!(items[3], Value::Int(26)); // found_bob["age"]
        assert_eq!(items[4], Value::Int(1)); // deleted_count
        assert_eq!(items[5], Value::Int(2)); // remaining
    } else {
        panic!("Expected Array result, got {:?}", res);
    }
}

#[test]
fn test_aetherdb_query_operators() {
    let code = r#"
h = db.open(":memory:")
col = "products"

db.insert(h, col, {"sku": "P1", "price": 10.0, "category": "books", "tags": ["rust", "tech"]})
db.insert(h, col, {"sku": "P2", "price": 25.5, "category": "electronics", "tags": ["gadget"]})
db.insert(h, col, {"sku": "P3", "price": 50.0, "category": "electronics", "tags": ["screen", "tech"]})
db.insert(h, col, {"sku": "P4", "price": 5.0, "category": "stationery", "tags": ["pen"]})

# Greater than
gt_res = db.find(h, col, {"price": {"$gt": 20.0}})

# In set
in_res = db.find(h, col, {"category": {"$in": ["books", "stationery"]}})

# Contains substring / element
contains_res = db.find(h, col, {"tags": {"$contains": "tech"}})

[len(gt_res), len(in_res), len(contains_res)]
"#;
    let res = run_source(code).expect("AetherDB query operators failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(2)); // P2, P3
        assert_eq!(items[1], Value::Int(2)); // P1, P4
        assert_eq!(items[2], Value::Int(2)); // P1, P3
    } else {
        panic!("Expected Array result, got {:?}", res);
    }
}

#[test]
fn test_aetherdb_file_persistence_and_wal_recovery() {
    let temp_dir = std::env::temp_dir();
    let db_path = temp_dir.join("aether_test_store.json");
    let db_str = db_path.to_str().unwrap().replace('\\', "/");

    // Clean up if exists
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(format!("{}.wal", db_str));

    let script1 = format!(r#"
h = db.open("{db_str}")
db.insert(h, "items", {{"title": "Note 1", "score": 90}})
db.insert(h, "items", {{"title": "Note 2", "score": 80}})
# WAL will receive update
db.update(h, "items", {{"title": "Note 1"}}, {{"score": 95}})
# Close compacts database snapshot
db.close(h)
"#, db_str = db_str);

    let _ = run_source(&script1).expect("Script 1 failed");

    // Reopen database from disk and verify persisted + replayed state
    let script2 = format!(r#"
h2 = db.open("{db_str}")
docs = db.find(h2, "items")
count = db.count(h2, "items")
note1 = db.find_one(h2, "items", {{"title": "Note 1"}})
db.close(h2)
[count, note1["score"]]
"#, db_str = db_str);

    let res = run_source(&script2).expect("Script 2 failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(2)); // count
        assert_eq!(items[1], Value::Int(95)); // updated score
    } else {
        panic!("Expected Array result, got {:?}", res);
    }

    // Clean up
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(format!("{}.wal", db_str));
}

#[test]
fn test_aetherdb_oop_wrapper() {
    let code = r#"
from aether_db import Database

database = Database(":memory:")
users = database.collection("accounts")

users.insert({"username": "devsami", "balance": 1000})
users.insert({"username": "antigravity", "balance": 5000})

initial_count = users.count()
users.update({"username": "devsami"}, {"balance": 1500})
updated_user = users.find_one({"username": "devsami"})

[initial_count, updated_user["balance"]]
"#;
    let res = run_source(code).expect("AetherDB OOP wrapper failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(2));
        assert_eq!(items[1], Value::Int(1500));
    } else {
        panic!("Expected Array result, got {:?}", res);
    }
}
