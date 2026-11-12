use aether::vm::nn::{load_safetensors_file, save_safetensors_file};
use aether::vm::tensor::TensorData;
use aether::vm::{run_source, Value};
use std::collections::HashMap;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_nn_rust_autograd_gradients_against_finite_differences() {
    let code = r#"
from aether_nn import Var, LayerNorm, Embedding, SelfAttention, cross_entropy_loss

# 1. LayerNorm + GELU + Linear chain check
let x = Var([[1.0, -2.0, 3.0, 0.5], [-0.5, 2.5, -1.0, 1.5]], true)
let ln = LayerNorm(4)
let h = ln.forward(x).gelu()
let targets = [1, 0]
let logits = h.matmul(Var([[0.2, -0.3], [0.4, 0.1], [-0.5, 0.6], [0.3, -0.2]], true))
let loss = cross_entropy_loss(logits, targets)
loss.backward()

let x_grad = x.grad().to_list()
let gamma_grad = ln.gamma.grad().to_list()
let beta_grad = ln.beta.grad().to_list()

# 2. Embedding + Causal Self-Attention gradient check
let emb = Embedding(6, 4)
let seq = emb.forward([1, 3, 4])
let attn = SelfAttention(4, true)
let out = attn.forward(seq)
let l2 = out.sum()
l2.backward()

let emb_grad = emb.weight.grad().to_list()

[
    loss.data().to_list()[0],
    len(x_grad),
    len(x_grad[0]),
    len(gamma_grad[0]),
    len(beta_grad[0]),
    len(emb_grad),
    # Row 0 of emb table was NOT indexed -> gradient must be all zeros
    emb_grad[0][0] == 0.0 and emb_grad[0][1] == 0.0,
    # Row 1 WAS indexed -> gradient non-zero
    emb_grad[1][0] != 0.0 or emb_grad[1][1] != 0.0
]
"#;

    let res = run_source(code).expect("NN gradient graph check failed");
    let Value::Array(items) = res else {
        panic!("Expected Array, got {:?}", res);
    };
    let v = items.lock().clone();
    let loss_val = match v[0] {
        Value::Float(f) => f,
        _ => panic!("Expected float loss"),
    };
    assert!(loss_val > 0.0 && loss_val.is_finite());
    assert_eq!(v[1], Value::Int(2));
    assert_eq!(v[2], Value::Int(4));
    assert_eq!(v[3], Value::Int(4));
    assert_eq!(v[4], Value::Int(4));
    assert_eq!(v[5], Value::Int(6));
    assert_eq!(v[6], Value::Bool(true));
    assert_eq!(v[7], Value::Bool(true));
}

#[test]
fn test_nn_mlp_adamw_and_transformer_convergence() {
    let code = r#"
from aether_nn import Var, Linear, Embedding, TransformerBlock, Sequential, AdamW, cross_entropy_loss

# Train a deep non-linear Classifier (Linear -> GELU -> Linear) with AdamW
let X = Var([
    [0.0, 0.0],
    [0.1, 0.2],
    [1.0, 1.0],
    [0.9, 0.85],
    [0.0, 1.0],
    [0.15, 0.9],
    [1.0, 0.0],
    [0.9, 0.1]
], false)
let targets = [0, 0, 0, 0, 1, 1, 1, 1]

let l1 = Linear(2, 8, true)
let l2 = Linear(8, 2, true)
let params = []
for p in l1.parameters():
    params.push(p)
for p in l2.parameters():
    params.push(p)

let opt = AdamW(params, 0.05, 0.9, 0.999, 0.00000001, 0.001)

let first_loss = 0.0
let last_loss = 0.0
for epoch in 0..80:
    opt.zero_grad()
    let h = l1.forward(X).gelu()
    let logits = l2.forward(h)
    let loss = cross_entropy_loss(logits, targets)
    let lv = loss.data().to_list()[0]
    if epoch == 0:
        first_loss = lv
    last_loss = lv
    loss.backward()
    opt.step()

# Check TransformerBlock next-token sequence training
let emb = Embedding(5, 4)
let block = TransformerBlock(4, 8, true)
let head = Linear(4, 5, true)
let t_params = []
for p in emb.parameters():
    t_params.push(p)
for p in block.parameters():
    t_params.push(p)
for p in head.parameters():
    t_params.push(p)
let t_opt = AdamW(t_params, 0.05)

let t_first = 0.0
let t_last = 0.0
for step in 0..45:
    t_opt.zero_grad()
    let seq_h = block.forward(emb.forward([0, 1, 2]))
    let seq_logits = head.forward(seq_h)
    let seq_loss = cross_entropy_loss(seq_logits, [1, 2, 3])
    let slv = seq_loss.data().to_list()[0]
    if step == 0:
        t_first = slv
    t_last = slv
    seq_loss.backward()
    t_opt.step()

[first_loss, last_loss, t_first, t_last]
"#;

    let res = run_source(code).expect("AdamW & Transformer convergence test failed");
    let Value::Array(items) = res else {
        panic!("Expected Array, got {:?}", res);
    };
    let v = items.lock().clone();
    let f_loss = match v[0] {
        Value::Float(f) => f,
        _ => panic!("Expected float"),
    };
    let l_loss = match v[1] {
        Value::Float(f) => f,
        _ => panic!("Expected float"),
    };
    let tf_loss = match v[2] {
        Value::Float(f) => f,
        _ => panic!("Expected float"),
    };
    let tl_loss = match v[3] {
        Value::Float(f) => f,
        _ => panic!("Expected float"),
    };

    assert!(
        l_loss < f_loss * 0.25,
        "MLP CrossEntropy did not converge enough: first={} last={}",
        f_loss,
        l_loss
    );
    assert!(
        tl_loss < tf_loss * 0.25,
        "TransformerBlock next-token prediction did not converge: first={} last={}",
        tf_loss,
        tl_loss
    );
}

#[test]
fn test_nn_safetensors_export_import_and_f16_bf16_f32_decoding() {
    let dir = tempdir().expect("tempdir");
    let st_path = dir.path().join("model.safetensors");
    let st_path_str = st_path.to_string_lossy().replace('\\', "/");

    // 1. Round-trip via AETHER `aether_nn` save_weights / load_weights
    let code = format!(
        r#"
from aether_nn import Linear, save_weights, load_weights

let layer = Linear(3, 2, true)
save_weights({{"fc.weight": layer.W, "fc.bias": layer.b}}, "{path}")
let restored = load_weights("{path}")

[
    restored["fc.weight"].shape(),
    restored["fc.bias"].shape(),
    restored["fc.weight"].to_list() == layer.W.data().to_list(),
    restored["fc.bias"].to_list() == layer.b.data().to_list()
]
"#,
        path = st_path_str
    );

    let res = run_source(&code).expect("Safetensors AETHER roundtrip failed");
    let Value::Array(items) = res else {
        panic!("Expected Array");
    };
    let v = items.lock().clone();
    assert_eq!(format!("{}", v[0]), "[3, 2]");
    assert_eq!(format!("{}", v[1]), "[1, 2]");
    assert_eq!(v[2], Value::Bool(true));
    assert_eq!(v[3], Value::Bool(true));

    // 2. Verify raw HuggingFace binary `.safetensors` decoding for F32, F16, and BF16 dtypes
    let hf_path = dir.path().join("hf_mixed.safetensors");
    let f32_val: f32 = 3.25;
    let f16_bits: u16 = 0x3C00; // 1.0 in IEEE 754 float16
    let bf16_bits: u16 = 0x4020; // 2.5 in bfloat16 (0x40200000 as f32 = 2.5)

    let mut payload = Vec::new();
    payload.extend_from_slice(&f32_val.to_le_bytes()); // offset 0..4
    payload.extend_from_slice(&f16_bits.to_le_bytes()); // offset 4..6
    payload.extend_from_slice(&bf16_bits.to_le_bytes()); // offset 6..8

    let header_json = serde_json::json!({
        "__metadata__": {"format": "pt"},
        "t_f32": {"dtype": "F32", "shape": [1], "data_offsets": [0, 4]},
        "t_f16": {"dtype": "F16", "shape": [1], "data_offsets": [4, 6]},
        "t_bf16": {"dtype": "BF16", "shape": [1], "data_offsets": [6, 8]}
    });
    let header_bytes = serde_json::to_vec(&header_json).unwrap();
    let header_len = header_bytes.len() as u64;
    let mut raw = Vec::new();
    raw.extend_from_slice(&header_len.to_le_bytes());
    raw.extend_from_slice(&header_bytes);
    raw.extend_from_slice(&payload);
    fs::write(&hf_path, raw).unwrap();

    let decoded =
        load_safetensors_file(hf_path.to_str().unwrap()).expect("Failed decoding HF file");
    assert!((decoded["t_f32"].data[0] - 3.25).abs() < 1e-6);
    assert!((decoded["t_f16"].data[0] - 1.0).abs() < 1e-6);
    assert!((decoded["t_bf16"].data[0] - 2.5).abs() < 1e-6);

    // Also verify Rust-side save helper sanity
    let mut map = HashMap::new();
    map.insert(
        "w".to_string(),
        TensorData::new(vec![2], vec![1.5, -2.5]).unwrap(),
    );
    let rust_st = dir.path().join("rust.safetensors");
    save_safetensors_file(&map, rust_st.to_str().unwrap()).unwrap();
    let reloaded = load_safetensors_file(rust_st.to_str().unwrap()).unwrap();
    assert_eq!(reloaded["w"].data, vec![1.5, -2.5]);
}
