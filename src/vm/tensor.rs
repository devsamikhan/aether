use super::value::Value;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

static NEXT_TENSOR_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub struct TensorData {
    pub shape: Vec<usize>,
    pub strides: Vec<usize>,
    pub data: Vec<f64>,
}

impl TensorData {
    pub fn new(shape: Vec<usize>, data: Vec<f64>) -> Result<Self, String> {
        let expected_size: usize = shape.iter().product();
        if expected_size != data.len() {
            return Err(format!(
                "Shape {:?} requires {} elements, but provided data has {}",
                shape, expected_size, data.len()
            ));
        }
        let strides = compute_strides(&shape);
        Ok(Self { shape, strides, data })
    }

    pub fn zeros(shape: Vec<usize>) -> Self {
        let size: usize = shape.iter().product();
        let strides = compute_strides(&shape);
        Self {
            shape,
            strides,
            data: vec![0.0; size],
        }
    }

    pub fn ones(shape: Vec<usize>) -> Self {
        let size: usize = shape.iter().product();
        let strides = compute_strides(&shape);
        Self {
            shape,
            strides,
            data: vec![1.0; size],
        }
    }

    pub fn eye(n: usize) -> Self {
        let mut data = vec![0.0; n * n];
        for i in 0..n {
            data[i * n + i] = 1.0;
        }
        Self {
            shape: vec![n, n],
            strides: vec![n, 1],
            data,
        }
    }

    pub fn random(shape: Vec<usize>, min: f64, max: f64) -> Self {
        let size: usize = shape.iter().product();
        let strides = compute_strides(&shape);
        // Simple linear-congruential pseudo-random generator (seedable & zero-dependency)
        let mut seed = 123456789u64.wrapping_add(size as u64 * 31);
        let mut data = Vec::with_capacity(size);
        for _ in 0..size {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let unit = ((seed >> 32) as u32 as f64) / (u32::MAX as f64);
            let val = min + unit * (max - min);
            data.push(val);
        }
        Self { shape, strides, data }
    }

    pub fn matmul(&self, other: &Self) -> Result<Self, String> {
        if self.shape.len() != 2 || other.shape.len() != 2 {
            return Err(format!(
                "matmul currently requires 2D matrices, got shapes {:?} and {:?}",
                self.shape, other.shape
            ));
        }
        let (m, k1) = (self.shape[0], self.shape[1]);
        let (k2, n) = (other.shape[0], other.shape[1]);
        if k1 != k2 {
            return Err(format!(
                "Incompatible matrix dimensions for matmul: ({}, {}) x ({}, {})",
                m, k1, k2, n
            ));
        }

        let mut out_data = vec![0.0; m * n];
        // Cache-friendly matrix multiplication (IKJ loop order)
        for i in 0..m {
            let a_row_offset = i * k1;
            let c_row_offset = i * n;
            for k in 0..k1 {
                let a_val = self.data[a_row_offset + k];
                let b_row_offset = k * n;
                for j in 0..n {
                    out_data[c_row_offset + j] += a_val * other.data[b_row_offset + j];
                }
            }
        }

        Self::new(vec![m, n], out_data)
    }

    pub fn transpose(&self) -> Result<Self, String> {
        if self.shape.len() != 2 {
            return Err(format!("transpose requires 2D matrix, got shape {:?}", self.shape));
        }
        let (rows, cols) = (self.shape[0], self.shape[1]);
        let mut out = vec![0.0; rows * cols];
        for r in 0..rows {
            for c in 0..cols {
                out[c * rows + r] = self.data[r * cols + c];
            }
        }
        Self::new(vec![cols, rows], out)
    }

    pub fn reshape(&self, new_shape: Vec<usize>) -> Result<Self, String> {
        Self::new(new_shape, self.data.clone())
    }

    pub fn sum(&self) -> f64 {
        self.data.iter().sum()
    }

    pub fn mean(&self) -> f64 {
        if self.data.is_empty() {
            0.0
        } else {
            self.sum() / (self.data.len() as f64)
        }
    }

    pub fn relu(&self) -> Self {
        let data: Vec<f64> = self.data.iter().map(|&x| if x > 0.0 { x } else { 0.0 }).collect();
        Self {
            shape: self.shape.clone(),
            strides: self.strides.clone(),
            data,
        }
    }

    pub fn sigmoid(&self) -> Self {
        let data: Vec<f64> = self.data.iter().map(|&x| 1.0 / (1.0 + (-x).exp())).collect();
        Self {
            shape: self.shape.clone(),
            strides: self.strides.clone(),
            data,
        }
    }

    pub fn softmax(&self) -> Result<Self, String> {
        if self.shape.len() != 2 {
            // Flattened 1D softmax
            let max_val = self.data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let exps: Vec<f64> = self.data.iter().map(|&x| (x - max_val).exp()).collect();
            let sum_exp: f64 = exps.iter().sum();
            let data = exps.into_iter().map(|x| x / sum_exp).collect();
            return Ok(Self {
                shape: self.shape.clone(),
                strides: self.strides.clone(),
                data,
            });
        }

        // 2D row-wise softmax (batch, classes)
        let (rows, cols) = (self.shape[0], self.shape[1]);
        let mut out = vec![0.0; rows * cols];
        for r in 0..rows {
            let offset = r * cols;
            let mut max_val = f64::NEG_INFINITY;
            for c in 0..cols {
                let v = self.data[offset + c];
                if v > max_val {
                    max_val = v;
                }
            }
            let mut sum_exp = 0.0;
            for c in 0..cols {
                let exp_v = (self.data[offset + c] - max_val).exp();
                out[offset + c] = exp_v;
                sum_exp += exp_v;
            }
            for c in 0..cols {
                out[offset + c] /= sum_exp;
            }
        }
        Self::new(vec![rows, cols], out)
    }

    pub fn zeros_like(&self) -> Self {
        Self::zeros(self.shape.clone())
    }

    pub fn ones_like(&self) -> Self {
        Self::ones(self.shape.clone())
    }

    pub fn gt_mask(&self, threshold: f64) -> Self {
        let data: Vec<f64> = self.data.iter().map(|&x| if x > threshold { 1.0 } else { 0.0 }).collect();
        Self {
            shape: self.shape.clone(),
            strides: self.strides.clone(),
            data,
        }
    }

    pub fn reduce_sum_to(&self, target_shape: &[usize]) -> Result<Self, String> {
        if self.shape == target_shape {
            return Ok(self.clone());
        }
        let target_size: usize = target_shape.iter().product();
        if target_size == 0 {
            return Self::new(target_shape.to_vec(), Vec::new());
        }

        let ndim = self.shape.len();
        let target_ndim = target_shape.len();
        if target_ndim > ndim {
            return Err(format!(
                "Cannot reduce tensor of shape {:?} to larger rank {:?}",
                self.shape, target_shape
            ));
        }

        let mut aligned_target_shape = vec![1; ndim];
        let offset = ndim - target_ndim;
        for i in 0..target_ndim {
            aligned_target_shape[offset + i] = target_shape[i];
        }

        for d in 0..ndim {
            if aligned_target_shape[d] != 1 && aligned_target_shape[d] != self.shape[d] {
                return Err(format!(
                    "Cannot reduce tensor of shape {:?} to incompatible shape {:?}",
                    self.shape, target_shape
                ));
            }
        }

        let aligned_strides = compute_strides(&aligned_target_shape);
        let mut out_data = vec![0.0; target_size];

        let mut coords = vec![0usize; ndim];
        for (i, &val) in self.data.iter().enumerate() {
            let mut rem = i;
            for d in 0..ndim {
                coords[d] = rem / self.strides[d];
                rem %= self.strides[d];
            }

            let mut target_idx = 0;
            for d in 0..ndim {
                let c = if aligned_target_shape[d] == 1 { 0 } else { coords[d] };
                target_idx += c * aligned_strides[d];
            }

            out_data[target_idx] += val;
        }

        Self::new(target_shape.to_vec(), out_data)
    }

    pub fn clip(&self, min_val: f64, max_val: f64) -> Self {
        let data: Vec<f64> = self.data.iter().map(|&x| x.clamp(min_val, max_val)).collect();
        Self {
            shape: self.shape.clone(),
            strides: self.strides.clone(),
            data,
        }
    }

    pub fn to_nested_value(&self) -> Value {
        fn build_rec(shape: &[usize], data: &[f64]) -> Value {
            if shape.is_empty() {
                return Value::Nil;
            }
            if shape.len() == 1 {
                let items: Vec<Value> = data.iter().map(|&x| Value::Float(x)).collect();
                return Value::array(items);
            }
            let sub_size: usize = shape[1..].iter().product();
            let mut list = Vec::new();
            for i in 0..shape[0] {
                let chunk = &data[i * sub_size..(i + 1) * sub_size];
                list.push(build_rec(&shape[1..], chunk));
            }
            Value::array(list)
        }
        build_rec(&self.shape, &self.data)
    }

    pub fn format_display(&self) -> String {
        let shape_str = format!("{:?}", self.shape);
        if self.shape.len() <= 2 && self.data.len() <= 20 {
            let nested = self.to_nested_value();
            format!("Tensor({}, shape={})", nested, shape_str)
        } else {
            format!("Tensor(data_len={}, shape={})", self.data.len(), shape_str)
        }
    }
}

fn compute_strides(shape: &[usize]) -> Vec<usize> {
    let mut strides = vec![1; shape.len()];
    for i in (0..shape.len().saturating_sub(1)).rev() {
        strides[i] = strides[i + 1] * shape[i + 1];
    }
    strides
}

static TENSORS: std::sync::OnceLock<Mutex<HashMap<u64, Arc<Mutex<TensorData>>>>> = std::sync::OnceLock::new();

fn get_tensors() -> &'static Mutex<HashMap<u64, Arc<Mutex<TensorData>>>> {
    TENSORS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn wrap_tensor_handle(data: TensorData) -> Value {
    let id = NEXT_TENSOR_ID.fetch_add(1, Ordering::SeqCst);
    let shape_vals: Vec<Value> = data.shape.iter().map(|&s| Value::Int(s as i64)).collect();
    let size = data.data.len() as i64;
    get_tensors().lock().insert(id, Arc::new(Mutex::new(data)));

    let mut map = HashMap::new();
    map.insert("_tensor_id".to_string(), Value::Int(id as i64));
    map.insert("shape".to_string(), Value::array(shape_vals));
    map.insert("size".to_string(), Value::Int(size));
    Value::map(map)
}

fn get_tensor_data(val: &Value) -> Result<TensorData, String> {
    match val {
        Value::Map(m) => {
            let map = m.lock();
            if let Some(Value::Int(id)) = map.get("_tensor_id") {
                let tensors = get_tensors().lock();
                let tensor_arc = tensors
                    .get(&(*id as u64))
                    .cloned()
                    .ok_or_else(|| "Invalid tensor handle".to_string())?;
                drop(tensors);
                let data = tensor_arc.lock().clone();
                Ok(data)
            } else {
                Err("Map is not a valid Tensor handle".into())
            }
        }
        Value::Array(_) => {
            // Auto convert flat or nested array into 1D or 2D tensor
            let (shape, data) = array_to_flat_f64(val)?;
            TensorData::new(shape, data)
        }
        Value::Int(i) => Ok(TensorData::new(vec![1], vec![*i as f64])?),
        Value::Float(f) => Ok(TensorData::new(vec![1], vec![*f])?),
        other => Err(format!("Expected Tensor or numeric array, got '{}'", other.type_name())),
    }
}

fn array_to_flat_f64(val: &Value) -> Result<(Vec<usize>, Vec<f64>), String> {
    match val {
        Value::Array(a) => {
            let list = a.lock();
            if list.is_empty() {
                return Ok((vec![0], Vec::new()));
            }
            if let Value::Array(_) = &list[0] {
                // 2D Array
                let rows = list.len();
                let mut data = Vec::new();
                let mut cols = None;
                for row in list.iter() {
                    if let Value::Array(r) = row {
                        let r_list = r.lock();
                        if let Some(c) = cols {
                            if c != r_list.len() {
                                return Err("Jagged 2D array cannot be converted to tensor".into());
                            }
                        } else {
                            cols = Some(r_list.len());
                        }
                        for item in r_list.iter() {
                            let n = match item {
                                Value::Int(i) => *i as f64,
                                Value::Float(f) => *f,
                                _ => return Err("Tensor array elements must be numbers".into()),
                            };
                            data.push(n);
                        }
                    } else {
                        return Err("Expected 2D array".into());
                    }
                }
                Ok((vec![rows, cols.unwrap_or(0)], data))
            } else {
                // 1D Array
                let mut data = Vec::with_capacity(list.len());
                for item in list.iter() {
                    let n = match item {
                        Value::Int(i) => *i as f64,
                        Value::Float(f) => *f,
                        _ => return Err("Tensor array elements must be numbers".into()),
                    };
                    data.push(n);
                }
                Ok((vec![data.len()], data))
            }
        }
        _ => Err("Expected Array".into()),
    }
}

fn extract_shape(val: &Value) -> Result<Vec<usize>, String> {
    match val {
        Value::Array(arr) => {
            let list = arr.lock();
            let mut shape = Vec::with_capacity(list.len());
            for v in list.iter() {
                match v {
                    Value::Int(i) => {
                        if *i <= 0 {
                            return Err("Shape dimensions must be positive integers".into());
                        }
                        shape.push(*i as usize);
                    }
                    _ => return Err("Shape must contain integers".into()),
                }
            }
            Ok(shape)
        }
        Value::Int(i) => {
            if *i <= 0 {
                return Err("Shape dimension must be positive".into());
            }
            Ok(vec![*i as usize])
        }
        other => Err(format!("Expected shape array or integer, got '{}'", other.type_name())),
    }
}

pub fn register_tensor_module(globals: &mut HashMap<String, Value>) {
    let mut tensor_module = HashMap::new();

    // 1. Tensor.zeros(shape)
    tensor_module.insert(
        "zeros".to_string(),
        Value::Native("Tensor.zeros".into(), |args| {
            if args.is_empty() {
                return Err("Tensor.zeros(shape) requires shape".into());
            }
            let shape = extract_shape(&args[0])?;
            Ok(wrap_tensor_handle(TensorData::zeros(shape)))
        }),
    );

    // 2. Tensor.ones(shape)
    tensor_module.insert(
        "ones".to_string(),
        Value::Native("Tensor.ones".into(), |args| {
            if args.is_empty() {
                return Err("Tensor.ones(shape) requires shape".into());
            }
            let shape = extract_shape(&args[0])?;
            Ok(wrap_tensor_handle(TensorData::ones(shape)))
        }),
    );

    // 3. Tensor.eye(n)
    tensor_module.insert(
        "eye".to_string(),
        Value::Native("Tensor.eye".into(), |args| {
            if args.is_empty() {
                return Err("Tensor.eye(n) requires integer size".into());
            }
            let n = match args[0] {
                Value::Int(i) if i > 0 => i as usize,
                _ => return Err("Tensor.eye(n) requires positive integer".into()),
            };
            Ok(wrap_tensor_handle(TensorData::eye(n)))
        }),
    );

    // 4. Tensor.random(shape, min?, max?)
    tensor_module.insert(
        "random".to_string(),
        Value::Native("Tensor.random".into(), |args| {
            if args.is_empty() {
                return Err("Tensor.random(shape, min=0.0, max=1.0) requires shape".into());
            }
            let shape = extract_shape(&args[0])?;
            let min = if args.len() > 1 {
                match args[1] {
                    Value::Float(f) => f,
                    Value::Int(i) => i as f64,
                    _ => 0.0,
                }
            } else {
                0.0
            };
            let max = if args.len() > 2 {
                match args[2] {
                    Value::Float(f) => f,
                    Value::Int(i) => i as f64,
                    _ => 1.0,
                }
            } else {
                1.0
            };
            Ok(wrap_tensor_handle(TensorData::random(shape, min, max)))
        }),
    );

    // 5. Tensor.from_array(array, shape?)
    tensor_module.insert(
        "from_array".to_string(),
        Value::Native("Tensor.from_array".into(), |args| {
            if args.is_empty() {
                return Err("Tensor.from_array(array, shape?) requires array".into());
            }
            let (inferred_shape, data) = array_to_flat_f64(&args[0])?;
            let final_shape = if args.len() > 1 {
                extract_shape(&args[1])?
            } else {
                inferred_shape
            };
            let t = TensorData::new(final_shape, data)?;
            Ok(wrap_tensor_handle(t))
        }),
    );

    // 6. Tensor.matmul(t1, t2)
    tensor_module.insert(
        "matmul".to_string(),
        Value::Native("Tensor.matmul".into(), |args| {
            if args.len() < 2 {
                return Err("Tensor.matmul(a, b) requires 2 tensors".into());
            }
            let a = get_tensor_data(&args[0])?;
            let b = get_tensor_data(&args[1])?;
            let c = a.matmul(&b)?;
            Ok(wrap_tensor_handle(c))
        }),
    );

    // 7. Element-wise arithmetic: add, sub, mul, div
    tensor_module.insert(
        "add".to_string(),
        Value::Native("Tensor.add".into(), |args| {
            binary_op(args, |x, y| x + y)
        }),
    );
    tensor_module.insert(
        "sub".to_string(),
        Value::Native("Tensor.sub".into(), |args| {
            binary_op(args, |x, y| x - y)
        }),
    );
    tensor_module.insert(
        "mul".to_string(),
        Value::Native("Tensor.mul".into(), |args| {
            binary_op(args, |x, y| x * y)
        }),
    );
    tensor_module.insert(
        "div".to_string(),
        Value::Native("Tensor.div".into(), |args| {
            binary_op(args, |x, y| x / y)
        }),
    );

    // 8. Tensor.transpose(t)
    tensor_module.insert(
        "transpose".to_string(),
        Value::Native("Tensor.transpose".into(), |args| {
            if args.is_empty() {
                return Err("Tensor.transpose(t) requires tensor".into());
            }
            let t = get_tensor_data(&args[0])?;
            let res = t.transpose()?;
            Ok(wrap_tensor_handle(res))
        }),
    );

    // 9. Tensor.reshape(t, new_shape)
    tensor_module.insert(
        "reshape".to_string(),
        Value::Native("Tensor.reshape".into(), |args| {
            if args.len() < 2 {
                return Err("Tensor.reshape(t, new_shape) requires 2 arguments".into());
            }
            let t = get_tensor_data(&args[0])?;
            let new_shape = extract_shape(&args[1])?;
            let res = t.reshape(new_shape)?;
            Ok(wrap_tensor_handle(res))
        }),
    );

    // 10. Reductions: sum, mean
    tensor_module.insert(
        "sum".to_string(),
        Value::Native("Tensor.sum".into(), |args| {
            if args.is_empty() {
                return Err("Tensor.sum(t) requires tensor".into());
            }
            let t = get_tensor_data(&args[0])?;
            Ok(Value::Float(t.sum()))
        }),
    );
    tensor_module.insert(
        "mean".to_string(),
        Value::Native("Tensor.mean".into(), |args| {
            if args.is_empty() {
                return Err("Tensor.mean(t) requires tensor".into());
            }
            let t = get_tensor_data(&args[0])?;
            Ok(Value::Float(t.mean()))
        }),
    );

    // 11. Activations: relu, sigmoid, softmax
    tensor_module.insert(
        "relu".to_string(),
        Value::Native("Tensor.relu".into(), |args| {
            if args.is_empty() {
                return Err("Tensor.relu(t) requires tensor".into());
            }
            let t = get_tensor_data(&args[0])?;
            Ok(wrap_tensor_handle(t.relu()))
        }),
    );
    tensor_module.insert(
        "sigmoid".to_string(),
        Value::Native("Tensor.sigmoid".into(), |args| {
            if args.is_empty() {
                return Err("Tensor.sigmoid(t) requires tensor".into());
            }
            let t = get_tensor_data(&args[0])?;
            Ok(wrap_tensor_handle(t.sigmoid()))
        }),
    );
    tensor_module.insert(
        "softmax".to_string(),
        Value::Native("Tensor.softmax".into(), |args| {
            if args.is_empty() {
                return Err("Tensor.softmax(t) requires tensor".into());
            }
            let t = get_tensor_data(&args[0])?;
            let res = t.softmax()?;
            Ok(wrap_tensor_handle(res))
        }),
    );

    // 12. Serialization: to_list, to_string
    tensor_module.insert(
        "to_list".to_string(),
        Value::Native("Tensor.to_list".into(), |args| {
            if args.is_empty() {
                return Err("Tensor.to_list(t) requires tensor".into());
            }
            let t = get_tensor_data(&args[0])?;
            Ok(t.to_nested_value())
        }),
    );
    tensor_module.insert(
        "to_string".to_string(),
        Value::Native("Tensor.to_string".into(), |args| {
            if args.is_empty() {
                return Err("Tensor.to_string(t) requires tensor".into());
            }
            let t = get_tensor_data(&args[0])?;
            Ok(Value::string(t.format_display()))
        }),
    );

    // 13. Autograd Acceleration helpers
    tensor_module.insert(
        "zeros_like".to_string(),
        Value::Native("Tensor.zeros_like".into(), |args| {
            if args.is_empty() {
                return Err("Tensor.zeros_like(t) requires tensor".into());
            }
            let t = get_tensor_data(&args[0])?;
            Ok(wrap_tensor_handle(t.zeros_like()))
        }),
    );
    tensor_module.insert(
        "ones_like".to_string(),
        Value::Native("Tensor.ones_like".into(), |args| {
            if args.is_empty() {
                return Err("Tensor.ones_like(t) requires tensor".into());
            }
            let t = get_tensor_data(&args[0])?;
            Ok(wrap_tensor_handle(t.ones_like()))
        }),
    );
    tensor_module.insert(
        "gt_mask".to_string(),
        Value::Native("Tensor.gt_mask".into(), |args| {
            if args.is_empty() {
                return Err("Tensor.gt_mask(t, [threshold]) requires tensor".into());
            }
            let t = get_tensor_data(&args[0])?;
            let threshold = if args.len() > 1 {
                match args[1] {
                    Value::Int(i) => i as f64,
                    Value::Float(f) => f,
                    _ => 0.0,
                }
            } else {
                0.0
            };
            Ok(wrap_tensor_handle(t.gt_mask(threshold)))
        }),
    );
    tensor_module.insert(
        "reduce_sum_to".to_string(),
        Value::Native("Tensor.reduce_sum_to".into(), |args| {
            if args.len() < 2 {
                return Err("Tensor.reduce_sum_to(t, target_shape) requires tensor and target_shape".into());
            }
            let t = get_tensor_data(&args[0])?;
            let target_shape = extract_shape(&args[1])?;
            let res = t.reduce_sum_to(&target_shape)?;
            Ok(wrap_tensor_handle(res))
        }),
    );
    tensor_module.insert(
        "clip".to_string(),
        Value::Native("Tensor.clip".into(), |args| {
            if args.len() < 3 {
                return Err("Tensor.clip(t, min_val, max_val) requires 3 arguments".into());
            }
            let t = get_tensor_data(&args[0])?;
            let min_val = match args[1] {
                Value::Int(i) => i as f64,
                Value::Float(f) => f,
                _ => return Err("min_val must be a number".into()),
            };
            let max_val = match args[2] {
                Value::Int(i) => i as f64,
                Value::Float(f) => f,
                _ => return Err("max_val must be a number".into()),
            };
            Ok(wrap_tensor_handle(t.clip(min_val, max_val)))
        }),
    );

    globals.entry("__native_tensor".to_string()).or_insert_with(|| Value::map(tensor_module.clone()));
    globals.entry("__native_matrix".to_string()).or_insert_with(|| Value::map(tensor_module.clone()));
    globals.entry("Matrix".to_string()).or_insert_with(|| Value::map(tensor_module.clone()));
    globals.entry("matrix".to_string()).or_insert_with(|| Value::map(tensor_module));
}

fn binary_op<F>(args: &[Value], op: F) -> Result<Value, String>
where
    F: Fn(f64, f64) -> f64,
{
    if args.len() < 2 {
        return Err("Binary tensor operation requires 2 arguments".into());
    }
    let t1 = get_tensor_data(&args[0])?;

    // Check if second argument is scalar
    match &args[1] {
        Value::Int(i) => {
            let s = *i as f64;
            let out = t1.data.iter().map(|&x| op(x, s)).collect();
            let res = TensorData::new(t1.shape.clone(), out)?;
            return Ok(wrap_tensor_handle(res));
        }
        Value::Float(f) => {
            let s = *f;
            let out = t1.data.iter().map(|&x| op(x, s)).collect();
            let res = TensorData::new(t1.shape.clone(), out)?;
            return Ok(wrap_tensor_handle(res));
        }
        _ => {}
    }

    let t2 = get_tensor_data(&args[1])?;

    if t1.shape == t2.shape {
        let out: Vec<f64> = t1.data.iter().zip(t2.data.iter()).map(|(&x, &y)| op(x, y)).collect();
        let res = TensorData::new(t1.shape.clone(), out)?;
        return Ok(wrap_tensor_handle(res));
    }

    // Scalar (length 1) broadcasting
    if t2.data.len() == 1 {
        let s = t2.data[0];
        let out = t1.data.iter().map(|&x| op(x, s)).collect();
        let res = TensorData::new(t1.shape.clone(), out)?;
        return Ok(wrap_tensor_handle(res));
    }
    if t1.data.len() == 1 {
        let s = t1.data[0];
        let out = t2.data.iter().map(|&y| op(s, y)).collect();
        let res = TensorData::new(t2.shape.clone(), out)?;
        return Ok(wrap_tensor_handle(res));
    }

    // Universal N-Dimensional Broadcasting
    let rank1 = t1.shape.len();
    let rank2 = t2.shape.len();
    let max_rank = rank1.max(rank2);
    let mut padded1 = vec![1; max_rank];
    let mut padded2 = vec![1; max_rank];
    for (i, &d) in t1.shape.iter().rev().enumerate() {
        padded1[max_rank - 1 - i] = d;
    }
    for (i, &d) in t2.shape.iter().rev().enumerate() {
        padded2[max_rank - 1 - i] = d;
    }

    let mut out_shape = vec![0; max_rank];
    let mut compatible = true;
    for i in 0..max_rank {
        let d1 = padded1[i];
        let d2 = padded2[i];
        if d1 == d2 {
            out_shape[i] = d1;
        } else if d1 == 1 {
            out_shape[i] = d2;
        } else if d2 == 1 {
            out_shape[i] = d1;
        } else {
            compatible = false;
            break;
        }
    }

    if compatible {
        let total_size: usize = out_shape.iter().product();
        let out_strides = compute_strides(&out_shape);

        let mut eff_strides1 = vec![0; max_rank];
        let mut eff_strides2 = vec![0; max_rank];

        let offset1 = max_rank - rank1;
        for i in 0..rank1 {
            if t1.shape[i] > 1 {
                eff_strides1[offset1 + i] = t1.strides[i];
            }
        }

        let offset2 = max_rank - rank2;
        for i in 0..rank2 {
            if t2.shape[i] > 1 {
                eff_strides2[offset2 + i] = t2.strides[i];
            }
        }

        let mut out = Vec::with_capacity(total_size);
        for idx in 0..total_size {
            let mut rem = idx;
            let mut idx1 = 0;
            let mut idx2 = 0;
            for dim in 0..max_rank {
                let coord = rem / out_strides[dim];
                rem %= out_strides[dim];
                idx1 += coord * eff_strides1[dim];
                idx2 += coord * eff_strides2[dim];
            }
            out.push(op(t1.data[idx1], t2.data[idx2]));
        }

        let res = TensorData::new(out_shape, out)?;
        return Ok(wrap_tensor_handle(res));
    }

    Err(format!(
        "Broadcasting not supported between shapes {:?} and {:?}",
        t1.shape, t2.shape
    ))
}
