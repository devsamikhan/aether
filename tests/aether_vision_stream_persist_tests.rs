use aether::vm::{run_source, run_source_sandboxed, SecurityPolicy};
use tempfile::TempDir;

#[test]
fn test_vision_bmp_ppm_ops_and_cnn_conv2d_maxpool_training() {
    let tmp = TempDir::new().expect("tempdir");
    let bmp_path = tmp
        .path()
        .join("test_img.bmp")
        .to_string_lossy()
        .replace('\\', "/");
    let ppm_path = tmp
        .path()
        .join("test_img.ppm")
        .to_string_lossy()
        .replace('\\', "/");

    let code = format!(
        r#"
from aether_vision import Image
from aether_tensor import Tensor
from aether_nn import Var, Conv2d, MaxPool2d, Flatten, Linear, AdamW, cross_entropy_loss

# 1. Vision BMP & PPM roundtrip + resize + grayscale + crop + flip + Sobel
let pixels = []
let y = 0
while y < 8:
    let x = 0
    while x < 8:
        let v = 0.1
        if x >= 4:
            v = 0.9
        pixels.append(v)
        pixels.append(v * 0.5)
        pixels.append(0.2)
        x = x + 1
    y = y + 1

let img = Image.create(8, 8, 3, pixels)
img.save_bmp("{bmp_path}")
img.save_ppm("{ppm_path}")

let from_bmp = Image.load("{bmp_path}")
let from_ppm = Image.load("{ppm_path}")
if from_bmp.info()["width"] != 8 or from_ppm.info()["height"] != 8:
    raise "BMP/PPM reload dims mismatch"

let small = from_bmp.resize(4, 4).grayscale().flip_h()
if small.info()["width"] != 4 or small.info()["channels"] != 1:
    raise "Resize/grayscale dims mismatch"

let edges = from_bmp.sobel_edges()
if edges.info()["max"] < 0.5:
    raise "Expected strong Sobel vertical edge on half-bright image"

let t = small.to_tensor()
let shp = t.shape()
if shp[0] != 1 or shp[1] != 1 or shp[2] != 4 or shp[3] != 4:
    raise "Expected NCHW [1,1,4,4] tensor shape"
let round_img = Image.from_tensor(t)
if round_img.info()["width"] != 4:
    raise "from_tensor width mismatch"

# 2. Conv2d + MaxPool2d + Flatten CNN Training Convergence
def make_pat(kind):
    let out = []
    let r = 0
    while r < 6:
        let c = 0
        while c < 6:
            let val = 0.0
            if kind == 0 and (r == 2 or r == 3):
                val = 1.0
            if kind == 1 and (c == 2 or c == 3):
                val = 1.0
            out.append(val)
            c = c + 1
        r = r + 1
    return out

let batch = []
for v in make_pat(0):
    batch.append(v)
for v in make_pat(1):
    batch.append(v)

let X = Tensor(batch, [2, 1, 6, 6])
let y_lbl = [0, 1]

let conv = Conv2d(1, 3, 3, 1, 0)
let pool = MaxPool2d(2, 2)
let flat = Flatten()
let fc = Linear(12, 2, 19)

let params = []
for p in conv.parameters():
    params.append(p)
for p in fc.parameters():
    params.append(p)

let opt = AdamW(params, 0.08)
let loss0 = 0.0
let loss_end = 0.0
let step = 0
while step < 30:
    opt.zero_grad()
    let logits = fc.forward(flat.forward(pool.forward(conv.forward(X).relu())))
    let loss = cross_entropy_loss(logits, y_lbl)
    let lv = loss.item()
    if step == 0:
        loss0 = lv
    loss_end = lv
    loss.backward()
    opt.step()
    step = step + 1

if loss_end >= loss0 * 0.15:
    raise "CNN failed to converge: " + str(loss0) + " -> " + str(loss_end)

"ok"
"#,
        bmp_path = bmp_path,
        ppm_path = ppm_path
    );

    let res = run_source(&code).expect("Vision + CNN test should pass");
    assert_eq!(format!("{}", res), "ok");
}

#[test]
fn test_llm_streaming_callback_and_vector_graph_disk_persistence() {
    let tmp = TempDir::new().expect("tempdir");
    let gguf_path = tmp
        .path()
        .join("stream_llama.gguf")
        .to_string_lossy()
        .replace('\\', "/");
    let vec_path = tmp
        .path()
        .join("vectors.json")
        .to_string_lossy()
        .replace('\\', "/");
    let graph_path = tmp
        .path()
        .join("graph.json")
        .to_string_lossy()
        .replace('\\', "/");

    let code = format!(
        r#"
from aether_tensor import Tensor
from aether_llm import BPETokenizer, GGUFModel, LocalLLM

# 1. LLM.generate_stream callback + early cancellation
let tok = BPETokenizer.train(["streaming tokens live in aether without python"], 265, ["<|bos|>", "<|eos|>"])
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
    "llama.context_length": 128,
    "llama.rope.freq_base": 10000.0
}}
GGUFModel.save("{gguf_path}", meta, tensors, "Q8_0")
let llm = LocalLLM.from_gguf("{gguf_path}", tok)

let seen_tokens = []
def on_token(piece, tid, step):
    seen_tokens.append(tid)
    # Cancel early after 3 tokens (steps 0, 1, 2)
    if step >= 2:
        return false
    return true

let sres = llm.generate_stream("streaming tokens", on_token, {{"max_tokens": 10, "temperature": 0.0}})
if sres["cancelled"] != true or len(seen_tokens) != 3:
    raise "Expected stream early cancellation at 3 tokens, got " + str(len(seen_tokens))

# 2. Vector.save & Vector.load persistence roundtrip
let vid = Vector.create_index()
Vector.add(vid, "doc1", [1.0, 0.0, 0.0], {{"topic": "vision"}})
Vector.add(vid, "doc2", [0.0, 1.0, 0.0], {{"topic": "llm"}})
Vector.save(vid, "{vec_path}")

let vid2 = Vector.load("{vec_path}")
if Vector.count(vid2) != 2:
    raise "Expected 2 vectors after Vector.load"
let hits = Vector.search(vid2, [0.95, 0.05, 0.0], 1, "cosine")
if hits[0]["id"] != "doc1" or hits[0]["metadata"]["topic"] != "vision":
    raise "Reloaded Vector search mismatch"

# 3. Graph.save & Graph.load persistence roundtrip
let gid = Graph.create("persisted_kg")
let n1 = Graph.add_node(gid, "AetherVM", {{"kind": "runtime"}}, [1.0, 0.0])
let n2 = Graph.add_node(gid, "WgpuBackend", {{"kind": "gpu"}}, [0.9, 0.1])
Graph.add_edge(gid, n1, n2, "uses", 1.5)
Graph.save(gid, "{graph_path}")

let gid2 = Graph.load("{graph_path}")
let bfs = Graph.bfs(gid2, n1, 2)
if len(bfs) != 2:
    raise "Expected 2 nodes in reloaded Graph.bfs, got " + str(len(bfs))

"ok"
"#,
        gguf_path = gguf_path,
        vec_path = vec_path,
        graph_path = graph_path
    );

    let res = run_source(&code).expect("Streaming + Vector/Graph persistence test should pass");
    assert_eq!(format!("{}", res), "ok");

    // Verify sandbox blocks Vector.load / Graph.load
    let err = run_source_sandboxed(
        &format!(r#"Vector.load("{}")"#, vec_path),
        SecurityPolicy::strict_sandbox(50_000),
    )
    .expect_err("Sandbox must block Vector.load");
    assert!(err.contains("PermissionDeniedError"));
}

#[test]
fn test_live_http_ai_microservice_endpoints() {
    // Pick an OS-assigned free TCP port, then close and hand it to AETHER HTTP server
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let port = probe.local_addr().unwrap().port();
    drop(probe);

    let server_thread = std::thread::spawn(move || {
        let code = format!(
            r#"
from projects.live_ai_server.ai_api_server import serve_ai_api
serve_ai_api({}, 2)
"#,
            port
        );
        // Set env so importing ai_api_server does not auto-start infinite loop
        std::env::set_var("AETHER_AI_SERVER_IMPORT_ONLY", "1");
        run_source(&code)
    });

    // Wait briefly for server to bind and send 2 live HTTP requests (/health and /api/score)
    std::thread::sleep(std::time::Duration::from_millis(250));

    let client_code = format!(
        r#"
let h = Http.get("http://127.0.0.1:{port}/health")
let h_json = Json.parse(h["body"])
if h_json["status"] != "ok" or h_json["runtime"] != "AETHER":
    raise "Unexpected /health response: " + str(h["body"])

let s = Http.post(
    "http://127.0.0.1:{port}/api/score",
    Json.stringify({{"amount": 2500.0, "hour": 2.0, "velocity": 8.0, "geo_km": 1600.0, "trust": 0.12}}),
    {{"Content-Type": "application/json"}}
)
let s_json = Json.parse(s["body"])
if s_json["verdict"] != "BLOCK_FRAUD":
    raise "Expected BLOCK_FRAUD from live AI API, got: " + str(s["body"])
"ok"
"#,
        port = port
    );

    let client_res = run_source(&client_code);
    let srv_res = server_thread.join().expect("server thread join");
    assert!(
        client_res.is_ok(),
        "Live HTTP AI client failed: {:?} | server: {:?}",
        client_res,
        srv_res
    );
    assert_eq!(format!("{}", client_res.unwrap()), "ok");
    assert!(srv_res.is_ok(), "Server should exit cleanly: {:?}", srv_res);
}
