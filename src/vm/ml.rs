// ==============================================================================
// AetherML — Pure-Rust Classical Machine Learning & Evaluation Engine
// Zero External ML Dependencies (Scikit-Learn Parity: Regression, Trees, Forests,
// K-Means, PCA, Scalers, Metrics & JSON Model Persistence)
// ==============================================================================

use super::tensor::get_tensor_data;
use super::value::Value;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

static NEXT_MODEL_ID: AtomicU64 = AtomicU64::new(1);
static ML_MODELS: OnceLock<Mutex<HashMap<u64, Arc<MlModel>>>> = OnceLock::new();

fn get_models() -> &'static Mutex<HashMap<u64, Arc<MlModel>>> {
    ML_MODELS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn store_model(model: MlModel) -> u64 {
    let id = NEXT_MODEL_ID.fetch_add(1, Ordering::SeqCst);
    get_models().lock().unwrap().insert(id, Arc::new(model));
    id
}

fn fetch_model(id: u64) -> Result<Arc<MlModel>, String> {
    get_models()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("ML model handle {} not found", id))
}

// ==============================================================================
// Serializable Model Registry Types
// ==============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TreeNode {
    Leaf {
        predicted_class: i64,
    },
    Split {
        feature: usize,
        threshold: f64,
        left: Box<TreeNode>,
        right: Box<TreeNode>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MlModel {
    StandardScaler {
        mean: Vec<f64>,
        std: Vec<f64>,
    },
    MinMaxScaler {
        min: Vec<f64>,
        scale: Vec<f64>,
    },
    LinearRegression {
        weights: Vec<f64>,
        bias: f64,
    },
    LogisticRegression {
        classes: Vec<i64>,
        /// Row `c` stores weights for class `c` (`classes[c]`)
        weights: Vec<Vec<f64>>,
        bias: Vec<f64>,
    },
    DecisionTree {
        root: TreeNode,
    },
    RandomForest {
        trees: Vec<TreeNode>,
    },
    Knn {
        x_train: Vec<Vec<f64>>,
        y_train: Vec<i64>,
        k: usize,
    },
    KMeans {
        centroids: Vec<Vec<f64>>,
    },
    Pca {
        mean: Vec<f64>,
        components: Vec<Vec<f64>>,
        explained_variance: Vec<f64>,
    },
}

// ==============================================================================
// Value Conversion Helpers (Accepts Tensor handles OR AETHER Arrays)
// ==============================================================================

fn val_as_f64(v: &Value, name: &str) -> Result<f64, String> {
    match v {
        Value::Float(f) => Ok(*f),
        Value::Int(i) => Ok(*i as f64),
        Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
        _ => Err(format!("{} must be numeric, got {}", name, v.type_name())),
    }
}

fn val_as_usize(v: &Value, name: &str) -> Result<usize, String> {
    match v {
        Value::Int(i) if *i >= 0 => Ok(*i as usize),
        _ => Err(format!(
            "{} must be non-negative integer, got {}",
            name,
            v.type_name()
        )),
    }
}

fn val_as_i64(v: &Value, name: &str) -> Result<i64, String> {
    match v {
        Value::Int(i) => Ok(*i),
        Value::Float(f) => Ok(f.round() as i64),
        Value::Bool(b) => Ok(if *b { 1 } else { 0 }),
        _ => Err(format!("{} must be integer, got {}", name, v.type_name())),
    }
}

pub fn extract_matrix_2d(val: &Value) -> Result<Vec<Vec<f64>>, String> {
    if let Value::Map(m) = val {
        if m.lock().contains_key("_tensor_id") {
            let t = get_tensor_data(val)?;
            if t.shape.len() != 2 {
                return Err(format!(
                    "Expected 2D Tensor matrix, got shape {:?}",
                    t.shape
                ));
            }
            let (rows, cols) = (t.shape[0], t.shape[1]);
            let mut out = Vec::with_capacity(rows);
            for r in 0..rows {
                out.push(t.data[r * cols..(r + 1) * cols].to_vec());
            }
            return Ok(out);
        }
    }

    match val {
        Value::Array(arr) => {
            let rows = arr.lock();
            if rows.is_empty() {
                return Err("Feature matrix X cannot be empty".into());
            }
            let mut matrix = Vec::with_capacity(rows.len());
            let mut expected_cols = None;
            for row_val in rows.iter() {
                match row_val {
                    Value::Array(cols_arr) => {
                        let cols = cols_arr.lock();
                        if let Some(ec) = expected_cols {
                            if cols.len() != ec {
                                return Err(format!(
                                    "Inconsistent feature count: expected {}, got {}",
                                    ec,
                                    cols.len()
                                ));
                            }
                        } else {
                            if cols.is_empty() {
                                return Err("Feature rows cannot be empty".into());
                            }
                            expected_cols = Some(cols.len());
                        }
                        let mut row = Vec::with_capacity(cols.len());
                        for cell in cols.iter() {
                            row.push(val_as_f64(cell, "feature")?);
                        }
                        matrix.push(row);
                    }
                    _ => {
                        return Err("Feature matrix X must be a 2D array (or 2D Tensor)".to_string())
                    }
                }
            }
            Ok(matrix)
        }
        _ => Err(format!(
            "Expected 2D Array or Tensor for matrix X, got {}",
            val.type_name()
        )),
    }
}

pub fn extract_vec_f64(val: &Value) -> Result<Vec<f64>, String> {
    if let Value::Map(m) = val {
        if m.lock().contains_key("_tensor_id") {
            let t = get_tensor_data(val)?;
            return Ok(t.data);
        }
    }
    match val {
        Value::Array(arr) => {
            let items = arr.lock();
            let mut out = Vec::with_capacity(items.len());
            for item in items.iter() {
                out.push(val_as_f64(item, "target")?);
            }
            Ok(out)
        }
        _ => Err(format!(
            "Expected 1D Array or Tensor, got {}",
            val.type_name()
        )),
    }
}

pub fn extract_vec_i64(val: &Value) -> Result<Vec<i64>, String> {
    if let Value::Map(m) = val {
        if m.lock().contains_key("_tensor_id") {
            let t = get_tensor_data(val)?;
            return Ok(t.data.into_iter().map(|v| v.round() as i64).collect());
        }
    }
    match val {
        Value::Array(arr) => {
            let items = arr.lock();
            let mut out = Vec::with_capacity(items.len());
            for item in items.iter() {
                out.push(val_as_i64(item, "label")?);
            }
            Ok(out)
        }
        _ => Err(format!(
            "Expected 1D integer Array or Tensor, got {}",
            val.type_name()
        )),
    }
}

fn matrix_to_value(m: &[Vec<f64>]) -> Value {
    let rows: Vec<Value> = m
        .iter()
        .map(|row| Value::array(row.iter().map(|&v| Value::Float(v)).collect()))
        .collect();
    Value::array(rows)
}

fn vec_f64_to_value(v: &[f64]) -> Value {
    Value::array(v.iter().map(|&x| Value::Float(x)).collect())
}

fn vec_i64_to_value(v: &[i64]) -> Value {
    Value::array(v.iter().map(|&x| Value::Int(x)).collect())
}

// ==============================================================================
// Deterministic PRNG (SplitMix64)
// ==============================================================================

struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x9e3779b97f4a7c15 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    fn gen_range(&mut self, upper: usize) -> usize {
        if upper <= 1 {
            0
        } else {
            (self.next_u64() as usize) % upper
        }
    }

    fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.gen_range(i + 1);
            slice.swap(i, j);
        }
    }
}

// ==============================================================================
// Core ML Mathematical Implementations
// ==============================================================================

fn solve_linear_system(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Result<Vec<f64>, String> {
    let n = b.len();
    for col in 0..n {
        let mut pivot_row = col;
        let mut max_abs = a[col][col].abs();
        for (r, row) in a.iter().enumerate().skip(col + 1) {
            if row[col].abs() > max_abs {
                max_abs = row[col].abs();
                pivot_row = r;
            }
        }
        if max_abs < 1e-12 {
            return Err("Singular matrix encountered in linear regression solve".into());
        }
        if pivot_row != col {
            a.swap(col, pivot_row);
            b.swap(col, pivot_row);
        }

        let pivot = a[col][col];
        for cell in a[col].iter_mut().skip(col) {
            *cell /= pivot;
        }
        b[col] /= pivot;

        let pivot_row = a[col].clone();
        let pivot_rhs = b[col];
        for r in 0..n {
            if r != col {
                let factor = a[r][col];
                if factor.abs() > 1e-15 {
                    for (cell, &pv) in a[r].iter_mut().zip(pivot_row.iter()).skip(col) {
                        *cell -= factor * pv;
                    }
                    b[r] -= factor * pivot_rhs;
                }
            }
        }
    }
    Ok(b)
}

fn fit_linear_regression(x: &[Vec<f64>], y: &[f64], l2: f64) -> Result<MlModel, String> {
    let n = x.len();
    if y.len() != n {
        return Err(format!("X has {} rows but y has {} elements", n, y.len()));
    }
    let d = x[0].len();
    // Solve augmented normal equations [X, 1]^T [X, 1] * w_aug = [X, 1]^T y
    let dim = d + 1;
    let mut xtx = vec![vec![0.0; dim]; dim];
    let mut xty = vec![0.0; dim];

    for (row, &target) in x.iter().zip(y.iter()) {
        for (i, &xi) in row.iter().enumerate().take(d) {
            xty[i] += xi * target;
            for (j, &xj) in row.iter().enumerate().take(d) {
                xtx[i][j] += xi * xj;
            }
            xtx[i][d] += xi;
            xtx[d][i] += xi;
        }
        xty[d] += target;
        xtx[d][d] += 1.0;
    }

    // Apply L2 ridge penalty on feature weights (not bias)
    for (i, row) in xtx.iter_mut().enumerate().take(dim) {
        row[i] += if i < d { l2.max(1e-9) } else { 1e-12 };
    }

    let solution = solve_linear_system(xtx, xty)?;
    let bias = solution[d];
    let weights = solution[..d].to_vec();
    Ok(MlModel::LinearRegression { weights, bias })
}

fn fit_logistic_regression(
    x: &[Vec<f64>],
    y: &[i64],
    lr: f64,
    epochs: usize,
    l2: f64,
) -> Result<MlModel, String> {
    let n = x.len();
    if y.len() != n {
        return Err(format!("X has {} rows but y has {} elements", n, y.len()));
    }
    let d = x[0].len();

    let classes: Vec<i64> = y
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<i64>>()
        .into_iter()
        .collect();
    let k = classes.len();
    if k < 2 {
        return Err("Logistic regression requires at least 2 distinct target classes".into());
    }
    let class_to_idx: HashMap<i64, usize> = classes
        .iter()
        .enumerate()
        .map(|(idx, &cls)| (cls, idx))
        .collect();
    let y_idx: Vec<usize> = y.iter().map(|lbl| class_to_idx[lbl]).collect();

    let mut weights = vec![vec![0.0; d]; k];
    let mut bias = vec![0.0; k];
    let inv_n = 1.0 / (n as f64);

    for _ in 0..epochs {
        let mut grad_w = vec![vec![0.0; d]; k];
        let mut grad_b = vec![0.0; k];

        for (row, &target_c) in x.iter().zip(y_idx.iter()) {
            let mut logits = vec![0.0; k];
            let mut max_logit = f64::NEG_INFINITY;
            for c in 0..k {
                let dot: f64 = weights[c]
                    .iter()
                    .zip(row.iter())
                    .map(|(&w, &xi)| w * xi)
                    .sum();
                let z = dot + bias[c];
                logits[c] = z;
                if z > max_logit {
                    max_logit = z;
                }
            }
            let mut sum_exp = 0.0;
            for z in &mut logits {
                *z = (*z - max_logit).exp();
                sum_exp += *z;
            }
            for c in 0..k {
                let prob = logits[c] / sum_exp;
                let err = prob - if c == target_c { 1.0 } else { 0.0 };
                grad_b[c] += err;
                for j in 0..d {
                    grad_w[c][j] += err * row[j];
                }
            }
        }

        for c in 0..k {
            bias[c] -= lr * (grad_b[c] * inv_n);
            for j in 0..d {
                let reg = l2 * weights[c][j];
                weights[c][j] -= lr * (grad_w[c][j] * inv_n + reg);
            }
        }
    }

    Ok(MlModel::LogisticRegression {
        classes,
        weights,
        bias,
    })
}

fn predict_logistic_proba(
    classes_count: usize,
    weights: &[Vec<f64>],
    bias: &[f64],
    x: &[Vec<f64>],
) -> Vec<Vec<f64>> {
    x.iter()
        .map(|row| {
            let mut logits = Vec::with_capacity(classes_count);
            let mut max_l = f64::NEG_INFINITY;
            for c in 0..classes_count {
                let z: f64 = weights[c]
                    .iter()
                    .zip(row.iter())
                    .map(|(&w, &xi)| w * xi)
                    .sum::<f64>()
                    + bias[c];
                if z > max_l {
                    max_l = z;
                }
                logits.push(z);
            }
            let mut sum_e = 0.0;
            for z in &mut logits {
                *z = (*z - max_l).exp();
                sum_e += *z;
            }
            for z in &mut logits {
                *z /= sum_e;
            }
            logits
        })
        .collect()
}

fn majority_class(indices: &[usize], y: &[i64]) -> (i64, bool) {
    let mut counts: BTreeMap<i64, usize> = BTreeMap::new();
    for &idx in indices {
        *counts.entry(y[idx]).or_insert(0) += 1;
    }
    let pure = counts.len() <= 1;
    let best = counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(cls, _)| cls)
        .unwrap_or(0);
    (best, pure)
}

fn gini_impurity(counts: &BTreeMap<i64, usize>, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    let inv = 1.0 / (total as f64);
    let sum_sq: f64 = counts
        .values()
        .map(|&c| {
            let p = (c as f64) * inv;
            p * p
        })
        .sum();
    1.0 - sum_sq
}

struct TreeConfig {
    max_depth: usize,
    min_samples_split: usize,
    feature_subsample: Option<usize>,
}

fn build_tree(
    x: &[Vec<f64>],
    y: &[i64],
    indices: &[usize],
    depth: usize,
    cfg: &TreeConfig,
    rng: &mut Rng,
) -> TreeNode {
    let (best_cls, is_pure) = majority_class(indices, y);
    if is_pure || depth >= cfg.max_depth || indices.len() < cfg.min_samples_split {
        return TreeNode::Leaf {
            predicted_class: best_cls,
        };
    }

    let d = x[0].len();
    let mut candidate_features: Vec<usize> = (0..d).collect();
    if let Some(m) = cfg.feature_subsample {
        if m < d {
            rng.shuffle(&mut candidate_features);
            candidate_features.truncate(m);
        }
    }

    let n = indices.len();
    let mut best_gini = f64::INFINITY;
    let mut best_feature = None;
    let mut best_threshold = 0.0;

    for feat in candidate_features {
        let mut pairs: Vec<(f64, i64)> =
            indices.iter().map(|&idx| (x[idx][feat], y[idx])).collect();
        pairs.sort_by(|a, b| a.0.total_cmp(&b.0));

        let mut left_counts: BTreeMap<i64, usize> = BTreeMap::new();
        let mut right_counts: BTreeMap<i64, usize> = BTreeMap::new();
        for &(_, cls) in &pairs {
            *right_counts.entry(cls).or_insert(0) += 1;
        }

        for i in 0..n - 1 {
            let (val, cls) = pairs[i];
            *left_counts.entry(cls).or_insert(0) += 1;
            if let Some(rc) = right_counts.get_mut(&cls) {
                *rc -= 1;
                if *rc == 0 {
                    right_counts.remove(&cls);
                }
            }

            let next_val = pairs[i + 1].0;
            if (next_val - val).abs() < 1e-12 {
                continue;
            }

            let left_n = i + 1;
            let right_n = n - left_n;
            let g_left = gini_impurity(&left_counts, left_n);
            let g_right = gini_impurity(&right_counts, right_n);
            let weighted = ((left_n as f64) * g_left + (right_n as f64) * g_right) / (n as f64);

            if weighted < best_gini {
                best_gini = weighted;
                best_feature = Some(feat);
                best_threshold = 0.5 * (val + next_val);
            }
        }
    }

    let Some(feat) = best_feature else {
        return TreeNode::Leaf {
            predicted_class: best_cls,
        };
    };

    let mut left_idx = Vec::new();
    let mut right_idx = Vec::new();
    for &idx in indices {
        if x[idx][feat] <= best_threshold {
            left_idx.push(idx);
        } else {
            right_idx.push(idx);
        }
    }

    if left_idx.is_empty() || right_idx.is_empty() {
        return TreeNode::Leaf {
            predicted_class: best_cls,
        };
    }

    let left = build_tree(x, y, &left_idx, depth + 1, cfg, rng);
    let right = build_tree(x, y, &right_idx, depth + 1, cfg, rng);

    TreeNode::Split {
        feature: feat,
        threshold: best_threshold,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn predict_tree(node: &TreeNode, row: &[f64]) -> i64 {
    let mut cur = node;
    loop {
        match cur {
            TreeNode::Leaf { predicted_class } => return *predicted_class,
            TreeNode::Split {
                feature,
                threshold,
                left,
                right,
            } => {
                cur = if row[*feature] <= *threshold {
                    left
                } else {
                    right
                };
            }
        }
    }
}

fn fit_kmeans(x: &[Vec<f64>], k: usize, max_iter: usize, seed: u64) -> Result<MlModel, String> {
    let n = x.len();
    if k == 0 || k > n {
        return Err(format!(
            "KMeans k={} must be between 1 and sample count {}",
            k, n
        ));
    }
    let d = x[0].len();
    let mut rng = Rng::new(seed);
    let mut order: Vec<usize> = (0..n).collect();
    rng.shuffle(&mut order);

    let mut centroids: Vec<Vec<f64>> = order.into_iter().take(k).map(|i| x[i].clone()).collect();
    let mut assignments = vec![0usize; n];

    for _ in 0..max_iter {
        let mut changed = false;
        for (i, row) in x.iter().enumerate() {
            let mut best_c = 0;
            let mut best_dist = f64::INFINITY;
            for (c_idx, cent) in centroids.iter().enumerate() {
                let dist: f64 = row
                    .iter()
                    .zip(cent.iter())
                    .map(|(&a, &b)| (a - b) * (a - b))
                    .sum();
                if dist < best_dist {
                    best_dist = dist;
                    best_c = c_idx;
                }
            }
            if assignments[i] != best_c {
                assignments[i] = best_c;
                changed = true;
            }
        }

        let mut sums = vec![vec![0.0; d]; k];
        let mut counts = vec![0usize; k];
        for (i, row) in x.iter().enumerate() {
            let c = assignments[i];
            counts[c] += 1;
            for j in 0..d {
                sums[c][j] += row[j];
            }
        }
        for c in 0..k {
            if counts[c] > 0 {
                let inv = 1.0 / (counts[c] as f64);
                for j in 0..d {
                    centroids[c][j] = sums[c][j] * inv;
                }
            }
        }
        if !changed {
            break;
        }
    }

    Ok(MlModel::KMeans { centroids })
}

fn fit_pca(x: &[Vec<f64>], n_components: usize) -> Result<MlModel, String> {
    let n = x.len();
    let d = x[0].len();
    if n_components == 0 || n_components > d {
        return Err(format!(
            "PCA n_components={} must be between 1 and feature count {}",
            n_components, d
        ));
    }

    let mut mean = vec![0.0; d];
    for row in x {
        for j in 0..d {
            mean[j] += row[j];
        }
    }
    for m in &mut mean {
        *m /= n as f64;
    }

    // Covariance matrix (d x d)
    let mut cov = vec![vec![0.0; d]; d];
    let denom = if n > 1 { (n - 1) as f64 } else { 1.0 };
    for row in x {
        for (i, cov_row) in cov.iter_mut().enumerate().take(d) {
            let di = row[i] - mean[i];
            for (j, slot) in cov_row.iter_mut().enumerate().take(d) {
                *slot += di * (row[j] - mean[j]);
            }
        }
    }
    for cov_row in &mut cov {
        for slot in cov_row.iter_mut() {
            *slot /= denom;
        }
    }

    let mut components = Vec::with_capacity(n_components);
    let mut explained_variance = Vec::with_capacity(n_components);
    let mut rng = Rng::new(1337);

    for _ in 0..n_components {
        // Initialize unit vector deterministically with tiny symmetry breaking
        let mut v: Vec<f64> = (0..d)
            .map(|idx| 1.0 + 0.01 * ((rng.next_u64() % 100) as f64) + 0.001 * (idx as f64))
            .collect();
        let norm = v.iter().map(|&x| x * x).sum::<f64>().sqrt().max(1e-12);
        for vi in &mut v {
            *vi /= norm;
        }

        // Power iteration
        for _ in 0..200 {
            let mut next_v = vec![0.0; d];
            for i in 0..d {
                next_v[i] = cov[i].iter().zip(v.iter()).map(|(&a, &b)| a * b).sum();
            }
            let nrm = next_v.iter().map(|&x| x * x).sum::<f64>().sqrt();
            if nrm < 1e-15 {
                break;
            }
            for x in &mut next_v {
                *x /= nrm;
            }
            v = next_v;
        }

        // Rayleigh quotient eigenvalue
        let mut av = vec![0.0; d];
        for i in 0..d {
            av[i] = cov[i].iter().zip(v.iter()).map(|(&a, &b)| a * b).sum();
        }
        let lambda = v
            .iter()
            .zip(av.iter())
            .map(|(&vi, &avi)| vi * avi)
            .sum::<f64>()
            .max(0.0);

        // Hotelling deflation: C <- C - lambda * v * v^T
        for i in 0..d {
            for j in 0..d {
                cov[i][j] -= lambda * v[i] * v[j];
            }
        }

        components.push(v);
        explained_variance.push(lambda);
    }

    Ok(MlModel::Pca {
        mean,
        components,
        explained_variance,
    })
}

fn sq_euclidean(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x - y) * (x - y))
        .sum()
}

// ==============================================================================
// Native VM Registration (`ML` & `__native_ml`)
// ==============================================================================

pub fn register_ml_module(globals: &mut HashMap<String, Value>) {
    let mut ml_mod = HashMap::new();

    // 1. Preprocessing: train_test_split
    ml_mod.insert(
        "train_test_split".to_string(),
        Value::Native("ML.train_test_split".into(), |args| {
            if args.len() < 2 {
                return Err("ML.train_test_split(X, y, [test_ratio=0.2], [seed=42]) requires at least 2 arguments".into());
            }
            let x = extract_matrix_2d(&args[0])?;
            let n = x.len();
            let y_list = match &args[1] {
                Value::Array(arr) => arr.lock().clone(),
                _ => {
                    let v = extract_vec_f64(&args[1])?;
                    v.into_iter().map(Value::Float).collect()
                }
            };
            if y_list.len() != n {
                return Err(format!("X has {} rows but y has {} elements", n, y_list.len()));
            }
            let test_ratio = if args.len() > 2 && args[2] != Value::Nil {
                val_as_f64(&args[2], "test_ratio")?.clamp(0.05, 0.95)
            } else {
                0.2
            };
            let seed = if args.len() > 3 && args[3] != Value::Nil {
                val_as_usize(&args[3], "seed")? as u64
            } else {
                42
            };

            let mut idx: Vec<usize> = (0..n).collect();
            let mut rng = Rng::new(seed);
            rng.shuffle(&mut idx);

            let test_len = ((n as f64) * test_ratio).round().max(1.0) as usize;
            let split_at = n.saturating_sub(test_len).max(1);

            let x_train: Vec<Vec<f64>> = idx[..split_at].iter().map(|&i| x[i].clone()).collect();
            let x_test: Vec<Vec<f64>> = idx[split_at..].iter().map(|&i| x[i].clone()).collect();
            let y_train: Vec<Value> = idx[..split_at].iter().map(|&i| y_list[i].clone()).collect();
            let y_test: Vec<Value> = idx[split_at..].iter().map(|&i| y_list[i].clone()).collect();

            let mut res = HashMap::new();
            res.insert("X_train".to_string(), matrix_to_value(&x_train));
            res.insert("X_test".to_string(), matrix_to_value(&x_test));
            res.insert("y_train".to_string(), Value::array(y_train));
            res.insert("y_test".to_string(), Value::array(y_test));
            Ok(Value::map(res))
        }),
    );

    // 2. StandardScaler & MinMaxScaler
    ml_mod.insert(
        "standard_scaler_fit".to_string(),
        Value::Native("ML.standard_scaler_fit".into(), |args| {
            if args.is_empty() {
                return Err("ML.standard_scaler_fit(X) requires 2D matrix X".into());
            }
            let x = extract_matrix_2d(&args[0])?;
            let n = x.len() as f64;
            let d = x[0].len();
            let mut mean = vec![0.0; d];
            for row in &x {
                for j in 0..d {
                    mean[j] += row[j];
                }
            }
            for m in &mut mean {
                *m /= n;
            }
            let mut var = vec![0.0; d];
            for row in &x {
                for j in 0..d {
                    let diff = row[j] - mean[j];
                    var[j] += diff * diff;
                }
            }
            let std: Vec<f64> = var
                .into_iter()
                .map(|v| {
                    let s = (v / n).sqrt();
                    if s < 1e-12 {
                        1.0
                    } else {
                        s
                    }
                })
                .collect();
            let id = store_model(MlModel::StandardScaler { mean, std });
            Ok(Value::Int(id as i64))
        }),
    );

    ml_mod.insert(
        "minmax_scaler_fit".to_string(),
        Value::Native("ML.minmax_scaler_fit".into(), |args| {
            if args.is_empty() {
                return Err("ML.minmax_scaler_fit(X) requires 2D matrix X".into());
            }
            let x = extract_matrix_2d(&args[0])?;
            let d = x[0].len();
            let mut min_v = vec![f64::INFINITY; d];
            let mut max_v = vec![f64::NEG_INFINITY; d];
            for row in &x {
                for j in 0..d {
                    min_v[j] = min_v[j].min(row[j]);
                    max_v[j] = max_v[j].max(row[j]);
                }
            }
            let scale: Vec<f64> = min_v
                .iter()
                .zip(max_v.iter())
                .map(|(&lo, &hi)| {
                    let span = hi - lo;
                    if span < 1e-12 {
                        1.0
                    } else {
                        1.0 / span
                    }
                })
                .collect();
            let id = store_model(MlModel::MinMaxScaler { min: min_v, scale });
            Ok(Value::Int(id as i64))
        }),
    );

    // 3. Linear / Ridge Regression
    ml_mod.insert(
        "linear_regression_fit".to_string(),
        Value::Native("ML.linear_regression_fit".into(), |args| {
            if args.len() < 2 {
                return Err("ML.linear_regression_fit(X, y, [l2=1e-6]) requires X and y".into());
            }
            let x = extract_matrix_2d(&args[0])?;
            let y = extract_vec_f64(&args[1])?;
            let l2 = if args.len() > 2 && args[2] != Value::Nil {
                val_as_f64(&args[2], "l2")?
            } else {
                1e-6
            };
            let id = store_model(fit_linear_regression(&x, &y, l2)?);
            Ok(Value::Int(id as i64))
        }),
    );

    // 4. Logistic Regression (Binary & Multinomial)
    ml_mod.insert(
        "logistic_regression_fit".to_string(),
        Value::Native("ML.logistic_regression_fit".into(), |args| {
            if args.len() < 2 {
                return Err(
                    "ML.logistic_regression_fit(X, y, [lr=0.1], [epochs=400], [l2=1e-4]) required"
                        .into(),
                );
            }
            let x = extract_matrix_2d(&args[0])?;
            let y = extract_vec_i64(&args[1])?;
            let lr = if args.len() > 2 && args[2] != Value::Nil {
                val_as_f64(&args[2], "lr")?
            } else {
                0.1
            };
            let epochs = if args.len() > 3 && args[3] != Value::Nil {
                val_as_usize(&args[3], "epochs")?
            } else {
                400
            };
            let l2 = if args.len() > 4 && args[4] != Value::Nil {
                val_as_f64(&args[4], "l2")?
            } else {
                1e-4
            };
            let id = store_model(fit_logistic_regression(&x, &y, lr, epochs, l2)?);
            Ok(Value::Int(id as i64))
        }),
    );

    // 5. Decision Tree Classifier
    ml_mod.insert(
        "decision_tree_fit".to_string(),
        Value::Native("ML.decision_tree_fit".into(), |args| {
            if args.len() < 2 {
                return Err(
                    "ML.decision_tree_fit(X, y, [max_depth=6], [min_samples_split=2]) required"
                        .into(),
                );
            }
            let x = extract_matrix_2d(&args[0])?;
            let y = extract_vec_i64(&args[1])?;
            if x.len() != y.len() {
                return Err("X and y length mismatch".into());
            }
            let max_depth = if args.len() > 2 && args[2] != Value::Nil {
                val_as_usize(&args[2], "max_depth")?
            } else {
                6
            };
            let min_samples_split = if args.len() > 3 && args[3] != Value::Nil {
                val_as_usize(&args[3], "min_samples_split")?.max(2)
            } else {
                2
            };
            let idx: Vec<usize> = (0..x.len()).collect();
            let mut rng = Rng::new(42);
            let cfg = TreeConfig {
                max_depth,
                min_samples_split,
                feature_subsample: None,
            };
            let root = build_tree(&x, &y, &idx, 0, &cfg, &mut rng);
            let id = store_model(MlModel::DecisionTree { root });
            Ok(Value::Int(id as i64))
        }),
    );

    // 6. Random Forest Classifier
    ml_mod.insert(
        "random_forest_fit".to_string(),
        Value::Native("ML.random_forest_fit".into(), |args| {
            if args.len() < 2 {
                return Err("ML.random_forest_fit(X, y, [n_estimators=15], [max_depth=6], [seed=42]) required".into());
            }
            let x = extract_matrix_2d(&args[0])?;
            let y = extract_vec_i64(&args[1])?;
            if x.len() != y.len() {
                return Err("X and y length mismatch".into());
            }
            let n_estimators = if args.len() > 2 && args[2] != Value::Nil {
                val_as_usize(&args[2], "n_estimators")?.max(1)
            } else {
                15
            };
            let max_depth = if args.len() > 3 && args[3] != Value::Nil {
                val_as_usize(&args[3], "max_depth")?.max(1)
            } else {
                6
            };
            let seed = if args.len() > 4 && args[4] != Value::Nil {
                val_as_usize(&args[4], "seed")? as u64
            } else {
                42
            };

            let n = x.len();
            let d = x[0].len();
            let feat_sub = ((d as f64).sqrt().ceil() as usize).max(1);
            let mut rng = Rng::new(seed);
            let mut trees = Vec::with_capacity(n_estimators);

            let cfg = TreeConfig {
                max_depth,
                min_samples_split: 2,
                feature_subsample: Some(feat_sub),
            };
            for _ in 0..n_estimators {
                let sample_idx: Vec<usize> = (0..n).map(|_| rng.gen_range(n)).collect();
                let tree = build_tree(&x, &y, &sample_idx, 0, &cfg, &mut rng);
                trees.push(tree);
            }

            let id = store_model(MlModel::RandomForest { trees });
            Ok(Value::Int(id as i64))
        }),
    );

    // 7. K-Nearest Neighbors
    ml_mod.insert(
        "knn_fit".to_string(),
        Value::Native("ML.knn_fit".into(), |args| {
            if args.len() < 2 {
                return Err("ML.knn_fit(X, y, [k=3]) requires X and y".into());
            }
            let x_train = extract_matrix_2d(&args[0])?;
            let y_train = extract_vec_i64(&args[1])?;
            if x_train.len() != y_train.len() {
                return Err("X and y length mismatch".into());
            }
            let k = if args.len() > 2 && args[2] != Value::Nil {
                val_as_usize(&args[2], "k")?.max(1)
            } else {
                3
            };
            let id = store_model(MlModel::Knn {
                x_train,
                y_train,
                k,
            });
            Ok(Value::Int(id as i64))
        }),
    );

    // 8. K-Means Clustering
    ml_mod.insert(
        "kmeans_fit".to_string(),
        Value::Native("ML.kmeans_fit".into(), |args| {
            if args.len() < 2 {
                return Err(
                    "ML.kmeans_fit(X, k, [max_iter=100], [seed=42]) requires X and k".into(),
                );
            }
            let x = extract_matrix_2d(&args[0])?;
            let k = val_as_usize(&args[1], "k")?;
            let max_iter = if args.len() > 2 && args[2] != Value::Nil {
                val_as_usize(&args[2], "max_iter")?
            } else {
                100
            };
            let seed = if args.len() > 3 && args[3] != Value::Nil {
                val_as_usize(&args[3], "seed")? as u64
            } else {
                42
            };
            let id = store_model(fit_kmeans(&x, k, max_iter, seed)?);
            Ok(Value::Int(id as i64))
        }),
    );

    ml_mod.insert(
        "kmeans_centroids".to_string(),
        Value::Native("ML.kmeans_centroids".into(), |args| {
            if args.is_empty() {
                return Err("ML.kmeans_centroids(model_id) requires model_id".into());
            }
            let id = val_as_usize(&args[0], "model_id")? as u64;
            let model = fetch_model(id)?;
            match model.as_ref() {
                MlModel::KMeans { centroids } => Ok(matrix_to_value(centroids)),
                _ => Err("Model handle is not a KMeans model".into()),
            }
        }),
    );

    // 9. Principal Component Analysis (PCA)
    ml_mod.insert(
        "pca_fit".to_string(),
        Value::Native("ML.pca_fit".into(), |args| {
            if args.len() < 2 {
                return Err("ML.pca_fit(X, n_components) requires X and n_components".into());
            }
            let x = extract_matrix_2d(&args[0])?;
            let n_components = val_as_usize(&args[1], "n_components")?;
            let id = store_model(fit_pca(&x, n_components)?);
            Ok(Value::Int(id as i64))
        }),
    );

    ml_mod.insert(
        "pca_explained_variance".to_string(),
        Value::Native("ML.pca_explained_variance".into(), |args| {
            if args.is_empty() {
                return Err("ML.pca_explained_variance(model_id) requires model_id".into());
            }
            let id = val_as_usize(&args[0], "model_id")? as u64;
            let model = fetch_model(id)?;
            match model.as_ref() {
                MlModel::Pca {
                    explained_variance, ..
                } => Ok(vec_f64_to_value(explained_variance)),
                _ => Err("Model handle is not a PCA model".into()),
            }
        }),
    );

    // 10. Unified Inference: transform / predict / predict_proba / model_weights
    ml_mod.insert(
        "transform".to_string(),
        Value::Native("ML.transform".into(), |args| {
            if args.len() < 2 {
                return Err("ML.transform(model_id, X) requires 2 arguments".into());
            }
            let id = val_as_usize(&args[0], "model_id")? as u64;
            let x = extract_matrix_2d(&args[1])?;
            let model = fetch_model(id)?;
            match model.as_ref() {
                MlModel::StandardScaler { mean, std } => {
                    let out: Vec<Vec<f64>> = x
                        .iter()
                        .map(|row| {
                            row.iter()
                                .enumerate()
                                .map(|(j, &v)| (v - mean[j]) / std[j])
                                .collect()
                        })
                        .collect();
                    Ok(matrix_to_value(&out))
                }
                MlModel::MinMaxScaler { min, scale } => {
                    let out: Vec<Vec<f64>> = x
                        .iter()
                        .map(|row| {
                            row.iter()
                                .enumerate()
                                .map(|(j, &v)| (v - min[j]) * scale[j])
                                .collect()
                        })
                        .collect();
                    Ok(matrix_to_value(&out))
                }
                MlModel::Pca {
                    mean, components, ..
                } => {
                    let out: Vec<Vec<f64>> = x
                        .iter()
                        .map(|row| {
                            components
                                .iter()
                                .map(|comp| {
                                    comp.iter()
                                        .enumerate()
                                        .map(|(j, &c)| (row[j] - mean[j]) * c)
                                        .sum()
                                })
                                .collect()
                        })
                        .collect();
                    Ok(matrix_to_value(&out))
                }
                _ => {
                    Err("ML.transform supports StandardScaler, MinMaxScaler, and PCA models".into())
                }
            }
        }),
    );

    ml_mod.insert(
        "predict".to_string(),
        Value::Native("ML.predict".into(), |args| {
            if args.len() < 2 {
                return Err("ML.predict(model_id, X) requires 2 arguments".into());
            }
            let id = val_as_usize(&args[0], "model_id")? as u64;
            let x = extract_matrix_2d(&args[1])?;
            let model = fetch_model(id)?;
            match model.as_ref() {
                MlModel::LinearRegression { weights, bias } => {
                    let preds: Vec<f64> = x
                        .iter()
                        .map(|row| {
                            weights
                                .iter()
                                .zip(row.iter())
                                .map(|(&w, &xi)| w * xi)
                                .sum::<f64>()
                                + *bias
                        })
                        .collect();
                    Ok(vec_f64_to_value(&preds))
                }
                MlModel::LogisticRegression {
                    classes,
                    weights,
                    bias,
                } => {
                    let probs = predict_logistic_proba(classes.len(), weights, bias, &x);
                    let preds: Vec<i64> = probs
                        .into_iter()
                        .map(|p| {
                            let best_idx = p
                                .iter()
                                .enumerate()
                                .max_by(|(_, a), (_, b)| a.total_cmp(b))
                                .map(|(i, _)| i)
                                .unwrap_or(0);
                            classes[best_idx]
                        })
                        .collect();
                    Ok(vec_i64_to_value(&preds))
                }
                MlModel::DecisionTree { root } => {
                    let preds: Vec<i64> = x.iter().map(|row| predict_tree(root, row)).collect();
                    Ok(vec_i64_to_value(&preds))
                }
                MlModel::RandomForest { trees } => {
                    let preds: Vec<i64> = x
                        .iter()
                        .map(|row| {
                            let mut votes: BTreeMap<i64, usize> = BTreeMap::new();
                            for t in trees {
                                *votes.entry(predict_tree(t, row)).or_insert(0) += 1;
                            }
                            votes
                                .into_iter()
                                .max_by_key(|&(_, count)| count)
                                .map(|(cls, _)| cls)
                                .unwrap_or(0)
                        })
                        .collect();
                    Ok(vec_i64_to_value(&preds))
                }
                MlModel::Knn {
                    x_train,
                    y_train,
                    k,
                } => {
                    let preds: Vec<i64> = x
                        .iter()
                        .map(|row| {
                            let mut dists: Vec<(f64, i64)> = x_train
                                .iter()
                                .zip(y_train.iter())
                                .map(|(tr, &lbl)| (sq_euclidean(row, tr), lbl))
                                .collect();
                            dists.sort_by(|a, b| a.0.total_cmp(&b.0));
                            let mut votes: BTreeMap<i64, usize> = BTreeMap::new();
                            for &(_, lbl) in dists.iter().take(*k) {
                                *votes.entry(lbl).or_insert(0) += 1;
                            }
                            votes
                                .into_iter()
                                .max_by_key(|&(_, count)| count)
                                .map(|(cls, _)| cls)
                                .unwrap_or(0)
                        })
                        .collect();
                    Ok(vec_i64_to_value(&preds))
                }
                MlModel::KMeans { centroids } => {
                    let labels: Vec<i64> = x
                        .iter()
                        .map(|row| {
                            centroids
                                .iter()
                                .enumerate()
                                .min_by(|(_, a), (_, b)| {
                                    sq_euclidean(row, a).total_cmp(&sq_euclidean(row, b))
                                })
                                .map(|(idx, _)| idx as i64)
                                .unwrap_or(0)
                        })
                        .collect();
                    Ok(vec_i64_to_value(&labels))
                }
                _ => Err("ML.predict is for trained supervised/clustering models; use ML.transform for scalers/PCA".into()),
            }
        }),
    );

    ml_mod.insert(
        "predict_proba".to_string(),
        Value::Native("ML.predict_proba".into(), |args| {
            if args.len() < 2 {
                return Err("ML.predict_proba(model_id, X) requires 2 arguments".into());
            }
            let id = val_as_usize(&args[0], "model_id")? as u64;
            let x = extract_matrix_2d(&args[1])?;
            let model = fetch_model(id)?;
            match model.as_ref() {
                MlModel::LogisticRegression {
                    classes,
                    weights,
                    bias,
                } => {
                    let probs = predict_logistic_proba(classes.len(), weights, bias, &x);
                    Ok(matrix_to_value(&probs))
                }
                _ => Err("ML.predict_proba currently supports LogisticRegression models".into()),
            }
        }),
    );

    ml_mod.insert(
        "linear_weights".to_string(),
        Value::Native("ML.linear_weights".into(), |args| {
            if args.is_empty() {
                return Err("ML.linear_weights(model_id) requires model_id".into());
            }
            let id = val_as_usize(&args[0], "model_id")? as u64;
            let model = fetch_model(id)?;
            match model.as_ref() {
                MlModel::LinearRegression { weights, bias } => {
                    let mut out = HashMap::new();
                    out.insert("weights".to_string(), vec_f64_to_value(weights));
                    out.insert("bias".to_string(), Value::Float(*bias));
                    Ok(Value::map(out))
                }
                _ => Err("Model handle is not a LinearRegression model".into()),
            }
        }),
    );

    // 11. Evaluation Metrics
    ml_mod.insert(
        "accuracy".to_string(),
        Value::Native("ML.accuracy".into(), |args| {
            if args.len() < 2 {
                return Err("ML.accuracy(y_true, y_pred) requires 2 arguments".into());
            }
            let yt = extract_vec_i64(&args[0])?;
            let yp = extract_vec_i64(&args[1])?;
            if yt.len() != yp.len() || yt.is_empty() {
                return Err("y_true and y_pred must have equal non-zero length".into());
            }
            let correct = yt.iter().zip(yp.iter()).filter(|(a, b)| a == b).count();
            Ok(Value::Float((correct as f64) / (yt.len() as f64)))
        }),
    );

    ml_mod.insert(
        "precision_recall_f1".to_string(),
        Value::Native("ML.precision_recall_f1".into(), |args| {
            if args.len() < 2 {
                return Err(
                    "ML.precision_recall_f1(y_true, y_pred, [pos_label=1]) requires 2 arguments"
                        .into(),
                );
            }
            let yt = extract_vec_i64(&args[0])?;
            let yp = extract_vec_i64(&args[1])?;
            if yt.len() != yp.len() || yt.is_empty() {
                return Err("y_true and y_pred must have equal non-zero length".into());
            }
            let pos = if args.len() > 2 && args[2] != Value::Nil {
                val_as_i64(&args[2], "pos_label")?
            } else {
                1
            };

            let mut tp = 0usize;
            let mut fp = 0usize;
            let mut fn_c = 0usize;
            for (&t, &p) in yt.iter().zip(yp.iter()) {
                if p == pos && t == pos {
                    tp += 1;
                } else if p == pos && t != pos {
                    fp += 1;
                } else if p != pos && t == pos {
                    fn_c += 1;
                }
            }
            let precision = if tp + fp == 0 {
                0.0
            } else {
                (tp as f64) / ((tp + fp) as f64)
            };
            let recall = if tp + fn_c == 0 {
                0.0
            } else {
                (tp as f64) / ((tp + fn_c) as f64)
            };
            let f1 = if precision + recall == 0.0 {
                0.0
            } else {
                2.0 * precision * recall / (precision + recall)
            };

            let mut res = HashMap::new();
            res.insert("precision".to_string(), Value::Float(precision));
            res.insert("recall".to_string(), Value::Float(recall));
            res.insert("f1".to_string(), Value::Float(f1));
            res.insert("support".to_string(), Value::Int((tp + fn_c) as i64));
            Ok(Value::map(res))
        }),
    );

    ml_mod.insert(
        "confusion_matrix".to_string(),
        Value::Native("ML.confusion_matrix".into(), |args| {
            if args.len() < 2 {
                return Err("ML.confusion_matrix(y_true, y_pred) requires 2 arguments".into());
            }
            let yt = extract_vec_i64(&args[0])?;
            let yp = extract_vec_i64(&args[1])?;
            if yt.len() != yp.len() {
                return Err("y_true and y_pred must have equal length".into());
            }
            let mut label_map = BTreeMap::new();
            for &v in yt.iter().chain(yp.iter()) {
                let next = label_map.len();
                label_map.entry(v).or_insert(next);
            }
            let k = label_map.len();
            let labels: Vec<i64> = label_map.keys().copied().collect();
            let mut mat = vec![vec![0i64; k]; k];
            for (&t, &p) in yt.iter().zip(yp.iter()) {
                mat[label_map[&t]][label_map[&p]] += 1;
            }
            let mat_val = Value::array(mat.iter().map(|r| vec_i64_to_value(r)).collect());
            let mut res = HashMap::new();
            res.insert("labels".to_string(), vec_i64_to_value(&labels));
            res.insert("matrix".to_string(), mat_val);
            Ok(Value::map(res))
        }),
    );

    ml_mod.insert(
        "mse".to_string(),
        Value::Native("ML.mse".into(), |args| {
            let (yt, yp) = extract_pair_f64(args, "ML.mse")?;
            let err: f64 = yt
                .iter()
                .zip(yp.iter())
                .map(|(&a, &b)| (a - b) * (a - b))
                .sum::<f64>()
                / (yt.len() as f64);
            Ok(Value::Float(err))
        }),
    );

    ml_mod.insert(
        "rmse".to_string(),
        Value::Native("ML.rmse".into(), |args| {
            let (yt, yp) = extract_pair_f64(args, "ML.rmse")?;
            let err: f64 = yt
                .iter()
                .zip(yp.iter())
                .map(|(&a, &b)| (a - b) * (a - b))
                .sum::<f64>()
                / (yt.len() as f64);
            Ok(Value::Float(err.sqrt()))
        }),
    );

    ml_mod.insert(
        "r2_score".to_string(),
        Value::Native("ML.r2_score".into(), |args| {
            let (yt, yp) = extract_pair_f64(args, "ML.r2_score")?;
            let mean_y = yt.iter().sum::<f64>() / (yt.len() as f64);
            let ss_tot: f64 = yt.iter().map(|&y| (y - mean_y) * (y - mean_y)).sum();
            let ss_res: f64 = yt
                .iter()
                .zip(yp.iter())
                .map(|(&a, &b)| (a - b) * (a - b))
                .sum();
            let r2 = if ss_tot < 1e-15 {
                if ss_res < 1e-15 {
                    1.0
                } else {
                    0.0
                }
            } else {
                1.0 - (ss_res / ss_tot)
            };
            Ok(Value::Float(r2))
        }),
    );

    ml_mod.insert(
        "roc_auc".to_string(),
        Value::Native("ML.roc_auc".into(), |args| {
            if args.len() < 2 {
                return Err("ML.roc_auc(y_true, y_scores) requires 2 arguments".into());
            }
            let yt = extract_vec_i64(&args[0])?;
            let scores = extract_vec_f64(&args[1])?;
            if yt.len() != scores.len() || yt.is_empty() {
                return Err("y_true and y_scores must have equal non-zero length".into());
            }
            let pos_total = yt.iter().filter(|&&y| y == 1).count() as f64;
            let neg_total = yt.iter().filter(|&&y| y != 1).count() as f64;
            if pos_total == 0.0 || neg_total == 0.0 {
                return Err(
                    "ML.roc_auc requires both positive (1) and negative (0) samples".into(),
                );
            }

            let mut pairs: Vec<(f64, i64)> = scores.into_iter().zip(yt).collect();
            pairs.sort_by(|a, b| b.0.total_cmp(&a.0));

            let mut tp = 0.0;
            let mut fp = 0.0;
            let mut prev_tpr = 0.0;
            let mut prev_fpr = 0.0;
            let mut auc = 0.0;

            for i in 0..pairs.len() {
                if pairs[i].1 == 1 {
                    tp += 1.0;
                } else {
                    fp += 1.0;
                }
                let is_last_or_diff =
                    i + 1 == pairs.len() || (pairs[i + 1].0 - pairs[i].0).abs() > 1e-12;
                if is_last_or_diff {
                    let tpr = tp / pos_total;
                    let fpr = fp / neg_total;
                    auc += 0.5 * (fpr - prev_fpr) * (tpr + prev_tpr);
                    prev_tpr = tpr;
                    prev_fpr = fpr;
                }
            }
            Ok(Value::Float(auc))
        }),
    );

    // 12. Model Persistence (JSON Save / Load for Edge Deployment)
    ml_mod.insert(
        "save_model".to_string(),
        Value::Native("ML.save_model".into(), |args| {
            if args.len() < 2 {
                return Err("ML.save_model(model_id, path) requires 2 arguments".into());
            }
            let id = val_as_usize(&args[0], "model_id")? as u64;
            let path = format!("{}", args[1]);
            crate::vm::interpreter::with_current_vm(|vm| vm.security_policy.check_fs_write(&path))?;
            let model = fetch_model(id)?;
            let json = serde_json::to_string_pretty(model.as_ref())
                .map_err(|e| format!("Failed to serialize ML model: {}", e))?;
            fs::write(&path, json)
                .map_err(|e| format!("Failed to save ML model to '{}': {}", path, e))?;
            Ok(Value::Bool(true))
        }),
    );

    ml_mod.insert(
        "load_model".to_string(),
        Value::Native("ML.load_model".into(), |args| {
            if args.is_empty() {
                return Err("ML.load_model(path) requires path".into());
            }
            let path = format!("{}", args[0]);
            crate::vm::interpreter::with_current_vm(|vm| vm.security_policy.check_fs_read(&path))?;
            let json = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read ML model from '{}': {}", path, e))?;
            let model: MlModel = serde_json::from_str(&json)
                .map_err(|e| format!("Invalid ML model JSON in '{}': {}", path, e))?;
            let id = store_model(model);
            Ok(Value::Int(id as i64))
        }),
    );

    globals.insert("ML".to_string(), Value::map(ml_mod.clone()));
    globals.insert("__native_ml".to_string(), Value::map(ml_mod));
}

fn extract_pair_f64(args: &[Value], fn_name: &str) -> Result<(Vec<f64>, Vec<f64>), String> {
    if args.len() < 2 {
        return Err(format!("{}(y_true, y_pred) requires 2 arguments", fn_name));
    }
    let yt = extract_vec_f64(&args[0])?;
    let yp = extract_vec_f64(&args[1])?;
    if yt.len() != yp.len() || yt.is_empty() {
        return Err(format!(
            "{}: y_true and y_pred must have equal non-zero length",
            fn_name
        ));
    }
    Ok((yt, yp))
}
