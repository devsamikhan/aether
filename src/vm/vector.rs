use super::value::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Debug, Clone)]
pub struct VectorRecord {
    pub id: String,
    pub vector: Vec<f64>,
    pub metadata: Value,
}

#[derive(Debug, Default)]
pub struct VectorIndexStore {
    pub records: HashMap<String, VectorRecord>,
}

static NEXT_INDEX_ID: AtomicU64 = AtomicU64::new(1);
static VECTOR_STORES: OnceLock<Mutex<HashMap<u64, Arc<Mutex<VectorIndexStore>>>>> = OnceLock::new();

fn get_vector_stores() -> &'static Mutex<HashMap<u64, Arc<Mutex<VectorIndexStore>>>> {
    VECTOR_STORES.get_or_init(|| Mutex::new(HashMap::new()))
}

// ==============================================================================
// Pure Mathematical Primitives
// ==============================================================================

pub fn dot_product(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum()
}

pub fn magnitude(v: &[f64]) -> f64 {
    dot_product(v, v).sqrt()
}

pub fn normalize(v: &[f64]) -> Vec<f64> {
    let mag = magnitude(v);
    if mag == 0.0 {
        v.to_vec()
    } else {
        v.iter().map(|&x| x / mag).collect()
    }
}

pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mag_a = magnitude(a);
    let mag_b = magnitude(b);
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    let dot = dot_product(a, b);
    let cos = dot / (mag_a * mag_b);
    cos.clamp(-1.0, 1.0)
}

pub fn euclidean_distance(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() {
        return f64::INFINITY;
    }
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| {
            let diff = x - y;
            diff * diff
        })
        .sum::<f64>()
        .sqrt()
}

fn extract_f64_slice(val: &Value) -> Result<Vec<f64>, String> {
    match val {
        Value::Array(arr) => {
            let items = arr.lock();
            let mut res = Vec::with_capacity(items.len());
            for item in items.iter() {
                match item {
                    Value::Float(f) => res.push(*f),
                    Value::Int(i) => res.push(*i as f64),
                    _ => return Err("Vector elements must be numbers".to_string()),
                }
            }
            Ok(res)
        }
        _ => Err("Expected array of numbers for vector".to_string()),
    }
}

// ==============================================================================
// Native Vector Module Registration
// ==============================================================================

pub fn register_vector_module(globals: &mut HashMap<String, Value>) {
    let mut mod_map = HashMap::new();

    // 1. Vector.create_index() -> index_id
    mod_map.insert(
        "create_index".to_string(),
        Value::Native("Vector.create_index".into(), |_args| {
            let id = NEXT_INDEX_ID.fetch_add(1, Ordering::SeqCst);
            let store = Arc::new(Mutex::new(VectorIndexStore::default()));
            get_vector_stores().lock().unwrap().insert(id, store);
            Ok(Value::Int(id as i64))
        }),
    );

    // 2. Vector.add(index_id, item_id, vector, [metadata]) -> bool
    mod_map.insert(
        "add".to_string(),
        Value::Native("Vector.add".into(), |args| {
            if args.len() < 3 {
                return Err("Vector.add(index_id, item_id, vector, [metadata]) requires at least 3 arguments".into());
            }
            let index_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("index_id must be integer".into()),
            };
            let item_id = args[1].to_string();
            let vector = extract_f64_slice(&args[2])?;
            let metadata = if args.len() > 3 {
                args[3].clone()
            } else {
                Value::map(HashMap::new())
            };

            let stores = get_vector_stores().lock().unwrap();
            let store_arc = stores.get(&index_id).ok_or_else(|| format!("Vector store {} not found", index_id))?;
            let mut store = store_arc.lock().unwrap();

            store.records.insert(
                item_id.clone(),
                VectorRecord {
                    id: item_id,
                    vector,
                    metadata,
                },
            );

            Ok(Value::Bool(true))
        }),
    );

    // 3. Vector.search(index_id, query_vec, top_k, metric, [filter_key, filter_val]) -> [ {id, score, metadata} ]
    mod_map.insert(
        "search".to_string(),
        Value::Native("Vector.search".into(), |args| {
            if args.len() < 2 {
                return Err("Vector.search(index_id, query_vec, [top_k], [metric], [filter_key, filter_val]) requires at least 2 arguments".into());
            }
            let index_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("index_id must be integer".into()),
            };
            let query_vec = extract_f64_slice(&args[1])?;

            let top_k = if args.len() > 2 {
                match args[2] {
                    Value::Int(k) if k > 0 => k as usize,
                    _ => 5,
                }
            } else {
                5
            };

            let metric = if args.len() > 3 {
                args[3].to_string()
            } else {
                "cosine".to_string()
            };

            let filter_key = if args.len() > 4 && args[4] != Value::Nil {
                Some(args[4].to_string())
            } else {
                None
            };
            let filter_val = if args.len() > 5 {
                Some(&args[5])
            } else {
                None
            };

            let stores = get_vector_stores().lock().unwrap();
            let store_arc = stores.get(&index_id).ok_or_else(|| format!("Vector store {} not found", index_id))?;
            let store = store_arc.lock().unwrap();

            let mut scored: Vec<(f64, &VectorRecord)> = Vec::new();

            for record in store.records.values() {
                // Check metadata filter if specified
                if let Some(ref fk) = filter_key {
                    if let Value::Map(m) = &record.metadata {
                        let map = m.lock();
                        match map.get(fk) {
                            Some(v) => {
                                if let Some(target) = filter_val {
                                    if v != target {
                                        continue;
                                    }
                                }
                            }
                            None => continue,
                        }
                    } else {
                        continue;
                    }
                }

                let score = match metric.as_str() {
                    "euclidean" => euclidean_distance(&query_vec, &record.vector),
                    "dot" => dot_product(&query_vec, &record.vector),
                    _ => cosine_similarity(&query_vec, &record.vector),
                };

                scored.push((score, record));
            }

            // Sort: For cosine & dot, higher is better (descending); For euclidean, lower is better (ascending)
            if metric == "euclidean" {
                scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            } else {
                scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            }

            let results: Vec<Value> = scored
                .into_iter()
                .take(top_k)
                .map(|(score, rec)| {
                    let mut entry = HashMap::new();
                    entry.insert("id".to_string(), Value::string(rec.id.clone()));
                    entry.insert("score".to_string(), Value::Float(score));
                    entry.insert("metadata".to_string(), rec.metadata.clone());
                    Value::map(entry)
                })
                .collect();

            Ok(Value::array(results))
        }),
    );

    // 4. Vector.get(index_id, item_id) -> map or nil
    mod_map.insert(
        "get".to_string(),
        Value::Native("Vector.get".into(), |args| {
            if args.len() < 2 {
                return Err("Vector.get(index_id, item_id) requires 2 arguments".into());
            }
            let index_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("index_id must be integer".into()),
            };
            let item_id = args[1].to_string();

            let stores = get_vector_stores().lock().unwrap();
            let store_arc = stores
                .get(&index_id)
                .ok_or_else(|| format!("Vector store {} not found", index_id))?;
            let store = store_arc.lock().unwrap();

            if let Some(rec) = store.records.get(&item_id) {
                let mut map = HashMap::new();
                map.insert("id".to_string(), Value::string(rec.id.clone()));
                let vec_vals: Vec<Value> = rec.vector.iter().map(|&x| Value::Float(x)).collect();
                map.insert("vector".to_string(), Value::array(vec_vals));
                map.insert("metadata".to_string(), rec.metadata.clone());
                Ok(Value::map(map))
            } else {
                Ok(Value::Nil)
            }
        }),
    );

    // 5. Vector.delete(index_id, item_id) -> bool
    mod_map.insert(
        "delete".to_string(),
        Value::Native("Vector.delete".into(), |args| {
            if args.len() < 2 {
                return Err("Vector.delete(index_id, item_id) requires 2 arguments".into());
            }
            let index_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("index_id must be integer".into()),
            };
            let item_id = args[1].to_string();

            let stores = get_vector_stores().lock().unwrap();
            let store_arc = stores
                .get(&index_id)
                .ok_or_else(|| format!("Vector store {} not found", index_id))?;
            let mut store = store_arc.lock().unwrap();

            Ok(Value::Bool(store.records.remove(&item_id).is_some()))
        }),
    );

    // 6. Vector.count(index_id) -> int
    mod_map.insert(
        "count".to_string(),
        Value::Native("Vector.count".into(), |args| {
            if args.is_empty() {
                return Err("Vector.count(index_id) requires index_id".into());
            }
            let index_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("index_id must be integer".into()),
            };
            let stores = get_vector_stores().lock().unwrap();
            let store_arc = stores
                .get(&index_id)
                .ok_or_else(|| format!("Vector store {} not found", index_id))?;
            let store = store_arc.lock().unwrap();
            Ok(Value::Int(store.records.len() as i64))
        }),
    );

    // 7. Vector.clear(index_id) -> bool
    mod_map.insert(
        "clear".to_string(),
        Value::Native("Vector.clear".into(), |args| {
            if args.is_empty() {
                return Err("Vector.clear(index_id) requires index_id".into());
            }
            let index_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("index_id must be integer".into()),
            };
            let stores = get_vector_stores().lock().unwrap();
            let store_arc = stores
                .get(&index_id)
                .ok_or_else(|| format!("Vector store {} not found", index_id))?;
            let mut store = store_arc.lock().unwrap();
            store.records.clear();
            Ok(Value::Bool(true))
        }),
    );

    // 8. Vector.cosine_similarity(v1, v2) -> float
    mod_map.insert(
        "cosine_similarity".to_string(),
        Value::Native("Vector.cosine_similarity".into(), |args| {
            if args.len() < 2 {
                return Err("cosine_similarity(v1, v2) requires 2 vector arguments".into());
            }
            let v1 = extract_f64_slice(&args[0])?;
            let v2 = extract_f64_slice(&args[1])?;
            Ok(Value::Float(cosine_similarity(&v1, &v2)))
        }),
    );

    // 9. Vector.euclidean_distance(v1, v2) -> float
    mod_map.insert(
        "euclidean_distance".to_string(),
        Value::Native("Vector.euclidean_distance".into(), |args| {
            if args.len() < 2 {
                return Err("euclidean_distance(v1, v2) requires 2 vector arguments".into());
            }
            let v1 = extract_f64_slice(&args[0])?;
            let v2 = extract_f64_slice(&args[1])?;
            Ok(Value::Float(euclidean_distance(&v1, &v2)))
        }),
    );

    // 10. Vector.dot_product(v1, v2) -> float
    mod_map.insert(
        "dot_product".to_string(),
        Value::Native("Vector.dot_product".into(), |args| {
            if args.len() < 2 {
                return Err("dot_product(v1, v2) requires 2 vector arguments".into());
            }
            let v1 = extract_f64_slice(&args[0])?;
            let v2 = extract_f64_slice(&args[1])?;
            Ok(Value::Float(dot_product(&v1, &v2)))
        }),
    );

    // 11. Vector.normalize(v) -> [float]
    mod_map.insert(
        "normalize".to_string(),
        Value::Native("Vector.normalize".into(), |args| {
            if args.is_empty() {
                return Err("normalize(v) requires vector argument".into());
            }
            let v = extract_f64_slice(&args[0])?;
            let norm = normalize(&v);
            let items: Vec<Value> = norm.into_iter().map(Value::Float).collect();
            Ok(Value::array(items))
        }),
    );

    // 12. Vector.save(index_id, path) -> Bool (Persistent JSON disk storage)
    mod_map.insert(
        "save".to_string(),
        Value::Native("Vector.save".into(), |args| {
            if args.len() < 2 {
                return Err("Vector.save(index_id, path) requires 2 arguments".into());
            }
            let index_id = match args[0] {
                Value::Int(i) if i > 0 => i as u64,
                _ => return Err("index_id must be positive integer".into()),
            };
            let path = format!("{}", args[1]);
            crate::vm::interpreter::with_current_vm(|vm| vm.security_policy.check_fs_write(&path))?;

            let stores = get_vector_stores().lock().unwrap();
            let store_arc = stores
                .get(&index_id)
                .cloned()
                .ok_or_else(|| format!("Vector store {} not found", index_id))?;
            drop(stores);

            let store = store_arc.lock().unwrap();
            let serialized_records: Vec<serde_json::Value> = store
                .records
                .values()
                .map(|rec| {
                    serde_json::json!({
                        "id": rec.id,
                        "vector": rec.vector,
                        "metadata": value_to_json_vec(&rec.metadata),
                    })
                })
                .collect();
            let payload = serde_json::json!({ "records": serialized_records });
            let text = serde_json::to_string_pretty(&payload)
                .map_err(|e| format!("Failed to serialize vector index: {}", e))?;
            std::fs::write(&path, text)
                .map_err(|e| format!("Failed to write vector index '{}': {}", path, e))?;
            Ok(Value::Bool(true))
        }),
    );

    // 13. Vector.load(path) -> index_id
    mod_map.insert(
        "load".to_string(),
        Value::Native("Vector.load".into(), |args| {
            if args.is_empty() {
                return Err("Vector.load(path) requires file path".into());
            }
            let path = format!("{}", args[0]);
            crate::vm::interpreter::with_current_vm(|vm| vm.security_policy.check_fs_read(&path))?;
            let raw = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read vector index '{}': {}", path, e))?;
            let root: serde_json::Value = serde_json::from_str(&raw)
                .map_err(|e| format!("Invalid vector index JSON '{}': {}", path, e))?;
            let mut store = VectorIndexStore::default();
            if let Some(arr) = root.get("records").and_then(|v| v.as_array()) {
                for item in arr {
                    let id = item
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let vector: Vec<f64> = item
                        .get("vector")
                        .and_then(|v| v.as_array())
                        .map(|a| a.iter().filter_map(|x| x.as_f64()).collect())
                        .unwrap_or_default();
                    let metadata = item
                        .get("metadata")
                        .map(json_to_value_vec)
                        .unwrap_or_else(|| Value::map(HashMap::new()));
                    if !id.is_empty() {
                        store.records.insert(
                            id.clone(),
                            VectorRecord {
                                id,
                                vector,
                                metadata,
                            },
                        );
                    }
                }
            }
            let new_id = NEXT_INDEX_ID.fetch_add(1, Ordering::SeqCst);
            get_vector_stores()
                .lock()
                .unwrap()
                .insert(new_id, Arc::new(Mutex::new(store)));
            Ok(Value::Int(new_id as i64))
        }),
    );

    globals.insert("Vector".to_string(), Value::map(mod_map.clone()));
    globals.insert("__native_vector".to_string(), Value::map(mod_map));
}

fn value_to_json_vec(val: &Value) -> serde_json::Value {
    match val {
        Value::Nil => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => serde_json::Value::from(*i),
        Value::Float(f) => serde_json::Value::from(*f),
        Value::String(s) => serde_json::Value::String(s.to_string()),
        Value::Array(a) => {
            serde_json::Value::Array(a.lock().iter().map(value_to_json_vec).collect())
        }
        Value::Map(m) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in m.lock().iter() {
                obj.insert(k.clone(), value_to_json_vec(v));
            }
            serde_json::Value::Object(obj)
        }
        other => serde_json::Value::String(format!("{}", other)),
    }
}

fn json_to_value_vec(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else {
                Value::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Value::string(s.clone()),
        serde_json::Value::Array(a) => Value::array(a.iter().map(json_to_value_vec).collect()),
        serde_json::Value::Object(obj) => {
            let mut m = HashMap::new();
            for (k, val) in obj {
                m.insert(k.clone(), json_to_value_vec(val));
            }
            Value::map(m)
        }
    }
}
