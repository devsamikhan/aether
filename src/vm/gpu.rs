// ==============================================================================
// AetherGPU — Cross-Platform Hardware GPU Compute Engine (DirectX 12 / Vulkan / Metal)
// Powered by wgpu + Compiled WGSL Compute Shaders with Transparent Multi-Core CPU Fallback
// ==============================================================================

use super::tensor::{get_tensor_data, wrap_tensor_handle, TensorData};
use super::value::Value;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

#[cfg(feature = "gpu")]
use std::borrow::Cow;

// ==============================================================================
// 1. WGSL Compute Shaders (Tiled GEMM, Elementwise, Activations, Norms, Flash-Style Attention)
// ==============================================================================

#[cfg(feature = "gpu")]
const WGSL_MATMUL: &str = r#"
struct Dims {
    m: u32,
    k: u32,
    n: u32,
    _pad: u32,
};

@group(0) @binding(0) var<uniform> dims: Dims;
@group(0) @binding(1) var<storage, read> a: array<f32>;
@group(0) @binding(2) var<storage, read> b: array<f32>;
@group(0) @binding(3) var<storage, read_write> c: array<f32>;

var<workgroup> tile_a: array<f32, 256>;
var<workgroup> tile_b: array<f32, 256>;

@compute @workgroup_size(16, 16, 1)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>
) {
    let row = gid.y;
    let col = gid.x;
    let lx = lid.x;
    let ly = lid.y;
    var acc: f32 = 0.0;
    let num_tiles = (dims.k + 15u) / 16u;

    for (var t: u32 = 0u; t < num_tiles; t = t + 1u) {
        let a_col = t * 16u + lx;
        if (row < dims.m && a_col < dims.k) {
            tile_a[ly * 16u + lx] = a[row * dims.k + a_col];
        } else {
            tile_a[ly * 16u + lx] = 0.0;
        }

        let b_row = t * 16u + ly;
        if (b_row < dims.k && col < dims.n) {
            tile_b[ly * 16u + lx] = b[b_row * dims.n + col];
        } else {
            tile_b[ly * 16u + lx] = 0.0;
        }

        workgroupBarrier();

        for (var p: u32 = 0u; p < 16u; p = p + 1u) {
            acc = acc + tile_a[ly * 16u + p] * tile_b[p * 16u + lx];
        }

        workgroupBarrier();
    }

    if (row < dims.m && col < dims.n) {
        c[row * dims.n + col] = acc;
    }
}
"#;

#[cfg(feature = "gpu")]
const WGSL_BINARY: &str = r#"
struct Params {
    len: u32,
    op: u32,
    _p0: u32,
    _p1: u32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> a: array<f32>;
@group(0) @binding(2) var<storage, read> b: array<f32>;
@group(0) @binding(3) var<storage, read_write> out: array<f32>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.len) {
        return;
    }
    let x = a[idx];
    let y = b[idx];
    if (params.op == 0u) {
        out[idx] = x + y;
    } else if (params.op == 1u) {
        out[idx] = x - y;
    } else if (params.op == 2u) {
        out[idx] = x * y;
    } else {
        out[idx] = x / y;
    }
}
"#;

#[cfg(feature = "gpu")]
const WGSL_UNARY: &str = r#"
struct Params {
    len: u32,
    op: u32,
    _p0: u32,
    _p1: u32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> x_in: array<f32>;
@group(0) @binding(2) var<storage, read_write> out: array<f32>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.len) {
        return;
    }
    let x = x_in[idx];
    if (params.op == 0u) {
        out[idx] = max(x, 0.0);
    } else if (params.op == 1u) {
        out[idx] = 1.0 / (1.0 + exp(-x));
    } else if (params.op == 2u) {
        out[idx] = tanh(x);
    } else if (params.op == 3u) {
        let c = 0.7978845608 * (x + 0.044715 * x * x * x);
        out[idx] = 0.5 * x * (1.0 + tanh(c));
    } else if (params.op == 4u) {
        out[idx] = x / (1.0 + exp(-x));
    } else if (params.op == 5u) {
        out[idx] = exp(x);
    } else if (params.op == 6u) {
        out[idx] = abs(x);
    } else {
        out[idx] = sqrt(max(x, 0.0));
    }
}
"#;

#[cfg(feature = "gpu")]
const WGSL_SOFTMAX_ROWS: &str = r#"
struct Params {
    rows: u32,
    cols: u32,
    _p0: u32,
    _p1: u32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> x_in: array<f32>;
@group(0) @binding(2) var<storage, read_write> out: array<f32>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let r = gid.x;
    if (r >= params.rows) {
        return;
    }
    let base = r * params.cols;
    var max_v: f32 = -3.4028234e38;
    for (var c: u32 = 0u; c < params.cols; c = c + 1u) {
        max_v = max(max_v, x_in[base + c]);
    }
    var sum_e: f32 = 0.0;
    for (var c: u32 = 0u; c < params.cols; c = c + 1u) {
        let e = exp(x_in[base + c] - max_v);
        out[base + c] = e;
        sum_e = sum_e + e;
    }
    let inv = 1.0 / max(sum_e, 1e-12);
    for (var c: u32 = 0u; c < params.cols; c = c + 1u) {
        out[base + c] = out[base + c] * inv;
    }
}
"#;

#[cfg(feature = "gpu")]
const WGSL_RMSNORM_ROWS: &str = r#"
struct Params {
    rows: u32,
    cols: u32,
    eps_bits: u32,
    _p1: u32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> x_in: array<f32>;
@group(0) @binding(2) var<storage, read> gamma: array<f32>;
@group(0) @binding(3) var<storage, read_write> out: array<f32>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let r = gid.x;
    if (r >= params.rows) {
        return;
    }
    let eps = bitcast<f32>(params.eps_bits);
    let base = r * params.cols;
    var ss: f32 = 0.0;
    for (var c: u32 = 0u; c < params.cols; c = c + 1u) {
        let v = x_in[base + c];
        ss = ss + v * v;
    }
    let inv = inverseSqrt(ss / f32(params.cols) + eps);
    for (var c: u32 = 0u; c < params.cols; c = c + 1u) {
        out[base + c] = x_in[base + c] * inv * gamma[c];
    }
}
"#;

#[cfg(feature = "gpu")]
const WGSL_LAYERNORM_ROWS: &str = r#"
struct Params {
    rows: u32,
    cols: u32,
    eps_bits: u32,
    _p1: u32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> x_in: array<f32>;
@group(0) @binding(2) var<storage, read> gamma: array<f32>;
@group(0) @binding(3) var<storage, read> beta: array<f32>;
@group(0) @binding(4) var<storage, read_write> out: array<f32>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let r = gid.x;
    if (r >= params.rows) {
        return;
    }
    let eps = bitcast<f32>(params.eps_bits);
    let base = r * params.cols;
    var sum: f32 = 0.0;
    for (var c: u32 = 0u; c < params.cols; c = c + 1u) {
        sum = sum + x_in[base + c];
    }
    let mean = sum / f32(params.cols);
    var var_sum: f32 = 0.0;
    for (var c: u32 = 0u; c < params.cols; c = c + 1u) {
        let d = x_in[base + c] - mean;
        var_sum = var_sum + d * d;
    }
    let inv = inverseSqrt(var_sum / f32(params.cols) + eps);
    for (var c: u32 = 0u; c < params.cols; c = c + 1u) {
        out[base + c] = (x_in[base + c] - mean) * inv * gamma[c] + beta[c];
    }
}
"#;

#[cfg(feature = "gpu")]
const WGSL_ATTENTION: &str = r#"
struct Params {
    seq_q: u32,
    seq_k: u32,
    d: u32,
    d_v: u32,
    causal: u32,
    scale_bits: u32,
    _p0: u32,
    _p1: u32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> q_mat: array<f32>;
@group(0) @binding(2) var<storage, read> k_mat: array<f32>;
@group(0) @binding(3) var<storage, read> v_mat: array<f32>;
@group(0) @binding(4) var<storage, read_write> out_mat: array<f32>;

// Online FlashAttention-style streaming softmax per (q_row, v_col)
@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    let total = params.seq_q * params.d_v;
    if (idx >= total) {
        return;
    }
    let qi = idx / params.d_v;
    let vj = idx % params.d_v;
    let scale = bitcast<f32>(params.scale_bits);
    let max_k = select(params.seq_k, min(params.seq_k, qi + 1u), params.causal != 0u);

    var m_i: f32 = -3.4028234e38;
    var l_i: f32 = 0.0;
    var acc: f32 = 0.0;

    for (var ki: u32 = 0u; ki < max_k; ki = ki + 1u) {
        var dot: f32 = 0.0;
        for (var c: u32 = 0u; c < params.d; c = c + 1u) {
            dot = dot + q_mat[qi * params.d + c] * k_mat[ki * params.d + c];
        }
        let s = dot * scale;
        let m_new = max(m_i, s);
        let alpha = exp(m_i - m_new);
        let p = exp(s - m_new);
        l_i = l_i * alpha + p;
        acc = acc * alpha + p * v_mat[ki * params.d_v + vj];
        m_i = m_new;
    }

    out_mat[idx] = acc / max(l_i, 1e-12);
}
"#;

// ==============================================================================
// 2. Hardware wgpu Context & Pipeline Cache
// ==============================================================================

#[cfg(feature = "gpu")]
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform4 {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
}

#[cfg(feature = "gpu")]
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform8 {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    e: u32,
    f: u32,
    g: u32,
    h: u32,
}

#[cfg(feature = "gpu")]
struct WgpuBackend {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_name: String,
    backend_name: String,
    device_type: String,
    driver_info: String,
    max_buffer_bytes: u64,
    pipelines: Mutex<HashMap<&'static str, wgpu::ComputePipeline>>,
}

#[cfg(feature = "gpu")]
impl WgpuBackend {
    fn try_init() -> Option<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .or_else(|| {
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
            }))
        })?;

        let info = adapter.get_info();
        let limits = adapter.limits();
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("AetherGPU Device"),
                required_features: wgpu::Features::empty(),
                required_limits:
                    wgpu::Limits::downlevel_defaults().using_resolution(limits.clone()),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .ok()?;

        Some(Self {
            device,
            queue,
            adapter_name: info.name,
            backend_name: format!("{:?}", info.backend),
            device_type: format!("{:?}", info.device_type),
            driver_info: if info.driver_info.is_empty() {
                info.driver
            } else {
                format!("{} ({})", info.driver, info.driver_info)
            },
            max_buffer_bytes: limits.max_storage_buffer_binding_size as u64,
            pipelines: Mutex::new(HashMap::new()),
        })
    }

    fn get_or_build_pipeline(
        &self,
        key: &'static str,
        wgsl: &'static str,
    ) -> wgpu::ComputePipeline {
        let mut cache = self.pipelines.lock();
        if let Some(p) = cache.get(key) {
            return p.clone();
        }
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(key),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(wgsl)),
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(key),
                layout: None,
                module: &module,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        cache.insert(key, pipeline.clone());
        pipeline
    }

    fn upload_f32(&self, data: &[f32]) -> wgpu::Buffer {
        let byte_size = (data.len().max(1) * 4) as u64;
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("aether_gpu_storage"),
            size: byte_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !data.is_empty() {
            self.queue.write_buffer(&buf, 0, bytemuck::cast_slice(data));
        }
        buf
    }

    fn create_output_f32(&self, len: usize) -> wgpu::Buffer {
        let byte_size = (len.max(1) * 4) as u64;
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("aether_gpu_out"),
            size: byte_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn create_uniform<T: bytemuck::Pod>(&self, val: &T) -> wgpu::Buffer {
        let bytes = bytemuck::bytes_of(val);
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("aether_gpu_uniform"),
            size: bytes.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&buf, 0, bytes);
        buf
    }

    fn readback_f32(&self, src: &wgpu::Buffer, len: usize) -> Result<Vec<f32>, String> {
        if len == 0 {
            return Ok(Vec::new());
        }
        let byte_size = (len * 4) as u64;
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("aether_gpu_staging"),
            size: byte_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("aether_readback_enc"),
            });
        enc.copy_buffer_to_buffer(src, 0, &staging, 0, byte_size);
        self.queue.submit(Some(enc.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| format!("GPU readback channel error: {}", e))?
            .map_err(|e| format!("GPU buffer map failed: {:?}", e))?;

        let mapped = slice.get_mapped_range();
        let floats: Vec<f32> = bytemuck::cast_slice(&mapped).to_vec();
        drop(mapped);
        staging.unmap();
        Ok(floats)
    }

    fn dispatch_pipeline(
        &self,
        pipeline: &wgpu::ComputePipeline,
        buffers: &[&wgpu::Buffer],
        workgroups: [u32; 3],
    ) {
        let entries: Vec<wgpu::BindGroupEntry> = buffers
            .iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let bg_layout = pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("aether_gpu_bg"),
            layout: &bg_layout,
            entries: &entries,
        });

        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("aether_compute_enc"),
            });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("aether_compute_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups[0], workgroups[1], workgroups[2]);
        }
        self.queue.submit(Some(enc.finish()));
    }

    fn matmul_buffers(
        &self,
        a: &wgpu::Buffer,
        b: &wgpu::Buffer,
        m: usize,
        k: usize,
        n: usize,
    ) -> wgpu::Buffer {
        let pipeline = self.get_or_build_pipeline("matmul", WGSL_MATMUL);
        let out = self.create_output_f32(m * n);
        let u = self.create_uniform(&Uniform4 {
            a: m as u32,
            b: k as u32,
            c: n as u32,
            d: 0,
        });
        let wg_x = (n as u32).div_ceil(16).max(1);
        let wg_y = (m as u32).div_ceil(16).max(1);
        self.dispatch_pipeline(&pipeline, &[&u, a, b, &out], [wg_x, wg_y, 1]);
        out
    }

    fn binary_buffers(
        &self,
        a: &wgpu::Buffer,
        b: &wgpu::Buffer,
        len: usize,
        op: u32,
    ) -> wgpu::Buffer {
        let pipeline = self.get_or_build_pipeline("binary", WGSL_BINARY);
        let out = self.create_output_f32(len);
        let u = self.create_uniform(&Uniform4 {
            a: len as u32,
            b: op,
            c: 0,
            d: 0,
        });
        let wg_x = (len as u32).div_ceil(64).max(1);
        self.dispatch_pipeline(&pipeline, &[&u, a, b, &out], [wg_x, 1, 1]);
        out
    }

    fn unary_buffer(&self, a: &wgpu::Buffer, len: usize, op: u32) -> wgpu::Buffer {
        let pipeline = self.get_or_build_pipeline("unary", WGSL_UNARY);
        let out = self.create_output_f32(len);
        let u = self.create_uniform(&Uniform4 {
            a: len as u32,
            b: op,
            c: 0,
            d: 0,
        });
        let wg_x = (len as u32).div_ceil(64).max(1);
        self.dispatch_pipeline(&pipeline, &[&u, a, &out], [wg_x, 1, 1]);
        out
    }

    fn softmax_rows_buffer(&self, a: &wgpu::Buffer, rows: usize, cols: usize) -> wgpu::Buffer {
        let pipeline = self.get_or_build_pipeline("softmax_rows", WGSL_SOFTMAX_ROWS);
        let out = self.create_output_f32(rows * cols);
        let u = self.create_uniform(&Uniform4 {
            a: rows as u32,
            b: cols as u32,
            c: 0,
            d: 0,
        });
        let wg_x = (rows as u32).div_ceil(64).max(1);
        self.dispatch_pipeline(&pipeline, &[&u, a, &out], [wg_x, 1, 1]);
        out
    }

    fn rmsnorm_rows_buffer(
        &self,
        x: &wgpu::Buffer,
        gamma: &wgpu::Buffer,
        rows: usize,
        cols: usize,
        eps: f32,
    ) -> wgpu::Buffer {
        let pipeline = self.get_or_build_pipeline("rmsnorm_rows", WGSL_RMSNORM_ROWS);
        let out = self.create_output_f32(rows * cols);
        let u = self.create_uniform(&Uniform4 {
            a: rows as u32,
            b: cols as u32,
            c: eps.to_bits(),
            d: 0,
        });
        let wg_x = (rows as u32).div_ceil(64).max(1);
        self.dispatch_pipeline(&pipeline, &[&u, x, gamma, &out], [wg_x, 1, 1]);
        out
    }

    fn layernorm_rows_buffer(
        &self,
        x: &wgpu::Buffer,
        gamma: &wgpu::Buffer,
        beta: &wgpu::Buffer,
        rows: usize,
        cols: usize,
        eps: f32,
    ) -> wgpu::Buffer {
        let pipeline = self.get_or_build_pipeline("layernorm_rows", WGSL_LAYERNORM_ROWS);
        let out = self.create_output_f32(rows * cols);
        let u = self.create_uniform(&Uniform4 {
            a: rows as u32,
            b: cols as u32,
            c: eps.to_bits(),
            d: 0,
        });
        let wg_x = (rows as u32).div_ceil(64).max(1);
        self.dispatch_pipeline(&pipeline, &[&u, x, gamma, beta, &out], [wg_x, 1, 1]);
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn attention_buffers(
        &self,
        q: &wgpu::Buffer,
        k: &wgpu::Buffer,
        v: &wgpu::Buffer,
        seq_q: usize,
        seq_k: usize,
        d: usize,
        d_v: usize,
        causal: bool,
    ) -> wgpu::Buffer {
        let pipeline = self.get_or_build_pipeline("attention", WGSL_ATTENTION);
        let out = self.create_output_f32(seq_q * d_v);
        let scale = 1.0f32 / (d as f32).sqrt();
        let u = self.create_uniform(&Uniform8 {
            a: seq_q as u32,
            b: seq_k as u32,
            c: d as u32,
            d: d_v as u32,
            e: if causal { 1 } else { 0 },
            f: scale.to_bits(),
            g: 0,
            h: 0,
        });
        let wg_x = ((seq_q * d_v) as u32).div_ceil(64).max(1);
        self.dispatch_pipeline(&pipeline, &[&u, q, k, v, &out], [wg_x, 1, 1]);
        out
    }

    fn run_custom_wgsl(
        &self,
        wgsl_src: &str,
        entry_point: &str,
        inputs: &[Vec<f32>],
        out_len: usize,
        workgroups: [u32; 3],
    ) -> Result<Vec<f32>, String> {
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("aether_custom_wgsl"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(wgsl_src)),
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("aether_custom_pipeline"),
                layout: None,
                module: &module,
                entry_point: Some(entry_point),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let gpu_ins: Vec<wgpu::Buffer> = inputs.iter().map(|v| self.upload_f32(v)).collect();
        let gpu_out = self.create_output_f32(out_len);
        let mut refs: Vec<&wgpu::Buffer> = gpu_ins.iter().collect();
        refs.push(&gpu_out);
        self.dispatch_pipeline(&pipeline, &refs, workgroups);

        if let Some(err) = pollster::block_on(self.device.pop_error_scope()) {
            return Err(format!("WGSL validation/execution error: {}", err));
        }
        self.readback_f32(&gpu_out, out_len)
    }
}

#[cfg(feature = "gpu")]
static WGPU_BACKEND: OnceLock<Option<WgpuBackend>> = OnceLock::new();

#[cfg(feature = "gpu")]
fn gpu_backend() -> Option<&'static WgpuBackend> {
    WGPU_BACKEND.get_or_init(WgpuBackend::try_init).as_ref()
}

pub fn is_hardware_gpu_available() -> bool {
    #[cfg(feature = "gpu")]
    {
        gpu_backend().is_some()
    }
    #[cfg(not(feature = "gpu"))]
    {
        false
    }
}

static GPU_TENSOR_ACCEL: AtomicBool = AtomicBool::new(false);

pub fn set_tensor_accel(enabled: bool) {
    GPU_TENSOR_ACCEL.store(enabled, Ordering::SeqCst);
}

pub fn is_tensor_accel_enabled() -> bool {
    GPU_TENSOR_ACCEL.load(Ordering::SeqCst)
}

/// Optional transparent hook for `Tensor.matmul` when `GPU.set_tensor_accel(true)` is active.
pub fn try_gpu_matmul_f64(a: &[f64], b: &[f64], m: usize, k: usize, n: usize) -> Option<Vec<f64>> {
    if !is_tensor_accel_enabled() || m * k * n < 262_144 {
        return None;
    }
    #[cfg(feature = "gpu")]
    {
        let backend = gpu_backend()?;
        let a32: Vec<f32> = a.iter().map(|&v| v as f32).collect();
        let b32: Vec<f32> = b.iter().map(|&v| v as f32).collect();
        let ba = backend.upload_f32(&a32);
        let bb = backend.upload_f32(&b32);
        let bc = backend.matmul_buffers(&ba, &bb, m, k, n);
        let out32 = backend.readback_f32(&bc, m * n).ok()?;
        Some(out32.into_iter().map(|v| v as f64).collect())
    }
    #[cfg(not(feature = "gpu"))]
    {
        let _ = (a, b, m, k, n);
        None
    }
}

// ==============================================================================
// 3. Multi-Core CPU Reference / Fallback Kernels (Guaranteed Zero-Fail Portability)
// ==============================================================================

fn cpu_matmul_f32(a: &[f32], b: &[f32], m: usize, k: usize, n: usize) -> Vec<f32> {
    let mut c = vec![0.0f32; m * n];
    let work = m.saturating_mul(k).saturating_mul(n);
    if work >= 65_536 && m >= 16 {
        let threads = std::thread::available_parallelism()
            .map(|t| t.get())
            .unwrap_or(4)
            .min(m / 4)
            .max(1);
        let chunk = m.div_ceil(threads);
        std::thread::scope(|s| {
            for (ti, c_chunk) in c.chunks_mut(chunk * n).enumerate() {
                let row0 = ti * chunk;
                let rows_here = c_chunk.len() / n;
                s.spawn(move || {
                    for lr in 0..rows_here {
                        let r = row0 + lr;
                        let a_row = &a[r * k..(r + 1) * k];
                        let c_row = &mut c_chunk[lr * n..(lr + 1) * n];
                        for p in 0..k {
                            let av = a_row[p];
                            let b_row = &b[p * n..(p + 1) * n];
                            for j in 0..n {
                                c_row[j] += av * b_row[j];
                            }
                        }
                    }
                });
            }
        });
    } else {
        for r in 0..m {
            for p in 0..k {
                let av = a[r * k + p];
                for j in 0..n {
                    c[r * n + j] += av * b[p * n + j];
                }
            }
        }
    }
    c
}

fn cpu_binary_f32(a: &[f32], b: &[f32], op: u32) -> Vec<f32> {
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| match op {
            0 => x + y,
            1 => x - y,
            2 => x * y,
            _ => x / y,
        })
        .collect()
}

fn cpu_unary_f32(a: &[f32], op: u32) -> Vec<f32> {
    a.iter()
        .map(|&x| match op {
            0 => x.max(0.0),
            1 => 1.0 / (1.0 + (-x).exp()),
            2 => x.tanh(),
            3 => {
                let c = 0.797_884_6 * (x + 0.044_715 * x * x * x);
                0.5 * x * (1.0 + c.tanh())
            }
            4 => x / (1.0 + (-x).exp()),
            5 => x.exp(),
            6 => x.abs(),
            _ => x.max(0.0).sqrt(),
        })
        .collect()
}

fn cpu_softmax_rows_f32(x: &[f32], rows: usize, cols: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; rows * cols];
    for r in 0..rows {
        let src = &x[r * cols..(r + 1) * cols];
        let dst = &mut out[r * cols..(r + 1) * cols];
        let max_v = src.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let mut sum = 0.0f32;
        for (d, &v) in dst.iter_mut().zip(src.iter()) {
            let e = (v - max_v).exp();
            *d = e;
            sum += e;
        }
        let inv = 1.0 / sum.max(1e-12);
        for d in dst.iter_mut() {
            *d *= inv;
        }
    }
    out
}

fn cpu_rmsnorm_rows_f32(x: &[f32], gamma: &[f32], rows: usize, cols: usize, eps: f32) -> Vec<f32> {
    let mut out = vec![0.0f32; rows * cols];
    for r in 0..rows {
        let src = &x[r * cols..(r + 1) * cols];
        let dst = &mut out[r * cols..(r + 1) * cols];
        let ss: f32 = src.iter().map(|&v| v * v).sum::<f32>() / (cols as f32);
        let inv = 1.0 / (ss + eps).sqrt();
        for c in 0..cols {
            dst[c] = src[c] * inv * gamma[c];
        }
    }
    out
}

fn cpu_layernorm_rows_f32(
    x: &[f32],
    gamma: &[f32],
    beta: &[f32],
    rows: usize,
    cols: usize,
    eps: f32,
) -> Vec<f32> {
    let mut out = vec![0.0f32; rows * cols];
    for r in 0..rows {
        let src = &x[r * cols..(r + 1) * cols];
        let dst = &mut out[r * cols..(r + 1) * cols];
        let mean: f32 = src.iter().sum::<f32>() / (cols as f32);
        let var: f32 = src.iter().map(|&v| (v - mean) * (v - mean)).sum::<f32>() / (cols as f32);
        let inv = 1.0 / (var + eps).sqrt();
        for c in 0..cols {
            dst[c] = (src[c] - mean) * inv * gamma[c] + beta[c];
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn cpu_attention_f32(
    q: &[f32],
    k: &[f32],
    v: &[f32],
    seq_q: usize,
    seq_k: usize,
    d: usize,
    d_v: usize,
    causal: bool,
) -> Vec<f32> {
    let mut out = vec![0.0f32; seq_q * d_v];
    let scale = 1.0f32 / (d as f32).sqrt();
    for qi in 0..seq_q {
        let max_k = if causal { (qi + 1).min(seq_k) } else { seq_k };
        let mut scores = vec![0.0f32; max_k];
        let mut max_s = f32::NEG_INFINITY;
        for (ki, slot) in scores.iter_mut().enumerate().take(max_k) {
            let mut dot = 0.0f32;
            for c in 0..d {
                dot += q[qi * d + c] * k[ki * d + c];
            }
            let s = dot * scale;
            *slot = s;
            if s > max_s {
                max_s = s;
            }
        }
        let mut sum = 0.0f32;
        for s in &mut scores {
            *s = (*s - max_s).exp();
            sum += *s;
        }
        let inv = 1.0 / sum.max(1e-12);
        for (ki, &w) in scores.iter().enumerate() {
            let wn = w * inv;
            for vj in 0..d_v {
                out[qi * d_v + vj] += wn * v[ki * d_v + vj];
            }
        }
    }
    out
}

// ==============================================================================
// 4. Persistent Device Buffer Registry (VRAM Resident Buffers)
// ==============================================================================

enum DeviceStorage {
    #[cfg(feature = "gpu")]
    Wgpu(wgpu::Buffer),
    Cpu(Vec<f32>),
}

struct GpuBufferRecord {
    shape: Vec<usize>,
    len: usize,
    storage: DeviceStorage,
}

static NEXT_GPU_BUF_ID: AtomicU64 = AtomicU64::new(1);
static GPU_BUFFERS: OnceLock<Mutex<HashMap<u64, Arc<GpuBufferRecord>>>> = OnceLock::new();

fn gpu_buffers() -> &'static Mutex<HashMap<u64, Arc<GpuBufferRecord>>> {
    GPU_BUFFERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn insert_buf(record: GpuBufferRecord) -> u64 {
    let id = NEXT_GPU_BUF_ID.fetch_add(1, Ordering::SeqCst);
    gpu_buffers().lock().insert(id, Arc::new(record));
    id
}

fn get_buf(id: u64) -> Result<Arc<GpuBufferRecord>, String> {
    gpu_buffers()
        .lock()
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("GPU buffer handle {} not found", id))
}

fn upload_tensor_to_device(td: &TensorData) -> GpuBufferRecord {
    let f32_data: Vec<f32> = td.data.iter().map(|&v| v as f32).collect();
    let len = f32_data.len();
    #[cfg(feature = "gpu")]
    if let Some(backend) = gpu_backend() {
        let buf = backend.upload_f32(&f32_data);
        return GpuBufferRecord {
            shape: td.shape.clone(),
            len,
            storage: DeviceStorage::Wgpu(buf),
        };
    }
    GpuBufferRecord {
        shape: td.shape.clone(),
        len,
        storage: DeviceStorage::Cpu(f32_data),
    }
}

fn download_buf_to_f32(rec: &GpuBufferRecord) -> Result<Vec<f32>, String> {
    match &rec.storage {
        #[cfg(feature = "gpu")]
        DeviceStorage::Wgpu(buf) => {
            let backend = gpu_backend().ok_or_else(|| "wgpu backend unavailable".to_string())?;
            backend.readback_f32(buf, rec.len)
        }
        DeviceStorage::Cpu(v) => Ok(v.clone()),
    }
}

fn parse_tensor_or_array(
    val: &Value,
    shape_override: Option<&Value>,
) -> Result<TensorData, String> {
    if let Ok(td) = get_tensor_data(val) {
        return Ok(td);
    }
    match val {
        Value::Array(arr) => {
            let items = arr.lock();
            if items.is_empty() {
                return Err("Cannot upload empty array to GPU".into());
            }
            if matches!(items[0], Value::Array(_)) {
                let rows = items.len();
                let mut flat = Vec::new();
                let mut cols = 0usize;
                for row_v in items.iter() {
                    if let Value::Array(ra) = row_v {
                        let r = ra.lock();
                        if cols == 0 {
                            cols = r.len();
                        } else if r.len() != cols {
                            return Err("Ragged 2D array cannot be uploaded to GPU".into());
                        }
                        for cell in r.iter() {
                            flat.push(val_to_f64(cell)?);
                        }
                    } else {
                        return Err("Mixed 2D/1D array cannot be uploaded to GPU".into());
                    }
                }
                let shape =
                    parse_optional_shape(shape_override).unwrap_or_else(|| vec![rows, cols]);
                TensorData::new(shape, flat)
            } else {
                let mut flat = Vec::with_capacity(items.len());
                for cell in items.iter() {
                    flat.push(val_to_f64(cell)?);
                }
                let len = flat.len();
                let shape = parse_optional_shape(shape_override).unwrap_or_else(|| vec![len]);
                TensorData::new(shape, flat)
            }
        }
        _ => Err("Expected Tensor handle or numeric Array".into()),
    }
}

fn parse_optional_shape(val: Option<&Value>) -> Option<Vec<usize>> {
    match val? {
        Value::Array(arr) => {
            let out: Vec<usize> = arr
                .lock()
                .iter()
                .filter_map(|v| match v {
                    Value::Int(i) if *i > 0 => Some(*i as usize),
                    _ => None,
                })
                .collect();
            if out.is_empty() {
                None
            } else {
                Some(out)
            }
        }
        _ => None,
    }
}

fn val_to_f64(v: &Value) -> Result<f64, String> {
    match v {
        Value::Float(f) => Ok(*f),
        Value::Int(i) => Ok(*i as f64),
        _ => Err(format!("Expected numeric value, got {:?}", v)),
    }
}

fn extract_buf_id(v: &Value) -> Result<u64, String> {
    match v {
        Value::Int(i) if *i > 0 => Ok(*i as u64),
        Value::Map(m) => match m.lock().get("_gpu_buf_id") {
            Some(Value::Int(i)) if *i > 0 => Ok(*i as u64),
            _ => Err("Map is not a valid GPU buffer handle".into()),
        },
        _ => Err("Expected GPU buffer handle integer".into()),
    }
}

fn exec_matmul_records(
    a: &GpuBufferRecord,
    b: &GpuBufferRecord,
) -> Result<GpuBufferRecord, String> {
    if a.shape.len() != 2 || b.shape.len() != 2 {
        return Err(format!(
            "GPU matmul requires 2D matrices, got {:?} and {:?}",
            a.shape, b.shape
        ));
    }
    let (m, k1) = (a.shape[0], a.shape[1]);
    let (k2, n) = (b.shape[0], b.shape[1]);
    if k1 != k2 {
        return Err(format!(
            "GPU matmul shape mismatch: {:?} @ {:?}",
            a.shape, b.shape
        ));
    }

    match (&a.storage, &b.storage) {
        #[cfg(feature = "gpu")]
        (DeviceStorage::Wgpu(ba), DeviceStorage::Wgpu(bb)) => {
            let backend = gpu_backend().ok_or_else(|| "wgpu backend unavailable".to_string())?;
            let out_buf = backend.matmul_buffers(ba, bb, m, k1, n);
            Ok(GpuBufferRecord {
                shape: vec![m, n],
                len: m * n,
                storage: DeviceStorage::Wgpu(out_buf),
            })
        }
        _ => {
            let fa = download_buf_to_f32(a)?;
            let fb = download_buf_to_f32(b)?;
            let fc = cpu_matmul_f32(&fa, &fb, m, k1, n);
            Ok(GpuBufferRecord {
                shape: vec![m, n],
                len: m * n,
                storage: DeviceStorage::Cpu(fc),
            })
        }
    }
}

fn exec_binary_records(
    a: &GpuBufferRecord,
    b: &GpuBufferRecord,
    op: u32,
) -> Result<GpuBufferRecord, String> {
    if a.len != b.len {
        return Err(format!(
            "GPU binary op length mismatch: {} vs {}",
            a.len, b.len
        ));
    }
    match (&a.storage, &b.storage) {
        #[cfg(feature = "gpu")]
        (DeviceStorage::Wgpu(ba), DeviceStorage::Wgpu(bb)) => {
            let backend = gpu_backend().ok_or_else(|| "wgpu backend unavailable".to_string())?;
            let out_buf = backend.binary_buffers(ba, bb, a.len, op);
            Ok(GpuBufferRecord {
                shape: a.shape.clone(),
                len: a.len,
                storage: DeviceStorage::Wgpu(out_buf),
            })
        }
        _ => {
            let fa = download_buf_to_f32(a)?;
            let fb = download_buf_to_f32(b)?;
            let fc = cpu_binary_f32(&fa, &fb, op);
            Ok(GpuBufferRecord {
                shape: a.shape.clone(),
                len: a.len,
                storage: DeviceStorage::Cpu(fc),
            })
        }
    }
}

fn exec_unary_record(a: &GpuBufferRecord, op: u32) -> Result<GpuBufferRecord, String> {
    match &a.storage {
        #[cfg(feature = "gpu")]
        DeviceStorage::Wgpu(ba) => {
            let backend = gpu_backend().ok_or_else(|| "wgpu backend unavailable".to_string())?;
            let out_buf = backend.unary_buffer(ba, a.len, op);
            Ok(GpuBufferRecord {
                shape: a.shape.clone(),
                len: a.len,
                storage: DeviceStorage::Wgpu(out_buf),
            })
        }
        DeviceStorage::Cpu(fa) => Ok(GpuBufferRecord {
            shape: a.shape.clone(),
            len: a.len,
            storage: DeviceStorage::Cpu(cpu_unary_f32(fa, op)),
        }),
    }
}

fn rows_cols_from_shape(shape: &[usize]) -> Result<(usize, usize), String> {
    match shape.len() {
        1 => Ok((1, shape[0])),
        2 => Ok((shape[0], shape[1])),
        _ => {
            let cols = *shape
                .last()
                .ok_or_else(|| "Empty tensor shape".to_string())?;
            let rows: usize = shape[..shape.len() - 1].iter().product();
            Ok((rows, cols))
        }
    }
}

fn exec_softmax_record(a: &GpuBufferRecord) -> Result<GpuBufferRecord, String> {
    let (rows, cols) = rows_cols_from_shape(&a.shape)?;
    match &a.storage {
        #[cfg(feature = "gpu")]
        DeviceStorage::Wgpu(ba) => {
            let backend = gpu_backend().ok_or_else(|| "wgpu backend unavailable".to_string())?;
            let out_buf = backend.softmax_rows_buffer(ba, rows, cols);
            Ok(GpuBufferRecord {
                shape: a.shape.clone(),
                len: a.len,
                storage: DeviceStorage::Wgpu(out_buf),
            })
        }
        DeviceStorage::Cpu(fa) => Ok(GpuBufferRecord {
            shape: a.shape.clone(),
            len: a.len,
            storage: DeviceStorage::Cpu(cpu_softmax_rows_f32(fa, rows, cols)),
        }),
    }
}

fn exec_rmsnorm_records(
    x: &GpuBufferRecord,
    gamma: &GpuBufferRecord,
    eps: f32,
) -> Result<GpuBufferRecord, String> {
    let (rows, cols) = rows_cols_from_shape(&x.shape)?;
    if gamma.len != cols {
        return Err(format!(
            "RMSNorm gamma length {} must match last dimension {}",
            gamma.len, cols
        ));
    }
    match (&x.storage, &gamma.storage) {
        #[cfg(feature = "gpu")]
        (DeviceStorage::Wgpu(bx), DeviceStorage::Wgpu(bg)) => {
            let backend = gpu_backend().ok_or_else(|| "wgpu backend unavailable".to_string())?;
            let out_buf = backend.rmsnorm_rows_buffer(bx, bg, rows, cols, eps);
            Ok(GpuBufferRecord {
                shape: x.shape.clone(),
                len: x.len,
                storage: DeviceStorage::Wgpu(out_buf),
            })
        }
        _ => {
            let fx = download_buf_to_f32(x)?;
            let fg = download_buf_to_f32(gamma)?;
            Ok(GpuBufferRecord {
                shape: x.shape.clone(),
                len: x.len,
                storage: DeviceStorage::Cpu(cpu_rmsnorm_rows_f32(&fx, &fg, rows, cols, eps)),
            })
        }
    }
}

fn exec_layernorm_records(
    x: &GpuBufferRecord,
    gamma: &GpuBufferRecord,
    beta: &GpuBufferRecord,
    eps: f32,
) -> Result<GpuBufferRecord, String> {
    let (rows, cols) = rows_cols_from_shape(&x.shape)?;
    if gamma.len != cols || beta.len != cols {
        return Err(format!(
            "LayerNorm gamma/beta lengths ({}, {}) must match cols {}",
            gamma.len, beta.len, cols
        ));
    }
    match (&x.storage, &gamma.storage, &beta.storage) {
        #[cfg(feature = "gpu")]
        (DeviceStorage::Wgpu(bx), DeviceStorage::Wgpu(bg), DeviceStorage::Wgpu(bb)) => {
            let backend = gpu_backend().ok_or_else(|| "wgpu backend unavailable".to_string())?;
            let out_buf = backend.layernorm_rows_buffer(bx, bg, bb, rows, cols, eps);
            Ok(GpuBufferRecord {
                shape: x.shape.clone(),
                len: x.len,
                storage: DeviceStorage::Wgpu(out_buf),
            })
        }
        _ => {
            let fx = download_buf_to_f32(x)?;
            let fg = download_buf_to_f32(gamma)?;
            let fb = download_buf_to_f32(beta)?;
            Ok(GpuBufferRecord {
                shape: x.shape.clone(),
                len: x.len,
                storage: DeviceStorage::Cpu(cpu_layernorm_rows_f32(&fx, &fg, &fb, rows, cols, eps)),
            })
        }
    }
}

fn exec_attention_records(
    q: &GpuBufferRecord,
    k: &GpuBufferRecord,
    v: &GpuBufferRecord,
    causal: bool,
) -> Result<GpuBufferRecord, String> {
    if q.shape.len() != 2 || k.shape.len() != 2 || v.shape.len() != 2 {
        return Err("GPU attention requires 2D Q, K, V matrices".into());
    }
    let (seq_q, d) = (q.shape[0], q.shape[1]);
    let (seq_k, dk) = (k.shape[0], k.shape[1]);
    let (seq_v, d_v) = (v.shape[0], v.shape[1]);
    if d != dk || seq_k != seq_v {
        return Err(format!(
            "GPU attention shape mismatch: Q={:?}, K={:?}, V={:?}",
            q.shape, k.shape, v.shape
        ));
    }
    match (&q.storage, &k.storage, &v.storage) {
        #[cfg(feature = "gpu")]
        (DeviceStorage::Wgpu(bq), DeviceStorage::Wgpu(bk), DeviceStorage::Wgpu(bv)) => {
            let backend = gpu_backend().ok_or_else(|| "wgpu backend unavailable".to_string())?;
            let out_buf = backend.attention_buffers(bq, bk, bv, seq_q, seq_k, d, d_v, causal);
            Ok(GpuBufferRecord {
                shape: vec![seq_q, d_v],
                len: seq_q * d_v,
                storage: DeviceStorage::Wgpu(out_buf),
            })
        }
        _ => {
            let fq = download_buf_to_f32(q)?;
            let fk = download_buf_to_f32(k)?;
            let fv = download_buf_to_f32(v)?;
            Ok(GpuBufferRecord {
                shape: vec![seq_q, d_v],
                len: seq_q * d_v,
                storage: DeviceStorage::Cpu(cpu_attention_f32(
                    &fq, &fk, &fv, seq_q, seq_k, d, d_v, causal,
                )),
            })
        }
    }
}

fn record_to_tensor_value(rec: &GpuBufferRecord) -> Result<Value, String> {
    let f32_out = download_buf_to_f32(rec)?;
    let f64_out: Vec<f64> = f32_out.into_iter().map(|v| v as f64).collect();
    let td = TensorData::new(rec.shape.clone(), f64_out)?;
    Ok(wrap_tensor_handle(td))
}

fn unary_op_code(name: &str) -> Result<u32, String> {
    match name.to_lowercase().as_str() {
        "relu" => Ok(0),
        "sigmoid" => Ok(1),
        "tanh" => Ok(2),
        "gelu" => Ok(3),
        "silu" | "swish" => Ok(4),
        "exp" => Ok(5),
        "abs" => Ok(6),
        "sqrt" => Ok(7),
        other => Err(format!(
            "Unknown GPU unary op '{}' (supported: relu, sigmoid, tanh, gelu, silu, exp, abs, sqrt)",
            other
        )),
    }
}

fn binary_op_code(name: &str) -> Result<u32, String> {
    match name.to_lowercase().as_str() {
        "add" => Ok(0),
        "sub" => Ok(1),
        "mul" => Ok(2),
        "div" => Ok(3),
        other => Err(format!(
            "Unknown GPU binary op '{}' (supported: add, sub, mul, div)",
            other
        )),
    }
}

// ==============================================================================
// 5. Native Module Registration (`GPU` & `__native_gpu`)
// ==============================================================================

pub fn register_gpu_module(globals: &mut HashMap<String, Value>) {
    let mut gpu_mod = HashMap::new();

    gpu_mod.insert(
        "available".to_string(),
        Value::Native("GPU.available".into(), |_args| {
            Ok(Value::Bool(is_hardware_gpu_available()))
        }),
    );

    gpu_mod.insert(
        "info".to_string(),
        Value::Native("GPU.info".into(), |_args| {
            let mut m = HashMap::new();
            #[cfg(feature = "gpu")]
            if let Some(b) = gpu_backend() {
                m.insert("available".to_string(), Value::Bool(true));
                m.insert("feature_enabled".to_string(), Value::Bool(true));
                m.insert("name".to_string(), Value::string(b.adapter_name.clone()));
                m.insert("backend".to_string(), Value::string(b.backend_name.clone()));
                m.insert(
                    "device_type".to_string(),
                    Value::string(b.device_type.clone()),
                );
                m.insert("driver".to_string(), Value::string(b.driver_info.clone()));
                m.insert(
                    "max_buffer_bytes".to_string(),
                    Value::Int(b.max_buffer_bytes as i64),
                );
                m.insert(
                    "tensor_accel".to_string(),
                    Value::Bool(is_tensor_accel_enabled()),
                );
                return Ok(Value::map(m));
            }
            m.insert("available".to_string(), Value::Bool(false));
            m.insert(
                "feature_enabled".to_string(),
                Value::Bool(cfg!(feature = "gpu")),
            );
            m.insert(
                "name".to_string(),
                Value::string("Multi-Core CPU Fallback".to_string()),
            );
            m.insert(
                "backend".to_string(),
                Value::string("CpuFallback".to_string()),
            );
            m.insert("device_type".to_string(), Value::string("Cpu".to_string()));
            m.insert(
                "driver".to_string(),
                Value::string("native-rust".to_string()),
            );
            m.insert(
                "tensor_accel".to_string(),
                Value::Bool(is_tensor_accel_enabled()),
            );
            Ok(Value::map(m))
        }),
    );

    gpu_mod.insert(
        "set_tensor_accel".to_string(),
        Value::Native("GPU.set_tensor_accel".into(), |args| {
            let enabled = match args.first() {
                Some(Value::Bool(b)) => *b,
                _ => return Err("GPU.set_tensor_accel(bool) requires a boolean".into()),
            };
            set_tensor_accel(enabled);
            Ok(Value::Bool(is_tensor_accel_enabled()))
        }),
    );

    // --- Resident Buffer Lifecycle ---
    gpu_mod.insert(
        "upload".to_string(),
        Value::Native("GPU.upload".into(), |args| {
            if args.is_empty() {
                return Err("GPU.upload(tensor_or_array, shape?) requires 1 arg".into());
            }
            let td = parse_tensor_or_array(&args[0], args.get(1))?;
            let rec = upload_tensor_to_device(&td);
            let id = insert_buf(rec);
            Ok(Value::Int(id as i64))
        }),
    );

    gpu_mod.insert(
        "download".to_string(),
        Value::Native("GPU.download".into(), |args| {
            if args.is_empty() {
                return Err("GPU.download(buf_id) requires buf_id".into());
            }
            let id = extract_buf_id(&args[0])?;
            let rec = get_buf(id)?;
            record_to_tensor_value(&rec)
        }),
    );

    gpu_mod.insert(
        "shape".to_string(),
        Value::Native("GPU.shape".into(), |args| {
            if args.is_empty() {
                return Err("GPU.shape(buf_id) requires buf_id".into());
            }
            let id = extract_buf_id(&args[0])?;
            let rec = get_buf(id)?;
            Ok(Value::array(
                rec.shape.iter().map(|&s| Value::Int(s as i64)).collect(),
            ))
        }),
    );

    gpu_mod.insert(
        "free".to_string(),
        Value::Native("GPU.free".into(), |args| {
            if args.is_empty() {
                return Err("GPU.free(buf_id) requires buf_id".into());
            }
            let id = extract_buf_id(&args[0])?;
            let existed = gpu_buffers().lock().remove(&id).is_some();
            Ok(Value::Bool(existed))
        }),
    );

    // --- Resident Buffer-to-Buffer Ops (Zero Host Roundtrip Chaining) ---
    gpu_mod.insert(
        "matmul_buf".to_string(),
        Value::Native("GPU.matmul_buf".into(), |args| {
            if args.len() < 2 {
                return Err("GPU.matmul_buf(a_id, b_id) requires 2 buffer IDs".into());
            }
            let a = get_buf(extract_buf_id(&args[0])?)?;
            let b = get_buf(extract_buf_id(&args[1])?)?;
            let out = exec_matmul_records(&a, &b)?;
            Ok(Value::Int(insert_buf(out) as i64))
        }),
    );

    gpu_mod.insert(
        "binary_buf".to_string(),
        Value::Native("GPU.binary_buf".into(), |args| {
            if args.len() < 3 {
                return Err("GPU.binary_buf(op, a_id, b_id) requires 3 arguments".into());
            }
            let op = binary_op_code(&format!("{}", args[0]))?;
            let a = get_buf(extract_buf_id(&args[1])?)?;
            let b = get_buf(extract_buf_id(&args[2])?)?;
            let out = exec_binary_records(&a, &b, op)?;
            Ok(Value::Int(insert_buf(out) as i64))
        }),
    );

    gpu_mod.insert(
        "unary_buf".to_string(),
        Value::Native("GPU.unary_buf".into(), |args| {
            if args.len() < 2 {
                return Err("GPU.unary_buf(op, a_id) requires 2 arguments".into());
            }
            let op = unary_op_code(&format!("{}", args[0]))?;
            let a = get_buf(extract_buf_id(&args[1])?)?;
            let out = exec_unary_record(&a, op)?;
            Ok(Value::Int(insert_buf(out) as i64))
        }),
    );

    gpu_mod.insert(
        "softmax_buf".to_string(),
        Value::Native("GPU.softmax_buf".into(), |args| {
            if args.is_empty() {
                return Err("GPU.softmax_buf(a_id) requires buf_id".into());
            }
            let a = get_buf(extract_buf_id(&args[0])?)?;
            let out = exec_softmax_record(&a)?;
            Ok(Value::Int(insert_buf(out) as i64))
        }),
    );

    gpu_mod.insert(
        "rmsnorm_buf".to_string(),
        Value::Native("GPU.rmsnorm_buf".into(), |args| {
            if args.len() < 2 {
                return Err("GPU.rmsnorm_buf(x_id, gamma_id, eps?) requires 2 args".into());
            }
            let x = get_buf(extract_buf_id(&args[0])?)?;
            let g = get_buf(extract_buf_id(&args[1])?)?;
            let eps = args.get(2).and_then(|v| val_to_f64(v).ok()).unwrap_or(1e-5) as f32;
            let out = exec_rmsnorm_records(&x, &g, eps)?;
            Ok(Value::Int(insert_buf(out) as i64))
        }),
    );

    gpu_mod.insert(
        "layernorm_buf".to_string(),
        Value::Native("GPU.layernorm_buf".into(), |args| {
            if args.len() < 3 {
                return Err(
                    "GPU.layernorm_buf(x_id, gamma_id, beta_id, eps?) requires 3 args".into(),
                );
            }
            let x = get_buf(extract_buf_id(&args[0])?)?;
            let g = get_buf(extract_buf_id(&args[1])?)?;
            let b = get_buf(extract_buf_id(&args[2])?)?;
            let eps = args.get(3).and_then(|v| val_to_f64(v).ok()).unwrap_or(1e-5) as f32;
            let out = exec_layernorm_records(&x, &g, &b, eps)?;
            Ok(Value::Int(insert_buf(out) as i64))
        }),
    );

    gpu_mod.insert(
        "attention_buf".to_string(),
        Value::Native("GPU.attention_buf".into(), |args| {
            if args.len() < 3 {
                return Err("GPU.attention_buf(q_id, k_id, v_id, causal?) requires 3 args".into());
            }
            let q = get_buf(extract_buf_id(&args[0])?)?;
            let k = get_buf(extract_buf_id(&args[1])?)?;
            let v = get_buf(extract_buf_id(&args[2])?)?;
            let causal = matches!(args.get(3), Some(Value::Bool(true)));
            let out = exec_attention_records(&q, &k, &v, causal)?;
            Ok(Value::Int(insert_buf(out) as i64))
        }),
    );

    // --- High-Level One-Shot GPU Tensor Ops (Accept Tensors or Arrays Directly) ---
    gpu_mod.insert(
        "matmul".to_string(),
        Value::Native("GPU.matmul".into(), |args| {
            if args.len() < 2 {
                return Err("GPU.matmul(a, b) requires 2 matrices".into());
            }
            let ra = upload_tensor_to_device(&parse_tensor_or_array(&args[0], None)?);
            let rb = upload_tensor_to_device(&parse_tensor_or_array(&args[1], None)?);
            let rc = exec_matmul_records(&ra, &rb)?;
            record_to_tensor_value(&rc)
        }),
    );

    fn gpu_binary_op<const OP: u32>(args: &[Value]) -> Result<Value, String> {
        if args.len() < 2 {
            return Err("GPU binary op requires 2 tensors".into());
        }
        let ra = upload_tensor_to_device(&parse_tensor_or_array(&args[0], None)?);
        let rb = upload_tensor_to_device(&parse_tensor_or_array(&args[1], None)?);
        let rc = exec_binary_records(&ra, &rb, OP)?;
        record_to_tensor_value(&rc)
    }

    fn gpu_unary_op<const OP: u32>(args: &[Value]) -> Result<Value, String> {
        if args.is_empty() {
            return Err("GPU unary op requires 1 tensor".into());
        }
        let rx = upload_tensor_to_device(&parse_tensor_or_array(&args[0], None)?);
        let ro = exec_unary_record(&rx, OP)?;
        record_to_tensor_value(&ro)
    }

    gpu_mod.insert(
        "add".to_string(),
        Value::Native("GPU.add".into(), gpu_binary_op::<0>),
    );
    gpu_mod.insert(
        "sub".to_string(),
        Value::Native("GPU.sub".into(), gpu_binary_op::<1>),
    );
    gpu_mod.insert(
        "mul".to_string(),
        Value::Native("GPU.mul".into(), gpu_binary_op::<2>),
    );
    gpu_mod.insert(
        "div".to_string(),
        Value::Native("GPU.div".into(), gpu_binary_op::<3>),
    );

    gpu_mod.insert(
        "relu".to_string(),
        Value::Native("GPU.relu".into(), gpu_unary_op::<0>),
    );
    gpu_mod.insert(
        "sigmoid".to_string(),
        Value::Native("GPU.sigmoid".into(), gpu_unary_op::<1>),
    );
    gpu_mod.insert(
        "tanh".to_string(),
        Value::Native("GPU.tanh".into(), gpu_unary_op::<2>),
    );
    gpu_mod.insert(
        "gelu".to_string(),
        Value::Native("GPU.gelu".into(), gpu_unary_op::<3>),
    );
    gpu_mod.insert(
        "silu".to_string(),
        Value::Native("GPU.silu".into(), gpu_unary_op::<4>),
    );

    gpu_mod.insert(
        "softmax".to_string(),
        Value::Native("GPU.softmax".into(), |args| {
            if args.is_empty() {
                return Err("GPU.softmax(x) requires 1 tensor".into());
            }
            let rx = upload_tensor_to_device(&parse_tensor_or_array(&args[0], None)?);
            let ro = exec_softmax_record(&rx)?;
            record_to_tensor_value(&ro)
        }),
    );

    gpu_mod.insert(
        "rmsnorm".to_string(),
        Value::Native("GPU.rmsnorm".into(), |args| {
            if args.len() < 2 {
                return Err("GPU.rmsnorm(x, gamma, eps?) requires 2 args".into());
            }
            let rx = upload_tensor_to_device(&parse_tensor_or_array(&args[0], None)?);
            let rg = upload_tensor_to_device(&parse_tensor_or_array(&args[1], None)?);
            let eps = args.get(2).and_then(|v| val_to_f64(v).ok()).unwrap_or(1e-5) as f32;
            let ro = exec_rmsnorm_records(&rx, &rg, eps)?;
            record_to_tensor_value(&ro)
        }),
    );

    gpu_mod.insert(
        "layernorm".to_string(),
        Value::Native("GPU.layernorm".into(), |args| {
            if args.len() < 3 {
                return Err("GPU.layernorm(x, gamma, beta, eps?) requires 3 args".into());
            }
            let rx = upload_tensor_to_device(&parse_tensor_or_array(&args[0], None)?);
            let rg = upload_tensor_to_device(&parse_tensor_or_array(&args[1], None)?);
            let rb = upload_tensor_to_device(&parse_tensor_or_array(&args[2], None)?);
            let eps = args.get(3).and_then(|v| val_to_f64(v).ok()).unwrap_or(1e-5) as f32;
            let ro = exec_layernorm_records(&rx, &rg, &rb, eps)?;
            record_to_tensor_value(&ro)
        }),
    );

    gpu_mod.insert(
        "attention".to_string(),
        Value::Native("GPU.attention".into(), |args| {
            if args.len() < 3 {
                return Err("GPU.attention(q, k, v, causal?) requires 3 args".into());
            }
            let rq = upload_tensor_to_device(&parse_tensor_or_array(&args[0], None)?);
            let rk = upload_tensor_to_device(&parse_tensor_or_array(&args[1], None)?);
            let rv = upload_tensor_to_device(&parse_tensor_or_array(&args[2], None)?);
            let causal = matches!(args.get(3), Some(Value::Bool(true)));
            let ro = exec_attention_records(&rq, &rk, &rv, causal)?;
            record_to_tensor_value(&ro)
        }),
    );

    // --- Custom User WGSL Compute Shader Execution ---
    gpu_mod.insert(
        "run_wgsl".to_string(),
        Value::Native("GPU.run_wgsl".into(), |args| {
            if args.len() < 4 {
                return Err(
                    "GPU.run_wgsl(wgsl_source, inputs_array, out_len, workgroups_xyz, entry_point?) requires 4 args"
                        .into(),
                );
            }
            let wgsl_src = format!("{}", args[0]);
            let inputs_vec: Vec<Vec<f32>> = match &args[1] {
                Value::Array(arr) => {
                    let items = arr.lock().clone();
                    let mut out = Vec::with_capacity(items.len());
                    for item in &items {
                        let td = parse_tensor_or_array(item, None)?;
                        out.push(td.data.iter().map(|&v| v as f32).collect());
                    }
                    out
                }
                _ => return Err("inputs_array must be an Array of Tensors or numeric Arrays".into()),
            };
            let out_len = match args[2] {
                Value::Int(i) if i > 0 => i as usize,
                _ => return Err("out_len must be a positive integer".into()),
            };
            let wg: [u32; 3] = match &args[3] {
                Value::Array(arr) => {
                    let items = arr.lock();
                    let x = items
                        .first()
                        .and_then(|v| match v {
                            Value::Int(i) if *i > 0 => Some(*i as u32),
                            _ => None,
                        })
                        .unwrap_or(1);
                    let y = items
                        .get(1)
                        .and_then(|v| match v {
                            Value::Int(i) if *i > 0 => Some(*i as u32),
                            _ => None,
                        })
                        .unwrap_or(1);
                    let z = items
                        .get(2)
                        .and_then(|v| match v {
                            Value::Int(i) if *i > 0 => Some(*i as u32),
                            _ => None,
                        })
                        .unwrap_or(1);
                    [x, y, z]
                }
                Value::Int(i) if *i > 0 => [*i as u32, 1, 1],
                _ => return Err("workgroups must be [x, y, z] or positive int".into()),
            };
            let entry = args
                .get(4)
                .filter(|v| !matches!(v, Value::Nil))
                .map(|v| format!("{}", v))
                .unwrap_or_else(|| "main".to_string());

            #[cfg(feature = "gpu")]
            if let Some(backend) = gpu_backend() {
                let res = backend.run_custom_wgsl(&wgsl_src, &entry, &inputs_vec, out_len, wg)?;
                return Ok(Value::array(
                    res.into_iter().map(|v| Value::Float(v as f64)).collect(),
                ));
            }
            let _ = (wgsl_src, inputs_vec, out_len, wg, entry);
            Err("Custom WGSL execution requires an active hardware GPU adapter (--features gpu)".into())
        }),
    );

    // --- Hardware GPU vs Multi-Core CPU Matmul Benchmark ---
    gpu_mod.insert(
        "benchmark".to_string(),
        Value::Native("GPU.benchmark".into(), |args| {
            let n = match args.first() {
                Some(Value::Int(i)) if *i >= 16 => (*i as usize).min(2048),
                _ => 256,
            };
            let len = n * n;
            let a_f32: Vec<f32> = (0..len).map(|i| ((i % 97) as f32 - 48.0) * 0.01).collect();
            let b_f32: Vec<f32> = (0..len)
                .map(|i| (((i * 7) % 89) as f32 - 44.0) * 0.01)
                .collect();

            let cpu_t0 = Instant::now();
            let _cpu_out = cpu_matmul_f32(&a_f32, &b_f32, n, n, n);
            let cpu_ms = cpu_t0.elapsed().as_secs_f64() * 1000.0;

            #[cfg(feature = "gpu")]
            let (gpu_ms, max_diff, backend_label) = if let Some(backend) = gpu_backend() {
                let label = format!("{} ({})", backend.adapter_name, backend.backend_name);
                // Warmup shader pipeline compilation
                let ba = backend.upload_f32(&a_f32);
                let bb = backend.upload_f32(&b_f32);
                let _ = backend.matmul_buffers(&ba, &bb, n, n, n);
                backend.device.poll(wgpu::Maintain::Wait);

                let gpu_t0 = Instant::now();
                let bc = backend.matmul_buffers(&ba, &bb, n, n, n);
                let gpu_out = backend.readback_f32(&bc, len)?;
                let elapsed = gpu_t0.elapsed().as_secs_f64() * 1000.0;

                let mut diff = 0.0f64;
                for (x, y) in _cpu_out.iter().zip(gpu_out.iter()) {
                    let d = (*x - *y).abs() as f64;
                    if d > diff {
                        diff = d;
                    }
                }
                (elapsed, diff, label)
            } else {
                (cpu_ms, 0.0f64, "CpuFallback".to_string())
            };

            #[cfg(not(feature = "gpu"))]
            let (gpu_ms, max_diff, backend_label) = (cpu_ms, 0.0f64, "CpuFallback".to_string());

            let flops = 2.0 * (n as f64) * (n as f64) * (n as f64);
            let cpu_gflops = flops / ((cpu_ms / 1000.0).max(1e-9) * 1e9);
            let gpu_gflops = flops / ((gpu_ms / 1000.0).max(1e-9) * 1e9);

            let mut out = HashMap::new();
            out.insert("size".to_string(), Value::Int(n as i64));
            out.insert("backend".to_string(), Value::string(backend_label));
            out.insert("cpu_ms".to_string(), Value::Float(cpu_ms));
            out.insert("gpu_ms".to_string(), Value::Float(gpu_ms));
            out.insert("cpu_gflops".to_string(), Value::Float(cpu_gflops));
            out.insert("gpu_gflops".to_string(), Value::Float(gpu_gflops));
            out.insert(
                "speedup".to_string(),
                Value::Float(cpu_ms / gpu_ms.max(1e-9)),
            );
            out.insert("max_abs_diff".to_string(), Value::Float(max_diff));
            Ok(Value::map(out))
        }),
    );

    globals.insert("GPU".to_string(), Value::map(gpu_mod.clone()));
    globals.insert("__native_gpu".to_string(), Value::map(gpu_mod));
}
