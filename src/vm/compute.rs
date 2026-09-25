// ==============================================================================
// AetherCompute & AetherSIMD — Software GPU & Multi-Core Work-Group Engine
// High-Throughput Parallel Compute Shader Kernels across CPU Workgroups
// Pure Rust Standard Library (Zero Third-Party Crates)
// ==============================================================================

use super::value::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Instant;

// ==============================================================================
// 1. Compute Buffer (1D, 2D, 3D Contiguous Memory Layout)
// ==============================================================================

#[derive(Clone, Debug)]
pub struct ComputeBuffer {
    pub dims: [usize; 3], // [X, Y, Z]
    pub data: Vec<f64>,
}

impl ComputeBuffer {
    pub fn new(dims: [usize; 3], data: Vec<f64>) -> Result<Self, String> {
        let expected_len = dims[0] * dims[1] * dims[2];
        if data.len() != expected_len {
            return Err(format!(
                "Buffer size mismatch: expected {} elements for dims {:?}, got {}",
                expected_len, dims, data.len()
            ));
        }
        Ok(Self { dims, data })
    }

    pub fn zeros(dims: [usize; 3]) -> Self {
        let len = dims[0] * dims[1] * dims[2];
        Self {
            dims,
            data: vec![0.0; len],
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline(always)]
    pub fn linear_index(&self, x: usize, y: usize, z: usize) -> usize {
        z * (self.dims[0] * self.dims[1]) + y * self.dims[0] + x
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> Option<f64> {
        if x < self.dims[0] && y < self.dims[1] && z < self.dims[2] {
            Some(self.data[self.linear_index(x, y, z)])
        } else {
            None
        }
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, val: f64) -> Result<(), String> {
        if x < self.dims[0] && y < self.dims[1] && z < self.dims[2] {
            let idx = self.linear_index(x, y, z);
            self.data[idx] = val;
            Ok(())
        } else {
            Err(format!(
                "Index out of bounds: ({}, {}, {}) for dims {:?}",
                x, y, z, self.dims
            ))
        }
    }
}

// ==============================================================================
// 2. Global Buffer Registry
// ==============================================================================

static COMPUTE_BUFFERS: OnceLock<Mutex<HashMap<u64, Arc<Mutex<ComputeBuffer>>>>> = OnceLock::new();
static NEXT_BUFFER_ID: AtomicU64 = AtomicU64::new(1);

fn get_buffers() -> &'static Mutex<HashMap<u64, Arc<Mutex<ComputeBuffer>>>> {
    COMPUTE_BUFFERS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn register_buffer(buf: ComputeBuffer) -> u64 {
    let id = NEXT_BUFFER_ID.fetch_add(1, Ordering::SeqCst);
    get_buffers().lock().unwrap().insert(id, Arc::new(Mutex::new(buf)));
    id
}

pub fn get_buffer(id: u64) -> Result<Arc<Mutex<ComputeBuffer>>, String> {
    get_buffers()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("ComputeBuffer ID {} not found", id))
}

// ==============================================================================
// 3. Multi-Core WorkGroup Scheduler & Kernel Dispatch
// ==============================================================================

pub struct WorkGroupScheduler;

impl WorkGroupScheduler {
    pub fn worker_count() -> usize {
        thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .max(1)
    }

    /// Dispatches a kernel across a 3D grid with work-groups mapped to CPU worker threads
    pub fn dispatch(
        kernel: &str,
        inputs: &[Arc<Mutex<ComputeBuffer>>],
        output: &Arc<Mutex<ComputeBuffer>>,
        global_size: [usize; 3],
        local_size: [usize; 3],
        push_constants: &[f64],
    ) -> Result<(), String> {
        let gx = global_size[0].max(1);
        let gy = global_size[1].max(1);
        let gz = global_size[2].max(1);

        let lx = local_size[0].max(1);
        let ly = local_size[1].max(1);
        let lz = local_size[2].max(1);

        // Number of workgroups along each dimension
        let num_groups_x = (gx + lx - 1) / lx;
        let num_groups_y = (gy + ly - 1) / ly;
        let num_groups_z = (gz + lz - 1) / lz;
        let total_workgroups = num_groups_x * num_groups_y * num_groups_z;

        if total_workgroups == 0 {
            return Ok(());
        }

        // Snapshot input buffer data for concurrent reads
        let in_snapshots: Vec<Vec<f64>> = inputs
            .iter()
            .map(|b| b.lock().unwrap().data.clone())
            .collect();
        let in_dims: Vec<[usize; 3]> = inputs
            .iter()
            .map(|b| b.lock().unwrap().dims)
            .collect();

        // Lock output buffer and get raw slice access
        let mut out_guard = output.lock().unwrap();
        let _out_dims = out_guard.dims;
        let out_data = &mut out_guard.data;

        let num_workers = Self::worker_count().min(total_workgroups);

        // Dispatch based on kernel name
        match kernel {
            "vec_add" => {
                if in_snapshots.len() < 2 {
                    return Err("vec_add requires 2 input buffers".into());
                }
                let a = &in_snapshots[0];
                let b = &in_snapshots[1];
                let n = gx.min(out_data.len()).min(a.len()).min(b.len());

                Self::parallel_chunked(n, num_workers, |start, end, chunk| {
                    for (local_idx, i) in (start..end).enumerate() {
                        chunk[local_idx] = a[i] + b[i];
                    }
                }, out_data);
            }

            "vec_sub" => {
                if in_snapshots.len() < 2 {
                    return Err("vec_sub requires 2 input buffers".into());
                }
                let a = &in_snapshots[0];
                let b = &in_snapshots[1];
                let n = gx.min(out_data.len()).min(a.len()).min(b.len());

                Self::parallel_chunked(n, num_workers, |start, end, chunk| {
                    for (local_idx, i) in (start..end).enumerate() {
                        chunk[local_idx] = a[i] - b[i];
                    }
                }, out_data);
            }

            "vec_mul" => {
                if in_snapshots.len() < 2 {
                    return Err("vec_mul requires 2 input buffers".into());
                }
                let a = &in_snapshots[0];
                let b = &in_snapshots[1];
                let n = gx.min(out_data.len()).min(a.len()).min(b.len());

                Self::parallel_chunked(n, num_workers, |start, end, chunk| {
                    for (local_idx, i) in (start..end).enumerate() {
                        chunk[local_idx] = a[i] * b[i];
                    }
                }, out_data);
            }

            "vec_div" => {
                if in_snapshots.len() < 2 {
                    return Err("vec_div requires 2 input buffers".into());
                }
                let a = &in_snapshots[0];
                let b = &in_snapshots[1];
                let n = gx.min(out_data.len()).min(a.len()).min(b.len());

                Self::parallel_chunked(n, num_workers, |start, end, chunk| {
                    for (local_idx, i) in (start..end).enumerate() {
                        chunk[local_idx] = if b[i] == 0.0 { 0.0 } else { a[i] / b[i] };
                    }
                }, out_data);
            }

            "fma" => {
                // Fused Multiply-Add: out[i] = a[i] * b[i] + c[i]
                if in_snapshots.len() < 3 {
                    return Err("fma requires 3 input buffers (a, b, c)".into());
                }
                let a = &in_snapshots[0];
                let b = &in_snapshots[1];
                let c = &in_snapshots[2];
                let n = gx.min(out_data.len()).min(a.len()).min(b.len()).min(c.len());

                Self::parallel_chunked(n, num_workers, |start, end, chunk| {
                    for (local_idx, i) in (start..end).enumerate() {
                        chunk[local_idx] = a[i].mul_add(b[i], c[i]);
                    }
                }, out_data);
            }

            "scale_bias" => {
                if in_snapshots.is_empty() {
                    return Err("scale_bias requires 1 input buffer".into());
                }
                let scale = push_constants.first().copied().unwrap_or(1.0);
                let bias = push_constants.get(1).copied().unwrap_or(0.0);
                let a = &in_snapshots[0];
                let n = gx.min(out_data.len()).min(a.len());

                Self::parallel_chunked(n, num_workers, |start, end, chunk| {
                    for (local_idx, i) in (start..end).enumerate() {
                        chunk[local_idx] = a[i] * scale + bias;
                    }
                }, out_data);
            }

            "clamp" => {
                if in_snapshots.is_empty() {
                    return Err("clamp requires 1 input buffer".into());
                }
                let min_val = push_constants.first().copied().unwrap_or(0.0);
                let max_val = push_constants.get(1).copied().unwrap_or(1.0);
                let a = &in_snapshots[0];
                let n = gx.min(out_data.len()).min(a.len());

                Self::parallel_chunked(n, num_workers, |start, end, chunk| {
                    for (local_idx, i) in (start..end).enumerate() {
                        chunk[local_idx] = a[i].clamp(min_val, max_val);
                    }
                }, out_data);
            }

            "relu" => {
                if in_snapshots.is_empty() {
                    return Err("relu requires 1 input buffer".into());
                }
                let a = &in_snapshots[0];
                let n = gx.min(out_data.len()).min(a.len());

                Self::parallel_chunked(n, num_workers, |start, end, chunk| {
                    for (local_idx, i) in (start..end).enumerate() {
                        chunk[local_idx] = if a[i] > 0.0 { a[i] } else { 0.0 };
                    }
                }, out_data);
            }

            "sigmoid" => {
                if in_snapshots.is_empty() {
                    return Err("sigmoid requires 1 input buffer".into());
                }
                let a = &in_snapshots[0];
                let n = gx.min(out_data.len()).min(a.len());

                Self::parallel_chunked(n, num_workers, |start, end, chunk| {
                    for (local_idx, i) in (start..end).enumerate() {
                        chunk[local_idx] = 1.0 / (1.0 + (-a[i]).exp());
                    }
                }, out_data);
            }

            "matmul" => {
                // A: [M, K], B: [K, N], Out: [M, N]
                if in_snapshots.len() < 2 {
                    return Err("matmul requires 2 input buffers (A, B)".into());
                }
                let m = in_dims[0][1].max(1);
                let k = in_dims[0][0].max(1);
                let n = in_dims[1][0].max(1);

                if in_dims[1][1] != k {
                    return Err(format!(
                        "matmul dimension mismatch: A is [{}x{}], B is [{}x{}]",
                        m, k, in_dims[1][1], n
                    ));
                }

                let a = &in_snapshots[0];
                let b = &in_snapshots[1];

                // Parallelize over rows M across CPU workers
                thread::scope(|s| {
                    let chunk_size = (m + num_workers - 1) / num_workers;
                    let out_chunks = out_data.chunks_mut(chunk_size * n);

                    for (worker_idx, chunk) in out_chunks.enumerate() {
                        s.spawn(move || {
                            let row_start = worker_idx * chunk_size;
                            let row_end = (row_start + chunk.len() / n).min(m);

                            for r in row_start..row_end {
                                let local_r = r - row_start;
                                for c in 0..n {
                                    let mut sum = 0.0;
                                    for p in 0..k {
                                        sum += a[r * k + p] * b[p * n + c];
                                    }
                                    chunk[local_r * n + c] = sum;
                                }
                            }
                        });
                    }
                });
            }

            "conv2d" => {
                // Input 0: Image [W, H, 1], Input 1: Kernel [KW, KH, 1]
                if in_snapshots.len() < 2 {
                    return Err("conv2d requires image buffer and kernel buffer".into());
                }
                let img_w = in_dims[0][0];
                let img_h = in_dims[0][1];
                let kern_w = in_dims[1][0];
                let kern_h = in_dims[1][1];

                if kern_w % 2 == 0 || kern_h % 2 == 0 {
                    return Err("conv2d kernel dimensions must be odd (e.g. 3x3, 5x5)".into());
                }

                let pad_x = (kern_w / 2) as isize;
                let pad_y = (kern_h / 2) as isize;
                let img = &in_snapshots[0];
                let kernel_weights = &in_snapshots[1];

                // Parallelize rows of the 2D image across CPU workers
                thread::scope(|s| {
                    let chunk_size = (img_h + num_workers - 1) / num_workers;
                    let out_chunks = out_data.chunks_mut(chunk_size * img_w);

                    for (worker_idx, chunk) in out_chunks.enumerate() {
                        s.spawn(move || {
                            let y_start = worker_idx * chunk_size;
                            let y_end = (y_start + chunk.len() / img_w).min(img_h);

                            for y in y_start..y_end {
                                let local_y = y - y_start;
                                for x in 0..img_w {
                                    let mut acc = 0.0;
                                    for ky in 0..kern_h {
                                        for kx in 0..kern_w {
                                            let src_x = x as isize + kx as isize - pad_x;
                                            let src_y = y as isize + ky as isize - pad_y;

                                            if src_x >= 0
                                                && src_x < img_w as isize
                                                && src_y >= 0
                                                && src_y < img_h as isize
                                            {
                                                let pixel = img[(src_y as usize) * img_w + (src_x as usize)];
                                                let weight = kernel_weights[ky * kern_w + kx];
                                                acc += pixel * weight;
                                            }
                                        }
                                    }
                                    chunk[local_y * img_w + x] = acc;
                                }
                            }
                        });
                    }
                });
            }

            "parallel_sum" => {
                if in_snapshots.is_empty() {
                    return Err("parallel_sum requires 1 input buffer".into());
                }
                let a = &in_snapshots[0];
                let n = a.len();
                let chunk_size = (n + num_workers - 1) / num_workers;

                let partial_sums: Vec<f64> = thread::scope(|s| {
                    let mut handles = Vec::new();
                    for chunk in a.chunks(chunk_size) {
                        handles.push(s.spawn(move || chunk.iter().sum::<f64>()));
                    }
                    handles.into_iter().map(|h| h.join().unwrap()).collect()
                });

                let total: f64 = partial_sums.iter().sum();
                if !out_data.is_empty() {
                    out_data[0] = total;
                }
            }

            "parallel_max" => {
                if in_snapshots.is_empty() {
                    return Err("parallel_max requires 1 input buffer".into());
                }
                let a = &in_snapshots[0];
                let n = a.len();
                let chunk_size = (n + num_workers - 1) / num_workers;

                let partial_maxs: Vec<f64> = thread::scope(|s| {
                    let mut handles = Vec::new();
                    for chunk in a.chunks(chunk_size) {
                        handles.push(s.spawn(move || {
                            chunk.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
                        }));
                    }
                    handles.into_iter().map(|h| h.join().unwrap()).collect()
                });

                let maximum = partial_maxs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                if !out_data.is_empty() {
                    out_data[0] = maximum;
                }
            }

            "dot_product" => {
                if in_snapshots.len() < 2 {
                    return Err("dot_product requires 2 input buffers".into());
                }
                let a = &in_snapshots[0];
                let b = &in_snapshots[1];
                let n = a.len().min(b.len());
                let chunk_size = (n + num_workers - 1) / num_workers;

                let partial_dots: Vec<f64> = thread::scope(|s| {
                    let mut handles = Vec::new();
                    let a_chunks = a[..n].chunks(chunk_size);
                    let b_chunks = b[..n].chunks(chunk_size);

                    for (ac, bc) in a_chunks.zip(b_chunks) {
                        handles.push(s.spawn(move || {
                            ac.iter().zip(bc).map(|(&x, &y)| x * y).sum::<f64>()
                        }));
                    }
                    handles.into_iter().map(|h| h.join().unwrap()).collect()
                });

                let total: f64 = partial_dots.iter().sum();
                if !out_data.is_empty() {
                    out_data[0] = total;
                }
            }

            custom => {
                return Err(format!("Unknown compute shader kernel '{}'", custom));
            }
        }

        Ok(())
    }

    /// Parallel chunk helper partitioning contiguous slices across worker threads
    fn parallel_chunked<F>(
        total_len: usize,
        num_workers: usize,
        kernel_fn: F,
        out_slice: &mut [f64],
    ) where
        F: Fn(usize, usize, &mut [f64]) + Send + Sync,
    {
        if total_len == 0 {
            return;
        }
        let chunk_size = (total_len + num_workers - 1) / num_workers;

        thread::scope(|s| {
            for (worker_idx, chunk) in out_slice[..total_len].chunks_mut(chunk_size).enumerate() {
                let k_fn = &kernel_fn;
                let global_start = worker_idx * chunk_size;
                s.spawn(move || {
                    let len = chunk.len();
                    k_fn(global_start, global_start + len, chunk);
                });
            }
        });
    }
}

// ==============================================================================
// 4. Native VM Module Bindings
// ==============================================================================

pub fn register_compute_module(globals: &mut HashMap<String, Value>) {
    let mut mod_map = HashMap::new();

    // 1. Compute.hardware_info() -> map
    mod_map.insert(
        "hardware_info".to_string(),
        Value::Native("Compute.hardware_info".into(), |_| {
            let mut info = HashMap::new();
            info.insert("device_type".to_string(), Value::string("Aether Software GPU / SIMD Work-Group Engine"));
            info.insert("cpu_cores".to_string(), Value::Int(WorkGroupScheduler::worker_count() as i64));
            info.insert("max_workgroup_size".to_string(), Value::Int(1024));
            info.insert("subgroup_simd_width".to_string(), Value::Int(64));
            info.insert("memory_unified".to_string(), Value::Bool(true));
            Ok(Value::map(info))
        }),
    );

    // 2. Compute.create_buffer(dims, initial_data?) -> buffer_id
    mod_map.insert(
        "create_buffer".to_string(),
        Value::Native("Compute.create_buffer".into(), |args| {
            if args.is_empty() {
                return Err("Compute.create_buffer(dims, initial_data?) requires dims array".into());
            }

            let dims = parse_dims(&args[0])?;
            let initial = if args.len() > 1 && args[1] != Value::Nil {
                match &args[1] {
                    Value::Array(arr) => arr
                        .lock()
                        .iter()
                        .map(|v| match v {
                            Value::Float(f) => *f,
                            Value::Int(i) => *i as f64,
                            _ => 0.0,
                        })
                        .collect(),
                    _ => return Err("initial_data must be an array of numbers".into()),
                }
            } else {
                vec![0.0; dims[0] * dims[1] * dims[2]]
            };

            let buf = ComputeBuffer::new(dims, initial)?;
            let id = register_buffer(buf);
            Ok(Value::Int(id as i64))
        }),
    );

    // 3. Compute.buffer_to_array(buffer_id) -> array
    mod_map.insert(
        "buffer_to_array".to_string(),
        Value::Native("Compute.buffer_to_array".into(), |args| {
            if args.is_empty() {
                return Err("Compute.buffer_to_array(buffer_id) requires buffer_id".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("buffer_id must be integer".into()),
            };

            let buf_arc = get_buffer(id)?;
            let buf = buf_arc.lock().unwrap();
            let vals: Vec<Value> = buf.data.iter().map(|&f| Value::Float(f)).collect();
            Ok(Value::array(vals))
        }),
    );

    // 4. Compute.buffer_shape(buffer_id) -> [x, y, z]
    mod_map.insert(
        "buffer_shape".to_string(),
        Value::Native("Compute.buffer_shape".into(), |args| {
            if args.is_empty() {
                return Err("Compute.buffer_shape(buffer_id) requires buffer_id".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("buffer_id must be integer".into()),
            };

            let buf_arc = get_buffer(id)?;
            let buf = buf_arc.lock().unwrap();
            Ok(Value::array(vec![
                Value::Int(buf.dims[0] as i64),
                Value::Int(buf.dims[1] as i64),
                Value::Int(buf.dims[2] as i64),
            ]))
        }),
    );

    // 5. Compute.dispatch(kernel, inputs, output, global_size, local_size, push_constants?)
    mod_map.insert(
        "dispatch".to_string(),
        Value::Native("Compute.dispatch".into(), compute_dispatch_internal),
    );

    // 6. Compute.benchmark(kernel, inputs, output, global_size, local_size, iterations?) -> map
    mod_map.insert(
        "benchmark".to_string(),
        Value::Native("Compute.benchmark".into(), |args| {
            if args.len() < 5 {
                return Err("Compute.benchmark(kernel, inputs, output, global_size, local_size, iterations?) requires at least 5 arguments".into());
            }

            let iterations = if args.len() > 6 {
                match args[6] {
                    Value::Int(i) if i > 0 => i as usize,
                    _ => 10,
                }
            } else {
                10
            };

            let start = Instant::now();
            for _ in 0..iterations {
                let _ = compute_dispatch_internal(args)?;
            }
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            let avg_ms = elapsed_ms / iterations as f64;

            let mut stats = HashMap::new();
            stats.insert("kernel".to_string(), args[0].clone());
            stats.insert("iterations".to_string(), Value::Int(iterations as i64));
            stats.insert("total_ms".to_string(), Value::Float(elapsed_ms));
            stats.insert("avg_ms".to_string(), Value::Float(avg_ms));
            stats.insert("cores_used".to_string(), Value::Int(WorkGroupScheduler::worker_count() as i64));
            Ok(Value::map(stats))
        }),
    );

    globals.insert("Compute".to_string(), Value::map(mod_map.clone()));
    globals.insert("__native_compute".to_string(), Value::map(mod_map));
}

fn compute_dispatch_internal(args: &[Value]) -> Result<Value, String> {
    if args.len() < 5 {
        return Err("Compute.dispatch(kernel, inputs, output, global_size, local_size, push_constants?) requires at least 5 arguments".into());
    }

    let kernel = args[0].to_string();

    let input_ids: Vec<u64> = match &args[1] {
        Value::Array(arr) => arr
            .lock()
            .iter()
            .map(|v| match v {
                Value::Int(i) => *i as u64,
                _ => 0,
            })
            .collect(),
        Value::Int(i) => vec![*i as u64],
        _ => return Err("inputs must be array of buffer IDs or single buffer ID".into()),
    };

    let output_id = match args[2] {
        Value::Int(i) => i as u64,
        _ => return Err("output must be buffer ID integer".into()),
    };

    let global_size = parse_dims(&args[3])?;
    let local_size = parse_dims(&args[4])?;

    let push_constants: Vec<f64> = if args.len() > 5 && args[5] != Value::Nil {
        match &args[5] {
            Value::Array(arr) => arr
                .lock()
                .iter()
                .map(|v| match v {
                    Value::Float(f) => *f,
                    Value::Int(i) => *i as f64,
                    _ => 0.0,
                })
                .collect(),
            Value::Float(f) => vec![*f],
            Value::Int(i) => vec![*i as f64],
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };

    let mut input_buffers = Vec::new();
    for id in input_ids {
        input_buffers.push(get_buffer(id)?);
    }

    let output_buf = get_buffer(output_id)?;

    WorkGroupScheduler::dispatch(
        &kernel,
        &input_buffers,
        &output_buf,
        global_size,
        local_size,
        &push_constants,
    )?;

    Ok(Value::Int(output_id as i64))
}

fn parse_dims(val: &Value) -> Result<[usize; 3], String> {
    match val {
        Value::Array(arr) => {
            let items = arr.lock();
            let x = items.first().and_then(|v| match v {
                Value::Int(i) => Some(*i as usize),
                _ => None,
            }).unwrap_or(1);
            let y = items.get(1).and_then(|v| match v {
                Value::Int(i) => Some(*i as usize),
                _ => None,
            }).unwrap_or(1);
            let z = items.get(2).and_then(|v| match v {
                Value::Int(i) => Some(*i as usize),
                _ => None,
            }).unwrap_or(1);
            Ok([x.max(1), y.max(1), z.max(1)])
        }
        Value::Int(i) => Ok([(*i as usize).max(1), 1, 1]),
        _ => Err("Dimensions must be an array [x, y, z] or single integer".into()),
    }
}
