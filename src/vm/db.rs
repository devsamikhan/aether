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
                if let Some(parent) = Path::new(&path_str).parent() {
                    if !parent.as_os_str().is_empty() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                }
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

    // 10. Database.execute(handle, sql_statement)
    db_module.insert(
        "execute".to_string(),
        Value::Native("Database.execute".into(), |args| {
            if args.len() < 2 {
                return Err("Database.execute(handle, sql_statement) requires at least 2 arguments".into());
            }
            let handle_id = get_handle_id(args)?;
            let sql_str = format!("{}", args[1]);

            let dbs = get_databases().lock();
            let db_arc = dbs.get(&handle_id).ok_or_else(|| "Invalid database handle".to_string())?;
            let mut db = db_arc.lock();

            execute_sql(&mut db, &sql_str)
        }),
    );

    // 11. Database.query(handle, sql_query, [params]) -> Array of Map rows
    db_module.insert(
        "query".to_string(),
        Value::Native("Database.query".into(), |args| {
            if args.len() < 2 {
                return Err("Database.query(handle, sql_query, [params]) requires at least 2 arguments".into());
            }
            let handle_id = get_handle_id(args)?;
            let sql_str = format!("{}", args[1]);

            let empty_vec = Vec::new();
            let params = if args.len() > 2 {
                match &args[2] {
                    Value::Array(arr) => arr.lock().clone(),
                    _ => empty_vec,
                }
            } else {
                empty_vec
            };

            let dbs = get_databases().lock();
            let db_arc = dbs.get(&handle_id).ok_or_else(|| "Invalid database handle".to_string())?;
            let db = db_arc.lock();

            let rows = query_sql(&db, &sql_str, &params)?;
            let array_val = rows.into_iter().map(Value::map).collect();
            Ok(Value::array(array_val))
        }),
    );

    // 12. Database.tables(handle) -> Array of table names
    db_module.insert(
        "tables".to_string(),
        Value::Native("Database.tables".into(), |args| {
            let handle_id = get_handle_id(args)?;
            let dbs = get_databases().lock();
            let db_arc = dbs.get(&handle_id).ok_or_else(|| "Invalid database handle".to_string())?;
            let db = db_arc.lock();

            let mut names: Vec<Value> = db.collections.keys().map(|k| Value::string(k.clone())).collect();
            names.sort_by_key(|a| format!("{}", a));
            Ok(Value::array(names))
        }),
    );

    globals.entry("__native_db".to_string()).or_insert_with(|| Value::map(db_module.clone()));
    globals.entry("Database".to_string()).or_insert_with(|| Value::map(db_module.clone()));
    globals.entry("DB".to_string()).or_insert_with(|| Value::map(db_module.clone()));
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

// -----------------------------------------------------------------------------
// SQL Relational Query Engine Implementation
// -----------------------------------------------------------------------------

fn execute_sql(db: &mut DbInstance, sql: &str) -> Result<Value, String> {
    let trimmed = sql.trim().trim_end_matches(';').trim();
    if trimmed.is_empty() {
        return Ok(Value::Bool(true));
    }

    let upper = trimmed.to_uppercase();
    if upper.starts_with("CREATE TABLE") {
        let rest = trimmed["CREATE TABLE".len()..].trim();
        let rest = if rest.to_uppercase().starts_with("IF NOT EXISTS") {
            rest["IF NOT EXISTS".len()..].trim()
        } else {
            rest
        };
        let table_name = rest
            .split(|c: char| c.is_whitespace() || c == '(')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches(|c| c == '`' || c == '"' || c == '\'');
        if table_name.is_empty() {
            return Err("Invalid CREATE TABLE syntax: missing table name".into());
        }
        db.collections.entry(table_name.to_string()).or_insert_with(Vec::new);
        Ok(Value::Bool(true))
    } else if upper.starts_with("DROP TABLE") {
        let rest = trimmed["DROP TABLE".len()..].trim();
        let rest = if rest.to_uppercase().starts_with("IF EXISTS") {
            rest["IF EXISTS".len()..].trim()
        } else {
            rest
        };
        let table_name = rest.trim().trim_matches(|c| c == '`' || c == '"' || c == '\'');
        db.collections.remove(table_name);
        Ok(Value::Bool(true))
    } else if upper.starts_with("INSERT INTO") {
        let rest = trimmed["INSERT INTO".len()..].trim();
        let (table_part, values_part) = match rest.to_uppercase().find("VALUES") {
            Some(idx) => (&rest[..idx].trim(), &rest[idx + "VALUES".len()..].trim()),
            None => return Err("Invalid INSERT syntax: missing VALUES".into()),
        };

        let (table_name, cols) = if let Some(open_paren) = table_part.find('(') {
            let tname = table_part[..open_paren].trim().trim_matches(|c| c == '`' || c == '"' || c == '\'');
            let end_paren = table_part.rfind(')').unwrap_or(table_part.len());
            let cols_str = &table_part[open_paren + 1..end_paren];
            let cols: Vec<String> = cols_str
                .split(',')
                .map(|s| s.trim().trim_matches(|c| c == '`' || c == '"' || c == '\'').to_string())
                .collect();
            (tname, Some(cols))
        } else {
            (table_part.trim().trim_matches(|c| c == '`' || c == '"' || c == '\''), None)
        };

        let val_trimmed = values_part.trim();
        let inner_vals = if val_trimmed.starts_with('(') && val_trimmed.ends_with(')') {
            &val_trimmed[1..val_trimmed.len() - 1]
        } else {
            val_trimmed
        };

        let parsed_values = split_sql_csv(inner_vals);
        let mut row_map = HashMap::new();

        if let Some(col_names) = cols {
            for (i, col) in col_names.into_iter().enumerate() {
                let v = if i < parsed_values.len() {
                    parse_sql_value(&parsed_values[i])
                } else {
                    Value::Nil
                };
                row_map.insert(col, v);
            }
        } else {
            for (i, val_str) in parsed_values.into_iter().enumerate() {
                row_map.insert(format!("col_{}", i + 1), parse_sql_value(&val_str));
            }
        }

        if !row_map.contains_key("_id") {
            let id = NEXT_DOC_ID.fetch_add(1, Ordering::SeqCst);
            row_map.insert("_id".to_string(), Value::string(format!("doc-{}", id)));
        }

        if !db.is_memory {
            if let Some(ref mut wal) = db.wal_file {
                let json_line = serialize_wal_op("insert", table_name, &row_map);
                let _ = writeln!(wal, "{}", json_line);
                let _ = wal.flush();
            }
        }

        db.collections.entry(table_name.to_string()).or_insert_with(Vec::new).push(row_map);
        Ok(Value::Int(1))
    } else if upper.starts_with("DELETE FROM") {
        let rest = trimmed["DELETE FROM".len()..].trim();
        let (table_name, where_clause) = match rest.to_uppercase().find("WHERE") {
            Some(idx) => (
                rest[..idx].trim().trim_matches(|c| c == '`' || c == '"' || c == '\''),
                Some(rest[idx + "WHERE".len()..].trim()),
            ),
            None => (rest.trim().trim_matches(|c| c == '`' || c == '"' || c == '\''), None),
        };

        let mut deleted = 0;
        if let Some(col) = db.collections.get_mut(table_name) {
            let initial_len = col.len();
            if let Some(where_cond) = where_clause {
                col.retain(|row| !eval_sql_condition(row, where_cond));
            } else {
                col.clear();
            }
            deleted = (initial_len - col.len()) as i64;
        }
        Ok(Value::Int(deleted))
    } else if upper.starts_with("UPDATE") {
        let rest = trimmed["UPDATE".len()..].trim();
        let set_idx = rest.to_uppercase().find("SET").ok_or_else(|| "UPDATE missing SET clause".to_string())?;
        let table_name = rest[..set_idx].trim().trim_matches(|c| c == '`' || c == '"' || c == '\'');
        let after_set = rest[set_idx + "SET".len()..].trim();

        let (set_part, where_clause) = match after_set.to_uppercase().find("WHERE") {
            Some(idx) => (&after_set[..idx].trim(), Some(after_set[idx + "WHERE".len()..].trim())),
            None => (&after_set, None),
        };

        let assignments = split_sql_csv(set_part);
        let mut updates = HashMap::new();
        for assign in assignments {
            if let Some(eq_idx) = assign.find('=') {
                let k = assign[..eq_idx].trim().trim_matches(|c| c == '`' || c == '"' || c == '\'');
                let v = parse_sql_value(assign[eq_idx + 1..].trim());
                updates.insert(k.to_string(), v);
            }
        }

        let mut updated = 0;
        if let Some(col) = db.collections.get_mut(table_name) {
            for row in col.iter_mut() {
                let matches = if let Some(where_cond) = where_clause {
                    eval_sql_condition(row, where_cond)
                } else {
                    true
                };
                if matches {
                    for (k, v) in &updates {
                        row.insert(k.clone(), v.clone());
                    }
                    updated += 1;
                }
            }
        }
        Ok(Value::Int(updated))
    } else {
        Err(format!("Unsupported SQL statement for execute(): {}", sql))
    }
}

fn query_sql(db: &DbInstance, sql: &str, params: &[Value]) -> Result<Vec<HashMap<String, Value>>, String> {
    let mut resolved_sql = sql.to_string();
    for param in params {
        if let Some(pos) = resolved_sql.find('?') {
            let replacement = match param {
                Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                Value::Int(i) => format!("{}", i),
                Value::Float(f) => format!("{}", f),
                Value::Bool(b) => format!("{}", b),
                Value::Nil => "NULL".to_string(),
                other => format!("'{}'", other),
            };
            resolved_sql.replace_range(pos..pos + 1, &replacement);
        }
    }

    let trimmed = resolved_sql.trim().trim_end_matches(';').trim();
    let upper = trimmed.to_uppercase();
    if !upper.starts_with("SELECT") {
        return Err(format!("query() expects SELECT statement, got: {}", sql));
    }

    let from_idx = upper.find("FROM").ok_or_else(|| "SELECT missing FROM clause".to_string())?;
    let select_part = trimmed["SELECT".len()..from_idx].trim();
    let after_from = trimmed[from_idx + "FROM".len()..].trim();

    let mut rest = after_from;
    let mut limit: Option<usize> = None;
    let mut order_by: Option<(String, bool)> = None;
    let mut where_cond: Option<String> = None;

    if let Some(lim_idx) = rest.to_uppercase().rfind("LIMIT") {
        let lim_str = rest[lim_idx + "LIMIT".len()..].trim();
        limit = lim_str.parse::<usize>().ok();
        rest = rest[..lim_idx].trim();
    }

    if let Some(order_idx) = rest.to_uppercase().rfind("ORDER BY") {
        let order_str = rest[order_idx + "ORDER BY".len()..].trim();
        let parts: Vec<&str> = order_str.split_whitespace().collect();
        if !parts.is_empty() {
            let col = parts[0].trim_matches(|c| c == '`' || c == '"' || c == '\'').to_string();
            let desc = parts.len() > 1 && parts[1].eq_ignore_ascii_case("DESC");
            order_by = Some((col, desc));
        }
        rest = rest[..order_idx].trim();
    }

    if let Some(where_idx) = rest.to_uppercase().find("WHERE") {
        let w_str = rest[where_idx + "WHERE".len()..].trim();
        where_cond = Some(w_str.to_string());
        rest = rest[..where_idx].trim();
    }

    let table_name = rest.trim().trim_matches(|c| c == '`' || c == '"' || c == '\'');
    let rows = match db.collections.get(table_name) {
        Some(list) => list,
        None => return Ok(Vec::new()),
    };

    let mut filtered_rows: Vec<HashMap<String, Value>> = Vec::new();
    for row in rows {
        if let Some(ref cond) = where_cond {
            if !eval_sql_condition(row, cond) {
                continue;
            }
        }
        filtered_rows.push(row.clone());
    }

    if let Some((col, desc)) = order_by {
        filtered_rows.sort_by(|a, b| {
            let va = a.get(&col).unwrap_or(&Value::Nil);
            let vb = b.get(&col).unwrap_or(&Value::Nil);
            let ord = compare_values_ord(va, vb);
            if desc {
                ord.reverse()
            } else {
                ord
            }
        });
    }

    if let Some(lim) = limit {
        filtered_rows.truncate(lim);
    }

    let selected_columns: Vec<String> = if select_part == "*" {
        Vec::new()
    } else {
        select_part
            .split(',')
            .map(|s| s.trim().trim_matches(|c| c == '`' || c == '"' || c == '\'').to_string())
            .collect()
    };

    if selected_columns.is_empty() {
        Ok(filtered_rows)
    } else {
        let projected = filtered_rows
            .into_iter()
            .map(|row| {
                let mut new_row = HashMap::new();
                for col in &selected_columns {
                    if let Some(v) = row.get(col) {
                        new_row.insert(col.clone(), v.clone());
                    } else {
                        new_row.insert(col.clone(), Value::Nil);
                    }
                }
                new_row
            })
            .collect();
        Ok(projected)
    }
}

fn split_sql_csv(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut quote_char = ' ';

    for c in s.chars() {
        if (c == '\'' || c == '"') && !in_quote {
            in_quote = true;
            quote_char = c;
            current.push(c);
        } else if in_quote && c == quote_char {
            in_quote = false;
            current.push(c);
        } else if c == ',' && !in_quote {
            result.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(c);
        }
    }
    if !current.trim().is_empty() {
        result.push(current.trim().to_string());
    }
    result
}

fn parse_sql_value(s: &str) -> Value {
    let trimmed = s.trim();
    if (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        || (trimmed.starts_with('"') && trimmed.ends_with('"'))
    {
        if trimmed.len() >= 2 {
            Value::string(&trimmed[1..trimmed.len() - 1])
        } else {
            Value::string("")
        }
    } else if trimmed.eq_ignore_ascii_case("true") {
        Value::Bool(true)
    } else if trimmed.eq_ignore_ascii_case("false") {
        Value::Bool(false)
    } else if trimmed.eq_ignore_ascii_case("null") || trimmed.eq_ignore_ascii_case("nil") {
        Value::Nil
    } else if let Ok(i) = trimmed.parse::<i64>() {
        Value::Int(i)
    } else if let Ok(f) = trimmed.parse::<f64>() {
        Value::Float(f)
    } else {
        Value::string(trimmed)
    }
}

fn eval_sql_condition(row: &HashMap<String, Value>, cond: &str) -> bool {
    let trimmed = cond.trim();
    if trimmed.is_empty() {
        return true;
    }

    for sub_cond in trimmed.split(" AND ") {
        for inner in sub_cond.split(" and ") {
            if !eval_single_cond(row, inner) {
                return false;
            }
        }
    }
    true
}

fn eval_single_cond(row: &HashMap<String, Value>, cond: &str) -> bool {
    let ops = [">=", "<=", "!=", "<>", ">", "<", " LIKE ", " like ", "="];
    for op in ops {
        if let Some(pos) = cond.to_uppercase().find(&op.to_uppercase()) {
            let col_name = cond[..pos].trim().trim_matches(|c| c == '`' || c == '"' || c == '\'');
            let val_str = cond[pos + op.len()..].trim();
            let row_val = row.get(col_name).unwrap_or(&Value::Nil);
            let target_val = parse_sql_value(val_str);

            return match op.trim().to_uppercase().as_str() {
                "=" => row_val == &target_val,
                "!=" | "<>" => row_val != &target_val,
                ">" => compare_vals(row_val, &target_val, |c| c > 0),
                ">=" => compare_vals(row_val, &target_val, |c| c >= 0),
                "<" => compare_vals(row_val, &target_val, |c| c < 0),
                "<=" => compare_vals(row_val, &target_val, |c| c <= 0),
                "LIKE" => {
                    let r_str = format!("{}", row_val);
                    let pattern = format!("{}", target_val).replace('%', "");
                    r_str.contains(&pattern)
                }
                _ => false,
            };
        }
    }
    true
}

fn compare_values_ord(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.cmp(y),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (Value::Int(x), Value::Float(y)) => (*x as f64).partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (Value::Float(x), Value::Int(y)) => x.partial_cmp(&(*y as f64)).unwrap_or(std::cmp::Ordering::Equal),
        (Value::String(x), Value::String(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    }
}
