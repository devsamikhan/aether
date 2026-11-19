use aether::vm::llm::{
    dequantize_ggml, quantize_ggml, GGUF_TYPE_BF16, GGUF_TYPE_F16, GGUF_TYPE_F32, GGUF_TYPE_Q4_0,
    GGUF_TYPE_Q4_1, GGUF_TYPE_Q5_0, GGUF_TYPE_Q8_0,
};
use aether::vm::{run_source, run_source_sandboxed, SecurityPolicy};
use tempfile::TempDir;

#[test]
fn test_bpe_tokenizer_train_roundtrip_and_json_persistence() {
    let tmp = TempDir::new().expect("tempdir");
    let tok_path = tmp
        .path()
        .join("aether_bpe.json")
        .to_string_lossy()
        .replace('\\', "/");

    let code = format!(
        r#"
from aether_llm import BPETokenizer

let corpus = [
    "aether language builds zero python edge ai pipelines",
    "aether language compiles fast native rust transformers",
    "zero python machine learning and local llm inference in aether",
    "edge ai pipelines run fast transformers without python"
]

let tok = BPETokenizer.train(corpus, 310, ["<|bos|>", "<|eos|>", "<|unk|>"])
let vs = tok.vocab_size()
if vs <= 260 or vs > 310:
    raise "Unexpected vocab size: " + str(vs)

let sample = "aether language runs edge ai pipelines!"
let ids = tok.encode(sample, false)
let decoded = tok.decode(ids, true)
if decoded != sample:
    raise "Roundtrip decode mismatch: " + decoded

# Verify BPE compression learned subword merges (fewer tokens than raw bytes)
if len(ids) >= len(sample):
    raise "Expected BPE merges to compress token count, got " + str(len(ids))

# Special token handling
let with_sp = tok.encode("<|bos|>aether<|eos|>", false)
let dec_skip = tok.decode(with_sp, true)
if dec_skip != "aether":
    raise "Special token skip decode mismatch: " + dec_skip

# JSON persistence & HuggingFace format reload
tok.save("{tok_path}")
let tok2 = BPETokenizer.load("{tok_path}")
let ids2 = tok2.encode(sample, false)
if len(ids2) != len(ids):
    raise "Reloaded tokenizer length mismatch"
if tok2.decode(ids2, true) != sample:
    raise "Reloaded tokenizer decode mismatch"
"ok"
"#,
        tok_path = tok_path
    );

    let res = run_source(&code).expect("BPE test should succeed");
    assert_eq!(format!("{}", res), "ok");
}

#[test]
fn test_gguf_all_quantization_formats_roundtrip() {
    let data: Vec<f32> = (0..128)
        .map(|i| ((i as f32) - 64.0) * 0.05 + ((i % 7) as f32) * 0.01)
        .collect();

    let cases: &[(u32, f32)] = &[
        (GGUF_TYPE_F32, 1e-6),
        (GGUF_TYPE_F16, 5e-3),
        (GGUF_TYPE_BF16, 4e-2),
        (GGUF_TYPE_Q8_0, 4e-2),
        (GGUF_TYPE_Q5_0, 0.25),
        (GGUF_TYPE_Q4_1, 0.35),
        (GGUF_TYPE_Q4_0, 0.55),
    ];

    for &(qtype, tol) in cases {
        let encoded = quantize_ggml(qtype, &data).expect("quantize");
        let decoded = dequantize_ggml(qtype, &encoded, data.len()).expect("dequantize");
        assert_eq!(decoded.len(), data.len());
        let max_err = data
            .iter()
            .zip(decoded.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(
            max_err <= tol,
            "Quant type {} exceeded tolerance {}: max_err={}",
            qtype,
            tol,
            max_err
        );
    }
}

#[test]
fn test_local_llama_gguf_safetensors_kv_cache_and_embeddings() {
    let tmp = TempDir::new().expect("tempdir");
    let gguf_path = tmp
        .path()
        .join("tiny_llama_q8.gguf")
        .to_string_lossy()
        .replace('\\', "/");
    let st_path = tmp
        .path()
        .join("tiny_llama.safetensors")
        .to_string_lossy()
        .replace('\\', "/");

    let code = format!(
        r#"
from aether_tensor import Tensor
from aether_llm import BPETokenizer, GGUFModel, LocalLLM, cosine_similarity

let vocab = 32
let dim = 32
let hidden = 64
let n_heads = 4
let n_kv_heads = 2
let kv_dim = 16

def make_mat(rows, cols, seed):
    let out = []
    let x = seed
    let i = 0
    while i < rows * cols:
        x = (x * 1103515245 + 12345) % 2147483647
        let v = ((x % 2001) - 1000) / 4000.0
        out.append(v)
        i = i + 1
    return Tensor(out, [rows, cols])

def ones_1d(n):
    let out = []
    let i = 0
    while i < n:
        out.append(1.0)
        i = i + 1
    return Tensor(out, [n])

let emb = make_mat(vocab, dim, 11)
let out_w = make_mat(vocab, dim, 17)

let tensors = {{
    "token_embd.weight": emb,
    "output_norm.weight": ones_1d(dim),
    "output.weight": out_w,
    "blk.0.attn_norm.weight": ones_1d(dim),
    "blk.0.attn_q.weight": make_mat(dim, dim, 23),
    "blk.0.attn_k.weight": make_mat(kv_dim, dim, 29),
    "blk.0.attn_v.weight": make_mat(kv_dim, dim, 31),
    "blk.0.attn_output.weight": make_mat(dim, dim, 37),
    "blk.0.ffn_norm.weight": ones_1d(dim),
    "blk.0.ffn_gate.weight": make_mat(hidden, dim, 41),
    "blk.0.ffn_up.weight": make_mat(hidden, dim, 43),
    "blk.0.ffn_down.weight": make_mat(dim, hidden, 47),
    "blk.1.attn_norm.weight": ones_1d(dim),
    "blk.1.attn_q.weight": make_mat(dim, dim, 53),
    "blk.1.attn_k.weight": make_mat(kv_dim, dim, 59),
    "blk.1.attn_v.weight": make_mat(kv_dim, dim, 61),
    "blk.1.attn_output.weight": make_mat(dim, dim, 67),
    "blk.1.ffn_norm.weight": ones_1d(dim),
    "blk.1.ffn_gate.weight": make_mat(hidden, dim, 71),
    "blk.1.ffn_up.weight": make_mat(hidden, dim, 73),
    "blk.1.ffn_down.weight": make_mat(dim, hidden, 79)
}}

let tok_list = []
let ti = 0
while ti < vocab:
    tok_list.append("t" + str(ti))
    ti = ti + 1

let meta = {{
    "general.architecture": "llama",
    "general.name": "AetherTinyLlama-GQA",
    "llama.block_count": 2,
    "llama.embedding_length": dim,
    "llama.feed_forward_length": hidden,
    "llama.attention.head_count": n_heads,
    "llama.attention.head_count_kv": n_kv_heads,
    "llama.context_length": 128,
    "llama.rope.freq_base": 10000.0,
    "tokenizer.ggml.model": "gpt2",
    "tokenizer.ggml.tokens": tok_list,
    "tokenizer.ggml.bos_token_id": 1,
    "tokenizer.ggml.eos_token_id": 2
}}

# 1. Export Q8_0 quantized GGUF and verify header/dtypes
GGUFModel.save("{gguf_path}", meta, tensors, "Q8_0")
let gf = GGUFModel.open("{gguf_path}")
let q_info = gf.tensor_info("blk.0.attn_q.weight")
if q_info["dtype"] != "Q8_0":
    raise "Expected Q8_0 dtype for 2D weight, got " + str(q_info["dtype"])
let n_info = gf.tensor_info("blk.0.attn_norm.weight")
if n_info["dtype"] != "F32":
    raise "Expected F32 dtype preserved for 1D norm weight"

# 2. Load LocalLLM from GGUF & verify KV-Cache incremental equivalence
let llm = LocalLLM.from_gguf(gf)
let info = llm.info()
if info["num_layers"] != 2 or info["num_kv_heads"] != 2 or info["vocab_size"] != 32:
    raise "Unexpected model info"

# Full sequence in one call
llm.reset_cache()
let full_logits = llm.logits([3, 7, 11])

# Incremental token-by-token using KV-cache
llm.reset_cache()
llm.logits([3])
llm.logits([7])
let inc_logits = llm.logits([11])

let max_kv_diff = 0.0
let j = 0
while j < len(full_logits):
    let d = Math.abs(full_logits[j] - inc_logits[j])
    if d > max_kv_diff:
        max_kv_diff = d
    j = j + 1
if max_kv_diff > 0.000001:
    raise "KV-cache incremental mismatch: " + str(max_kv_diff)

# 3. Deterministic generation (greedy & top-k/top-p)
let g1 = llm.generate([3, 7], {{"max_tokens": 5, "temperature": 0.0}})
let g2 = llm.generate([3, 7], {{"max_tokens": 5, "temperature": 0.0}})
if len(g1["tokens"]) == 0 or len(g1["tokens"]) != len(g2["tokens"]):
    raise "Greedy generation mismatch"
if g1["tokens"][0] != g2["tokens"][0]:
    raise "Greedy generation not deterministic"

let gs = llm.generate([3, 7], {{"max_tokens": 4, "temperature": 0.8, "top_k": 8, "top_p": 0.9, "seed": 1234}})
if len(gs["tokens"]) == 0:
    raise "Sampled generation empty"

# 4. Dense Transformer embeddings & cosine similarity
let e1 = llm.embed([3, 7, 11], "mean")
let e2 = llm.embed([3, 7, 11], "mean")
let e3 = llm.embed([20, 21, 22], "mean")
if len(e1) != dim:
    raise "Embedding dim mismatch"
let sim_same = cosine_similarity(e1, e2)
let sim_diff = cosine_similarity(e1, e3)
if sim_same < 0.9999:
    raise "Self cosine similarity should be 1.0, got " + str(sim_same)
if sim_diff >= sim_same:
    raise "Distinct prompts should have lower cosine similarity"

# 5. Also verify LocalLLM.from_safetensors path
let raw_tensors = {{}}
for k in tensors.keys():
    raw_tensors[k] = tensors[k].handle
NN.save_safetensors(raw_tensors, "{st_path}")
let llm_st = LocalLLM.from_safetensors("{st_path}", {{
    "num_layers": 2,
    "num_heads": 4,
    "num_kv_heads": 2,
    "max_seq_len": 128
}})
let st_logits = llm_st.logits([3, 7, 11])
if len(st_logits) != vocab:
    raise "Safetensors LLM logits size mismatch"

"ok"
"#,
        gguf_path = gguf_path,
        st_path = st_path
    );

    let res = run_source(&code).expect("Local LLM GGUF/Safetensors test should succeed");
    assert_eq!(format!("{}", res), "ok");

    // 6. Verify strict sandbox blocks GGUF file read
    let blocked_code = format!(r#"LLM.load_gguf("{}")"#, gguf_path);
    let err = run_source_sandboxed(&blocked_code, SecurityPolicy::strict_sandbox(100_000))
        .expect_err("Sandbox must block GGUF read");
    assert!(err.contains("PermissionDeniedError"));
}
