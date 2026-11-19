// ==============================================================================
// AetherNN — Pure-Rust Reverse-Mode Autograd Tape, Deep Learning Layers,
// True AdamW Optimizer & HuggingFace `.safetensors` Model Loader/Exporter
// Zero External ML/Python Dependencies
// ==============================================================================

use super::tensor::{get_tensor_data, wrap_tensor_handle, TensorData};
use super::value::Value;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

static NEXT_VAR_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_OPT_ID: AtomicU64 = AtomicU64::new(1);
static NN_USE_GPU: AtomicBool = AtomicBool::new(false);

fn nn_matmul(a: &TensorData, b: &TensorData) -> Result<TensorData, String> {
    if NN_USE_GPU.load(Ordering::SeqCst) && a.shape.len() == 2 && b.shape.len() == 2 {
        let (m, k1) = (a.shape[0], a.shape[1]);
        let (k2, n) = (b.shape[0], b.shape[1]);
        if k1 == k2 {
            if let Some(out) = super::gpu::gpu_matmul_f64_direct(&a.data, &b.data, m, k1, n) {
                return TensorData::new(vec![m, n], out);
            }
        }
    }
    a.matmul(b)
}

static AUTOGRAD_NODES: OnceLock<Mutex<HashMap<u64, TapeVar>>> = OnceLock::new();
static OPTIMIZERS: OnceLock<Mutex<HashMap<u64, AdamWState>>> = OnceLock::new();

fn nodes_store() -> &'static Mutex<HashMap<u64, TapeVar>> {
    AUTOGRAD_NODES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn opt_store() -> &'static Mutex<HashMap<u64, AdamWState>> {
    OPTIMIZERS.get_or_init(|| Mutex::new(HashMap::new()))
}

// ==============================================================================
// 1. Autograd Tape Graph Definitions
// ==============================================================================

#[derive(Clone, Debug)]
pub enum TapeOp {
    Leaf,
    Add(u64, u64),
    Sub(u64, u64),
    Mul(u64, u64),
    Div(u64, u64),
    Matmul(u64, u64),
    Transpose(u64),
    Reshape(u64, Vec<usize>),
    Relu(u64),
    Sigmoid(u64),
    Tanh(u64),
    Gelu(u64),
    Silu(u64),
    Sum(u64),
    Mean(u64),
    MseLoss {
        pred: u64,
        target: u64,
    },
    CrossEntropyLoss {
        logits: u64,
        targets: Vec<usize>,
        probs: Vec<f64>,
        num_classes: usize,
    },
    LayerNorm {
        x: u64,
        gamma: u64,
        beta: u64,
        eps: f64,
        x_hat: Vec<f64>,
        inv_std: Vec<f64>,
    },
    Embedding {
        table: u64,
        indices: Vec<usize>,
    },
    Attention {
        q: u64,
        k: u64,
        v: u64,
        probs: Vec<f64>,
        seq_q: usize,
        seq_k: usize,
        d_k: usize,
        causal: bool,
    },
}

#[derive(Clone, Debug)]
pub struct TapeVar {
    pub id: u64,
    pub data: TensorData,
    pub grad: Option<TensorData>,
    pub requires_grad: bool,
    pub op: TapeOp,
}

impl TapeOp {
    fn parents(&self) -> Vec<u64> {
        match self {
            TapeOp::Leaf => Vec::new(),
            TapeOp::Add(a, b)
            | TapeOp::Sub(a, b)
            | TapeOp::Mul(a, b)
            | TapeOp::Div(a, b)
            | TapeOp::Matmul(a, b) => vec![*a, *b],
            TapeOp::MseLoss { pred, target } => vec![*pred, *target],
            TapeOp::Transpose(a)
            | TapeOp::Reshape(a, _)
            | TapeOp::Relu(a)
            | TapeOp::Sigmoid(a)
            | TapeOp::Tanh(a)
            | TapeOp::Gelu(a)
            | TapeOp::Silu(a)
            | TapeOp::Sum(a)
            | TapeOp::Mean(a) => vec![*a],
            TapeOp::CrossEntropyLoss { logits, .. } => vec![*logits],
            TapeOp::LayerNorm { x, gamma, beta, .. } => vec![*x, *gamma, *beta],
            TapeOp::Embedding { table, .. } => vec![*table],
            TapeOp::Attention { q, k, v, .. } => vec![*q, *k, *v],
        }
    }
}

fn alloc_var(data: TensorData, requires_grad: bool, op: TapeOp) -> u64 {
    let id = NEXT_VAR_ID.fetch_add(1, Ordering::SeqCst);
    let var = TapeVar {
        id,
        data,
        grad: None,
        requires_grad,
        op,
    };
    nodes_store().lock().unwrap().insert(id, var);
    id
}

fn get_var_clone(id: u64) -> Result<TapeVar, String> {
    nodes_store()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("NN node handle {} not found", id))
}

fn accumulate_grad(
    store: &mut HashMap<u64, TapeVar>,
    target_id: u64,
    grad: TensorData,
) -> Result<(), String> {
    let Some(node) = store.get_mut(&target_id) else {
        return Ok(());
    };
    if !node.requires_grad {
        return Ok(());
    }
    let reduced = if grad.shape == node.data.shape {
        grad
    } else if grad.data.len() == node.data.data.len() {
        TensorData::new(node.data.shape.clone(), grad.data)?
    } else {
        grad.reduce_sum_to(&node.data.shape)?
    };
    if let Some(existing) = &mut node.grad {
        for (e, &g) in existing.data.iter_mut().zip(reduced.data.iter()) {
            *e += g;
        }
    } else {
        node.grad = Some(reduced);
    }
    Ok(())
}

// ==============================================================================
// 2. Binary Broadcasting & Backward Kernel Dispatch
// ==============================================================================

fn broadcast_binary<F>(a: &TensorData, b: &TensorData, f: F) -> Result<TensorData, String>
where
    F: Fn(f64, f64) -> f64,
{
    if a.shape == b.shape {
        let out = a
            .data
            .iter()
            .zip(b.data.iter())
            .map(|(&x, &y)| f(x, y))
            .collect();
        return TensorData::new(a.shape.clone(), out);
    }
    if b.data.len() == 1 {
        let s = b.data[0];
        let out = a.data.iter().map(|&x| f(x, s)).collect();
        return TensorData::new(a.shape.clone(), out);
    }
    if a.data.len() == 1 {
        let s = a.data[0];
        let out = b.data.iter().map(|&y| f(s, y)).collect();
        return TensorData::new(b.shape.clone(), out);
    }

    let r1 = a.shape.len();
    let r2 = b.shape.len();
    let max_r = r1.max(r2);
    let mut pad1 = vec![1; max_r];
    let mut pad2 = vec![1; max_r];
    for (i, &d) in a.shape.iter().rev().enumerate() {
        pad1[max_r - 1 - i] = d;
    }
    for (i, &d) in b.shape.iter().rev().enumerate() {
        pad2[max_r - 1 - i] = d;
    }

    let mut out_shape = vec![0; max_r];
    for i in 0..max_r {
        if pad1[i] == pad2[i] {
            out_shape[i] = pad1[i];
        } else if pad1[i] == 1 {
            out_shape[i] = pad2[i];
        } else if pad2[i] == 1 {
            out_shape[i] = pad1[i];
        } else {
            return Err(format!(
                "Incompatible shapes for NN broadcast: {:?} and {:?}",
                a.shape, b.shape
            ));
        }
    }

    let total: usize = out_shape.iter().product();
    let mut out_strides = vec![1; max_r];
    for i in (0..max_r.saturating_sub(1)).rev() {
        out_strides[i] = out_strides[i + 1] * out_shape[i + 1];
    }

    let mut eff1 = vec![0; max_r];
    let mut eff2 = vec![0; max_r];
    for i in 0..r1 {
        if a.shape[i] > 1 {
            eff1[(max_r - r1) + i] = a.strides[i];
        }
    }
    for i in 0..r2 {
        if b.shape[i] > 1 {
            eff2[(max_r - r2) + i] = b.strides[i];
        }
    }

    let mut out = Vec::with_capacity(total);
    for idx in 0..total {
        let mut rem = idx;
        let mut i1 = 0;
        let mut i2 = 0;
        for d in 0..max_r {
            let coord = rem / out_strides[d];
            rem %= out_strides[d];
            i1 += coord * eff1[d];
            i2 += coord * eff2[d];
        }
        out.push(f(a.data[i1], b.data[i2]));
    }
    TensorData::new(out_shape, out)
}

pub fn backward_from(loss_id: u64) -> Result<(), String> {
    let mut store = nodes_store().lock().unwrap();
    let loss_node = store
        .get(&loss_id)
        .cloned()
        .ok_or_else(|| format!("NN loss node {} not found", loss_id))?;

    // Iterative post-order topological sort
    let mut topo = Vec::new();
    let mut visited = HashSet::new();
    let mut stack = vec![(loss_id, false)];

    while let Some((nid, is_exit)) = stack.pop() {
        if is_exit {
            topo.push(nid);
            continue;
        }
        if !visited.insert(nid) {
            continue;
        }
        stack.push((nid, true));
        if let Some(node) = store.get(&nid) {
            for p in node.op.parents().into_iter().rev() {
                if !visited.contains(&p) {
                    stack.push((p, false));
                }
            }
        }
    }

    // Seed gradient = ones_like(loss)
    let seed = TensorData::ones(loss_node.data.shape.clone());
    if let Some(root) = store.get_mut(&loss_id) {
        root.grad = Some(seed);
    }

    // Reverse topological pass
    for nid in topo.into_iter().rev() {
        let Some(node) = store.get(&nid).cloned() else {
            continue;
        };
        let Some(g) = node.grad.clone() else {
            continue;
        };

        match node.op {
            TapeOp::Leaf => {}
            TapeOp::Add(a_id, b_id) => {
                accumulate_grad(&mut store, a_id, g.clone())?;
                accumulate_grad(&mut store, b_id, g)?;
            }
            TapeOp::Sub(a_id, b_id) => {
                accumulate_grad(&mut store, a_id, g.clone())?;
                accumulate_grad(&mut store, b_id, g.map_unary(|x| -x))?;
            }
            TapeOp::Mul(a_id, b_id) => {
                let a = store.get(&a_id).map(|v| v.data.clone());
                let b = store.get(&b_id).map(|v| v.data.clone());
                if let (Some(ad), Some(bd)) = (a, b) {
                    let ga = broadcast_binary(&g, &bd, |x, y| x * y)?;
                    let gb = broadcast_binary(&g, &ad, |x, y| x * y)?;
                    accumulate_grad(&mut store, a_id, ga)?;
                    accumulate_grad(&mut store, b_id, gb)?;
                }
            }
            TapeOp::Div(a_id, b_id) => {
                let a = store.get(&a_id).map(|v| v.data.clone());
                let b = store.get(&b_id).map(|v| v.data.clone());
                if let (Some(ad), Some(bd)) = (a, b) {
                    let ga = broadcast_binary(&g, &bd, |x, y| x / y)?;
                    let num = broadcast_binary(&g, &ad, |gx, ax| -gx * ax)?;
                    let gb = broadcast_binary(&num, &bd, |nx, bx| nx / (bx * bx))?;
                    accumulate_grad(&mut store, a_id, ga)?;
                    accumulate_grad(&mut store, b_id, gb)?;
                }
            }
            TapeOp::Matmul(a_id, b_id) => {
                let a = store.get(&a_id).map(|v| v.data.clone());
                let b = store.get(&b_id).map(|v| v.data.clone());
                if let (Some(ad), Some(bd)) = (a, b) {
                    let ga = nn_matmul(&g, &bd.transpose()?)?;
                    let gb = nn_matmul(&ad.transpose()?, &g)?;
                    accumulate_grad(&mut store, a_id, ga)?;
                    accumulate_grad(&mut store, b_id, gb)?;
                }
            }
            TapeOp::Transpose(a_id) => {
                accumulate_grad(&mut store, a_id, g.transpose()?)?;
            }
            TapeOp::Reshape(a_id, orig_shape) => {
                accumulate_grad(&mut store, a_id, g.reshape(orig_shape)?)?;
            }
            TapeOp::Relu(a_id) => {
                if let Some(a) = store.get(&a_id).map(|v| v.data.clone()) {
                    let grad_a =
                        broadcast_binary(&g, &a, |gx, ax| if ax > 0.0 { gx } else { 0.0 })?;
                    accumulate_grad(&mut store, a_id, grad_a)?;
                }
            }
            TapeOp::Sigmoid(a_id) => {
                let y = &node.data;
                let grad_a = broadcast_binary(&g, y, |gx, yx| gx * yx * (1.0 - yx))?;
                accumulate_grad(&mut store, a_id, grad_a)?;
            }
            TapeOp::Tanh(a_id) => {
                let y = &node.data;
                let grad_a = broadcast_binary(&g, y, |gx, yx| gx * (1.0 - yx * yx))?;
                accumulate_grad(&mut store, a_id, grad_a)?;
            }
            TapeOp::Gelu(a_id) => {
                if let Some(a) = store.get(&a_id).map(|v| v.data.clone()) {
                    let sqrt_2_pi = (2.0 / std::f64::consts::PI).sqrt();
                    let grad_a = broadcast_binary(&g, &a, |gx, x| {
                        let inner = sqrt_2_pi * (x + 0.044715 * x * x * x);
                        let t = inner.tanh();
                        let sech2 = 1.0 - t * t;
                        let d_inner = sqrt_2_pi * (1.0 + 3.0 * 0.044715 * x * x);
                        let deriv = 0.5 * (1.0 + t) + 0.5 * x * sech2 * d_inner;
                        gx * deriv
                    })?;
                    accumulate_grad(&mut store, a_id, grad_a)?;
                }
            }
            TapeOp::Silu(a_id) => {
                if let Some(a) = store.get(&a_id).map(|v| v.data.clone()) {
                    let grad_a = broadcast_binary(&g, &a, |gx, x| {
                        let sig = 1.0 / (1.0 + (-x).exp());
                        let deriv = sig * (1.0 + x * (1.0 - sig));
                        gx * deriv
                    })?;
                    accumulate_grad(&mut store, a_id, grad_a)?;
                }
            }
            TapeOp::Sum(a_id) => {
                if let Some(a) = store.get(&a_id) {
                    let s = g.data.first().copied().unwrap_or(1.0);
                    let grad_a = TensorData::new(a.data.shape.clone(), vec![s; a.data.data.len()])?;
                    accumulate_grad(&mut store, a_id, grad_a)?;
                }
            }
            TapeOp::Mean(a_id) => {
                if let Some(a) = store.get(&a_id) {
                    let n = a.data.data.len().max(1) as f64;
                    let s = g.data.first().copied().unwrap_or(1.0) / n;
                    let grad_a = TensorData::new(a.data.shape.clone(), vec![s; a.data.data.len()])?;
                    accumulate_grad(&mut store, a_id, grad_a)?;
                }
            }
            TapeOp::MseLoss { pred, target } => {
                let p = store.get(&pred).map(|v| v.data.clone());
                let t = store.get(&target).map(|v| v.data.clone());
                if let (Some(pd), Some(td)) = (p, t) {
                    let n = pd.data.len().max(1) as f64;
                    let scale = 2.0 * g.data.first().copied().unwrap_or(1.0) / n;
                    let gp = broadcast_binary(&pd, &td, |px, tx| scale * (px - tx))?;
                    let gt = gp.map_unary(|x| -x);
                    accumulate_grad(&mut store, pred, gp)?;
                    accumulate_grad(&mut store, target, gt)?;
                }
            }
            TapeOp::CrossEntropyLoss {
                logits,
                targets,
                probs,
                num_classes,
            } => {
                let rows = targets.len();
                let scale = g.data.first().copied().unwrap_or(1.0) / (rows.max(1) as f64);
                let mut dz = probs;
                for (r, &cls) in targets.iter().enumerate() {
                    let offset = r * num_classes;
                    if cls < num_classes {
                        dz[offset + cls] -= 1.0;
                    }
                }
                for val in &mut dz {
                    *val *= scale;
                }
                let grad_logits = TensorData::new(vec![rows, num_classes], dz)?;
                accumulate_grad(&mut store, logits, grad_logits)?;
            }
            TapeOp::LayerNorm {
                x,
                gamma,
                beta,
                x_hat,
                inv_std,
                ..
            } => {
                let x_shape = store.get(&x).map(|v| v.data.shape.clone());
                let gamma_data = store.get(&gamma).map(|v| v.data.clone());
                if let (Some(shape), Some(gd)) = (x_shape, gamma_data) {
                    if shape.len() == 2 {
                        let (rows, cols) = (shape[0], shape[1]);
                        let mut dgamma = vec![0.0; cols];
                        let mut dbeta = vec![0.0; cols];
                        let mut dx = vec![0.0; rows * cols];
                        let cols_f = cols as f64;

                        for (r, &istd) in inv_std.iter().enumerate().take(rows) {
                            let off = r * cols;
                            let mut sum_dg = 0.0;
                            let mut sum_dg_xhat = 0.0;
                            for c in 0..cols {
                                let dy = g.data[off + c];
                                let xh = x_hat[off + c];
                                dbeta[c] += dy;
                                dgamma[c] += dy * xh;
                                let dg_i = dy * gd.data[c];
                                sum_dg += dg_i;
                                sum_dg_xhat += dg_i * xh;
                            }
                            let scale = istd / cols_f;
                            for c in 0..cols {
                                let dg_i = g.data[off + c] * gd.data[c];
                                dx[off + c] =
                                    scale * (cols_f * dg_i - sum_dg - x_hat[off + c] * sum_dg_xhat);
                            }
                        }

                        accumulate_grad(&mut store, x, TensorData::new(vec![rows, cols], dx)?)?;
                        accumulate_grad(
                            &mut store,
                            gamma,
                            TensorData::new(vec![1, cols], dgamma)?,
                        )?;
                        accumulate_grad(&mut store, beta, TensorData::new(vec![1, cols], dbeta)?)?;
                    }
                }
            }
            TapeOp::Embedding { table, indices } => {
                if let Some(tbl) = store.get(&table) {
                    let (vocab, dim) = (tbl.data.shape[0], tbl.data.shape[1]);
                    let mut grad_tbl = vec![0.0; vocab * dim];
                    for (pos, &tok_id) in indices.iter().enumerate() {
                        if tok_id < vocab {
                            let dst = tok_id * dim;
                            let src = pos * dim;
                            for d in 0..dim {
                                grad_tbl[dst + d] += g.data[src + d];
                            }
                        }
                    }
                    accumulate_grad(
                        &mut store,
                        table,
                        TensorData::new(vec![vocab, dim], grad_tbl)?,
                    )?;
                }
            }
            TapeOp::Attention {
                q,
                k,
                v,
                probs,
                seq_q,
                seq_k,
                d_k,
                causal,
            } => {
                let q_data = store.get(&q).map(|n| n.data.clone());
                let k_data = store.get(&k).map(|n| n.data.clone());
                let v_data = store.get(&v).map(|n| n.data.clone());
                if let (Some(qd), Some(kd), Some(vd)) = (q_data, k_data, v_data) {
                    let p_mat = TensorData::new(vec![seq_q, seq_k], probs)?;
                    let dv = p_mat.transpose()?.matmul(&g)?;
                    let dp = g.matmul(&vd.transpose()?)?;

                    let mut ds = vec![0.0; seq_q * seq_k];
                    for i in 0..seq_q {
                        let off = i * seq_k;
                        let mut dot = 0.0;
                        for j in 0..seq_k {
                            dot += dp.data[off + j] * p_mat.data[off + j];
                        }
                        for j in 0..seq_k {
                            if causal && j > i {
                                ds[off + j] = 0.0;
                            } else {
                                ds[off + j] = p_mat.data[off + j] * (dp.data[off + j] - dot);
                            }
                        }
                    }
                    let scale = 1.0 / (d_k as f64).sqrt();
                    let ds_mat = TensorData::new(vec![seq_q, seq_k], ds)?.map_unary(|x| x * scale);
                    let dq = ds_mat.matmul(&kd)?;
                    let dk = ds_mat.transpose()?.matmul(&qd)?;

                    accumulate_grad(&mut store, q, dq)?;
                    accumulate_grad(&mut store, k, dk)?;
                    accumulate_grad(&mut store, v, dv)?;
                }
            }
        }
    }

    Ok(())
}

// ==============================================================================
// 3. True AdamW Optimizer
// ==============================================================================

#[derive(Clone, Debug)]
struct AdamWState {
    param_ids: Vec<u64>,
    lr: f64,
    beta1: f64,
    beta2: f64,
    eps: f64,
    weight_decay: f64,
    step: u64,
    m: Vec<Vec<f64>>,
    v: Vec<Vec<f64>>,
}

impl AdamWState {
    fn new(
        param_ids: Vec<u64>,
        lr: f64,
        beta1: f64,
        beta2: f64,
        eps: f64,
        weight_decay: f64,
    ) -> Result<Self, String> {
        let store = nodes_store().lock().unwrap();
        let mut m = Vec::with_capacity(param_ids.len());
        let mut v = Vec::with_capacity(param_ids.len());
        for &pid in &param_ids {
            let p = store
                .get(&pid)
                .ok_or_else(|| format!("AdamW parameter {} not found", pid))?;
            let len = p.data.data.len();
            m.push(vec![0.0; len]);
            v.push(vec![0.0; len]);
        }
        Ok(Self {
            param_ids,
            lr,
            beta1,
            beta2,
            eps,
            weight_decay,
            step: 0,
            m,
            v,
        })
    }

    fn step_update(&mut self) -> Result<(), String> {
        self.step += 1;
        let t = self.step as i32;
        let bc1 = 1.0 - self.beta1.powi(t);
        let bc2 = 1.0 - self.beta2.powi(t);

        let mut store = nodes_store().lock().unwrap();
        for (idx, &pid) in self.param_ids.iter().enumerate() {
            let Some(param) = store.get_mut(&pid) else {
                continue;
            };
            let Some(grad) = &param.grad else {
                continue;
            };
            let m_buf = &mut self.m[idx];
            let v_buf = &mut self.v[idx];

            for ((w, &g), (mi, vi)) in param
                .data
                .data
                .iter_mut()
                .zip(grad.data.iter())
                .zip(m_buf.iter_mut().zip(v_buf.iter_mut()))
            {
                // Decoupled weight decay
                if self.weight_decay > 0.0 {
                    *w *= 1.0 - self.lr * self.weight_decay;
                }
                *mi = self.beta1 * (*mi) + (1.0 - self.beta1) * g;
                *vi = self.beta2 * (*vi) + (1.0 - self.beta2) * g * g;
                let m_hat = *mi / bc1;
                let v_hat = *vi / bc2;
                *w -= self.lr * m_hat / (v_hat.sqrt() + self.eps);
            }
        }
        Ok(())
    }

    fn zero_grad(&self) {
        let mut store = nodes_store().lock().unwrap();
        for &pid in &self.param_ids {
            if let Some(p) = store.get_mut(&pid) {
                p.grad = None;
            }
        }
    }
}

// ==============================================================================
// 4. Safetensors Binary Format Serializer & Deserializer (Zero External Crates)
// ==============================================================================

#[derive(Serialize, Deserialize)]
struct SafeTensorEntry {
    dtype: String,
    shape: Vec<usize>,
    data_offsets: [usize; 2],
}

fn f16_to_f64(bits: u16) -> f64 {
    let sign = ((bits >> 15) & 1) as u32;
    let exp = ((bits >> 10) & 0x1F) as u32;
    let mant = (bits & 0x3FF) as u32;
    let f32_bits = if exp == 0 {
        if mant == 0 {
            sign << 31
        } else {
            // Normalize subnormal f16
            let mut m = mant;
            let mut e = 127i32 - 14;
            while (m & 0x400) == 0 {
                m <<= 1;
                e -= 1;
            }
            let frac = (m & 0x3FF) << 13;
            (sign << 31) | ((e as u32) << 23) | frac
        }
    } else if exp == 31 {
        (sign << 31) | (0xFF << 23) | (mant << 13)
    } else {
        (sign << 31) | ((exp + 112) << 23) | (mant << 13)
    };
    f32::from_bits(f32_bits) as f64
}

fn bf16_to_f64(bits: u16) -> f64 {
    f32::from_bits((bits as u32) << 16) as f64
}

pub fn save_safetensors_file(
    tensors: &HashMap<String, TensorData>,
    path: &str,
) -> Result<(), String> {
    let mut keys: Vec<&String> = tensors.keys().collect();
    keys.sort();

    let mut header_map = serde_json::Map::new();
    let mut raw_bytes = Vec::new();

    for key in keys {
        let t = &tensors[key];
        let start = raw_bytes.len();
        for &val in &t.data {
            raw_bytes.extend_from_slice(&val.to_le_bytes());
        }
        let end = raw_bytes.len();
        let entry = SafeTensorEntry {
            dtype: "F64".to_string(),
            shape: t.shape.clone(),
            data_offsets: [start, end],
        };
        header_map.insert(
            key.clone(),
            serde_json::to_value(entry).map_err(|e| format!("Safetensors header error: {}", e))?,
        );
    }

    let mut header_bytes = serde_json::to_vec(&serde_json::Value::Object(header_map))
        .map_err(|e| format!("Safetensors JSON error: {}", e))?;
    while header_bytes.len() % 8 != 0 {
        header_bytes.push(b' ');
    }

    let header_len = header_bytes.len() as u64;
    let mut file_bytes = Vec::with_capacity(8 + header_bytes.len() + raw_bytes.len());
    file_bytes.extend_from_slice(&header_len.to_le_bytes());
    file_bytes.extend_from_slice(&header_bytes);
    file_bytes.extend_from_slice(&raw_bytes);

    fs::write(path, file_bytes)
        .map_err(|e| format!("Failed to write safetensors '{}': {}", path, e))
}

pub fn load_safetensors_file(path: &str) -> Result<HashMap<String, TensorData>, String> {
    let bytes =
        fs::read(path).map_err(|e| format!("Failed to read safetensors '{}': {}", path, e))?;
    if bytes.len() < 8 {
        return Err("Invalid safetensors file: shorter than 8-byte header length".into());
    }
    let header_len = u64::from_le_bytes(bytes[..8].try_into().unwrap()) as usize;
    let payload_start = 8usize
        .checked_add(header_len)
        .ok_or_else(|| "Safetensors header length overflow".to_string())?;
    if payload_start > bytes.len() {
        return Err("Safetensors header exceeds file bounds".into());
    }

    let header_str = std::str::from_utf8(&bytes[8..payload_start])
        .map_err(|e| format!("Invalid UTF-8 safetensors header: {}", e))?;
    let header_json: serde_json::Value = serde_json::from_str(header_str)
        .map_err(|e| format!("Malformed safetensors JSON header: {}", e))?;
    let obj = header_json
        .as_object()
        .ok_or_else(|| "Safetensors header must be JSON object".to_string())?;

    let payload = &bytes[payload_start..];
    let mut result = HashMap::new();

    for (name, val) in obj {
        if name == "__metadata__" {
            continue;
        }
        let entry: SafeTensorEntry = serde_json::from_value(val.clone())
            .map_err(|e| format!("Invalid tensor entry '{}': {}", name, e))?;
        let [start, end] = entry.data_offsets;
        if end < start || end > payload.len() {
            return Err(format!("Tensor '{}' offsets out of file bounds", name));
        }
        let slice = &payload[start..end];
        let mut floats = Vec::new();
        match entry.dtype.as_str() {
            "F64" => {
                for ch in slice.chunks_exact(8) {
                    floats.push(f64::from_le_bytes(ch.try_into().unwrap()));
                }
            }
            "F32" => {
                for ch in slice.chunks_exact(4) {
                    floats.push(f32::from_le_bytes(ch.try_into().unwrap()) as f64);
                }
            }
            "F16" => {
                for ch in slice.chunks_exact(2) {
                    floats.push(f16_to_f64(u16::from_le_bytes(ch.try_into().unwrap())));
                }
            }
            "BF16" => {
                for ch in slice.chunks_exact(2) {
                    floats.push(bf16_to_f64(u16::from_le_bytes(ch.try_into().unwrap())));
                }
            }
            "I64" => {
                for ch in slice.chunks_exact(8) {
                    floats.push(i64::from_le_bytes(ch.try_into().unwrap()) as f64);
                }
            }
            "I32" => {
                for ch in slice.chunks_exact(4) {
                    floats.push(i32::from_le_bytes(ch.try_into().unwrap()) as f64);
                }
            }
            other => {
                return Err(format!(
                    "Unsupported safetensors dtype '{}' for tensor '{}'",
                    other, name
                ))
            }
        }
        let shape = if entry.shape.is_empty() {
            vec![floats.len()]
        } else {
            entry.shape
        };
        result.insert(name.clone(), TensorData::new(shape, floats)?);
    }

    Ok(result)
}

// ==============================================================================
// 5. Native Module Registration (`NN` & `__native_nn`)
// ==============================================================================

fn arg_u64(val: &Value, name: &str) -> Result<u64, String> {
    match val {
        Value::Int(i) if *i > 0 => Ok(*i as u64),
        _ => Err(format!("{} must be a positive node ID integer", name)),
    }
}

fn arg_f64(val: &Value, name: &str) -> Result<f64, String> {
    match val {
        Value::Float(f) => Ok(*f),
        Value::Int(i) => Ok(*i as f64),
        _ => Err(format!("{} must be numeric", name)),
    }
}

fn arg_usize_vec(val: &Value, name: &str) -> Result<Vec<usize>, String> {
    match val {
        Value::Array(arr) => {
            let items = arr.lock();
            let mut out = Vec::with_capacity(items.len());
            for v in items.iter() {
                match v {
                    Value::Int(i) if *i >= 0 => out.push(*i as usize),
                    _ => return Err(format!("{} elements must be non-negative integers", name)),
                }
            }
            Ok(out)
        }
        _ => Err(format!("{} must be an array of integers", name)),
    }
}

fn unary_tape_op(
    args: &[Value],
    fn_name: &str,
    forward: fn(&TensorData) -> TensorData,
    make_op: fn(u64) -> TapeOp,
) -> Result<Value, String> {
    if args.is_empty() {
        return Err(format!("{}(node_id) requires node ID", fn_name));
    }
    let a_id = arg_u64(&args[0], "node_id")?;
    let a = get_var_clone(a_id)?;
    let out = forward(&a.data);
    let id = alloc_var(out, a.requires_grad, make_op(a_id));
    Ok(Value::Int(id as i64))
}

fn binary_tape_op(
    args: &[Value],
    fn_name: &str,
    elem_op: fn(f64, f64) -> f64,
    make_op: fn(u64, u64) -> TapeOp,
) -> Result<Value, String> {
    if args.len() < 2 {
        return Err(format!("{}(a_id, b_id) requires 2 node IDs", fn_name));
    }
    let a_id = arg_u64(&args[0], "a_id")?;
    let b_id = arg_u64(&args[1], "b_id")?;
    let a = get_var_clone(a_id)?;
    let b = get_var_clone(b_id)?;
    let out = broadcast_binary(&a.data, &b.data, elem_op)?;
    let id = alloc_var(out, a.requires_grad || b.requires_grad, make_op(a_id, b_id));
    Ok(Value::Int(id as i64))
}

pub fn register_nn_module(globals: &mut HashMap<String, Value>) {
    let mut nn_mod = HashMap::new();

    // 1. Variable / Parameter creation
    nn_mod.insert(
        "var".to_string(),
        Value::Native("NN.var".into(), |args| {
            if args.is_empty() {
                return Err("NN.var(tensor_or_array, [requires_grad=true]) requires data".into());
            }
            let data = get_tensor_data(&args[0])?;
            let req = if args.len() > 1 {
                matches!(args[1], Value::Bool(true))
            } else {
                true
            };
            let id = alloc_var(data, req, TapeOp::Leaf);
            Ok(Value::Int(id as i64))
        }),
    );

    nn_mod.insert(
        "data".to_string(),
        Value::Native("NN.data".into(), |args| {
            if args.is_empty() {
                return Err("NN.data(node_id) requires node ID".into());
            }
            let id = arg_u64(&args[0], "node_id")?;
            Ok(wrap_tensor_handle(get_var_clone(id)?.data))
        }),
    );

    nn_mod.insert(
        "grad".to_string(),
        Value::Native("NN.grad".into(), |args| {
            if args.is_empty() {
                return Err("NN.grad(node_id) requires node ID".into());
            }
            let id = arg_u64(&args[0], "node_id")?;
            let var = get_var_clone(id)?;
            match var.grad {
                Some(g) => Ok(wrap_tensor_handle(g)),
                None => Ok(Value::Nil),
            }
        }),
    );

    nn_mod.insert(
        "set_data".to_string(),
        Value::Native("NN.set_data".into(), |args| {
            if args.len() < 2 {
                return Err("NN.set_data(node_id, tensor_or_array) requires 2 arguments".into());
            }
            let id = arg_u64(&args[0], "node_id")?;
            let new_t = get_tensor_data(&args[1])?;
            let mut store = nodes_store().lock().unwrap();
            let node = store
                .get_mut(&id)
                .ok_or_else(|| format!("NN node handle {} not found", id))?;
            node.data = new_t;
            Ok(Value::Bool(true))
        }),
    );

    // 2. Differentiable Arithmetic & Linear Algebra
    nn_mod.insert(
        "add".to_string(),
        Value::Native("NN.add".into(), |a| {
            binary_tape_op(a, "NN.add", |x, y| x + y, TapeOp::Add)
        }),
    );
    nn_mod.insert(
        "sub".to_string(),
        Value::Native("NN.sub".into(), |a| {
            binary_tape_op(a, "NN.sub", |x, y| x - y, TapeOp::Sub)
        }),
    );
    nn_mod.insert(
        "mul".to_string(),
        Value::Native("NN.mul".into(), |a| {
            binary_tape_op(a, "NN.mul", |x, y| x * y, TapeOp::Mul)
        }),
    );
    nn_mod.insert(
        "div".to_string(),
        Value::Native("NN.div".into(), |a| {
            binary_tape_op(a, "NN.div", |x, y| x / y, TapeOp::Div)
        }),
    );

    nn_mod.insert(
        "matmul".to_string(),
        Value::Native("NN.matmul".into(), |args| {
            if args.len() < 2 {
                return Err("NN.matmul(a_id, b_id) requires 2 node IDs".into());
            }
            let a_id = arg_u64(&args[0], "a_id")?;
            let b_id = arg_u64(&args[1], "b_id")?;
            let a = get_var_clone(a_id)?;
            let b = get_var_clone(b_id)?;
            let out = nn_matmul(&a.data, &b.data)?;
            let id = alloc_var(
                out,
                a.requires_grad || b.requires_grad,
                TapeOp::Matmul(a_id, b_id),
            );
            Ok(Value::Int(id as i64))
        }),
    );

    nn_mod.insert(
        "transpose".to_string(),
        Value::Native("NN.transpose".into(), |args| {
            if args.is_empty() {
                return Err("NN.transpose(node_id) requires node ID".into());
            }
            let a_id = arg_u64(&args[0], "node_id")?;
            let a = get_var_clone(a_id)?;
            let out = a.data.transpose()?;
            let id = alloc_var(out, a.requires_grad, TapeOp::Transpose(a_id));
            Ok(Value::Int(id as i64))
        }),
    );

    nn_mod.insert(
        "reshape".to_string(),
        Value::Native("NN.reshape".into(), |args| {
            if args.len() < 2 {
                return Err("NN.reshape(node_id, new_shape) requires 2 arguments".into());
            }
            let a_id = arg_u64(&args[0], "node_id")?;
            let new_shape = arg_usize_vec(&args[1], "new_shape")?;
            let a = get_var_clone(a_id)?;
            let orig_shape = a.data.shape.clone();
            let out = a.data.reshape(new_shape)?;
            let id = alloc_var(out, a.requires_grad, TapeOp::Reshape(a_id, orig_shape));
            Ok(Value::Int(id as i64))
        }),
    );

    // 3. Differentiable Activations & Reductions
    nn_mod.insert(
        "relu".to_string(),
        Value::Native("NN.relu".into(), |a| {
            unary_tape_op(a, "NN.relu", TensorData::relu, TapeOp::Relu)
        }),
    );
    nn_mod.insert(
        "sigmoid".to_string(),
        Value::Native("NN.sigmoid".into(), |a| {
            unary_tape_op(a, "NN.sigmoid", TensorData::sigmoid, TapeOp::Sigmoid)
        }),
    );
    nn_mod.insert(
        "tanh".to_string(),
        Value::Native("NN.tanh".into(), |a| {
            unary_tape_op(a, "NN.tanh", TensorData::tanh, TapeOp::Tanh)
        }),
    );
    nn_mod.insert(
        "gelu".to_string(),
        Value::Native("NN.gelu".into(), |a| {
            unary_tape_op(a, "NN.gelu", TensorData::gelu, TapeOp::Gelu)
        }),
    );
    nn_mod.insert(
        "silu".to_string(),
        Value::Native("NN.silu".into(), |a| {
            unary_tape_op(a, "NN.silu", TensorData::silu, TapeOp::Silu)
        }),
    );

    nn_mod.insert(
        "sum".to_string(),
        Value::Native("NN.sum".into(), |args| {
            if args.is_empty() {
                return Err("NN.sum(node_id) requires node ID".into());
            }
            let a_id = arg_u64(&args[0], "node_id")?;
            let a = get_var_clone(a_id)?;
            let out = TensorData::new(vec![1], vec![a.data.sum()])?;
            let id = alloc_var(out, a.requires_grad, TapeOp::Sum(a_id));
            Ok(Value::Int(id as i64))
        }),
    );

    nn_mod.insert(
        "mean".to_string(),
        Value::Native("NN.mean".into(), |args| {
            if args.is_empty() {
                return Err("NN.mean(node_id) requires node ID".into());
            }
            let a_id = arg_u64(&args[0], "node_id")?;
            let a = get_var_clone(a_id)?;
            let out = TensorData::new(vec![1], vec![a.data.mean()])?;
            let id = alloc_var(out, a.requires_grad, TapeOp::Mean(a_id));
            Ok(Value::Int(id as i64))
        }),
    );

    // 4. Loss Functions (MSE + Numerically Stable Cross-Entropy)
    nn_mod.insert(
        "mse_loss".to_string(),
        Value::Native("NN.mse_loss".into(), |args| {
            if args.len() < 2 {
                return Err("NN.mse_loss(pred_id, target_id) requires 2 node IDs".into());
            }
            let p_id = arg_u64(&args[0], "pred_id")?;
            let t_id = arg_u64(&args[1], "target_id")?;
            let p = get_var_clone(p_id)?;
            let t = get_var_clone(t_id)?;
            let diff = broadcast_binary(&p.data, &t.data, |px, tx| (px - tx) * (px - tx))?;
            let loss_val = diff.mean();
            let out = TensorData::new(vec![1], vec![loss_val])?;
            let id = alloc_var(
                out,
                p.requires_grad || t.requires_grad,
                TapeOp::MseLoss {
                    pred: p_id,
                    target: t_id,
                },
            );
            Ok(Value::Int(id as i64))
        }),
    );

    nn_mod.insert(
        "cross_entropy".to_string(),
        Value::Native("NN.cross_entropy".into(), |args| {
            if args.len() < 2 {
                return Err(
                    "NN.cross_entropy(logits_id, target_indices) requires 2 arguments".into(),
                );
            }
            let logits_id = arg_u64(&args[0], "logits_id")?;
            let targets = arg_usize_vec(&args[1], "target_indices")?;
            let logits = get_var_clone(logits_id)?;
            if logits.data.shape.len() != 2 {
                return Err(
                    "NN.cross_entropy requires 2D logits tensor [batch, num_classes]".into(),
                );
            }
            let (rows, num_classes) = (logits.data.shape[0], logits.data.shape[1]);
            if targets.len() != rows {
                return Err(format!(
                    "cross_entropy batch mismatch: {} rows vs {} targets",
                    rows,
                    targets.len()
                ));
            }
            let mut probs = vec![0.0; rows * num_classes];
            let mut total_loss = 0.0;
            for (r, &cls) in targets.iter().enumerate() {
                if cls >= num_classes {
                    return Err(format!(
                        "Target class {} out of range [0, {})",
                        cls, num_classes
                    ));
                }
                let off = r * num_classes;
                let max_l = logits.data.data[off..off + num_classes]
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max);
                let mut sum_e = 0.0;
                for c in 0..num_classes {
                    let e = (logits.data.data[off + c] - max_l).exp();
                    probs[off + c] = e;
                    sum_e += e;
                }
                for c in 0..num_classes {
                    probs[off + c] /= sum_e;
                }
                total_loss -= probs[off + cls].max(1e-15).ln();
            }
            let mean_loss = total_loss / (rows.max(1) as f64);
            let out = TensorData::new(vec![1], vec![mean_loss])?;
            let id = alloc_var(
                out,
                logits.requires_grad,
                TapeOp::CrossEntropyLoss {
                    logits: logits_id,
                    targets,
                    probs,
                    num_classes,
                },
            );
            Ok(Value::Int(id as i64))
        }),
    );

    // 5. Transformer / Deep Neural Layers: LayerNorm, Embedding, Self-Attention
    nn_mod.insert(
        "layer_norm".to_string(),
        Value::Native("NN.layer_norm".into(), |args| {
            if args.len() < 3 {
                return Err(
                    "NN.layer_norm(x_id, gamma_id, beta_id, [eps=1e-5]) requires 3+ args".into(),
                );
            }
            let x_id = arg_u64(&args[0], "x_id")?;
            let gamma_id = arg_u64(&args[1], "gamma_id")?;
            let beta_id = arg_u64(&args[2], "beta_id")?;
            let eps = if args.len() > 3 && args[3] != Value::Nil {
                arg_f64(&args[3], "eps")?
            } else {
                1e-5
            };

            let x = get_var_clone(x_id)?;
            let gamma = get_var_clone(gamma_id)?;
            let beta = get_var_clone(beta_id)?;
            if x.data.shape.len() != 2 {
                return Err("NN.layer_norm requires 2D input [rows, dim]".into());
            }
            let (rows, cols) = (x.data.shape[0], x.data.shape[1]);
            if gamma.data.data.len() != cols || beta.data.data.len() != cols {
                return Err(format!(
                    "NN.layer_norm gamma/beta length must match feature dim {}",
                    cols
                ));
            }
            let mut x_hat = vec![0.0; rows * cols];
            let mut inv_std = vec![0.0; rows];
            let mut out = vec![0.0; rows * cols];

            for (r, istd_slot) in inv_std.iter_mut().enumerate().take(rows) {
                let off = r * cols;
                let row = &x.data.data[off..off + cols];
                let mean = row.iter().sum::<f64>() / (cols as f64);
                let var = row.iter().map(|&v| (v - mean) * (v - mean)).sum::<f64>() / (cols as f64);
                let istd = 1.0 / (var + eps).sqrt();
                *istd_slot = istd;
                for c in 0..cols {
                    let xh = (row[c] - mean) * istd;
                    x_hat[off + c] = xh;
                    out[off + c] = xh * gamma.data.data[c] + beta.data.data[c];
                }
            }

            let id = alloc_var(
                TensorData::new(vec![rows, cols], out)?,
                x.requires_grad || gamma.requires_grad || beta.requires_grad,
                TapeOp::LayerNorm {
                    x: x_id,
                    gamma: gamma_id,
                    beta: beta_id,
                    eps,
                    x_hat,
                    inv_std,
                },
            );
            Ok(Value::Int(id as i64))
        }),
    );

    nn_mod.insert(
        "embedding".to_string(),
        Value::Native("NN.embedding".into(), |args| {
            if args.len() < 2 {
                return Err("NN.embedding(table_id, token_indices) requires 2 arguments".into());
            }
            let table_id = arg_u64(&args[0], "table_id")?;
            let indices = arg_usize_vec(&args[1], "token_indices")?;
            let table = get_var_clone(table_id)?;
            if table.data.shape.len() != 2 {
                return Err("NN.embedding table must be 2D [vocab_size, dim]".into());
            }
            let (vocab, dim) = (table.data.shape[0], table.data.shape[1]);
            let mut out = Vec::with_capacity(indices.len() * dim);
            for &idx in &indices {
                if idx >= vocab {
                    return Err(format!("Token index {} >= vocab_size {}", idx, vocab));
                }
                let start = idx * dim;
                out.extend_from_slice(&table.data.data[start..start + dim]);
            }
            let id = alloc_var(
                TensorData::new(vec![indices.len(), dim], out)?,
                table.requires_grad,
                TapeOp::Embedding {
                    table: table_id,
                    indices,
                },
            );
            Ok(Value::Int(id as i64))
        }),
    );

    nn_mod.insert(
        "attention".to_string(),
        Value::Native("NN.attention".into(), |args| {
            if args.len() < 3 {
                return Err(
                    "NN.attention(q_id, k_id, v_id, [causal=false]) requires 3+ args".into(),
                );
            }
            let q_id = arg_u64(&args[0], "q_id")?;
            let k_id = arg_u64(&args[1], "k_id")?;
            let v_id = arg_u64(&args[2], "v_id")?;
            let causal = args.len() > 3 && matches!(args[3], Value::Bool(true));

            let q = get_var_clone(q_id)?;
            let k = get_var_clone(k_id)?;
            let v = get_var_clone(v_id)?;
            if q.data.shape.len() != 2 || k.data.shape.len() != 2 || v.data.shape.len() != 2 {
                return Err("NN.attention requires 2D Q, K, V matrices".into());
            }
            let (seq_q, d_k) = (q.data.shape[0], q.data.shape[1]);
            let (seq_k, dk2) = (k.data.shape[0], k.data.shape[1]);
            if d_k != dk2 || v.data.shape[0] != seq_k {
                return Err("Q/K/V dimension mismatch in NN.attention".into());
            }

            let scale = 1.0 / (d_k as f64).sqrt();
            let mut scores = q
                .data
                .matmul(&k.data.transpose()?)?
                .map_unary(|x| x * scale);
            if causal {
                for i in 0..seq_q {
                    for j in (i + 1)..seq_k {
                        scores.data[i * seq_k + j] = -1e9;
                    }
                }
            }
            let probs = scores.softmax()?;
            let out = probs.matmul(&v.data)?;
            let id = alloc_var(
                out,
                q.requires_grad || k.requires_grad || v.requires_grad,
                TapeOp::Attention {
                    q: q_id,
                    k: k_id,
                    v: v_id,
                    probs: probs.data,
                    seq_q,
                    seq_k,
                    d_k,
                    causal,
                },
            );
            Ok(Value::Int(id as i64))
        }),
    );

    // 6. Autograd Execution & Cleanup
    nn_mod.insert(
        "backward".to_string(),
        Value::Native("NN.backward".into(), |args| {
            if args.is_empty() {
                return Err("NN.backward(loss_id) requires loss node ID".into());
            }
            let loss_id = arg_u64(&args[0], "loss_id")?;
            backward_from(loss_id)?;
            Ok(Value::Bool(true))
        }),
    );

    nn_mod.insert(
        "zero_grad".to_string(),
        Value::Native("NN.zero_grad".into(), |args| {
            if args.is_empty() {
                return Err("NN.zero_grad(param_ids) requires array of parameter IDs".into());
            }
            let pids = match &args[0] {
                Value::Array(arr) => {
                    let items = arr.lock();
                    let mut out = Vec::with_capacity(items.len());
                    for v in items.iter() {
                        out.push(arg_u64(v, "param_id")?);
                    }
                    out
                }
                _ => vec![arg_u64(&args[0], "param_id")?],
            };
            let mut store = nodes_store().lock().unwrap();
            for pid in pids {
                if let Some(node) = store.get_mut(&pid) {
                    node.grad = None;
                }
            }
            Ok(Value::Bool(true))
        }),
    );

    // 7. True AdamW Optimizer
    nn_mod.insert(
        "adamw_create".to_string(),
        Value::Native("NN.adamw_create".into(), |args| {
            if args.is_empty() {
                return Err("NN.adamw_create(param_ids, [lr=0.001], [beta1=0.9], [beta2=0.999], [eps=1e-8], [weight_decay=0.01]) required".into());
            }
            let pids = match &args[0] {
                Value::Array(arr) => {
                    let items = arr.lock();
                    let mut out = Vec::with_capacity(items.len());
                    for v in items.iter() {
                        out.push(arg_u64(v, "param_id")?);
                    }
                    out
                }
                _ => return Err("param_ids must be an array of parameter node IDs".into()),
            };
            let lr = if args.len() > 1 && args[1] != Value::Nil {
                arg_f64(&args[1], "lr")?
            } else {
                0.001
            };
            let beta1 = if args.len() > 2 && args[2] != Value::Nil {
                arg_f64(&args[2], "beta1")?
            } else {
                0.9
            };
            let beta2 = if args.len() > 3 && args[3] != Value::Nil {
                arg_f64(&args[3], "beta2")?
            } else {
                0.999
            };
            let eps = if args.len() > 4 && args[4] != Value::Nil {
                arg_f64(&args[4], "eps")?
            } else {
                1e-8
            };
            let wd = if args.len() > 5 && args[5] != Value::Nil {
                arg_f64(&args[5], "weight_decay")?
            } else {
                0.01
            };
            let state = AdamWState::new(pids, lr, beta1, beta2, eps, wd)?;
            let id = NEXT_OPT_ID.fetch_add(1, Ordering::SeqCst);
            opt_store().lock().unwrap().insert(id, state);
            Ok(Value::Int(id as i64))
        }),
    );

    nn_mod.insert(
        "adamw_step".to_string(),
        Value::Native("NN.adamw_step".into(), |args| {
            if args.is_empty() {
                return Err("NN.adamw_step(opt_id) requires optimizer ID".into());
            }
            let opt_id = arg_u64(&args[0], "opt_id")?;
            let mut store = opt_store().lock().unwrap();
            let opt = store
                .get_mut(&opt_id)
                .ok_or_else(|| format!("AdamW optimizer {} not found", opt_id))?;
            opt.step_update()?;
            Ok(Value::Bool(true))
        }),
    );

    nn_mod.insert(
        "adamw_zero_grad".to_string(),
        Value::Native("NN.adamw_zero_grad".into(), |args| {
            if args.is_empty() {
                return Err("NN.adamw_zero_grad(opt_id) requires optimizer ID".into());
            }
            let opt_id = arg_u64(&args[0], "opt_id")?;
            let store = opt_store().lock().unwrap();
            let opt = store
                .get(&opt_id)
                .ok_or_else(|| format!("AdamW optimizer {} not found", opt_id))?;
            opt.zero_grad();
            Ok(Value::Bool(true))
        }),
    );

    nn_mod.insert(
        "sgd_step".to_string(),
        Value::Native("NN.sgd_step".into(), |args| {
            if args.len() < 2 {
                return Err("NN.sgd_step(param_ids, lr) requires 2 arguments".into());
            }
            let pids = match &args[0] {
                Value::Array(arr) => {
                    let items = arr.lock();
                    let mut out = Vec::with_capacity(items.len());
                    for v in items.iter() {
                        out.push(arg_u64(v, "param_id")?);
                    }
                    out
                }
                _ => return Err("param_ids must be an array".into()),
            };
            let lr = arg_f64(&args[1], "lr")?;
            let mut store = nodes_store().lock().unwrap();
            for pid in pids {
                if let Some(p) = store.get_mut(&pid) {
                    if let Some(g) = &p.grad {
                        for (w, &gw) in p.data.data.iter_mut().zip(g.data.iter()) {
                            *w -= lr * gw;
                        }
                    }
                }
            }
            Ok(Value::Bool(true))
        }),
    );

    // 8. HuggingFace `.safetensors` Binary Save / Load
    nn_mod.insert(
        "save_safetensors".to_string(),
        Value::Native("NN.save_safetensors".into(), |args| {
            if args.len() < 2 {
                return Err("NN.save_safetensors(tensors_map, path) requires 2 arguments".into());
            }
            let path = format!("{}", args[1]);
            crate::vm::interpreter::with_current_vm(|vm| vm.security_policy.check_fs_write(&path))?;
            let mut tensors = HashMap::new();
            match &args[0] {
                Value::Map(m) => {
                    let entries: Vec<(String, Value)> = m
                        .lock()
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    for (name, val) in entries {
                        let tdata = match val {
                            Value::Int(nid) if nid > 0 => get_var_clone(nid as u64)?.data,
                            other => get_tensor_data(&other)?,
                        };
                        tensors.insert(name, tdata);
                    }
                }
                _ => return Err("tensors_map must be a Map of name -> Tensor or NN node".into()),
            }
            save_safetensors_file(&tensors, &path)?;
            Ok(Value::Bool(true))
        }),
    );

    nn_mod.insert(
        "load_safetensors".to_string(),
        Value::Native("NN.load_safetensors".into(), |args| {
            if args.is_empty() {
                return Err("NN.load_safetensors(path) requires path".into());
            }
            let path = format!("{}", args[0]);
            crate::vm::interpreter::with_current_vm(|vm| vm.security_policy.check_fs_read(&path))?;
            let loaded = load_safetensors_file(&path)?;
            let mut out_map = HashMap::new();
            for (name, tdata) in loaded {
                out_map.insert(name, wrap_tensor_handle(tdata));
            }
            Ok(Value::map(out_map))
        }),
    );

    // 9. Hardware Device Selection (CPU vs Cross-Platform wgpu GPU Training)
    nn_mod.insert(
        "set_device".to_string(),
        Value::Native("NN.set_device".into(), |args| {
            if args.is_empty() {
                return Err("NN.set_device(\"gpu\" | \"cpu\") requires a device string".into());
            }
            let req = format!("{}", args[0]).to_lowercase();
            match req.as_str() {
                "gpu" | "wgpu" | "vulkan" | "dx12" | "metal" => {
                    if super::gpu::is_hardware_gpu_available() {
                        NN_USE_GPU.store(true, Ordering::SeqCst);
                        Ok(Value::string("gpu".to_string()))
                    } else {
                        NN_USE_GPU.store(false, Ordering::SeqCst);
                        Ok(Value::string("cpu".to_string()))
                    }
                }
                "cpu" => {
                    NN_USE_GPU.store(false, Ordering::SeqCst);
                    Ok(Value::string("cpu".to_string()))
                }
                other => Err(format!(
                    "Unknown NN device '{}' (use \"gpu\" or \"cpu\")",
                    other
                )),
            }
        }),
    );

    nn_mod.insert(
        "device".to_string(),
        Value::Native("NN.device".into(), |_args| {
            let active =
                if NN_USE_GPU.load(Ordering::SeqCst) && super::gpu::is_hardware_gpu_available() {
                    "gpu"
                } else {
                    "cpu"
                };
            Ok(Value::string(active.to_string()))
        }),
    );

    globals.insert("NN".to_string(), Value::map(nn_mod.clone()));
    globals.insert("__native_nn".to_string(), Value::map(nn_mod));
}
