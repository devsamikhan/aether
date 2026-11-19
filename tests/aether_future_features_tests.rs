use aether::codegen::aot_builder::AotBuilder;
use aether::vm::run_source;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_feature1_schema_guaranteed_llm_decoding_and_choices() {
    let tmp = TempDir::new().expect("tempdir");
    let gguf_path = tmp
        .path()
        .join("schema_llama.gguf")
        .to_string_lossy()
        .replace('\\', "/");

    let code = format!(
        r#"
from aether_tensor import Tensor
from aether_llm import BPETokenizer, GGUFModel, LocalLLM

let tok = BPETokenizer.train([
    "approve reject escalate priority high medium low true false 12345 0.95 summary ok",
    "json structured schema decoding guarantees valid output 42 7 3.14"
], 280, ["<|bos|>", "<|eos|>", "<|unk|>"])

let vocab = tok.vocab_size()
let dim = 32
let hidden = 64
let kv_dim = 16

def synth(rows, cols, seed):
    let out = []
    let x = seed
    let i = 0
    while i < rows * cols:
        x = (x * 1103515245 + 12345) % 2147483647
        out.append(((x % 2001) - 1000) / 5000.0)
        i = i + 1
    return Tensor(out, [rows, cols])

def ones_v(n):
    let out = []
    let i = 0
    while i < n:
        out.append(1.0)
        i = i + 1
    return Tensor(out, [n])

let tensors = {{
    "token_embd.weight": synth(vocab, dim, 11),
    "output_norm.weight": ones_v(dim),
    "output.weight": synth(vocab, dim, 13),
    "blk.0.attn_norm.weight": ones_v(dim),
    "blk.0.attn_q.weight": synth(dim, dim, 17),
    "blk.0.attn_k.weight": synth(kv_dim, dim, 19),
    "blk.0.attn_v.weight": synth(kv_dim, dim, 23),
    "blk.0.attn_output.weight": synth(dim, dim, 29),
    "blk.0.ffn_norm.weight": ones_v(dim),
    "blk.0.ffn_gate.weight": synth(hidden, dim, 31),
    "blk.0.ffn_up.weight": synth(hidden, dim, 37),
    "blk.0.ffn_down.weight": synth(dim, hidden, 41)
}}

let meta = {{
    "general.architecture": "llama",
    "llama.block_count": 1,
    "llama.embedding_length": dim,
    "llama.feed_forward_length": hidden,
    "llama.attention.head_count": 4,
    "llama.attention.head_count_kv": 2,
    "llama.context_length": 256,
    "llama.rope.freq_base": 10000.0
}}

GGUFModel.save("{gguf_path}", meta, tensors, "Q8_0")
let llm = LocalLLM.from_gguf("{gguf_path}", tok)

# 1. Schema-guaranteed JSON generation
let schema = {{
    "priority": ["low", "medium", "high"],
    "count": "int",
    "confidence": "float",
    "approved": "bool",
    "summary": "string"
}}
let res = llm.generate_schema("Classify incident ticket:", schema, {{"temperature": 0.0}})
if res["valid"] != true:
    raise "Expected valid schema result"
let data = res["data"]
let p = data["priority"]
if p != "low" and p != "medium" and p != "high":
    raise "Enum constraint violated: " + str(p)
if type(data["count"]) != "int":
    raise "Expected int field 'count', got " + type(data["count"])
if type(data["approved"]) != "bool":
    raise "Expected bool field 'approved', got " + type(data["approved"])
if type(data["summary"]) != "string":
    raise "Expected string field 'summary', got " + type(data["summary"])

# Ensure raw JSON parses cleanly
let reparsed = Json.parse(res["json"])
if reparsed["priority"] != p:
    raise "Reparsed JSON mismatch"

# 2. Multi-choice constrained scoring
let ch = llm.generate_choices("Decision:", ["approve", "reject", "escalate"], {{"temperature": 0.0}})
let probs = ch["probabilities"]
let sum_p = probs[0] + probs[1] + probs[2]
if Math.abs(sum_p - 1.0) > 0.001:
    raise "Choice probabilities must sum to 1, got " + str(sum_p)
if ch["choice"] != "approve" and ch["choice"] != "reject" and ch["choice"] != "escalate":
    raise "Invalid choice returned"

"ok"
"#,
        gguf_path = gguf_path
    );

    let res = run_source(&code).expect("Feature 1 schema-constrained LLM test should pass");
    assert_eq!(format!("{}", res), "ok");
}

#[test]
fn test_feature2_first_class_sandbox_blocks_and_monotonic_attenuation() {
    let code = r#"
# 1. Syntax-level capability sandbox block blocks unauthorized FS read
let blocked = false
try:
    sandbox(fuel=50000, fs_read=false, fs_write=false, net=false):
        let pol = Sandbox.policy()
        if pol["fs_read"] != false or pol["net"] != false:
            raise "Sandbox policy did not restrict capabilities"
        File.read("Cargo.toml")
except e:
    blocked = true

if !blocked:
    raise "Expected sandbox block to block File.read"

# 2. Verify outer policy was cleanly restored after exception unwind
let outer = Sandbox.policy()
if outer["fs_read"] != true or outer["nesting_depth"] != 0:
    raise "Outer SecurityPolicy was not restored cleanly"

# 3. Monotonic privilege attenuation: inner sandbox CANNOT escalate above parent!
let inner_fs_read = true
sandbox(fuel=50000, fs_read=false):
    sandbox(fuel=999999, fs_read=true):
        let p = Sandbox.policy()
        inner_fs_read = p["fs_read"]
        if p["nesting_depth"] != 2:
            raise "Expected nesting_depth 2"

if inner_fs_read != false:
    raise "Privilege escalation occurred inside nested sandbox!"

# 4. Fuel exhaustion protection inside sandbox block
let fuel_stopped = false
try:
    sandbox(fuel=250):
        let i = 0
        while i < 100000:
            i = i + 1
except e:
    fuel_stopped = true

if !fuel_stopped:
    raise "Expected fuel limit to halt runaway loop inside sandbox"

# 5. Programmatic Sandbox.run for untrusted AI-generated code
let good = Sandbox.run("let a = 6\nlet b = 7\na * b", {"fuel": 20000, "fs_read": false})
if good["ok"] != true or good["value"] != 42:
    raise "Sandbox.run valid script failed"

let bad = Sandbox.run("File.read(\"Cargo.toml\")", {"fuel": 20000, "fs_read": false})
if bad["ok"] != false:
    raise "Sandbox.run failed to block FS access"

"ok"
"#;

    let res = run_source(code).expect("Feature 2 capability sandbox test should pass");
    assert_eq!(format!("{}", res), "ok");
}

#[test]
fn test_feature3_gpu_autograd_training_and_lora_finetuning() {
    let tmp = TempDir::new().expect("tempdir");
    let adapter_path = tmp
        .path()
        .join("lora_adapter.safetensors")
        .to_string_lossy()
        .replace('\\', "/");

    let code = format!(
        r#"
from aether_tensor import Tensor
from aether_nn import Var, LoRALinear, AdamW, mse_loss, set_device, device

# Switch NN autograd engine to GPU (Vulkan/DX12/Metal when available, else CPU)
let active_dev = set_device("gpu")
if active_dev != "gpu" and active_dev != "cpu":
    raise "Unexpected NN device: " + str(active_dev)

# Frozen base weight (identity 4x4)
let base_w = Tensor([
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0]
])
let lora = LoRALinear(base_w, 2, 4.0)

let x = Tensor([
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [1.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 1.0]
])
# Target: adapt base identity to 2x on first two dims
let y_target = Tensor([
    [2.0, 0.0, 0.0, 0.0],
    [0.0, 2.0, 0.0, 0.0],
    [2.0, 2.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 1.0]
])

# Step-0 check: LoRA B is zero-initialized -> exact base output
let pred0 = lora.forward(x).data().to_array()
if Math.abs(pred0[0][0] - 1.0) > 0.0001:
    raise "Step-0 LoRA output must match frozen base weight"

let opt = AdamW(lora.parameters(), 0.08)
let initial_loss = 0.0
let final_loss = 0.0
let step = 0
while step < 50:
    opt.zero_grad()
    let pred = lora.forward(x)
    let loss = mse_loss(pred, y_target)
    let lv = loss.item()
    if step == 0:
        initial_loss = lv
    final_loss = lv
    loss.backward()
    opt.step()
    step = step + 1

if final_loss >= initial_loss * 0.1:
    raise "LoRA GPU training failed to converge: " + str(initial_loss) + " -> " + str(final_loss)

# Verify frozen base weight never changed
let bw_after = lora.base_weight.data().to_array()
if Math.abs(bw_after[0][0] - 1.0) > 0.0001:
    raise "Frozen base weight was modified during LoRA training!"

# Verify merged weight produces exact same inference output as LoRA forward
let merged_w = lora.merge()
let merged_out = x.matmul(merged_w).to_array()
let lora_out = lora.forward(x).data().to_array()
if Math.abs(merged_out[0][0] - lora_out[0][0]) > 0.01:
    raise "Merged LoRA weight diverged from LoRA forward"

# Save & reload tiny LoRA adapter via .safetensors
lora.save_adapter("{adapter_path}")
let lora2 = LoRALinear(base_w, 2, 4.0)
lora2.load_adapter("{adapter_path}")
let reloaded_out = lora2.forward(x).data().to_array()
if Math.abs(reloaded_out[0][0] - lora_out[0][0]) > 0.001:
    raise "Reloaded LoRA adapter output mismatch"

set_device("cpu")
"ok"
"#,
        adapter_path = adapter_path
    );

    let res = run_source(&code).expect("Feature 3 GPU autograd + LoRA test should pass");
    assert_eq!(format!("{}", res), "ok");
}

#[test]
fn test_feature4_rewindable_timetravel_agent_with_kv_cache_branching() {
    let tmp = TempDir::new().expect("tempdir");
    let gguf_path = tmp
        .path()
        .join("agent_llama.gguf")
        .to_string_lossy()
        .replace('\\', "/");

    let code = format!(
        r#"
from aether_tensor import Tensor
from aether_llm import GGUFModel, LocalLLM
from aether_agent import RewindableAgent

let vocab = 32
let dim = 32
let hidden = 64
let kv_dim = 16

def synth(rows, cols, seed):
    let out = []
    let x = seed
    let i = 0
    while i < rows * cols:
        x = (x * 1103515245 + 12345) % 2147483647
        out.append(((x % 2001) - 1000) / 5000.0)
        i = i + 1
    return Tensor(out, [rows, cols])

def ones_v(n):
    let out = []
    let i = 0
    while i < n:
        out.append(1.0)
        i = i + 1
    return Tensor(out, [n])

let tensors = {{
    "token_embd.weight": synth(vocab, dim, 5),
    "output_norm.weight": ones_v(dim),
    "output.weight": synth(vocab, dim, 7),
    "blk.0.attn_norm.weight": ones_v(dim),
    "blk.0.attn_q.weight": synth(dim, dim, 11),
    "blk.0.attn_k.weight": synth(kv_dim, dim, 13),
    "blk.0.attn_v.weight": synth(kv_dim, dim, 17),
    "blk.0.attn_output.weight": synth(dim, dim, 19),
    "blk.0.ffn_norm.weight": ones_v(dim),
    "blk.0.ffn_gate.weight": synth(hidden, dim, 23),
    "blk.0.ffn_up.weight": synth(hidden, dim, 29),
    "blk.0.ffn_down.weight": synth(dim, hidden, 31)
}}
let meta = {{
    "general.architecture": "llama",
    "llama.block_count": 1,
    "llama.embedding_length": dim,
    "llama.feed_forward_length": hidden,
    "llama.attention.head_count": 4,
    "llama.attention.head_count_kv": 2,
    "llama.context_length": 128,
    "llama.rope.freq_base": 10000.0
}}
GGUFModel.save("{gguf_path}", meta, tensors, "Q8_0")
let llm = LocalLLM.from_gguf("{gguf_path}")

let agent = RewindableAgent("EdgePlanner", llm)
agent.ask([1, 2, 3], {{"max_tokens": 3, "temperature": 0.0}})
agent.set("plan", "initial")
agent.checkpoint("before_branch")
let kv_at_cp = llm.cache_len()

# Take a bad path (mutates state + grows KV-Cache)
agent.set("plan", "risky_path")
agent.ask([4, 5], {{"max_tokens": 3, "temperature": 0.0}})
let kv_after_bad = llm.cache_len()
if kv_after_bad <= kv_at_cp:
    raise "Expected KV-cache to advance on turn 2"

# Safe sandboxed tool execution inside agent
let tool_res = agent.run_safe_tool("let s = 10 + 20\ns * 2")
if tool_res["ok"] != true or tool_res["value"] != 60:
    raise "Agent safe tool execution failed"

# Time-Travel branch rewind restores BOTH state and LLM KV-cache!
agent.branch_from("before_branch", "safe_timeline")
if agent.get("plan") != "initial":
    raise "Agent state did not rewind to checkpoint!"
if llm.cache_len() != kv_at_cp:
    raise "LLM KV-Cache did not rewind to checkpoint!"

let hist = agent.history()
if len(hist) == 0:
    raise "Expected non-empty TimeTravel audit history"

"ok"
"#,
        gguf_path = gguf_path
    );

    let res = run_source(&code).expect("Feature 4 rewindable TimeTravel agent test should pass");
    assert_eq!(format!("{}", res), "ok");
}

#[test]
fn test_feature5_neuro_symbolic_sql_vector_graph_llm_proof() {
    let tmp = TempDir::new().expect("tempdir");
    let gguf_path = tmp
        .path()
        .join("neuro_llama.gguf")
        .to_string_lossy()
        .replace('\\', "/");

    let code = format!(
        r#"
from aether_tensor import Tensor
from aether_llm import BPETokenizer, GGUFModel, LocalLLM
from aether_neuro import NeuroSymbolicEngine

let tok = BPETokenizer.train([
    "drug alpha targets kras pathway mapk safe verified true false 95 0.9"
], 270, ["<|bos|>", "<|eos|>", "<|unk|>"])
let vocab = tok.vocab_size()
let dim = 32
let hidden = 64
let kv_dim = 16

def synth(rows, cols, seed):
    let out = []
    let x = seed
    let i = 0
    while i < rows * cols:
        x = (x * 1103515245 + 12345) % 2147483647
        out.append(((x % 2001) - 1000) / 5000.0)
        i = i + 1
    return Tensor(out, [rows, cols])

def ones_v(n):
    let out = []
    let i = 0
    while i < n:
        out.append(1.0)
        i = i + 1
    return Tensor(out, [n])

let tensors = {{
    "token_embd.weight": synth(vocab, dim, 3),
    "output_norm.weight": ones_v(dim),
    "output.weight": synth(vocab, dim, 5),
    "blk.0.attn_norm.weight": ones_v(dim),
    "blk.0.attn_q.weight": synth(dim, dim, 7),
    "blk.0.attn_k.weight": synth(kv_dim, dim, 11),
    "blk.0.attn_v.weight": synth(kv_dim, dim, 13),
    "blk.0.attn_output.weight": synth(dim, dim, 17),
    "blk.0.ffn_norm.weight": ones_v(dim),
    "blk.0.ffn_gate.weight": synth(hidden, dim, 19),
    "blk.0.ffn_up.weight": synth(hidden, dim, 23),
    "blk.0.ffn_down.weight": synth(dim, hidden, 29)
}}
let meta = {{
    "general.architecture": "llama",
    "llama.block_count": 1,
    "llama.embedding_length": dim,
    "llama.feed_forward_length": hidden,
    "llama.attention.head_count": 4,
    "llama.attention.head_count_kv": 2,
    "llama.context_length": 256,
    "llama.rope.freq_base": 10000.0
}}
GGUFModel.save("{gguf_path}", meta, tensors, "Q8_0")
let llm = LocalLLM.from_gguf("{gguf_path}", tok)

let engine = NeuroSymbolicEngine(llm)
engine.add_entity("drug_alpha", "Drug Alpha is a selective KRAS inhibitor")
engine.add_entity("target_kras", "KRAS G12D drives oncogenic MAPK signaling")
engine.add_entity("pathway_mapk", "MAPK cascade regulates tumor cell proliferation")
engine.add_relation("drug_alpha", "target_kras", "inhibits", 1.0)
engine.add_relation("target_kras", "pathway_mapk", "activates", 1.0)

engine.register_sql_table("trials", {{
    "drug": ["drug_alpha", "drug_beta"],
    "efficacy": [92.5, 61.0]
}})
let sql_res = engine.query_sql("SELECT drug, efficacy FROM trials WHERE efficacy > 80")
if sql_res["rows"] != 1:
    raise "Expected 1 high-efficacy SQL row, got " + str(sql_res["rows"])

let safe_contract = "
intent safe_dose(dose):
    require dose > 0
    ensure result >= 1
    return dose + 1
"
let out = engine.reason_verified(
    "KRAS inhibitor pathway",
    {{"verdict": ["safe", "caution"], "confidence": "float"}},
    safe_contract,
    1,
    2
)
if len(out["context"]) < 2:
    raise "Expected GraphRAG multi-hop context expansion >= 2 entities"
if out["structured"]["valid"] != true:
    raise "Expected valid constrained structured output"
if out["verified"] != true:
    raise "Expected formal Proof verification to pass for safe_contract"

# Also confirm unsafe contract is rejected by Proof verifier with counter-example
let unsafe_contract = "
intent unsafe_dose(dose):
    require dose > 0
    ensure result <= 50
    return dose + 100
"
let bad_check = engine.verify_contract(unsafe_contract)
if bad_check["all_verified"] != false:
    raise "Expected Proof verifier to reject unsafe_dose contract violation"

"ok"
"#,
        gguf_path = gguf_path
    );

    let res = run_source(&code).expect("Feature 5 neuro-symbolic engine test should pass");
    assert_eq!(format!("{}", res), "ok");
}

#[test]
fn test_feature6_standalone_single_binary_packager_with_modules_and_assets() {
    let build_dir = TempDir::new().expect("build tempdir");
    let isolated_run_dir = TempDir::new().expect("isolated run tempdir");

    // Local helper module
    let helper_path = build_dir.path().join("my_helper.ae");
    std::fs::write(&helper_path, "def greet_calc(x):\n    return x * 3 + 1\n")
        .expect("write helper");

    // Embedded asset file
    let asset_path = build_dir.path().join("config.txt");
    std::fs::write(&asset_path, "EDGE_MODE=ZERO_PYTHON").expect("write asset");

    // Entry script importing BOTH local helper AND stdlib aether_tensor + reading embedded asset
    let entry_path = build_dir.path().join("app.ae");
    std::fs::write(
        &entry_path,
        r#"from my_helper import greet_calc
from aether_tensor import Tensor

let t = Tensor([[2.0, 3.0], [4.0, 5.0]])
let sum_v = t.sum()
let cfg = Bundle.read_text("config.txt")
print("STANDALONE=" + str(Bundle.is_standalone()) + "|CALC=" + str(greet_calc(7)) + "|SUM=" + str(sum_v) + "|CFG=" + cfg)
"#,
    )
    .expect("write entry");

    let exe_name = if cfg!(windows) {
        "aether_standalone_test_app.exe"
    } else {
        "aether_standalone_test_app"
    };
    let out_exe = isolated_run_dir.path().join(exe_name);

    let builder = AotBuilder::new(false);
    builder
        .build_executable_with_assets(
            &entry_path,
            &out_exe,
            &[asset_path.to_string_lossy().into_owned()],
        )
        .expect("build standalone executable with modules and assets");

    assert!(out_exe.is_file(), "Standalone binary must exist");

    // Execute standalone binary inside a completely empty working directory (no libraries/ folder, no source files!)
    let output = Command::new(&out_exe)
        .current_dir(isolated_run_dir.path())
        .output()
        .expect("execute standalone binary");

    assert!(
        output.status.success(),
        "Standalone binary failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("STANDALONE=true|CALC=22|SUM=14|CFG=EDGE_MODE=ZERO_PYTHON"),
        "Unexpected standalone stdout: {}",
        stdout
    );
}
