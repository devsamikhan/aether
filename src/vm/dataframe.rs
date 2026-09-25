// ==============================================================================
// AetherDataFrame — In-Memory Columnar Vector Analytics & Relational Engine
// Pure Rust Standard Library (Apache Arrow & Polars Grade Vectorization)
// ==============================================================================

use super::value::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

// ==============================================================================
// 1. Typed Columnar Storage
// ==============================================================================

#[derive(Clone, Debug)]
pub enum TypedColumn {
    Int(Vec<i64>),
    Float(Vec<f64>),
    String(Vec<String>),
    Bool(Vec<bool>),
    Vector(Vec<Vec<f64>>),
}

impl TypedColumn {
    pub fn len(&self) -> usize {
        match self {
            TypedColumn::Int(v) => v.len(),
            TypedColumn::Float(v) => v.len(),
            TypedColumn::String(v) => v.len(),
            TypedColumn::Bool(v) => v.len(),
            TypedColumn::Vector(v) => v.len(),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            TypedColumn::Int(_) => "int64",
            TypedColumn::Float(_) => "float64",
            TypedColumn::String(_) => "string",
            TypedColumn::Bool(_) => "bool",
            TypedColumn::Vector(_) => "vector",
        }
    }

    pub fn get(&self, idx: usize) -> Option<Value> {
        match self {
            TypedColumn::Int(v) => v.get(idx).map(|&n| Value::Int(n)),
            TypedColumn::Float(v) => v.get(idx).map(|&f| Value::Float(f)),
            TypedColumn::String(v) => v.get(idx).map(|s| Value::string(s.clone())),
            TypedColumn::Bool(v) => v.get(idx).map(|&b| Value::Bool(b)),
            TypedColumn::Vector(v) => v.get(idx).map(|vec| {
                Value::array(vec.iter().map(|&f| Value::Float(f)).collect())
            }),
        }
    }

    pub fn to_values(&self) -> Vec<Value> {
        match self {
            TypedColumn::Int(v) => v.iter().map(|&n| Value::Int(n)).collect(),
            TypedColumn::Float(v) => v.iter().map(|&f| Value::Float(f)).collect(),
            TypedColumn::String(v) => v.iter().map(|s| Value::string(s.clone())).collect(),
            TypedColumn::Bool(v) => v.iter().map(|&b| Value::Bool(b)).collect(),
            TypedColumn::Vector(v) => v.iter().map(|vec| {
                Value::array(vec.iter().map(|&f| Value::Float(f)).collect())
            }).collect(),
        }
    }

    pub fn from_values(vals: &[Value]) -> Self {
        if vals.is_empty() {
            return TypedColumn::Int(Vec::new());
        }

        // Infer type from non-nil entries
        let mut is_float = false;
        let mut is_string = false;
        let mut is_bool = false;
        let mut is_vector = false;

        for v in vals {
            match v {
                Value::Float(_) => is_float = true,
                Value::String(_) => is_string = true,
                Value::Bool(_) => is_bool = true,
                Value::Array(_) => is_vector = true,
                _ => {}
            }
        }

        if is_vector {
            let mut vecs = Vec::with_capacity(vals.len());
            for v in vals {
                match v {
                    Value::Array(arr) => {
                        let f_vec = arr.lock().iter().map(|item| match item {
                            Value::Float(f) => *f,
                            Value::Int(i) => *i as f64,
                            _ => 0.0,
                        }).collect();
                        vecs.push(f_vec);
                    }
                    _ => vecs.push(Vec::new()),
                }
            }
            TypedColumn::Vector(vecs)
        } else if is_string {
            TypedColumn::String(vals.iter().map(|v| v.to_string()).collect())
        } else if is_float {
            TypedColumn::Float(
                vals.iter()
                    .map(|v| match v {
                        Value::Float(f) => *f,
                        Value::Int(i) => *i as f64,
                        _ => 0.0,
                    })
                    .collect(),
            )
        } else if is_bool {
            TypedColumn::Bool(
                vals.iter()
                    .map(|v| match v {
                        Value::Bool(b) => *b,
                        Value::Int(i) => *i != 0,
                        _ => false,
                    })
                    .collect(),
            )
        } else {
            TypedColumn::Int(
                vals.iter()
                    .map(|v| match v {
                        Value::Int(i) => *i,
                        Value::Float(f) => *f as i64,
                        _ => 0,
                    })
                    .collect(),
            )
        }
    }

    pub fn filter_by_mask(&self, mask: &[bool]) -> Self {
        match self {
            TypedColumn::Int(v) => {
                let filtered = v.iter().zip(mask).filter_map(|(&n, &m)| if m { Some(n) } else { None }).collect();
                TypedColumn::Int(filtered)
            }
            TypedColumn::Float(v) => {
                let filtered = v.iter().zip(mask).filter_map(|(&f, &m)| if m { Some(f) } else { None }).collect();
                TypedColumn::Float(filtered)
            }
            TypedColumn::String(v) => {
                let filtered = v.iter().zip(mask).filter_map(|(s, &m)| if m { Some(s.clone()) } else { None }).collect();
                TypedColumn::String(filtered)
            }
            TypedColumn::Bool(v) => {
                let filtered = v.iter().zip(mask).filter_map(|(&b, &m)| if m { Some(b) } else { None }).collect();
                TypedColumn::Bool(filtered)
            }
            TypedColumn::Vector(v) => {
                let filtered = v.iter().zip(mask).filter_map(|(vec, &m)| if m { Some(vec.clone()) } else { None }).collect();
                TypedColumn::Vector(filtered)
            }
        }
    }

    pub fn reorder_by_indices(&self, indices: &[usize]) -> Self {
        match self {
            TypedColumn::Int(v) => TypedColumn::Int(indices.iter().map(|&i| v[i]).collect()),
            TypedColumn::Float(v) => TypedColumn::Float(indices.iter().map(|&i| v[i]).collect()),
            TypedColumn::String(v) => TypedColumn::String(indices.iter().map(|&i| v[i].clone()).collect()),
            TypedColumn::Bool(v) => TypedColumn::Bool(indices.iter().map(|&i| v[i]).collect()),
            TypedColumn::Vector(v) => TypedColumn::Vector(indices.iter().map(|&i| v[i].clone()).collect()),
        }
    }

    pub fn slice(&self, start: usize, len: usize) -> Self {
        let end = (start + len).min(self.len());
        match self {
            TypedColumn::Int(v) => TypedColumn::Int(v[start..end].to_vec()),
            TypedColumn::Float(v) => TypedColumn::Float(v[start..end].to_vec()),
            TypedColumn::String(v) => TypedColumn::String(v[start..end].to_vec()),
            TypedColumn::Bool(v) => TypedColumn::Bool(v[start..end].to_vec()),
            TypedColumn::Vector(v) => TypedColumn::Vector(v[start..end].to_vec()),
        }
    }

    // Statistical Reductions
    pub fn sum(&self) -> Value {
        match self {
            TypedColumn::Int(v) => Value::Int(v.iter().sum()),
            TypedColumn::Float(v) => Value::Float(v.iter().sum()),
            TypedColumn::Bool(v) => Value::Int(v.iter().filter(|&&b| b).count() as i64),
            TypedColumn::String(_) | TypedColumn::Vector(_) => Value::Nil,
        }
    }

    pub fn mean(&self) -> Value {
        if self.len() == 0 {
            return Value::Float(0.0);
        }
        match self {
            TypedColumn::Int(v) => Value::Float(v.iter().sum::<i64>() as f64 / v.len() as f64),
            TypedColumn::Float(v) => Value::Float(v.iter().sum::<f64>() / v.len() as f64),
            TypedColumn::Bool(v) => Value::Float(v.iter().filter(|&&b| b).count() as f64 / v.len() as f64),
            TypedColumn::String(_) | TypedColumn::Vector(_) => Value::Nil,
        }
    }

    pub fn min(&self) -> Value {
        if self.len() == 0 {
            return Value::Nil;
        }
        match self {
            TypedColumn::Int(v) => Value::Int(*v.iter().min().unwrap()),
            TypedColumn::Float(v) => Value::Float(v.iter().cloned().fold(f64::INFINITY, f64::min)),
            TypedColumn::String(v) => Value::string(v.iter().min().cloned().unwrap_or_default()),
            TypedColumn::Bool(v) => Value::Bool(*v.iter().min().unwrap()),
            TypedColumn::Vector(_) => Value::Nil,
        }
    }

    pub fn max(&self) -> Value {
        if self.len() == 0 {
            return Value::Nil;
        }
        match self {
            TypedColumn::Int(v) => Value::Int(*v.iter().max().unwrap()),
            TypedColumn::Float(v) => Value::Float(v.iter().cloned().fold(f64::NEG_INFINITY, f64::max)),
            TypedColumn::String(v) => Value::string(v.iter().max().cloned().unwrap_or_default()),
            TypedColumn::Bool(v) => Value::Bool(*v.iter().max().unwrap()),
            TypedColumn::Vector(_) => Value::Nil,
        }
    }

    pub fn std_dev(&self) -> Value {
        if self.len() <= 1 {
            return Value::Float(0.0);
        }
        let n = self.len() as f64;
        match self {
            TypedColumn::Int(v) => {
                let mean = v.iter().sum::<i64>() as f64 / n;
                let var = v.iter().map(|&x| (x as f64 - mean).powi(2)).sum::<f64>() / (n - 1.0);
                Value::Float(var.sqrt())
            }
            TypedColumn::Float(v) => {
                let mean = v.iter().sum::<f64>() / n;
                let var = v.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
                Value::Float(var.sqrt())
            }
            _ => Value::Nil,
        }
    }
}

// ==============================================================================
// 2. Native DataFrame Definition & Operations
// ==============================================================================

#[derive(Clone, Debug)]
pub struct NativeDataFrame {
    pub column_names: Vec<String>,
    pub columns: HashMap<String, TypedColumn>,
    pub row_count: usize,
}

impl NativeDataFrame {
    pub fn new(column_names: Vec<String>, columns: HashMap<String, TypedColumn>) -> Self {
        let row_count = column_names
            .first()
            .and_then(|name| columns.get(name))
            .map(|col| col.len())
            .unwrap_or(0);
        Self {
            column_names,
            columns,
            row_count,
        }
    }

    pub fn from_map(map: HashMap<String, Value>) -> Self {
        let mut column_names = Vec::new();
        let mut columns = HashMap::new();
        let mut row_count = 0;

        for (k, v) in map {
            if let Value::Array(arr) = v {
                let vals = arr.lock().clone();
                row_count = row_count.max(vals.len());
                let col = TypedColumn::from_values(&vals);
                column_names.push(k.clone());
                columns.insert(k, col);
            }
        }
        column_names.sort();
        Self {
            column_names,
            columns,
            row_count,
        }
    }

    pub fn select(&self, cols: &[String]) -> Self {
        let mut new_names = Vec::new();
        let mut new_cols = HashMap::new();
        for name in cols {
            if let Some(col) = self.columns.get(name) {
                new_names.push(name.clone());
                new_cols.insert(name.clone(), col.clone());
            }
        }
        Self::new(new_names, new_cols)
    }

    pub fn filter(&self, mask: &[bool]) -> Self {
        let mut new_cols = HashMap::new();
        for (name, col) in &self.columns {
            new_cols.insert(name.clone(), col.filter_by_mask(mask));
        }
        Self::new(self.column_names.clone(), new_cols)
    }

    pub fn with_column(&self, name: String, col: TypedColumn) -> Self {
        let mut new_names = self.column_names.clone();
        if !new_names.contains(&name) {
            new_names.push(name.clone());
        }
        let mut new_cols = self.columns.clone();
        new_cols.insert(name, col);
        Self::new(new_names, new_cols)
    }

    pub fn sort_by(&self, col_name: &str, descending: bool) -> Result<Self, String> {
        let col = self.columns.get(col_name).ok_or_else(|| format!("Column '{}' not found", col_name))?;
        let mut indices: Vec<usize> = (0..self.row_count).collect();

        match col {
            TypedColumn::Int(v) => {
                indices.sort_by(|&a, &b| {
                    if descending {
                        v[b].cmp(&v[a])
                    } else {
                        v[a].cmp(&v[b])
                    }
                });
            }
            TypedColumn::Float(v) => {
                indices.sort_by(|&a, &b| {
                    if descending {
                        v[b].partial_cmp(&v[a]).unwrap_or(std::cmp::Ordering::Equal)
                    } else {
                        v[a].partial_cmp(&v[b]).unwrap_or(std::cmp::Ordering::Equal)
                    }
                });
            }
            TypedColumn::String(v) => {
                indices.sort_by(|&a, &b| {
                    if descending {
                        v[b].cmp(&v[a])
                    } else {
                        v[a].cmp(&v[b])
                    }
                });
            }
            TypedColumn::Bool(v) => {
                indices.sort_by(|&a, &b| {
                    if descending {
                        v[b].cmp(&v[a])
                    } else {
                        v[a].cmp(&v[b])
                    }
                });
            }
            TypedColumn::Vector(v) => {
                indices.sort_by(|&a, &b| {
                    let ord = v[a].partial_cmp(&v[b]).unwrap_or(std::cmp::Ordering::Equal);
                    if descending {
                        ord.reverse()
                    } else {
                        ord
                    }
                });
            }
        }

        let mut new_cols = HashMap::new();
        for (name, c) in &self.columns {
            new_cols.insert(name.clone(), c.reorder_by_indices(&indices));
        }

        Ok(Self::new(self.column_names.clone(), new_cols))
    }

    pub fn group_by(&self, by_col: &str, aggs: &HashMap<String, String>) -> Result<Self, String> {
        let group_col = self.columns.get(by_col).ok_or_else(|| format!("Column '{}' not found for group_by", by_col))?;

        // 1. Build buckets of row indices by group key
        let mut buckets: HashMap<String, Vec<usize>> = HashMap::new();
        let mut group_keys_ordered = Vec::new();

        for i in 0..self.row_count {
            let key = match group_col {
                TypedColumn::String(v) => v[i].clone(),
                TypedColumn::Int(v) => v[i].to_string(),
                TypedColumn::Float(v) => v[i].to_string(),
                TypedColumn::Bool(v) => v[i].to_string(),
                TypedColumn::Vector(v) => format!("{:?}", v[i]),
            };
            if !buckets.contains_key(&key) {
                group_keys_ordered.push(key.clone());
            }
            buckets.entry(key).or_default().push(i);
        }

        let mut res_cols: HashMap<String, TypedColumn> = HashMap::new();
        let mut col_names = vec![by_col.to_string()];

        // Convert group keys column
        res_cols.insert(
            by_col.to_string(),
            TypedColumn::String(group_keys_ordered.clone()),
        );

        // 2. Compute aggregated metrics per bucket
        for (target_col_name, op_name) in aggs {
            let src_col = self.columns.get(target_col_name)
                .ok_or_else(|| format!("Target aggregation column '{}' not found", target_col_name))?;

            let result_col_name = format!("{}_{}", target_col_name, op_name);
            col_names.push(result_col_name.clone());

            let mut float_results = Vec::new();

            for key in &group_keys_ordered {
                let row_indices = &buckets[key];
                let sub_col = src_col.reorder_by_indices(row_indices);

                let agg_val = match op_name.to_lowercase().as_str() {
                    "sum" => sub_col.sum(),
                    "mean" | "avg" => sub_col.mean(),
                    "min" => sub_col.min(),
                    "max" => sub_col.max(),
                    "std" => sub_col.std_dev(),
                    "count" => Value::Int(sub_col.len() as i64),
                    _ => sub_col.sum(),
                };

                let f = match agg_val {
                    Value::Float(f) => f,
                    Value::Int(i) => i as f64,
                    _ => 0.0,
                };
                float_results.push(f);
            }

            res_cols.insert(result_col_name, TypedColumn::Float(float_results));
        }

        Ok(Self::new(col_names, res_cols))
    }

    pub fn join(&self, other: &NativeDataFrame, on_key: &str, how: &str) -> Result<Self, String> {
        let left_key_col = self.columns.get(on_key).ok_or_else(|| format!("Key '{}' not found in left DataFrame", on_key))?;
        let right_key_col = other.columns.get(on_key).ok_or_else(|| format!("Key '{}' not found in right DataFrame", on_key))?;

        // Index right DataFrame by key
        let mut right_index: HashMap<String, Vec<usize>> = HashMap::new();
        for i in 0..other.row_count {
            let k = match right_key_col {
                TypedColumn::String(v) => v[i].clone(),
                TypedColumn::Int(v) => v[i].to_string(),
                _ => right_key_col.get(i).map(|v| v.to_string()).unwrap_or_default(),
            };
            right_index.entry(k).or_default().push(i);
        }

        let mut left_matched_rows = Vec::new();
        let mut right_matched_rows = Vec::new();

        for i in 0..self.row_count {
            let k = match left_key_col {
                TypedColumn::String(v) => v[i].clone(),
                TypedColumn::Int(v) => v[i].to_string(),
                _ => left_key_col.get(i).map(|v| v.to_string()).unwrap_or_default(),
            };

            if let Some(r_indices) = right_index.get(&k) {
                for &r_idx in r_indices {
                    left_matched_rows.push(Some(i));
                    right_matched_rows.push(Some(r_idx));
                }
            } else if how.to_lowercase() == "left" || how.to_lowercase() == "outer" {
                left_matched_rows.push(Some(i));
                right_matched_rows.push(None);
            }
        }

        // Construct merged columns
        let mut merged_names = self.column_names.clone();
        let mut merged_cols = HashMap::new();

        for name in &self.column_names {
            let col = &self.columns[name];
            let mut vals = Vec::with_capacity(left_matched_rows.len());
            for l_opt in &left_matched_rows {
                if let Some(idx) = l_opt {
                    vals.push(col.get(*idx).unwrap_or(Value::Nil));
                } else {
                    vals.push(Value::Nil);
                }
            }
            merged_cols.insert(name.clone(), TypedColumn::from_values(&vals));
        }

        for name in &other.column_names {
            if name == on_key {
                continue;
            }
            let dest_name = if merged_names.contains(name) {
                format!("{}_right", name)
            } else {
                name.clone()
            };
            merged_names.push(dest_name.clone());

            let col = &other.columns[name];
            let mut vals = Vec::with_capacity(right_matched_rows.len());
            for r_opt in &right_matched_rows {
                if let Some(idx) = r_opt {
                    vals.push(col.get(*idx).unwrap_or(Value::Nil));
                } else {
                    vals.push(Value::Nil);
                }
            }
            merged_cols.insert(dest_name, TypedColumn::from_values(&vals));
        }

        Ok(Self::new(merged_names, merged_cols))
    }

    pub fn to_csv(&self) -> String {
        let mut out = String::new();
        out.push_str(&self.column_names.join(","));
        out.push('\n');

        for row in 0..self.row_count {
            let mut row_items = Vec::with_capacity(self.column_names.len());
            for col_name in &self.column_names {
                let s = self.columns[col_name].get(row).map(|v| v.to_string()).unwrap_or_default();
                row_items.push(s);
            }
            out.push_str(&row_items.join(","));
            out.push('\n');
        }
        out
    }

    pub fn from_csv(csv_str: &str) -> Result<Self, String> {
        let lines: Vec<&str> = csv_str.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        if lines.is_empty() {
            return Ok(Self::new(Vec::new(), HashMap::new()));
        }

        let headers: Vec<String> = lines[0].split(',').map(|s| s.trim().to_string()).collect();
        let mut raw_columns: Vec<Vec<String>> = vec![Vec::with_capacity(lines.len() - 1); headers.len()];

        for line in &lines[1..] {
            let fields: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            for (col_idx, &field) in fields.iter().enumerate() {
                if col_idx < raw_columns.len() {
                    raw_columns[col_idx].push(field.to_string());
                }
            }
        }

        let mut cols = HashMap::new();
        for (i, h) in headers.iter().enumerate() {
            let raw_vals = &raw_columns[i];
            let parsed_vals: Vec<Value> = raw_vals
                .iter()
                .map(|s| {
                    if let Ok(num) = s.parse::<i64>() {
                        Value::Int(num)
                    } else if let Ok(f) = s.parse::<f64>() {
                        Value::Float(f)
                    } else if s.eq_ignore_ascii_case("true") {
                        Value::Bool(true)
                    } else if s.eq_ignore_ascii_case("false") {
                        Value::Bool(false)
                    } else {
                        Value::string(s.clone())
                    }
                })
                .collect();
            cols.insert(h.clone(), TypedColumn::from_values(&parsed_vals));
        }

        Ok(Self::new(headers, cols))
    }

    pub fn shape(&self) -> (usize, usize) {
        (self.row_count, self.column_names.len())
    }

    pub fn head(&self, n: usize) -> Self {
        let count = n.min(self.row_count);
        let mut new_cols = HashMap::new();
        for (name, col) in &self.columns {
            new_cols.insert(name.clone(), col.slice(0, count));
        }
        Self {
            column_names: self.column_names.clone(),
            columns: new_cols,
            row_count: count,
        }
    }

    pub fn describe(&self) -> HashMap<String, String> {
        let mut stats = HashMap::new();
        stats.insert("rows".to_string(), self.row_count.to_string());
        stats.insert("columns".to_string(), self.column_names.len().to_string());
        for name in &self.column_names {
            if let Some(col) = self.columns.get(name) {
                stats.insert(format!("{}.type", name), col.type_name().to_string());
                match col {
                    TypedColumn::Int(_) | TypedColumn::Float(_) => {
                        stats.insert(format!("{}.mean", name), col.mean().to_string());
                        stats.insert(format!("{}.min", name), col.min().to_string());
                        stats.insert(format!("{}.max", name), col.max().to_string());
                        stats.insert(format!("{}.std", name), col.std_dev().to_string());
                    }
                    _ => {}
                }
            }
        }
        stats
    }
}

// ==============================================================================
// 3. Global In-Memory Registry
// ==============================================================================

static DATAFRAMES: OnceLock<Mutex<HashMap<u64, Arc<Mutex<NativeDataFrame>>>>> = OnceLock::new();
static NEXT_DF_ID: AtomicU64 = AtomicU64::new(1);

fn get_dataframes() -> &'static Mutex<HashMap<u64, Arc<Mutex<NativeDataFrame>>>> {
    DATAFRAMES.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn register_df(df: NativeDataFrame) -> u64 {
    let id = NEXT_DF_ID.fetch_add(1, Ordering::SeqCst);
    get_dataframes().lock().unwrap().insert(id, Arc::new(Mutex::new(df)));
    id
}

pub fn get_df(id: u64) -> Result<Arc<Mutex<NativeDataFrame>>, String> {
    get_dataframes()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("DataFrame with ID {} not found", id))
}

// ==============================================================================
// 4. Native VM Module Registration
// ==============================================================================

pub fn register_dataframe_module(globals: &mut HashMap<String, Value>) {
    let mut mod_map = HashMap::new();

    // 1. DataFrame.create(data_map) -> df_id
    mod_map.insert(
        "create".to_string(),
        Value::Native("DataFrame.create".into(), |args| {
            if args.is_empty() {
                return Err("DataFrame.create(dict) requires a column dictionary".into());
            }
            match &args[0] {
                Value::Map(m) => {
                    let map = m.lock().clone();
                    let df = NativeDataFrame::from_map(map);
                    let id = register_df(df);
                    Ok(Value::Int(id as i64))
                }
                _ => Err("DataFrame.create expects a Map of columns".into()),
            }
        }),
    );

    // 2. DataFrame.from_csv(csv_content_or_file) -> df_id
    mod_map.insert(
        "from_csv".to_string(),
        Value::Native("DataFrame.from_csv".into(), |args| {
            if args.is_empty() {
                return Err("DataFrame.from_csv(path_or_text) requires argument".into());
            }
            let content_or_path = args[0].to_string();
            let csv_text = if std::path::Path::new(&content_or_path).exists() {
                std::fs::read_to_string(&content_or_path)
                    .map_err(|e| format!("Failed to read CSV file '{}': {}", content_or_path, e))?
            } else {
                content_or_path
            };

            let df = NativeDataFrame::from_csv(&csv_text)?;
            let id = register_df(df);
            Ok(Value::Int(id as i64))
        }),
    );

    // 3. DataFrame.to_csv(df_id, [file_path]) -> string or bool
    mod_map.insert(
        "to_csv".to_string(),
        Value::Native("DataFrame.to_csv".into(), |args| {
            if args.is_empty() {
                return Err("DataFrame.to_csv(df_id, [path]) requires df_id".into());
            }
            let df_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("df_id must be integer".into()),
            };
            let df_arc = get_df(df_id)?;
            let csv = df_arc.lock().unwrap().to_csv();

            if args.len() > 1 {
                let path = args[1].to_string();
                std::fs::write(&path, &csv).map_err(|e| e.to_string())?;
                Ok(Value::Bool(true))
            } else {
                Ok(Value::string(csv))
            }
        }),
    );

    // 4. DataFrame.select(df_id, [col_names...]) -> new_df_id
    mod_map.insert(
        "select".to_string(),
        Value::Native("DataFrame.select".into(), |args| {
            if args.len() < 2 {
                return Err("DataFrame.select(df_id, col_names) requires df_id and array of columns".into());
            }
            let df_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("df_id must be integer".into()),
            };
            let col_names: Vec<String> = match &args[1] {
                Value::Array(arr) => arr.lock().iter().map(|v| v.to_string()).collect(),
                _ => return Err("col_names must be array of strings".into()),
            };
            let df_arc = get_df(df_id)?;
            let new_df = df_arc.lock().unwrap().select(&col_names);
            Ok(Value::Int(register_df(new_df) as i64))
        }),
    );

    // 5. DataFrame.filter(df_id, bool_mask_array) -> new_df_id
    mod_map.insert(
        "filter".to_string(),
        Value::Native("DataFrame.filter".into(), |args| {
            if args.len() < 2 {
                return Err("DataFrame.filter(df_id, mask) requires df_id and boolean mask".into());
            }
            let df_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("df_id must be integer".into()),
            };
            let mask: Vec<bool> = match &args[1] {
                Value::Array(arr) => arr
                    .lock()
                    .iter()
                    .map(|v| match v {
                        Value::Bool(b) => *b,
                        Value::Int(i) => *i != 0,
                        _ => false,
                    })
                    .collect(),
                _ => return Err("filter mask must be an array of booleans".into()),
            };

            let df_arc = get_df(df_id)?;
            let new_df = df_arc.lock().unwrap().filter(&mask);
            Ok(Value::Int(register_df(new_df) as i64))
        }),
    );

    // 6. DataFrame.with_column(df_id, name, values_array) -> new_df_id
    mod_map.insert(
        "with_column".to_string(),
        Value::Native("DataFrame.with_column".into(), |args| {
            if args.len() < 3 {
                return Err("DataFrame.with_column(df_id, name, values) requires 3 arguments".into());
            }
            let df_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("df_id must be integer".into()),
            };
            let col_name = args[1].to_string();
            let col = match &args[2] {
                Value::Array(arr) => TypedColumn::from_values(&arr.lock()),
                _ => return Err("values must be an array".into()),
            };

            let df_arc = get_df(df_id)?;
            let new_df = df_arc.lock().unwrap().with_column(col_name, col);
            Ok(Value::Int(register_df(new_df) as i64))
        }),
    );

    // 7. DataFrame.sort_by(df_id, col_name, [descending]) -> new_df_id
    mod_map.insert(
        "sort_by".to_string(),
        Value::Native("DataFrame.sort_by".into(), |args| {
            if args.len() < 2 {
                return Err("DataFrame.sort_by(df_id, col_name, [descending]) requires at least 2 arguments".into());
            }
            let df_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("df_id must be integer".into()),
            };
            let col_name = args[1].to_string();
            let descending = if args.len() > 2 {
                match args[2] {
                    Value::Bool(b) => b,
                    Value::Int(i) => i != 0,
                    _ => false,
                }
            } else {
                false
            };

            let df_arc = get_df(df_id)?;
            let new_df = df_arc.lock().unwrap().sort_by(&col_name, descending)?;
            Ok(Value::Int(register_df(new_df) as i64))
        }),
    );

    // 8. DataFrame.group_by(df_id, group_col, aggs_map) -> new_df_id
    mod_map.insert(
        "group_by".to_string(),
        Value::Native("DataFrame.group_by".into(), |args| {
            if args.len() < 3 {
                return Err("DataFrame.group_by(df_id, group_col, aggs) requires 3 arguments".into());
            }
            let df_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("df_id must be integer".into()),
            };
            let group_col = args[1].to_string();
            let aggs: HashMap<String, String> = match &args[2] {
                Value::Map(m) => m.lock().iter().map(|(k, v)| (k.clone(), v.to_string())).collect(),
                _ => return Err("aggs must be a Map of column -> operation (e.g. {'salary': 'mean'})".into()),
            };

            let df_arc = get_df(df_id)?;
            let new_df = df_arc.lock().unwrap().group_by(&group_col, &aggs)?;
            Ok(Value::Int(register_df(new_df) as i64))
        }),
    );

    // 9. DataFrame.join(df_id, other_df_id, on_key, [how]) -> new_df_id
    mod_map.insert(
        "join".to_string(),
        Value::Native("DataFrame.join".into(), |args| {
            if args.len() < 3 {
                return Err("DataFrame.join(left_id, right_id, on_key, [how]) requires at least 3 arguments".into());
            }
            let left_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("left_id must be integer".into()),
            };
            let right_id = match args[1] {
                Value::Int(i) => i as u64,
                _ => return Err("right_id must be integer".into()),
            };
            let on_key = args[2].to_string();
            let how = if args.len() > 3 {
                args[3].to_string()
            } else {
                "inner".to_string()
            };

            let left_arc = get_df(left_id)?;
            let right_arc = get_df(right_id)?;

            let left_df = left_arc.lock().unwrap().clone();
            let right_df = right_arc.lock().unwrap().clone();

            let joined_df = left_df.join(&right_df, &on_key, &how)?;
            Ok(Value::Int(register_df(joined_df) as i64))
        }),
    );

    // 10. DataFrame.shape(df_id) -> [rows, cols]
    mod_map.insert(
        "shape".to_string(),
        Value::Native("DataFrame.shape".into(), |args| {
            if args.is_empty() {
                return Err("DataFrame.shape(df_id) requires df_id".into());
            }
            let df_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("df_id must be integer".into()),
            };
            let df_arc = get_df(df_id)?;
            let df = df_arc.lock().unwrap();
            Ok(Value::array(vec![
                Value::Int(df.row_count as i64),
                Value::Int(df.column_names.len() as i64),
            ]))
        }),
    );

    // 11. DataFrame.columns(df_id) -> array of string names
    mod_map.insert(
        "columns".to_string(),
        Value::Native("DataFrame.columns".into(), |args| {
            if args.is_empty() {
                return Err("DataFrame.columns(df_id) requires df_id".into());
            }
            let df_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("df_id must be integer".into()),
            };
            let df_arc = get_df(df_id)?;
            let df = df_arc.lock().unwrap();
            let items: Vec<Value> = df.column_names.iter().map(|s| Value::string(s.clone())).collect();
            Ok(Value::array(items))
        }),
    );

    // 12. DataFrame.get_column(df_id, col_name) -> array of values
    mod_map.insert(
        "get_column".to_string(),
        Value::Native("DataFrame.get_column".into(), |args| {
            if args.len() < 2 {
                return Err("DataFrame.get_column(df_id, col_name) requires 2 arguments".into());
            }
            let df_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("df_id must be integer".into()),
            };
            let name = args[1].to_string();
            let df_arc = get_df(df_id)?;
            let df = df_arc.lock().unwrap();
            let col = df.columns.get(&name).ok_or_else(|| format!("Column '{}' not found", name))?;
            Ok(Value::array(col.to_values()))
        }),
    );

    // 13. DataFrame.head(df_id, [n]) -> new_df_id
    mod_map.insert(
        "head".to_string(),
        Value::Native("DataFrame.head".into(), |args| {
            if args.is_empty() {
                return Err("DataFrame.head(df_id, [n]) requires df_id".into());
            }
            let df_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("df_id must be integer".into()),
            };
            let n = if args.len() > 1 {
                match args[1] {
                    Value::Int(i) => i as usize,
                    _ => 5,
                }
            } else {
                5
            };
            let df_arc = get_df(df_id)?;
            let df = df_arc.lock().unwrap();
            let mut new_cols = HashMap::new();
            for (name, col) in &df.columns {
                new_cols.insert(name.clone(), col.slice(0, n));
            }
            Ok(Value::Int(register_df(NativeDataFrame::new(df.column_names.clone(), new_cols)) as i64))
        }),
    );

    // 14. DataFrame.describe(df_id) -> summary map
    mod_map.insert(
        "describe".to_string(),
        Value::Native("DataFrame.describe".into(), |args| {
            if args.is_empty() {
                return Err("DataFrame.describe(df_id) requires df_id".into());
            }
            let df_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("df_id must be integer".into()),
            };
            let df_arc = get_df(df_id)?;
            let df = df_arc.lock().unwrap();

            let mut summary = HashMap::new();
            for name in &df.column_names {
                let col = &df.columns[name];
                let mut col_stats = HashMap::new();
                col_stats.insert("count".to_string(), Value::Int(col.len() as i64));
                col_stats.insert("type".to_string(), Value::string(col.type_name()));
                col_stats.insert("mean".to_string(), col.mean());
                col_stats.insert("min".to_string(), col.min());
                col_stats.insert("max".to_string(), col.max());
                col_stats.insert("std".to_string(), col.std_dev());
                summary.insert(name.clone(), Value::map(col_stats));
            }
            Ok(Value::map(summary))
        }),
    );

    // 15. Vectorized arithmetic: DataFrame.vec_op(op, col_a, col_b_or_scalar) -> array
    mod_map.insert(
        "vec_op".to_string(),
        Value::Native("DataFrame.vec_op".into(), |args| {
            if args.len() < 3 {
                return Err("DataFrame.vec_op(op, col_a, col_b_or_scalar) requires 3 arguments".into());
            }
            let op = args[0].to_string();

            let unpack_vector = |val: &Value| -> Option<TypedColumn> {
                match val {
                    Value::Array(a) => Some(TypedColumn::from_values(&a.lock())),
                    Value::ClassInstance(inst) => {
                        if let Some(Value::Array(a)) = inst.fields.lock().get("data") {
                            Some(TypedColumn::from_values(&a.lock()))
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            };

            let col_a = match unpack_vector(&args[1]) {
                Some(col) => col,
                None => return Err("col_a must be array or Series".into()),
            };

            let mut results = Vec::with_capacity(col_a.len());

            if let Some(col_b) = unpack_vector(&args[2]) {
                let n = col_a.len().min(col_b.len());
                for i in 0..n {
                    let va = col_a.get(i).unwrap();
                    let vb = col_b.get(i).unwrap();
                    let res = evaluate_scalar_binop(&op, &va, &vb)?;
                    results.push(res);
                }
            } else {
                let scalar = &args[2];
                for i in 0..col_a.len() {
                    let va = col_a.get(i).unwrap();
                    let res = evaluate_scalar_binop(&op, &va, scalar)?;
                    results.push(res);
                }
            }

            Ok(Value::array(results))
        }),
    );

    // 16. DataFrame.to_tensor_flat(df_id, col_names) -> [rows, cols, flat_data_array]
    mod_map.insert(
        "to_tensor_flat".to_string(),
        Value::Native("DataFrame.to_tensor_flat".into(), |args| {
            if args.len() < 2 {
                return Err("DataFrame.to_tensor_flat(df_id, col_names) requires 2 arguments".into());
            }
            let df_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("df_id must be integer".into()),
            };
            let col_names: Vec<String> = match &args[1] {
                Value::Array(arr) => arr.lock().iter().map(|v| v.to_string()).collect(),
                _ => return Err("col_names must be an array of strings".into()),
            };

            let df_arc = get_df(df_id)?;
            let df = df_arc.lock().unwrap();

            let rows = df.row_count;
            let cols = col_names.len();
            let mut flat = Vec::with_capacity(rows * cols);

            for r in 0..rows {
                for name in &col_names {
                    let col = df.columns.get(name)
                        .ok_or_else(|| format!("Tensor conversion column '{}' not found", name))?;
                    let val = match col.get(r).unwrap_or(Value::Float(0.0)) {
                        Value::Float(f) => Value::Float(f),
                        Value::Int(i) => Value::Float(i as f64),
                        Value::Bool(b) => Value::Float(if b { 1.0 } else { 0.0 }),
                        _ => Value::Float(0.0),
                    };
                    flat.push(val);
                }
            }

            let mut res = HashMap::new();
            res.insert("rows".to_string(), Value::Int(rows as i64));
            res.insert("cols".to_string(), Value::Int(cols as i64));
            res.insert("data".to_string(), Value::array(flat));
            Ok(Value::map(res))
        }),
    );

    globals.insert("DataFrame".to_string(), Value::map(mod_map.clone()));
    globals.insert("__native_df".to_string(), Value::map(mod_map));
}

fn evaluate_scalar_binop(op: &str, a: &Value, b: &Value) -> Result<Value, String> {
    match op {
        "+" | "add" => match (a, b) {
            (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x + y)),
            (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x + y)),
            (Value::Int(x), Value::Float(y)) => Ok(Value::Float(*x as f64 + y)),
            (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x + *y as f64)),
            (Value::String(x), _) => Ok(Value::string(format!("{}{}", x, b))),
            _ => Err("Invalid types for +".into()),
        },
        "-" | "sub" => match (a, b) {
            (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x - y)),
            (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x - y)),
            (Value::Int(x), Value::Float(y)) => Ok(Value::Float(*x as f64 - y)),
            (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x - *y as f64)),
            _ => Err("Invalid types for -".into()),
        },
        "*" | "mul" => match (a, b) {
            (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x * y)),
            (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x * y)),
            (Value::Int(x), Value::Float(y)) => Ok(Value::Float(*x as f64 * y)),
            (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x * *y as f64)),
            _ => Err("Invalid types for *".into()),
        },
        "/" | "div" => match (a, b) {
            (Value::Int(x), Value::Int(y)) => {
                if *y == 0 {
                    Err("Division by zero in vectorized operation".into())
                } else {
                    Ok(Value::Float(*x as f64 / *y as f64))
                }
            }
            (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x / y)),
            (Value::Int(x), Value::Float(y)) => Ok(Value::Float(*x as f64 / y)),
            (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x / *y as f64)),
            _ => Err("Invalid types for /".into()),
        },
        "==" | "eq" => match (a, b) {
            (Value::Int(x), Value::Int(y)) => Ok(Value::Bool(x == y)),
            (Value::Float(x), Value::Float(y)) => Ok(Value::Bool((x - y).abs() < 1e-9)),
            (Value::String(x), Value::String(y)) => Ok(Value::Bool(x == y)),
            (Value::Bool(x), Value::Bool(y)) => Ok(Value::Bool(x == y)),
            _ => Ok(Value::Bool(a.to_string() == b.to_string())),
        },
        ">" | "gt" => match (a, b) {
            (Value::Int(x), Value::Int(y)) => Ok(Value::Bool(x > y)),
            (Value::Float(x), Value::Float(y)) => Ok(Value::Bool(x > y)),
            (Value::Int(x), Value::Float(y)) => Ok(Value::Bool((*x as f64) > *y)),
            (Value::Float(x), Value::Int(y)) => Ok(Value::Bool(*x > (*y as f64))),
            _ => Err("Invalid types for >".into()),
        },
        "<" | "lt" => match (a, b) {
            (Value::Int(x), Value::Int(y)) => Ok(Value::Bool(x < y)),
            (Value::Float(x), Value::Float(y)) => Ok(Value::Bool(x < y)),
            (Value::Int(x), Value::Float(y)) => Ok(Value::Bool((*x as f64) < *y)),
            (Value::Float(x), Value::Int(y)) => Ok(Value::Bool(*x < (*y as f64))),
            _ => Err("Invalid types for <".into()),
        },
        ">=" | "ge" => match (a, b) {
            (Value::Int(x), Value::Int(y)) => Ok(Value::Bool(x >= y)),
            (Value::Float(x), Value::Float(y)) => Ok(Value::Bool(x >= y)),
            (Value::Int(x), Value::Float(y)) => Ok(Value::Bool((*x as f64) >= *y)),
            (Value::Float(x), Value::Int(y)) => Ok(Value::Bool(*x >= (*y as f64))),
            _ => Err("Invalid types for >=".into()),
        },
        "<=" | "le" => match (a, b) {
            (Value::Int(x), Value::Int(y)) => Ok(Value::Bool(x <= y)),
            (Value::Float(x), Value::Float(y)) => Ok(Value::Bool(x <= y)),
            (Value::Int(x), Value::Float(y)) => Ok(Value::Bool((*x as f64) <= *y)),
            (Value::Float(x), Value::Int(y)) => Ok(Value::Bool(*x <= (*y as f64))),
            _ => Err("Invalid types for <=".into()),
        },
        _ => Err(format!("Unsupported vectorized operator '{}'", op)),
    }
}
