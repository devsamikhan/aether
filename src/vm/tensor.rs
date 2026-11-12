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
                shape,
                expected_size,
                data.len()
            ));
        }
        let strides = compute_strides(&shape);
        Ok(Self {
            shape,
            strides,
            data,
        })
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
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let unit = ((seed >> 32) as u32 as f64) / (u32::MAX as f64);
            let val = min + unit * (max - min);
            data.push(val);
        }
        Self {
            shape,
            strides,
            data,
        }
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
        // Parallel multi-core IKJ matrix multiplication for medium/large matrices
        let work = m.saturating_mul(k1).saturating_mul(n);
        let workers = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1);

        if work >= 65_536 && m >= 8 && workers > 1 && n > 0 {
            let chunk_rows = m.div_ceil(workers);
            let a_data = &self.data;
            let b_data = &other.data;
            std::thread::scope(|s| {
                for (chunk_idx, out_chunk) in out_data.chunks_mut(chunk_rows * n).enumerate() {
                    let row_start = chunk_idx * chunk_rows;
                    let rows_in_chunk = out_chunk.len() / n;
                    s.spawn(move || {
                        for local_i in 0..rows_in_chunk {
                            let a_row_offset = (row_start + local_i) * k1;
                            let c_row_offset = local_i * n;
                            for k in 0..k1 {
                                let a_val = a_data[a_row_offset + k];
                                let b_row_offset = k * n;
                                for j in 0..n {
                                    out_chunk[c_row_offset + j] += a_val * b_data[b_row_offset + j];
                                }
                            }
                        }
                    });
                }
            });
        } else {
            // Cache-friendly IKJ loop order for compact tensors
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
        }

        Self::new(vec![m, n], out_data)
    }

    pub fn transpose(&self) -> Result<Self, String> {
        if self.shape.len() != 2 {
            return Err(format!(
                "transpose requires 2D matrix, got shape {:?}",
                self.shape
            ));
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

    pub fn min_val(&self) -> f64 {
        self.data.iter().copied().fold(f64::INFINITY, f64::min)
    }

    pub fn max_val(&self) -> f64 {
        self.data.iter().copied().fold(f64::NEG_INFINITY, f64::max)
    }

    pub fn variance(&self) -> f64 {
        if self.data.is_empty() {
            return 0.0;
        }
        let m = self.mean();
        self.data.iter().map(|&x| (x - m) * (x - m)).sum::<f64>() / (self.data.len() as f64)
    }

    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }

    pub fn argmax(&self) -> usize {
        self.data
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    pub fn argmin(&self) -> usize {
        self.data
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    pub fn sum_axis(&self, axis: usize) -> Result<Self, String> {
        if self.shape.len() != 2 {
            if axis == 0 && self.shape.len() == 1 {
                return Self::new(vec![1], vec![self.sum()]);
            }
            return Err(format!(
                "sum_axis requires 1D or 2D tensor, got shape {:?}",
                self.shape
            ));
        }
        let (rows, cols) = (self.shape[0], self.shape[1]);
        match axis {
            0 => {
                let mut out = vec![0.0; cols];
                for r in 0..rows {
                    for (c, slot) in out.iter_mut().enumerate() {
                        *slot += self.data[r * cols + c];
                    }
                }
                Self::new(vec![1, cols], out)
            }
            1 => {
                let out: Vec<f64> = (0..rows)
                    .map(|r| self.data[r * cols..(r + 1) * cols].iter().sum())
                    .collect();
                Self::new(vec![rows, 1], out)
            }
            _ => Err(format!("Invalid axis {} for 2D tensor", axis)),
        }
    }

    pub fn mean_axis(&self, axis: usize) -> Result<Self, String> {
        let summed = self.sum_axis(axis)?;
        let denom = *self
            .shape
            .get(axis)
            .ok_or_else(|| format!("Invalid axis {} for shape {:?}", axis, self.shape))?;
        let denom_f = (denom.max(1)) as f64;
        Ok(summed.map_unary(|x| x / denom_f))
    }

    pub fn argmax_axis(&self, axis: usize) -> Result<Self, String> {
        if self.shape.len() != 2 {
            if axis == 0 && self.shape.len() == 1 {
                return Self::new(vec![1], vec![self.argmax() as f64]);
            }
            return Err(format!(
                "argmax_axis requires 1D or 2D tensor, got shape {:?}",
                self.shape
            ));
        }
        let (rows, cols) = (self.shape[0], self.shape[1]);
        match axis {
            0 => {
                let mut out = Vec::with_capacity(cols);
                for c in 0..cols {
                    let mut best_r = 0;
                    let mut best_v = f64::NEG_INFINITY;
                    for r in 0..rows {
                        let v = self.data[r * cols + c];
                        if v > best_v {
                            best_v = v;
                            best_r = r;
                        }
                    }
                    out.push(best_r as f64);
                }
                Self::new(vec![cols], out)
            }
            1 => {
                let mut out = Vec::with_capacity(rows);
                for r in 0..rows {
                    let mut best_c = 0;
                    let mut best_v = f64::NEG_INFINITY;
                    for c in 0..cols {
                        let v = self.data[r * cols + c];
                        if v > best_v {
                            best_v = v;
                            best_c = c;
                        }
                    }
                    out.push(best_c as f64);
                }
                Self::new(vec![rows], out)
            }
            _ => Err(format!("Invalid axis {} for 2D tensor", axis)),
        }
    }

    pub fn map_unary<F>(&self, f: F) -> Self
    where
        F: Fn(f64) -> f64,
    {
        let data: Vec<f64> = self.data.iter().copied().map(f).collect();
        Self {
            shape: self.shape.clone(),
            strides: self.strides.clone(),
            data,
        }
    }

    pub fn relu(&self) -> Self {
        self.map_unary(|x| if x > 0.0 { x } else { 0.0 })
    }

    pub fn sigmoid(&self) -> Self {
        self.map_unary(|x| 1.0 / (1.0 + (-x).exp()))
    }

    pub fn exp(&self) -> Self {
        self.map_unary(f64::exp)
    }

    pub fn log(&self) -> Self {
        self.map_unary(|x| x.max(1e-15).ln())
    }

    pub fn sqrt(&self) -> Self {
        self.map_unary(|x| x.max(0.0).sqrt())
    }

    pub fn abs(&self) -> Self {
        self.map_unary(f64::abs)
    }

    pub fn tanh(&self) -> Self {
        self.map_unary(f64::tanh)
    }

    pub fn gelu(&self) -> Self {
        let sqrt_2_over_pi = (2.0 / std::f64::consts::PI).sqrt();
        self.map_unary(|x| {
            let inner = sqrt_2_over_pi * (x + 0.044715 * x * x * x);
            0.5 * x * (1.0 + inner.tanh())
        })
    }

    pub fn silu(&self) -> Self {
        self.map_unary(|x| x / (1.0 + (-x).exp()))
    }

    pub fn pow_scalar(&self, p: f64) -> Self {
        self.map_unary(|x| x.powf(p))
    }

    pub fn arange(start: f64, stop: f64, step: f64) -> Result<Self, String> {
        if step == 0.0 {
            return Err("Tensor.arange step cannot be zero".into());
        }
        let mut data = Vec::new();
        let mut cur = start;
        if step > 0.0 {
            while cur < stop - 1e-12 {
                data.push(cur);
                cur += step;
            }
        } else {
            while cur > stop + 1e-12 {
                data.push(cur);
                cur += step;
            }
        }
        let n = data.len();
        Self::new(vec![n], data)
    }

    pub fn linspace(start: f64, stop: f64, num: usize) -> Result<Self, String> {
        if num == 0 {
            return Self::new(vec![0], Vec::new());
        }
        if num == 1 {
            return Self::new(vec![1], vec![start]);
        }
        let step = (stop - start) / ((num - 1) as f64);
        let data: Vec<f64> = (0..num).map(|i| start + (i as f64) * step).collect();
        Self::new(vec![num], data)
    }

    pub fn slice_rows(&self, start: usize, end: usize) -> Result<Self, String> {
        if self.shape.is_empty() {
            return Err("Cannot slice empty tensor".into());
        }
        let rows = self.shape[0];
        let end_clamped = end.min(rows);
        if start > end_clamped {
            return Err(format!(
                "Invalid row slice {}..{} for tensor of {} rows",
                start, end, rows
            ));
        }
        let row_stride: usize = self.shape[1..].iter().product::<usize>().max(1);
        let mut new_shape = self.shape.clone();
        new_shape[0] = end_clamped - start;
        let sliced = self.data[start * row_stride..end_clamped * row_stride].to_vec();
        Self::new(new_shape, sliced)
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
        let data: Vec<f64> = self
            .data
            .iter()
            .map(|&x| if x > threshold { 1.0 } else { 0.0 })
            .collect();
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

        for (&aligned_dim, &self_dim) in aligned_target_shape
            .iter()
            .zip(self.shape.iter())
            .take(ndim)
        {
            if aligned_dim != 1 && aligned_dim != self_dim {
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
            for (coord, &stride) in coords.iter_mut().zip(self.strides.iter()).take(ndim) {
                *coord = rem / stride;
                rem %= stride;
            }

            let mut target_idx = 0;
            for d in 0..ndim {
                let c = if aligned_target_shape[d] == 1 {
                    0
                } else {
                    coords[d]
                };
                target_idx += c * aligned_strides[d];
            }

            out_data[target_idx] += val;
        }

        Self::new(target_shape.to_vec(), out_data)
    }

    pub fn clip(&self, min_val: f64, max_val: f64) -> Self {
        let data: Vec<f64> = self
            .data
            .iter()
            .map(|&x| x.clamp(min_val, max_val))
            .collect();
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

static TENSORS: std::sync::OnceLock<Mutex<HashMap<u64, Arc<Mutex<TensorData>>>>> =
    std::sync::OnceLock::new();

fn get_tensors() -> &'static Mutex<HashMap<u64, Arc<Mutex<TensorData>>>> {
    TENSORS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn wrap_tensor_handle(data: TensorData) -> Value {
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

pub fn get_tensor_data(val: &Value) -> Result<TensorData, String> {
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
        other => Err(format!(
            "Expected Tensor or numeric array, got '{}'",
            other.type_name()
        )),
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
        other => Err(format!(
            "Expected shape array or integer, got '{}'",
            other.type_name()
        )),
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
        Value::Native("Tensor.add".into(), |args| binary_op(args, |x, y| x + y)),
    );
    tensor_module.insert(
        "sub".to_string(),
        Value::Native("Tensor.sub".into(), |args| binary_op(args, |x, y| x - y)),
    );
    tensor_module.insert(
        "mul".to_string(),
        Value::Native("Tensor.mul".into(), |args| binary_op(args, |x, y| x * y)),
    );
    tensor_module.insert(
        "div".to_string(),
        Value::Native("Tensor.div".into(), |args| binary_op(args, |x, y| x / y)),
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
                return Err(
                    "Tensor.reduce_sum_to(t, target_shape) requires tensor and target_shape".into(),
                );
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

    // 14. Extended NumPy / Deep-Learning Math Ops
    tensor_module.insert(
        "exp".to_string(),
        Value::Native("Tensor.exp".into(), |a| {
            unary_op(a, "Tensor.exp", TensorData::exp)
        }),
    );
    tensor_module.insert(
        "log".to_string(),
        Value::Native("Tensor.log".into(), |a| {
            unary_op(a, "Tensor.log", TensorData::log)
        }),
    );
    tensor_module.insert(
        "sqrt".to_string(),
        Value::Native("Tensor.sqrt".into(), |a| {
            unary_op(a, "Tensor.sqrt", TensorData::sqrt)
        }),
    );
    tensor_module.insert(
        "abs".to_string(),
        Value::Native("Tensor.abs".into(), |a| {
            unary_op(a, "Tensor.abs", TensorData::abs)
        }),
    );
    tensor_module.insert(
        "tanh".to_string(),
        Value::Native("Tensor.tanh".into(), |a| {
            unary_op(a, "Tensor.tanh", TensorData::tanh)
        }),
    );
    tensor_module.insert(
        "gelu".to_string(),
        Value::Native("Tensor.gelu".into(), |a| {
            unary_op(a, "Tensor.gelu", TensorData::gelu)
        }),
    );
    tensor_module.insert(
        "silu".to_string(),
        Value::Native("Tensor.silu".into(), |a| {
            unary_op(a, "Tensor.silu", TensorData::silu)
        }),
    );
    tensor_module.insert(
        "pow".to_string(),
        Value::Native("Tensor.pow".into(), |args| {
            if args.len() < 2 {
                return Err("Tensor.pow(t, exponent) requires 2 arguments".into());
            }
            let t = get_tensor_data(&args[0])?;
            let p = arg_f64(&args[1], "exponent")?;
            Ok(wrap_tensor_handle(t.pow_scalar(p)))
        }),
    );

    // 15. Statistical & Index Reductions
    tensor_module.insert(
        "min".to_string(),
        Value::Native("Tensor.min".into(), |a| {
            scalar_op(a, "Tensor.min", TensorData::min_val)
        }),
    );
    tensor_module.insert(
        "max".to_string(),
        Value::Native("Tensor.max".into(), |a| {
            scalar_op(a, "Tensor.max", TensorData::max_val)
        }),
    );
    tensor_module.insert(
        "var".to_string(),
        Value::Native("Tensor.var".into(), |a| {
            scalar_op(a, "Tensor.var", TensorData::variance)
        }),
    );
    tensor_module.insert(
        "std".to_string(),
        Value::Native("Tensor.std".into(), |a| {
            scalar_op(a, "Tensor.std", TensorData::std_dev)
        }),
    );
    tensor_module.insert(
        "argmax".to_string(),
        Value::Native("Tensor.argmax".into(), |args| {
            if args.is_empty() {
                return Err("Tensor.argmax(t, [axis]) requires tensor".into());
            }
            let t = get_tensor_data(&args[0])?;
            if args.len() > 1 && args[1] != Value::Nil {
                let axis = arg_usize(&args[1], "axis")?;
                return Ok(wrap_tensor_handle(t.argmax_axis(axis)?));
            }
            Ok(Value::Int(t.argmax() as i64))
        }),
    );
    tensor_module.insert(
        "argmin".to_string(),
        Value::Native("Tensor.argmin".into(), |args| {
            if args.is_empty() {
                return Err("Tensor.argmin(t) requires tensor".into());
            }
            let t = get_tensor_data(&args[0])?;
            Ok(Value::Int(t.argmin() as i64))
        }),
    );
    tensor_module.insert(
        "sum_axis".to_string(),
        Value::Native("Tensor.sum_axis".into(), |a| {
            axis_op(a, "Tensor.sum_axis", TensorData::sum_axis)
        }),
    );
    tensor_module.insert(
        "mean_axis".to_string(),
        Value::Native("Tensor.mean_axis".into(), |a| {
            axis_op(a, "Tensor.mean_axis", TensorData::mean_axis)
        }),
    );

    // 16. Creation & Slicing Utilities (NumPy parity)
    tensor_module.insert(
        "arange".to_string(),
        Value::Native("Tensor.arange".into(), |args| {
            if args.is_empty() {
                return Err(
                    "Tensor.arange(stop) or Tensor.arange(start, stop, [step]) required".into(),
                );
            }
            let (start, stop, step) = if args.len() == 1 {
                (0.0, arg_f64(&args[0], "stop")?, 1.0)
            } else {
                let st = arg_f64(&args[0], "start")?;
                let sp = arg_f64(&args[1], "stop")?;
                let step = if args.len() > 2 && args[2] != Value::Nil {
                    arg_f64(&args[2], "step")?
                } else if sp >= st {
                    1.0
                } else {
                    -1.0
                };
                (st, sp, step)
            };
            Ok(wrap_tensor_handle(TensorData::arange(start, stop, step)?))
        }),
    );
    tensor_module.insert(
        "linspace".to_string(),
        Value::Native("Tensor.linspace".into(), |args| {
            if args.len() < 3 {
                return Err("Tensor.linspace(start, stop, num) requires 3 arguments".into());
            }
            let start = arg_f64(&args[0], "start")?;
            let stop = arg_f64(&args[1], "stop")?;
            let num = arg_usize(&args[2], "num")?;
            Ok(wrap_tensor_handle(TensorData::linspace(start, stop, num)?))
        }),
    );
    tensor_module.insert(
        "slice_rows".to_string(),
        Value::Native("Tensor.slice_rows".into(), |args| {
            if args.len() < 3 {
                return Err("Tensor.slice_rows(t, start, end) requires 3 arguments".into());
            }
            let t = get_tensor_data(&args[0])?;
            let start = arg_usize(&args[1], "start")?;
            let end = arg_usize(&args[2], "end")?;
            Ok(wrap_tensor_handle(t.slice_rows(start, end)?))
        }),
    );

    globals
        .entry("__native_tensor".to_string())
        .or_insert_with(|| Value::map(tensor_module.clone()));
    globals
        .entry("__native_matrix".to_string())
        .or_insert_with(|| Value::map(tensor_module.clone()));
    globals
        .entry("Matrix".to_string())
        .or_insert_with(|| Value::map(tensor_module.clone()));
    globals
        .entry("matrix".to_string())
        .or_insert_with(|| Value::map(tensor_module));
}

fn arg_f64(val: &Value, name: &str) -> Result<f64, String> {
    match val {
        Value::Float(f) => Ok(*f),
        Value::Int(i) => Ok(*i as f64),
        _ => Err(format!("{} must be a number", name)),
    }
}

fn arg_usize(val: &Value, name: &str) -> Result<usize, String> {
    match val {
        Value::Int(i) if *i >= 0 => Ok(*i as usize),
        _ => Err(format!("{} must be a non-negative integer", name)),
    }
}

fn unary_op(
    args: &[Value],
    fn_name: &str,
    op: fn(&TensorData) -> TensorData,
) -> Result<Value, String> {
    if args.is_empty() {
        return Err(format!("{}(t) requires tensor", fn_name));
    }
    let t = get_tensor_data(&args[0])?;
    Ok(wrap_tensor_handle(op(&t)))
}

fn scalar_op(args: &[Value], fn_name: &str, op: fn(&TensorData) -> f64) -> Result<Value, String> {
    if args.is_empty() {
        return Err(format!("{}(t) requires tensor", fn_name));
    }
    let t = get_tensor_data(&args[0])?;
    Ok(Value::Float(op(&t)))
}

fn axis_op(
    args: &[Value],
    fn_name: &str,
    op: fn(&TensorData, usize) -> Result<TensorData, String>,
) -> Result<Value, String> {
    if args.len() < 2 {
        return Err(format!("{}(t, axis) requires 2 arguments", fn_name));
    }
    let t = get_tensor_data(&args[0])?;
    let axis = arg_usize(&args[1], "axis")?;
    Ok(wrap_tensor_handle(op(&t, axis)?))
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
        let out: Vec<f64> = t1
            .data
            .iter()
            .zip(t2.data.iter())
            .map(|(&x, &y)| op(x, y))
            .collect();
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
