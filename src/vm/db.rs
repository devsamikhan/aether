use super::value::Value;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

static NEXT_HANDLE_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_DOC_ID: AtomicU64 = AtomicU64::new(1001);

/// Internal state of an active database instance
struct DbInstance {
    path: String,
    is_memory: bool,
    collections: HashMap<String, Vec<HashMap<String, Value>>>,
    wal_file: Option<File>,
}

static DATABASES: std::sync::OnceLock<Mutex<HashMap<u64, Arc<Mutex<DbInstance>>>>> = std::sync::OnceLock::new();

fn get_databases() -> &'static Mutex<HashMap<u64, Arc<Mutex<DbInstance>>>> {
    DATABASES.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn register_db_module(globals: &mut HashMap<String, Value>) {
    let mut db_module = HashMap::new();

    // 1. Database.open(path)
    db_module.insert(
        "open".to_string(),
        Value::Native("Database.open".into(), |args| {
            let path_str = if args.is_empty() {
                ":memory:".to_string()
            } else {
                format!("{}", args[0])
            };

            let is_memory = path_str == ":memory:";
            let handle_id = NEXT_HANDLE_ID.fetch_add(1, Ordering::SeqCst);

            let mut instance = DbInstance {
                path: path_str.clone(),
                is_memory,
                collections: HashMap::new(),
                wal_file: None,
            };

            if !is_memory {
                load_database_from_disk(&mut instance)?;
                let wal_path = format!("{}.wal", path_str);
                let wal = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&wal_path)
                    .map_err(|e| format!("Failed to open WAL file '{}': {}", wal_path, e))?;
                instance.wal_file = Some(wal);
            }

            get_databases().lock().insert(handle_id, Arc::new(Mutex::new(instance)));

            let mut handle_map = HashMap::new();
            handle_map.insert("_db_id".to_string(), Value::Int(handle_id as i64));
            handle_map.insert("path".to_string(), Value::string(path_str));
            Ok(Value::map(handle_map))
        }),
    );

    // 2. Database.close(handle)
    db_module.insert(
        "close".to_string(),
        Value::Native("Database.close".into(), |args| {
            let handle_id = get_handle_id(args)?;
            let mut dbs = get_databases().lock();
            if let Some(db_arc) = dbs.remove(&handle_id) {
                let mut db = db_arc.lock();
                if !db.is_memory {
                    compact_database(&mut db)?;
                }
            }
            Ok(Value::Bool(true))
        }),
    );

    // 3. Database.insert(handle, collection, document)
    db_module.insert(
        "insert".to_string(),
        Value::Native("Database.insert".into(), |args| {
            if args.len() < 3 {
                return Err("Database.insert(handle, collection, document) requires 3 arguments".into());
            }
            let handle_id = get_handle_id(args)?;
            let col_name = format!("{}", args[1]);

            let doc_map = match &args[2] {
                Value::Map(m) => m.lock().clone(),
                other => return Err(format!("Document must be a map, got '{}'", other.type_name())),
            };

            let dbs = get_databases().lock();
            let db_arc = dbs.get(&handle_id).ok_or_else(|| "Invalid database handle".to_string())?;
            let mut db = db_arc.lock();

            let mut final_doc = doc_map.clone();
            if !final_doc.contains_key("_id") {
                let id = NEXT_DOC_ID.fetch_add(1, Ordering::SeqCst);
                final_doc.insert("_id".to_string(), Value::string(format!("doc-{}", id)));
            }

            if !db.is_memory {
                if let Some(ref mut wal) = db.wal_file {
                    let json_line = serialize_wal_op("insert", &col_name, &final_doc);
                    let _ = writeln!(wal, "{}", json_line);
                    let _ = wal.flush();
                }
            }

            let col = db.collections.entry(col_name).or_insert_with(Vec::new);
            col.push(final_doc.clone());

            Ok(Value::map(final_doc))
        }),
    );

    // 4. Database.find(handle, collection, query)
    db_module.insert(
        "find".to_string(),
        Value::Native("Database.find".into(), |args| {
            if args.len() < 2 {
                return Err("Database.find(handle, collection, query?) requires at least 2 arguments".into());
            }
            let handle_id = get_handle_id(args)?;
            let col_name = format!("{}", args[1]);
            let query = if args.len() > 2 {
                match &args[2] {
                    Value::Map(m) => m.lock().clone(),
                    _ => HashMap::new(),
                }
            } else {
                HashMap::new()
            };

            let dbs = get_databases().lock();
            let db_arc = dbs.get(&handle_id).ok_or_else(|| "Invalid database handle".to_string())?;
            let db = db_arc.lock();

            let mut results = Vec::new();
            if let Some(col) = db.collections.get(&col_name) {
                for doc in col {
                    if matches_query(doc, &query) {
                        results.push(Value::map(doc.clone()));
                    }
                }
            }

            Ok(Value::array(results))
        }),
    );

    // 5. Database.find_one(handle, collection, query)
    db_module.insert(
        "find_one".to_string(),
        Value::Native("Database.find_one".into(), |args| {
            if args.len() < 2 {
                return Err("Database.find_one(handle, collection, query?) requires at least 2 arguments".into());
            }
            let handle_id = get_handle_id(args)?;
            let col_name = format!("{}", args[1]);
            let query = if args.len() > 2 {
                match &args[2] {
                    Value::Map(m) => m.lock().clone(),
                    _ => HashMap::new(),
                }
            } else {
                HashMap::new()
            };

            let dbs = get_databases().lock();
            let db_arc = dbs.get(&handle_id).ok_or_else(|| "Invalid database handle".to_string())?;
            let db = db_arc.lock();

            if let Some(col) = db.collections.get(&col_name) {
                for doc in col {
                    if matches_query(doc, &query) {
                        return Ok(Value::map(doc.clone()));
                    }
                }
            }

            Ok(Value::Nil)
        }),
    );

    // 6. Database.update(handle, collection, query, update_data, upsert=false)
    db_module.insert(
        "update".to_string(),
        Value::Native("Database.update".into(), |args| {
            if args.len() < 4 {
                return Err("Database.update(handle, collection, query, update_data) requires at least 4 arguments".into());
            }
            let handle_id = get_handle_id(args)?;
            let col_name = format!("{}", args[1]);
            let query = match &args[2] {
                Value::Map(m) => m.lock().clone(),
                _ => HashMap::new(),
            };
            let update_data = match &args[3] {
                Value::Map(m) => m.lock().clone(),
                other => return Err(format!("Update data must be map, got '{}'", other.type_name())),
            };
            let upsert = args.len() > 4 && args[4].is_truthy();

            let dbs = get_databases().lock();
            let db_arc = dbs.get(&handle_id).ok_or_else(|| "Invalid database handle".to_string())?;
            let mut db = db_arc.lock();

            let mut count = 0;
            let mut modified_any = false;
            if let Some(col) = db.collections.get_mut(&col_name) {
                for doc in col.iter_mut() {
                    if matches_query(doc, &query) {
                        for (k, v) in &update_data {
                            if k != "_id" {
                                doc.insert(k.clone(), v.clone());
                            }
                        }
                        count += 1;
                        modified_any = true;
                    }
                }
            }

            if !modified_any && upsert {
                let mut new_doc = query.clone();
                for (k, v) in &update_data {
                    new_doc.insert(k.clone(), v.clone());
                }
                if !new_doc.contains_key("_id") {
                    let id = NEXT_DOC_ID.fetch_add(1, Ordering::SeqCst);
                    new_doc.insert("_id".to_string(), Value::string(format!("doc-{}", id)));
                }
                let col = db.collections.entry(col_name.clone()).or_insert_with(Vec::new);
                col.push(new_doc.clone());
                count = 1;
            }

            if !db.is_memory && count > 0 {
                if let Some(ref mut wal) = db.wal_file {
                    let mut op_data = HashMap::new();
                    op_data.insert("query".to_string(), Value::map(query));
                    op_data.insert("update".to_string(), Value::map(update_data));
                    let json_line = serialize_wal_op("update", &col_name, &op_data);
                    let _ = writeln!(wal, "{}", json_line);
                    let _ = wal.flush();
                }
            }

            Ok(Value::Int(count))
        }),
    );

    // 7. Database.delete(handle, collection, query)
    db_module.insert(
        "delete".to_string(),
        Value::Native("Database.delete".into(), |args| {
            if args.len() < 3 {
                return Err("Database.delete(handle, collection, query) requires 3 arguments".into());
            }
            let handle_id = get_handle_id(args)?;
            let col_name = format!("{}", args[1]);
            let query = match &args[2] {
                Value::Map(m) => m.lock().clone(),
                _ => HashMap::new(),
            };

            let dbs = get_databases().lock();
            let db_arc = dbs.get(&handle_id).ok_or_else(|| "Invalid database handle".to_string())?;
            let mut db = db_arc.lock();

            let mut count = 0;
            if let Some(col) = db.collections.get_mut(&col_name) {
                let initial_len = col.len();
                col.retain(|doc| !matches_query(doc, &query));
                count = (initial_len - col.len()) as i64;
            }

            if !db.is_memory && count > 0 {
                if let Some(ref mut wal) = db.wal_file {
                    let mut op_data = HashMap::new();
                    op_data.insert("query".to_string(), Value::map(query));
                    let json_line = serialize_wal_op("delete", &col_name, &op_data);
                    let _ = writeln!(wal, "{}", json_line);
                    let _ = wal.flush();
                }
            }

            Ok(Value::Int(count))
        }),
    );

    // 8. Database.count(handle, collection, query)
    db_module.insert(
        "count".to_string(),
        Value::Native("Database.count".into(), |args| {
            if args.len() < 2 {
                return Err("Database.count(handle, collection, query?) requires at least 2 arguments".into());
            }
            let handle_id = get_handle_id(args)?;
            let col_name = format!("{}", args[1]);
            let query = if args.len() > 2 {
                match &args[2] {
                    Value::Map(m) => m.lock().clone(),
                    _ => HashMap::new(),
                }
            } else {
                HashMap::new()
            };

            let dbs = get_databases().lock();
            let db_arc = dbs.get(&handle_id).ok_or_else(|| "Invalid database handle".to_string())?;
            let db = db_arc.lock();

            let mut count = 0;
            if let Some(col) = db.collections.get(&col_name) {
                for doc in col {
                    if matches_query(doc, &query) {
                        count += 1;
                    }
                }
            }

            Ok(Value::Int(count))
        }),
    );

    // 9. Database.compact(handle)
    db_module.insert(
        "compact".to_string(),
        Value::Native("Database.compact".into(), |args| {
            let handle_id = get_handle_id(args)?;
            let dbs = get_databases().lock();
            let db_arc = dbs.get(&handle_id).ok_or_else(|| "Invalid database handle".to_string())?;
            let mut db = db_arc.lock();
            compact_database(&mut db)?;
            Ok(Value::Bool(true))
        }),
    );

    globals.entry("__native_db".to_string()).or_insert_with(|| Value::map(db_module.clone()));
    globals.entry("Database".to_string()).or_insert_with(|| Value::map(db_module.clone()));
    globals.entry("db".to_string()).or_insert_with(|| Value::map(db_module));
}

// -----------------------------------------------------------------------------
// Helper Functions for Queries and Persistence
// -----------------------------------------------------------------------------

fn get_handle_id(args: &[Value]) -> Result<u64, String> {
    if args.is_empty() {
        return Err("Missing database handle".into());
    }
    match &args[0] {
        Value::Map(m) => {
            let map = m.lock();
            if let Some(Value::Int(id)) = map.get("_db_id") {
                Ok(*id as u64)
            } else {
                Err("Invalid database handle structure".into())
            }
        }
        Value::Int(id) => Ok(*id as u64),
        other => Err(format!("Expected database handle, got '{}'", other.type_name())),
    }
}

/// Evaluates if a document matches the query criteria
fn matches_query(doc: &HashMap<String, Value>, query: &HashMap<String, Value>) -> bool {
    for (key, query_val) in query {
        match query_val {
            Value::Map(op_map) => {
                let ops = op_map.lock();
                let doc_field = doc.get(key).unwrap_or(&Value::Nil);

                for (op, target_val) in ops.iter() {
                    match op.as_str() {
                        "$eq" => {
                            if doc_field != target_val {
                                return false;
                            }
                        }
                        "$ne" => {
                            if doc_field == target_val {
                                return false;
                            }
                        }
                        "$gt" => {
                            if !compare_vals(doc_field, target_val, |c| c > 0) {
                                return false;
                            }
                        }
                        "$gte" => {
                            if !compare_vals(doc_field, target_val, |c| c >= 0) {
                                return false;
                            }
                        }
                        "$lt" => {
                            if !compare_vals(doc_field, target_val, |c| c < 0) {
                                return false;
                            }
                        }
                        "$lte" => {
                            if !compare_vals(doc_field, target_val, |c| c <= 0) {
                                return false;
                            }
                        }
                        "$in" => {
                            if let Value::Array(arr) = target_val {
                                let list = arr.lock();
                                if !list.iter().any(|item| item == doc_field) {
                                    return false;
                                }
                            } else {
                                return false;
                            }
                        }
                        "$contains" => {
                            let doc_str = format!("{}", doc_field);
                            let target_str = format!("{}", target_val);
                            if !doc_str.contains(&target_str) {
                                return false;
                            }
                        }
                        _ => return false,
                    }
                }
            }
            _ => {
                let doc_field = doc.get(key).unwrap_or(&Value::Nil);
                if doc_field != query_val {
                    return false;
                }
            }
        }
    }
    true
}

fn compare_vals<F>(a: &Value, b: &Value, cmp_fn: F) -> bool
where
    F: FnOnce(i32) -> bool,
{
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => cmp_fn(x.cmp(y) as i32),
        (Value::Float(x), Value::Float(y)) => {
            if x < y {
                cmp_fn(-1)
            } else if x > y {
                cmp_fn(1)
            } else {
                cmp_fn(0)
            }
        }
        (Value::Int(x), Value::Float(y)) => {
            let xf = *x as f64;
            if xf < *y {
                cmp_fn(-1)
            } else if xf > *y {
                cmp_fn(1)
            } else {
                cmp_fn(0)
            }
        }
        (Value::Float(x), Value::Int(y)) => {
            let yf = *y as f64;
            if *x < yf {
                cmp_fn(-1)
            } else if *x > yf {
                cmp_fn(1)
            } else {
                cmp_fn(0)
            }
        }
        (Value::String(x), Value::String(y)) => cmp_fn(x.cmp(y) as i32),
        _ => false,
    }
}

fn serialize_wal_op(op: &str, collection: &str, data: &HashMap<String, Value>) -> String {
    let mut root = HashMap::new();
    root.insert("op".to_string(), Value::string(op));
    root.insert("col".to_string(), Value::string(collection));
    root.insert("data".to_string(), Value::map(data.clone()));
    serde_json::to_string(&Value::map(root).to_json()).unwrap_or_else(|_| "{}".to_string())
}

fn compact_database(db: &mut DbInstance) -> Result<(), String> {
    if db.is_memory {
        return Ok(());
    }

    // 1. Write current state to primary file
    let mut root_cols = HashMap::new();
    for (col_name, docs) in &db.collections {
        let doc_values: Vec<Value> = docs.iter().map(|d| Value::map(d.clone())).collect();
        root_cols.insert(col_name.clone(), Value::array(doc_values));
    }
    let full_json = serde_json::to_string_pretty(&Value::map(root_cols).to_json())
        .map_err(|e| format!("Failed to serialize database: {}", e))?;

    let path = Path::new(&db.path);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&db.path, full_json).map_err(|e| format!("Failed to write database snapshot: {}", e))?;

    // 2. Truncate WAL file
    let wal_path = format!("{}.wal", db.path);
    let wal = File::create(&wal_path).map_err(|e| format!("Failed to truncate WAL: {}", e))?;
    db.wal_file = Some(wal);

    Ok(())
}

fn load_database_from_disk(db: &mut DbInstance) -> Result<(), String> {
    // 1. Read primary snapshot
    if Path::new(&db.path).exists() {
        if let Ok(content) = std::fs::read_to_string(&db.path) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                if let serde_json::Value::Object(cols) = parsed {
                    for (col_name, col_val) in cols.iter() {
                        if let serde_json::Value::Array(docs) = col_val {
                            let mut list = Vec::new();
                            for doc in docs {
                                let val = Value::from_json(doc);
                                if let Value::Map(m) = val {
                                    list.push(m.lock().clone());
                                }
                            }
                            db.collections.insert(col_name.to_string(), list);
                        }
                    }
                }
            }
        }
    }

    // 2. Replay WAL
    let wal_path = format!("{}.wal", db.path);
    if Path::new(&wal_path).exists() {
        if let Ok(file) = File::open(&wal_path) {
            let reader = BufReader::new(file);
            for line_res in reader.lines() {
                if let Ok(line) = line_res {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&line) {
                        apply_wal_entry(db, &parsed);
                    }
                }
            }
        }
    }

    Ok(())
}

fn apply_wal_entry(db: &mut DbInstance, entry: &serde_json::Value) {
    let op = entry.get("op").and_then(|v| v.as_str()).unwrap_or("");
    let col_name = entry.get("col").and_then(|v| v.as_str()).unwrap_or("");
    let data_val = match entry.get("data") {
        Some(d) => Value::from_json(d),
        None => return,
    };

    let data_map = match data_val {
        Value::Map(m) => m.lock().clone(),
        _ => return,
    };

    match op {
        "insert" => {
            let col = db.collections.entry(col_name.to_string()).or_insert_with(Vec::new);
            col.push(data_map);
        }
        "update" => {
            let query = match data_map.get("query") {
                Some(Value::Map(m)) => m.lock().clone(),
                _ => HashMap::new(),
            };
            let update_data = match data_map.get("update") {
                Some(Value::Map(m)) => m.lock().clone(),
                _ => HashMap::new(),
            };
            if let Some(col) = db.collections.get_mut(col_name) {
                for doc in col.iter_mut() {
                    if matches_query(doc, &query) {
                        for (k, v) in &update_data {
                            if k != "_id" {
                                doc.insert(k.clone(), v.clone());
                            }
                        }
                    }
                }
            }
        }
        "delete" => {
            let query = match data_map.get("query") {
                Some(Value::Map(m)) => m.lock().clone(),
                _ => HashMap::new(),
            };
            if let Some(col) = db.collections.get_mut(col_name) {
                col.retain(|doc| !matches_query(doc, &query));
            }
        }
        _ => {}
    }
}
