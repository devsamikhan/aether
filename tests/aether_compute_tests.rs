// ==============================================================================
// AetherCompute Unit & Integration Test Suite
// Verifying Software GPU & Multi-Core Workgroup Compute Shader Execution
// ==============================================================================

use aether::vm::compute::{ComputeBuffer, WorkGroupScheduler};
use aether::vm::{run_source, Value};
use std::sync::{Arc, Mutex};

#[test]
fn test_compute_buffer_allocation_and_indexing() {
    // 1D Buffer
    let mut b1 = ComputeBuffer::new([4, 1, 1], vec![1.0, 2.0, 3.0, 4.0]).unwrap();
    assert_eq!(b1.len(), 4);
    assert_eq!(b1.get(2, 0, 0), Some(3.0));
    b1.set(2, 0, 0, 99.0).unwrap();
    assert_eq!(b1.get(2, 0, 0), Some(99.0));

    // 2D Buffer [width=3, height=2, depth=1]
    let b2 = ComputeBuffer::new([3, 2, 1], vec![
        1.0, 2.0, 3.0,
        4.0, 5.0, 6.0,
    ]).unwrap();
    assert_eq!(b2.get(1, 1, 0), Some(5.0));
    assert_eq!(b2.get(2, 1, 0), Some(6.0));
    assert_eq!(b2.get(3, 1, 0), None); // Out of bounds
}

#[test]
fn test_compute_parallel_vector_add_and_fma() {
    let n = 2048;
    let data_a: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let data_b: Vec<f64> = (0..n).map(|i| (i * 2) as f64).collect();
    let data_c: Vec<f64> = vec![10.0; n];

    let buf_a = Arc::new(Mutex::new(ComputeBuffer::new([n, 1, 1], data_a).unwrap()));
    let buf_b = Arc::new(Mutex::new(ComputeBuffer::new([n, 1, 1], data_b).unwrap()));
    let buf_c = Arc::new(Mutex::new(ComputeBuffer::new([n, 1, 1], data_c).unwrap()));
    let buf_out = Arc::new(Mutex::new(ComputeBuffer::zeros([n, 1, 1])));

    // 1. Vector Add: out = a + b
    WorkGroupScheduler::dispatch(
        "vec_add",
        &[buf_a.clone(), buf_b.clone()],
        &buf_out,
        [n, 1, 1],
        [64, 1, 1],
        &[],
    ).unwrap();

    let out_guard = buf_out.lock().unwrap();
    for i in 0..n {
        assert_eq!(out_guard.data[i], (i * 3) as f64);
    }
    drop(out_guard);

    // 2. Fused Multiply-Add: out = a * b + c
    WorkGroupScheduler::dispatch(
        "fma",
        &[buf_a, buf_b, buf_c],
        &buf_out,
        [n, 1, 1],
        [64, 1, 1],
        &[],
    ).unwrap();

    let out_guard2 = buf_out.lock().unwrap();
    for i in 0..n {
        let expected = (i as f64) * ((i * 2) as f64) + 10.0;
        assert_eq!(out_guard2.data[i], expected);
    }
}

#[test]
fn test_compute_parallel_matmul() {
    // A: 2x3 matrix
    // [ [1, 2, 3],
    //   [4, 5, 6] ]
    let a_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let buf_a = Arc::new(Mutex::new(ComputeBuffer::new([3, 2, 1], a_data).unwrap()));

    // B: 3x2 matrix
    // [ [7, 8],
    //   [9, 1],
    //   [2, 3] ]
    let b_data = vec![7.0, 8.0, 9.0, 1.0, 2.0, 3.0];
    let buf_b = Arc::new(Mutex::new(ComputeBuffer::new([2, 3, 1], b_data).unwrap()));

    // Out: 2x2 matrix
    // [ [1*7 + 2*9 + 3*2,  1*8 + 2*1 + 3*3],
    //   [4*7 + 5*9 + 6*2,  4*8 + 5*1 + 6*3] ]
    // = [ [7+18+6, 8+2+9], [28+45+12, 32+5+18] ]
    // = [ [31, 19], [85, 55] ]
    let buf_out = Arc::new(Mutex::new(ComputeBuffer::zeros([2, 2, 1])));

    WorkGroupScheduler::dispatch(
        "matmul",
        &[buf_a, buf_b],
        &buf_out,
        [2, 2, 1],
        [16, 16, 1],
        &[],
    ).unwrap();

    let res = buf_out.lock().unwrap();
    assert_eq!(res.data, vec![31.0, 19.0, 85.0, 55.0]);
}

#[test]
fn test_compute_parallel_conv2d() {
    // 5x5 Image
    let img_w = 5;
    let img_h = 5;
    let mut img_data = vec![0.0; img_w * img_h];
    // Set center pixel to 10.0
    img_data[2 * img_w + 2] = 10.0;
    let img_buf = Arc::new(Mutex::new(ComputeBuffer::new([img_w, img_h, 1], img_data).unwrap()));

    // 3x3 Box blur filter (uniform 1/9)
    let k_val = 1.0 / 9.0;
    let kernel_data = vec![k_val; 9];
    let kernel_buf = Arc::new(Mutex::new(ComputeBuffer::new([3, 3, 1], kernel_data).unwrap()));

    let out_buf = Arc::new(Mutex::new(ComputeBuffer::zeros([img_w, img_h, 1])));

    WorkGroupScheduler::dispatch(
        "conv2d",
        &[img_buf, kernel_buf],
        &out_buf,
        [img_w, img_h, 1],
        [16, 16, 1],
        &[],
    ).unwrap();

    let out = out_buf.lock().unwrap();
    // Center pixel of blurred image should receive 10.0 * (1/9)
    let center_blur = out.data[2 * img_w + 2];
    assert!((center_blur - (10.0 / 9.0)).abs() < 1e-6);
}

#[test]
fn test_compute_parallel_reductions() {
    let n = 10000;
    let data: Vec<f64> = (1..=n).map(|i| i as f64).collect();
    let buf = Arc::new(Mutex::new(ComputeBuffer::new([n, 1, 1], data).unwrap()));
    let out = Arc::new(Mutex::new(ComputeBuffer::zeros([1, 1, 1])));

    // Parallel Sum: 10000 * 10001 / 2 = 50,005,000
    WorkGroupScheduler::dispatch(
        "parallel_sum",
        &[buf.clone()],
        &out,
        [n, 1, 1],
        [64, 1, 1],
        &[],
    ).unwrap();
    assert_eq!(out.lock().unwrap().data[0], 50005000.0);

    // Parallel Max: 10000.0
    WorkGroupScheduler::dispatch(
        "parallel_max",
        &[buf],
        &out,
        [n, 1, 1],
        [64, 1, 1],
        &[],
    ).unwrap();
    assert_eq!(out.lock().unwrap().data[0], 10000.0);
}

#[test]
fn test_compute_aether_stdlib_script() {
    let code = r#"
from aether_compute import Buffer, ComputePipeline, vec_add, parallel_sum, parallel_max

# 1. Create two 1D Buffers
let b1 = Buffer.from_array([10.0, 20.0, 30.0, 40.0, 50.0])
let b2 = Buffer.from_array([1.0, 2.0, 3.0, 4.0, 5.0])

# 2. Vector Addition across workgroups
let b_sum = vec_add(b1, b2)
let result_list = b_sum.to_list()

# 3. Parallel Reduction Sum: 11 + 22 + 33 + 44 + 55 = 165
let total = parallel_sum(b_sum)

# 4. Parallel Max: 55.0
let maximum = parallel_max(b_sum)

[
    result_list[0],
    result_list[4],
    total,
    maximum
]
"#;

    let res = run_source(code).expect("AetherCompute script execution failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Float(11.0));
        assert_eq!(items[1], Value::Float(55.0));
        assert_eq!(items[2], Value::Float(165.0));
        assert_eq!(items[3], Value::Float(55.0));
    } else {
        panic!("Expected array, got {:?}", res);
    }
}
