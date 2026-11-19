use aether::vm::run_source;

#[test]
fn test_gpu_kernels_numerical_parity_with_cpu() {
    let code = r#"
from aether_tensor import Tensor

let info = GPU.info()
if info["backend"] == nil or info["name"] == nil:
    raise "GPU.info() missing required fields"

# 1. Non-multiple-of-16 rectangular MatMul (19x37 @ 37x23) tests tile boundary guards
let m = 19
let k = 37
let n = 23
let a_arr = []
let b_arr = []
let i = 0
while i < m * k:
    a_arr.append(((i % 13) - 6) * 0.1)
    i = i + 1
let j = 0
while j < k * n:
    b_arr.append((((j * 5) % 11) - 5) * 0.1)
    j = j + 1

let ta = Tensor(a_arr, [m, k])
let tb = Tensor(b_arr, [k, n])

GPU.set_tensor_accel(false)
let cpu_c = ta.matmul(tb).to_array()
let gpu_c = Tensor(GPU.matmul(ta.handle, tb.handle)).to_array()

if len(cpu_c) != m or len(cpu_c[0]) != n:
    raise "Unexpected matmul output shape"

let max_diff = 0.0
let r = 0
while r < m:
    let c = 0
    while c < n:
        let d = Math.abs(cpu_c[r][c] - gpu_c[r][c])
        if d > max_diff:
            max_diff = d
        c = c + 1
    r = r + 1
if max_diff > 0.001:
    raise "GPU matmul diverged from CPU: max_diff=" + str(max_diff)

# 2. Elementwise binary & unary ops
let u = Tensor([-2.0, -0.5, 0.0, 0.5, 2.0], [1, 5])
let v = Tensor([1.0, 2.0, 3.0, 4.0, 5.0], [1, 5])
let g_add = Tensor(GPU.add(u.handle, v.handle)).to_array()[0]
if Math.abs(g_add[0] - (-1.0)) > 0.001 or Math.abs(g_add[4] - 7.0) > 0.001:
    raise "GPU.add mismatch"

let g_relu = Tensor(GPU.relu(u.handle)).to_array()[0]
if Math.abs(g_relu[0] - 0.0) > 0.001 or Math.abs(g_relu[4] - 2.0) > 0.001:
    raise "GPU.relu mismatch"

let g_gelu = Tensor(GPU.gelu(u.handle)).to_array()[0]
let c_gelu = u.gelu().to_array()[0]
let idx = 0
while idx < 5:
    if Math.abs(g_gelu[idx] - c_gelu[idx]) > 0.002:
        raise "GPU.gelu mismatch at " + str(idx)
    idx = idx + 1

let g_silu = Tensor(GPU.silu(u.handle)).to_array()[0]
let c_silu = u.silu().to_array()[0]
idx = 0
while idx < 5:
    if Math.abs(g_silu[idx] - c_silu[idx]) > 0.002:
        raise "GPU.silu mismatch at " + str(idx)
    idx = idx + 1

# 3. Softmax rows (sum to 1.0)
let sm = Tensor(GPU.softmax(Tensor([[1.0, 2.0, 3.0], [-1.0, 0.0, 1.0]]).handle)).to_array()
let s0 = sm[0][0] + sm[0][1] + sm[0][2]
let s1 = sm[1][0] + sm[1][1] + sm[1][2]
if Math.abs(s0 - 1.0) > 0.001 or Math.abs(s1 - 1.0) > 0.001:
    raise "GPU.softmax rows do not sum to 1"

# 4. RMSNorm & LayerNorm
let x_norm = Tensor([[1.0, 2.0, 3.0, 4.0], [2.0, 4.0, 6.0, 8.0]])
let gamma = Tensor([1.0, 1.0, 1.0, 1.0], [4])
let beta = Tensor([0.0, 0.0, 0.0, 0.0], [4])
let rms = Tensor(GPU.rmsnorm(x_norm.handle, gamma.handle, 0.00001)).to_array()
# Row 0 RMS = sqrt((1+4+9+16)/4) = sqrt(7.5) ~= 2.73861278
if Math.abs(rms[0][3] - (4.0 / 2.73861278)) > 0.002:
    raise "GPU.rmsnorm mismatch: " + str(rms[0][3])

let ln = Tensor(GPU.layernorm(x_norm.handle, gamma.handle, beta.handle, 0.00001)).to_array()
let ln_mean = (ln[0][0] + ln[0][1] + ln[0][2] + ln[0][3]) / 4.0
if Math.abs(ln_mean) > 0.002:
    raise "GPU.layernorm mean should be ~0, got " + str(ln_mean)

# 5. Causal Self-Attention (first token must ONLY attend to itself -> equals V[0])
let q = Tensor([[1.0, 0.0], [0.0, 1.0], [1.0, 1.0]])
let kv = Tensor([[1.0, 0.0], [0.0, 1.0], [1.0, 1.0]])
let val = Tensor([[10.0, 20.0], [30.0, 40.0], [50.0, 60.0]])
let attn = Tensor(GPU.attention(q.handle, kv.handle, val.handle, true)).to_array()
if Math.abs(attn[0][0] - 10.0) > 0.001 or Math.abs(attn[0][1] - 20.0) > 0.001:
    raise "Causal attention row 0 leaked future tokens!"

"ok"
"#;

    let res = run_source(code).expect("GPU kernel parity test should pass");
    assert_eq!(format!("{}", res), "ok");
}

#[test]
fn test_gpu_tensor_vram_chaining_benchmark_and_custom_wgsl() {
    let code = r#"
from aether_tensor import Tensor
from aether_gpu import GpuTensor, available, benchmark, run_wgsl, set_tensor_accel

# 1. VRAM-resident pipeline chaining via aether_gpu.ae
let gx = GpuTensor([[1.0, -2.0, 3.0, 4.0], [-1.0, 2.0, -3.0, 4.0]])
let gw = GpuTensor([
    [0.5, -0.5, 0.2, 0.1],
    [0.1,  0.4, -0.3, 0.2],
    [-0.2, 0.3, 0.5, -0.1],
    [0.4,  0.1, 0.2, 0.3]
])
let g_gamma = GpuTensor([1.0, 1.0, 1.0, 1.0], [4])

let out_gpu = gx.matmul(gw).rmsnorm(g_gamma).gelu().softmax()
let out_cpu = out_gpu.cpu().to_array()
let row_sum = out_cpu[0][0] + out_cpu[0][1] + out_cpu[0][2] + out_cpu[0][3]
if Math.abs(row_sum - 1.0) > 0.002:
    raise "VRAM chained softmax row sum mismatch: " + str(row_sum)

gx.free()
gw.free()
g_gamma.free()
out_gpu.free()

# 2. Benchmark CPU vs GPU on 128x128 matmul
let bench = benchmark(128)
if bench["max_abs_diff"] > 0.01:
    raise "Benchmark GPU vs CPU diff too large: " + str(bench["max_abs_diff"])

# 3. Transparent Tensor matmul acceleration toggle
set_tensor_accel(true)
let big_a = Tensor.ones([64, 64])
let big_b = Tensor.ones([64, 64])
let big_c = big_a.matmul(big_b).to_array()
set_tensor_accel(false)
if Math.abs(big_c[0][0] - 64.0) > 0.01:
    raise "Accelerated Tensor.matmul mismatch: " + str(big_c[0][0])

# 4. Custom WGSL shader test when hardware GPU adapter is active
if available():
    let wgsl = "
@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> out: array<f32>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i < arrayLength(&out)) {
        out[i] = a[i] * 3.0 + b[i];
    }
}
"
    let res = run_wgsl(wgsl, [[1.0, 2.0, 3.0, 4.0], [10.0, 20.0, 30.0, 40.0]], 4, [1, 1, 1])
    if Math.abs(res[0] - 13.0) > 0.001 or Math.abs(res[3] - 52.0) > 0.001:
        raise "Custom WGSL output mismatch"

"ok"
"#;

    let res = run_source(code).expect("GPU VRAM chaining & WGSL test should pass");
    assert_eq!(format!("{}", res), "ok");
}
