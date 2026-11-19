// ==============================================================================
// AETHER Language Server Protocol (LSP) Engine
// JSON-RPC 2.0 Language Server for VS Code, Cursor & Modern IDEs
// ==============================================================================

use serde_json::{json, Value as JsonValue};
use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct LspDiagnostic {
    pub line: usize,
    pub col: usize,
    pub message: String,
    pub severity: usize, // 1 = Error, 2 = Warning, 3 = Information
}

pub struct AetherLanguageServer {
    documents: HashMap<String, String>,
}

impl Default for AetherLanguageServer {
    fn default() -> Self {
        Self::new()
    }
}

impl AetherLanguageServer {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    /// Analyzes source code and produces structured LSP diagnostics
    pub fn analyze(&self, source: &str) -> Vec<LspDiagnostic> {
        let mut diagnostics = Vec::new();

        match crate::syntax::parse(source) {
            Ok(program) => {
                let mut checker = crate::types::checker::TypeChecker::new();
                if let Err(type_errs) = checker.check_program_with_spans(&program) {
                    for err in type_errs {
                        diagnostics.push(LspDiagnostic {
                            line: err.line.saturating_sub(1),
                            col: err.col.saturating_sub(1),
                            message: format!("Type error: {}", err.message),
                            severity: 2,
                        });
                    }
                }
                for finding in crate::vm::security::scan_program_security(&program) {
                    diagnostics.push(LspDiagnostic {
                        line: finding.line.saturating_sub(1),
                        col: 0,
                        message: format!(
                            "[{}] {}: {}",
                            finding.severity, finding.category, finding.message
                        ),
                        severity: 2,
                    });
                }
            }
            Err((err_msg, span)) => {
                diagnostics.push(LspDiagnostic {
                    line: span.line.saturating_sub(1),
                    col: span.col.saturating_sub(1),
                    message: format!("Syntax error: {}", err_msg),
                    severity: 1,
                });
            }
        }

        diagnostics
    }

    /// Returns Markdown hover documentation for a symbol or keyword
    pub fn get_hover(&self, word: &str) -> Option<String> {
        match word {
            "AI" => Some(
                "### AETHER Native On-Device AI Engine\n\
                Pure Rust high-speed 128-D dense semantic embeddings, vector cosine RAG store, and offline autonomous local agents.\n\
                * `AI.embed(text)`: Generates normalized 128-D float vector\n\
                * `AI.similarity(v1, v2)`: Cosine similarity (-1.0 to 1.0)\n\
                * `AI.rag_search(query, docs, top_k)`: Fast vector retrieval\n\
                * `AI.Agent(name, role)`: Creates local intelligent agent\n\
                * `AI.chat(prompt)`: Chats with default on-device agent"
                    .to_string(),
            ),
            "Bio" => Some(
                "### AETHER Bioinformatics Engine\n\
                2-bit packed DNA sequences, translation, Needleman-Wunsch alignment, FASTA/FASTQ parsing, and ORF finder.\n\
                * `Bio.dna(seq)`: Compact 2-bit packed DNA sequence\n\
                * `Bio.parse_fasta(text)` / `Bio.parse_fastq(text)`\n\
                * `Bio.align(seq1, seq2, match, mismatch, gap)`\n\
                * `Bio.orf_finder(dna, min_aa_len)`"
                    .to_string(),
            ),
            "ML" => Some(
                "### AETHER Native Machine Learning Engine (Zero Python)\n\
                Pure-Rust supervised & unsupervised algorithms, preprocessing, metrics, and edge model persistence.\n\
                * `ML.train_test_split(X, y, [test_ratio, seed])`\n\
                * `ML.standard_scaler_fit(X)` / `ML.minmax_scaler_fit(X)`\n\
                * `ML.linear_regression_fit(X, y)` / `ML.logistic_regression_fit(X, y)`\n\
                * `ML.decision_tree_fit(X, y)` / `ML.random_forest_fit(X, y)` / `ML.knn_fit(X, y)`\n\
                * `ML.kmeans_fit(X, k)` / `ML.pca_fit(X, n_components)`\n\
                * `ML.predict(id, X)` / `ML.transform(id, X)` / `ML.save_model(id, path)` / `ML.load_model(path)`"
                    .to_string(),
            ),
            "NN" => Some(
                "### AETHER Pure-Rust Deep Learning & Transformer Autograd Engine\n\
                Native reverse-mode autograd tape, CrossEntropy, LayerNorm, Embedding, Causal Self-Attention, AdamW, and HuggingFace `.safetensors` weights.\n\
                * `NN.var(data, [requires_grad])` / `NN.matmul(a, b)` / `NN.backward(loss)`\n\
                * `NN.cross_entropy(logits, targets)` / `NN.layer_norm(x, g, b)` / `NN.attention(q, k, v, causal)`\n\
                * `NN.adamw_create(params, lr, ...)` / `NN.save_safetensors(map, path)` / `NN.load_safetensors(path)`"
                    .to_string(),
            ),
            "LLM" => Some(
                "### AETHER Pure-Rust BPE Tokenizer, GGUF Quantized Loader & Local LLM Engine\n\
                Train/load BPE tokenizers, read/write quantized `.gguf` v2/v3 models (F32/F16/BF16/Q8_0/Q4_0/Q4_1/Q5_0), and run local Llama-architecture inference with RoPE, GQA, SwiGLU, KV-Cache, and dense Transformer embeddings.\n\
                * `LLM.bpe_train(texts, vocab_size)` / `LLM.bpe_encode(tid, text)` / `LLM.bpe_decode(tid, ids)`\n\
                * `LLM.load_gguf(path)` / `LLM.save_gguf(path, meta, tensors, quant_mode)`\n\
                * `LLM.load_model_gguf(path)` / `LLM.generate(mid, prompt, opts)` / `LLM.embed(mid, text)`"
                    .to_string(),
            ),
            "GPU" => Some(
                "### AETHER Cross-Platform Hardware GPU Compute Engine (`wgpu` — DX12 / Vulkan / Metal)\n\
                Runs compiled WGSL compute shaders for Tiled GEMM, activations, Softmax, RMSNorm, LayerNorm, Flash-style Attention, VRAM-resident tensor chaining, and custom WGSL shaders with transparent multi-core CPU fallback.\n\
                * `GPU.available()` / `GPU.info()` / `GPU.benchmark(size)` / `GPU.set_tensor_accel(bool)`\n\
                * `GPU.upload(tensor)` / `GPU.matmul_buf(a, b)` / `GPU.attention_buf(q, k, v, causal)` / `GPU.download(id)`\n\
                * `GPU.run_wgsl(wgsl_source, inputs, out_len, workgroups)`"
                    .to_string(),
            ),
            "Sandbox" => Some(
                "### AETHER Capability-Based Sandbox & Zero-Trust Execution\n\
                Run untrusted or AI-generated code with monotonic capability attenuation (FS/Net/FFI/Env/Fuel limits) either via `sandbox(fuel=50000, fs_read=false): ...` blocks or `Sandbox.run(code, policy)`.\n\
                * `Sandbox.run(source_code, [policy_map]) -> Map`\n\
                * `Sandbox.policy() -> Map` / `Sandbox.fuel_used() -> Int`"
                    .to_string(),
            ),
            "Bundle" => Some(
                "### AETHER Standalone Single-Binary Bundle Runtime\n\
                Access embedded virtual modules and binary/text assets packaged via `aether build app.ae --standalone --asset <file>`.\n\
                * `Bundle.is_standalone()` / `Bundle.modules()` / `Bundle.assets()`\n\
                * `Bundle.read_text(name)` / `Bundle.extract_asset(name, dest)`"
                    .to_string(),
            ),
            "Chem" => Some(
                "### AETHER Cheminformatics Engine\n\
                SMILES/SDF molecule parsing, physicochemical descriptors, Lipinski Ro5, Morgan fingerprints, Tanimoto, and affinity prediction.\n\
                * `Chem.from_smiles(smiles)` / `Chem.parse_sdf(sdf_text)`\n\
                * `Chem.tanimoto_similarity(fp1, fp2)`\n\
                * `Chem.predict_binding_affinity(mol_or_fp, weights, bias)`\n\
                * `Chem.substructure_match(mol, smarts)`"
                    .to_string(),
            ),
            "Http" | "Net" => Some(
                "### AETHER HTTP/HTTPS & TLS Client/Server Engine\n\
                Pure-Rust DNS resolution, rustls TLS verification, chunked transfer decoding, and redirects.\n\
                * `Http.get(url, [headers])` / `Http.post(url, body, [headers])`\n\
                * `Http.listen(host, port)` / `Http.accept(server_id)`"
                    .to_string(),
            ),
            "WebSocket" => Some(
                "### AETHER WebSocket Client & Server Engine\n\
                RFC 6455 framed full-duplex sockets supporting `ws://` and `wss://` TLS.\n\
                * `WebSocket.connect(url)` / `WebSocket.upgrade(req)`\n\
                * `WebSocket.send(ws_id, data)` / `WebSocket.recv(ws_id)`"
                    .to_string(),
            ),
            "Json" => Some(
                "### AETHER JSON Engine\n\
                * `Json.parse(str)` / `Json.stringify(val)`"
                    .to_string(),
            ),
            "File" => Some(
                "### AETHER File System Engine\n\
                * `File.read(path)` / `File.write(path, content)` / `File.exists(path)`"
                    .to_string(),
            ),
            "AetherBridge" => Some(
                "### AetherBridge Native Mobile & Web Substrate\n\
                Bidirectional hardware bridge connecting frontends directly to mobile hardware:\n\
                * `AetherBridge.getLocation()`: Real-time GPS coordinates\n\
                * `AetherBridge.scanQR()`: Camera viewfinder barcode/QR scanner\n\
                * `AetherBridge.scheduleAlarm(title, timeMs)`: Native exact OS alarm\n\
                * `AetherBridge.showNotification(title, body)`: System push notifications\n\
                * `AetherBridge.pickFile(accept)`: Native file picker\n\
                * `AetherBridge.isDarkMode()`: System theme detection"
                    .to_string(),
            ),
            "DB" => Some(
                "### AETHER Embedded Relational SQL Engine\n\
                In-memory or file-backed ACID relational tables with high-speed query execution:\n\
                * `DB.open(path)`: Opens SQLite or in-memory DB\n\
                * `db.execute(sql)`: Executes DDL/DML statements\n\
                * `db.query(sql)`: Returns array of result rows as maps"
                    .to_string(),
            ),
            "fn" | "def" => Some(
                "### Function Declaration\n\
                Defines a function with optional typed parameters and trailing expression returns.\n\
                ```aether\n\
                fn calculate_total(price: Float, qty: Int) -> Float:\n\
                    price * qty\n\
                ```"
                .to_string(),
            ),
            "let" => Some(
                "### Variable Binding\n\
                Binds an immutable variable. Use `let mut` for mutable variables."
                    .to_string(),
            ),
            "match" => Some(
                "### Pattern Matching\n\
                Matches literals, integer ranges (`a..b`), enum variants, structs, and optional `if` guards."
                    .to_string(),
            ),
            "test" => Some(
                "### AETHER Native Test Block\n\
                Declares an in-language VM test executed by `aether test`."
                    .to_string(),
            ),
            "intent" => Some(
                "### AETHER Intent Contract\n\
                Declarative specification block bounding execution constraints and invariant schemas."
                    .to_string(),
            ),
            "require" => Some(
                "### Intent Precondition Invariant\n\
                Asserts constraints that must hold prior to intent function execution."
                    .to_string(),
            ),
            "ensure" => Some(
                "### Intent Postcondition Invariant\n\
                Asserts constraints that must hold upon function return."
                    .to_string(),
            ),
            "M3" => Some(
                "### Google Material 3 UI Engine\n\
                Generates dynamic color harmonies, surface elevations, and responsive card widgets."
                    .to_string(),
            ),
            "QuantumSimulator" | "qubit" => Some(
                "### AETHER Quantum State Vector Simulator\n\
                Native Unitary Matrix simulation engine with Bloch sphere projections.\n\
                * `sim.hadamard(target)`: Applies Hadamard gate\n\
                * `sim.cnot(control, target)`: Entangles two qubits\n\
                * `sim.measure_all()`: Collapses quantum wave function"
                    .to_string(),
            ),
            "Tensor" => Some(
                "### AETHER Tensor & Autograd Engine\n\
                First-class n-dimensional array primitive with reverse-mode automatic differentiation.\n\
                * `Tensor.randn(shape, requires_grad=True)`\n\
                * `tensor.matmul(other)`\n\
                * `loss.backward()`: Computes analytic gradients"
                    .to_string(),
            ),
            "Graph" => Some(
                "### AetherGraph Property Knowledge Engine\n\
                High-performance in-memory graph traversal and centrality engine.\n\
                * `graph.dijkstra(from, to)`: Shortest path routing\n\
                * `graph.pagerank(damping, iterations)`: Authority ranking"
                    .to_string(),
            ),
            "LiveReloader" => Some(
                "### AetherLive Zero-Downtime Hot Code Reloader\n\
                Live code swapping preserving process heap state."
                    .to_string(),
            ),
            "spawn" | "fiber" => Some(
                "### Fiber Concurrency\n\
                Lightweight green thread scheduled across work-stealing threadpools."
                    .to_string(),
            ),
            other => {
                // Fall back to user-defined declarations across open workspace docs
                for (uri, doc) in &self.documents {
                    if let Some((line_idx, _)) = find_definition_in_doc(doc, other) {
                        if let Some(sig_line) = doc.lines().nth(line_idx) {
                            return Some(format!(
                                "```aether\n{}\n```\n*Defined in `{}` (line {})*",
                                sig_line.trim(),
                                uri,
                                line_idx + 1
                            ));
                        }
                    }
                }
                None
            }
        }
    }

    /// Runs the standard JSON-RPC 2.0 LSP event loop over stdin/stdout
    pub fn run_stdio_server(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut stdout = io::stdout();

        loop {
            let mut line = String::new();
            let mut content_length: usize = 0;

            loop {
                line.clear();
                if handle.read_line(&mut line)? == 0 {
                    return Ok(());
                }
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    break;
                }
                if let Some((k, v)) = trimmed.split_once(':') {
                    if k.trim().eq_ignore_ascii_case("Content-Length") {
                        content_length = v.trim().parse::<usize>().unwrap_or(0);
                    }
                }
            }

            if content_length == 0 {
                continue;
            }

            let mut buffer = vec![0u8; content_length];
            handle.read_exact(&mut buffer)?;
            let payload = String::from_utf8_lossy(&buffer);

            if let Some(response) = self.handle_jsonrpc(&payload) {
                let resp_bytes = response.as_bytes();
                write!(
                    stdout,
                    "Content-Length: {}\r\n\r\n{}",
                    resp_bytes.len(),
                    response
                )?;
                stdout.flush()?;
            }
        }
    }

    /// Parses and routes JSON-RPC 2.0 messages using `serde_json`
    pub fn handle_jsonrpc(&mut self, payload: &str) -> Option<String> {
        let msg: JsonValue = serde_json::from_str(payload).ok()?;
        let method = msg.get("method")?.as_str()?;
        let id = msg.get("id").cloned().unwrap_or(JsonValue::Null);
        let params = msg.get("params").cloned().unwrap_or(JsonValue::Null);

        match method {
            "initialize" => Some(
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "capabilities": {
                            "textDocumentSync": 1,
                            "hoverProvider": true,
                            "definitionProvider": true,
                            "documentSymbolProvider": true,
                            "completionProvider": {
                                "triggerCharacters": [".", ":"]
                            }
                        }
                    }
                })
                .to_string(),
            ),
            "initialized" => None,
            "shutdown" => Some(json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string()),
            "textDocument/didOpen" => {
                let uri = params
                    .pointer("/textDocument/uri")
                    .or_else(|| params.get("uri"))
                    .and_then(|v| v.as_str())?
                    .to_string();
                let text = params
                    .pointer("/textDocument/text")
                    .or_else(|| params.get("text"))
                    .and_then(|v| v.as_str())?
                    .to_string();
                let diags = self.analyze(&text);
                self.documents.insert(uri.clone(), text);
                Some(format_publish_diagnostics(&uri, &diags))
            }
            "textDocument/didChange" => {
                let uri = params
                    .pointer("/textDocument/uri")
                    .or_else(|| params.get("uri"))
                    .and_then(|v| v.as_str())?
                    .to_string();
                let text = params
                    .pointer("/contentChanges/0/text")
                    .or_else(|| params.pointer("/textDocument/text"))
                    .or_else(|| params.get("text"))
                    .and_then(|v| v.as_str())?
                    .to_string();
                let diags = self.analyze(&text);
                self.documents.insert(uri.clone(), text);
                Some(format_publish_diagnostics(&uri, &diags))
            }
            "textDocument/didClose" => {
                if let Some(uri) = params
                    .pointer("/textDocument/uri")
                    .or_else(|| params.get("uri"))
                    .and_then(|v| v.as_str())
                {
                    self.documents.remove(uri);
                    return Some(
                        json!({
                            "jsonrpc": "2.0",
                            "method": "textDocument/publishDiagnostics",
                            "params": {
                                "uri": uri,
                                "diagnostics": []
                            }
                        })
                        .to_string(),
                    );
                }
                None
            }
            "textDocument/hover" => {
                let word = params
                    .get("word")
                    .and_then(|w| w.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| self.word_from_params(&params));

                if let Some(w) = word {
                    if let Some(docs) = self.get_hover(&w) {
                        return Some(
                            json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "contents": {
                                        "kind": "markdown",
                                        "value": docs
                                    }
                                }
                            })
                            .to_string(),
                        );
                    }
                }
                Some(
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {"contents": []}
                    })
                    .to_string(),
                )
            }
            "textDocument/definition" => {
                let uri = params
                    .pointer("/textDocument/uri")
                    .or_else(|| params.get("uri"))
                    .and_then(|v| v.as_str());
                let pos = extract_json_position(&params);
                if let (Some(doc_uri), Some((line, col))) = (uri, pos) {
                    if let Some(doc) = self.documents.get(doc_uri) {
                        if let Some(word) = get_word_at_position(doc, line, col) {
                            // 1. Search current open document
                            if let Some((def_line, def_col)) = find_definition_in_doc(doc, &word) {
                                return Some(definition_location_json(
                                    &id,
                                    doc_uri,
                                    def_line,
                                    def_col,
                                    word.len(),
                                ));
                            }
                            // 2. Search other open workspace documents
                            for (other_uri, other_doc) in &self.documents {
                                if other_uri != doc_uri {
                                    if let Some((def_line, def_col)) =
                                        find_definition_in_doc(other_doc, &word)
                                    {
                                        return Some(definition_location_json(
                                            &id,
                                            other_uri,
                                            def_line,
                                            def_col,
                                            word.len(),
                                        ));
                                    }
                                }
                            }
                            // 3. Search imported `.ae` files referenced by this document
                            for import_path in collect_imported_ae_paths(doc_uri, doc) {
                                if let Ok(ext_src) = std::fs::read_to_string(&import_path) {
                                    if let Some((def_line, def_col)) =
                                        find_definition_in_doc(&ext_src, &word)
                                    {
                                        let target_uri = path_to_file_uri(&import_path);
                                        return Some(definition_location_json(
                                            &id,
                                            &target_uri,
                                            def_line,
                                            def_col,
                                            word.len(),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
                Some(json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string())
            }
            "textDocument/documentSymbol" => {
                let mut symbols = Vec::new();
                if let Some(uri) = params
                    .pointer("/textDocument/uri")
                    .or_else(|| params.get("uri"))
                    .and_then(|v| v.as_str())
                {
                    if let Some(doc) = self.documents.get(uri) {
                        for (line_idx, name, kind) in extract_document_symbols(doc) {
                            symbols.push(json!({
                                "name": name,
                                "kind": kind,
                                "location": {
                                    "uri": uri,
                                    "range": {
                                        "start": {"line": line_idx, "character": 0},
                                        "end": {"line": line_idx, "character": name.len()}
                                    }
                                }
                            }));
                        }
                    }
                }
                Some(json!({"jsonrpc": "2.0", "id": id, "result": symbols}).to_string())
            }
            "textDocument/completion" => {
                let items = self.build_completion_items(&params, payload);
                Some(
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "isIncomplete": false,
                            "items": items
                        }
                    })
                    .to_string(),
                )
            }
            _ => None,
        }
    }

    fn word_from_params(&self, params: &JsonValue) -> Option<String> {
        let uri = params
            .pointer("/textDocument/uri")
            .or_else(|| params.get("uri"))
            .and_then(|v| v.as_str())?;
        let doc = self.documents.get(uri)?;
        let (line, ch) = extract_json_position(params)?;
        get_word_at_position(doc, line, ch)
    }

    fn build_completion_items(&self, params: &JsonValue, raw_payload: &str) -> Vec<JsonValue> {
        let prefix = self
            .member_prefix_at_cursor(params)
            .unwrap_or_else(|| detect_member_prefix_fallback(raw_payload));

        if let Some(ns) = prefix.as_deref() {
            return member_completions(ns);
        }

        let kws = [
            ("fn", 14, "Function Declaration"),
            ("def", 14, "Pythonic Function Declaration"),
            ("let", 14, "Immutable Variable"),
            ("mut", 14, "Mutable Variable Modifier"),
            ("intent", 14, "Formal Contract Invariant Block"),
            ("require", 14, "Intent Precondition Requirement"),
            ("ensure", 14, "Intent Postcondition Guarantee"),
            ("struct", 7, "Custom Composite Struct Type"),
            ("class", 7, "Object-Oriented Class Definition"),
            ("enum", 10, "Algebraic Data Type Enum"),
            ("trait", 8, "Interface Trait Definition"),
            ("match", 14, "Pattern Matching Expression"),
            ("test", 14, "Native Test Block"),
            ("try", 14, "Exception Handling Block"),
            ("catch", 14, "Exception Handler"),
            ("spawn", 14, "Spawn Concurrent Green Fiber"),
            ("return", 14, "Return Value from Function"),
            ("Bio", 9, "Bioinformatics & Sequence Alignment Module"),
            ("Chem", 9, "Cheminformatics & Drug Discovery Module"),
            ("Http", 9, "Pure-Rust HTTP/HTTPS Client & Server"),
            ("WebSocket", 9, "RFC 6455 WebSocket Module"),
            ("Json", 9, "JSON Parser & Serializer"),
            ("File", 9, "File System I/O Module"),
            ("AI", 9, "Native On-Device AI & Embeddings Module"),
            ("AetherBridge", 9, "Hardware Bridge & Native Mobile API"),
            ("DB", 9, "Embedded Relational SQL & DataFrame Engine"),
            (
                "ML",
                9,
                "Pure-Rust Classical Machine Learning & Metrics Module",
            ),
            (
                "NN",
                9,
                "Pure-Rust Deep Learning Autograd, Attention & Safetensors Module",
            ),
            (
                "LLM",
                9,
                "Pure-Rust BPE Tokenizer, GGUF Quantized Loader & Local LLM Engine",
            ),
            (
                "GPU",
                9,
                "Cross-Platform Hardware GPU Compute Engine (wgpu DX12/Vulkan/Metal)",
            ),
            (
                "Sandbox",
                9,
                "Capability-Based Scoped Sandbox & Zero-Trust Agent Execution",
            ),
            (
                "Bundle",
                9,
                "Standalone Single-Binary Embedded Modules & Assets Runtime",
            ),
            (
                "Matrix",
                9,
                "Native Multi-Core Tensor & Linear Algebra Module",
            ),
            ("Tensor", 9, "Native Tensor & Autograd Engine"),
            ("Graph", 9, "Knowledge Graph Traversal Engine"),
            ("M3", 9, "Material 3 Dynamic UI Engine"),
        ];

        let mut items: Vec<JsonValue> = kws
            .iter()
            .map(|(lbl, kind, detail)| json!({"label": lbl, "kind": kind, "detail": detail}))
            .collect();

        let mut seen: HashSet<String> = kws.iter().map(|(l, _, _)| (*l).to_string()).collect();
        for doc in self.documents.values() {
            for (_, name, kind) in extract_document_symbols(doc) {
                if seen.insert(name.clone()) {
                    items.push(json!({"label": name, "kind": kind, "detail": "Workspace Symbol"}));
                }
            }
            for line in doc.lines() {
                if let Some(rest) = line.trim().strip_prefix("let ") {
                    let var_name = rest
                        .split('=')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .trim_start_matches("mut ")
                        .trim();
                    if !var_name.is_empty() && seen.insert(var_name.to_string()) {
                        items.push(
                            json!({"label": var_name, "kind": 6, "detail": "Local Variable"}),
                        );
                    }
                }
            }
        }
        items
    }

    fn member_prefix_at_cursor(&self, params: &JsonValue) -> Option<Option<String>> {
        let uri = params
            .pointer("/textDocument/uri")
            .or_else(|| params.get("uri"))
            .and_then(|v| v.as_str())?;
        let doc = self.documents.get(uri)?;
        let (line_idx, col) = extract_json_position(params)?;
        let line = doc.lines().nth(line_idx)?;
        let up_to_cursor: String = line.chars().take(col).collect();
        let trimmed = up_to_cursor.trim_end();
        if let Some(before_dot) = trimmed.strip_suffix('.') {
            let ns: String = before_dot
                .chars()
                .rev()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            if !ns.is_empty() {
                return Some(Some(ns));
            }
        }
        Some(None)
    }
}

fn detect_member_prefix_fallback(raw_payload: &str) -> Option<String> {
    for ns in [
        "Bio",
        "Chem",
        "Http",
        "WebSocket",
        "Json",
        "File",
        "AI",
        "ML",
        "NN",
        "LLM",
        "GPU",
        "Matrix",
        "AetherBridge",
        "DB",
    ] {
        if raw_payload.contains(&format!("{}.", ns)) {
            return Some(ns.to_string());
        }
    }
    None
}

fn member_completions(ns: &str) -> Vec<JsonValue> {
    let entries: &[(&str, usize, &str)] = match ns {
        "Bio" => &[
            ("dna", 2, "Bio.dna(seq) -> PackedDna"),
            ("gc_content", 2, "Bio.gc_content(dna) -> Float"),
            (
                "reverse_complement",
                2,
                "Bio.reverse_complement(dna) -> PackedDna",
            ),
            (
                "parse_fasta",
                2,
                "Bio.parse_fasta(text) -> Array<FastaRecord>",
            ),
            (
                "parse_fastq",
                2,
                "Bio.parse_fastq(text) -> Array<FastqRead>",
            ),
            (
                "filter_fastq",
                2,
                "Bio.filter_fastq(reads, min_phred, min_len) -> Array<FastqRead>",
            ),
            (
                "align",
                2,
                "Bio.align(seq1, seq2, [match, mismatch, gap]) -> Alignment",
            ),
            (
                "orf_finder",
                2,
                "Bio.orf_finder(dna, [min_aa_len]) -> Array<OrfHit>",
            ),
            ("parse_pdb", 2, "Bio.parse_pdb(pdb_text) -> Structure"),
        ],
        "Chem" => &[
            ("from_smiles", 2, "Chem.from_smiles(smiles) -> Molecule"),
            (
                "parse_sdf",
                2,
                "Chem.parse_sdf(sdf_text) -> Array<SdfMolecule>",
            ),
            (
                "substructure_match",
                2,
                "Chem.substructure_match(mol_or_smiles, pattern) -> Bool",
            ),
            (
                "tanimoto_similarity",
                2,
                "Chem.tanimoto_similarity(fp1, fp2) -> Float",
            ),
            (
                "predict_binding_affinity",
                2,
                "Chem.predict_binding_affinity(mol_or_fp, weights, bias) -> AffinityResult",
            ),
        ],
        "Http" => &[
            ("get", 2, "Http.get(url, [headers]) -> Response"),
            ("post", 2, "Http.post(url, body, [headers]) -> Response"),
            (
                "request",
                2,
                "Http.request(method, url, body, [headers]) -> Response",
            ),
            ("listen", 2, "Http.listen(host, port) -> server_id"),
            (
                "accept",
                2,
                "Http.accept(server_id, [timeout_ms]) -> Request",
            ),
        ],
        "WebSocket" => &[
            ("connect", 2, "WebSocket.connect(url) -> ws_id"),
            ("upgrade", 2, "WebSocket.upgrade(req) -> ws_id"),
            ("send", 2, "WebSocket.send(ws_id, data) -> Bool"),
            ("recv", 2, "WebSocket.recv(ws_id, [timeout_ms]) -> Frame"),
            ("close", 2, "WebSocket.close(ws_id) -> Bool"),
        ],
        "Json" => &[
            ("parse", 2, "Json.parse(json_str) -> Value"),
            ("stringify", 2, "Json.stringify(val) -> String"),
        ],
        "File" => &[
            ("read", 2, "File.read(path) -> String"),
            ("write", 2, "File.write(path, content) -> Bool"),
            ("exists", 2, "File.exists(path) -> Bool"),
        ],
        "AI" => &[
            ("embed", 2, "AI.embed(text) -> 128-D Dense Vector"),
            ("similarity", 2, "AI.similarity(v1, v2) -> Float Cosine"),
            (
                "rag_search",
                2,
                "AI.rag_search(query, docs, top_k) -> Array",
            ),
            ("chat", 2, "AI.chat(prompt) -> String"),
            ("Agent", 7, "AI.Agent(name, role) -> Local Agent"),
        ],
        "AetherBridge" => &[
            ("getLocation", 2, "AetherBridge.getLocation() -> Location"),
            ("scanQR", 2, "AetherBridge.scanQR() -> QRResult"),
            (
                "scheduleAlarm",
                2,
                "AetherBridge.scheduleAlarm(title, timeMs)",
            ),
            (
                "showNotification",
                2,
                "AetherBridge.showNotification(title, body)",
            ),
            ("pickFile", 2, "AetherBridge.pickFile(accept) -> File"),
            ("isDarkMode", 2, "AetherBridge.isDarkMode() -> Bool"),
        ],
        "DB" => &[
            ("open", 2, "DB.open(path) -> DatabaseHandle"),
            ("execute", 2, "db.execute(sql) -> Result"),
            ("query", 2, "db.query(sql) -> Array<Map>"),
        ],
        "ML" => &[
            (
                "train_test_split",
                2,
                "ML.train_test_split(X, y, [test_ratio, seed]) -> SplitMap",
            ),
            (
                "standard_scaler_fit",
                2,
                "ML.standard_scaler_fit(X) -> scaler_id",
            ),
            (
                "minmax_scaler_fit",
                2,
                "ML.minmax_scaler_fit(X) -> scaler_id",
            ),
            (
                "linear_regression_fit",
                2,
                "ML.linear_regression_fit(X, y, [l2]) -> model_id",
            ),
            (
                "logistic_regression_fit",
                2,
                "ML.logistic_regression_fit(X, y, [lr, epochs, l2]) -> model_id",
            ),
            (
                "decision_tree_fit",
                2,
                "ML.decision_tree_fit(X, y, [max_depth, min_samples_split]) -> model_id",
            ),
            (
                "random_forest_fit",
                2,
                "ML.random_forest_fit(X, y, [n_estimators, max_depth, seed]) -> model_id",
            ),
            ("knn_fit", 2, "ML.knn_fit(X, y, [k]) -> model_id"),
            (
                "kmeans_fit",
                2,
                "ML.kmeans_fit(X, k, [max_iter, seed]) -> model_id",
            ),
            ("pca_fit", 2, "ML.pca_fit(X, n_components) -> model_id"),
            ("predict", 2, "ML.predict(model_id, X) -> Array"),
            (
                "predict_proba",
                2,
                "ML.predict_proba(model_id, X) -> Array<Array<Float>>",
            ),
            (
                "transform",
                2,
                "ML.transform(model_id, X) -> Array<Array<Float>>",
            ),
            ("accuracy", 2, "ML.accuracy(y_true, y_pred) -> Float"),
            (
                "precision_recall_f1",
                2,
                "ML.precision_recall_f1(y_true, y_pred, [pos_label]) -> MetricsMap",
            ),
            (
                "confusion_matrix",
                2,
                "ML.confusion_matrix(y_true, y_pred) -> ConfusionMap",
            ),
            ("mse", 2, "ML.mse(y_true, y_pred) -> Float"),
            ("rmse", 2, "ML.rmse(y_true, y_pred) -> Float"),
            ("r2_score", 2, "ML.r2_score(y_true, y_pred) -> Float"),
            ("roc_auc", 2, "ML.roc_auc(y_true, y_scores) -> Float"),
            ("save_model", 2, "ML.save_model(model_id, path) -> Bool"),
            ("load_model", 2, "ML.load_model(path) -> model_id"),
        ],
        "NN" => &[
            (
                "var",
                2,
                "NN.var(tensor_or_array, [requires_grad]) -> node_id",
            ),
            ("data", 2, "NN.data(node_id) -> Tensor"),
            ("grad", 2, "NN.grad(node_id) -> Tensor | Nil"),
            ("matmul", 2, "NN.matmul(a_id, b_id) -> node_id"),
            ("gelu", 2, "NN.gelu(node_id) -> node_id"),
            ("silu", 2, "NN.silu(node_id) -> node_id"),
            (
                "layer_norm",
                2,
                "NN.layer_norm(x_id, gamma_id, beta_id, [eps]) -> node_id",
            ),
            (
                "embedding",
                2,
                "NN.embedding(table_id, token_indices) -> node_id",
            ),
            (
                "attention",
                2,
                "NN.attention(q_id, k_id, v_id, [causal]) -> node_id",
            ),
            (
                "cross_entropy",
                2,
                "NN.cross_entropy(logits_id, target_indices) -> loss_id",
            ),
            ("mse_loss", 2, "NN.mse_loss(pred_id, target_id) -> loss_id"),
            ("backward", 2, "NN.backward(loss_id) -> Bool"),
            (
                "adamw_create",
                2,
                "NN.adamw_create(params, [lr, beta1, beta2, eps, wd]) -> opt_id",
            ),
            ("adamw_step", 2, "NN.adamw_step(opt_id) -> Bool"),
            (
                "save_safetensors",
                2,
                "NN.save_safetensors(tensors_map, path) -> Bool",
            ),
            (
                "load_safetensors",
                2,
                "NN.load_safetensors(path) -> Map<String, Tensor>",
            ),
            (
                "set_device",
                2,
                "NN.set_device(\"gpu\" | \"cpu\") -> String",
            ),
            ("device", 2, "NN.device() -> String"),
        ],
        "LLM" => &[
            (
                "bpe_train",
                2,
                "LLM.bpe_train(texts, vocab_size, [special]) -> tok_id",
            ),
            ("bpe_load", 2, "LLM.bpe_load(path) -> tok_id"),
            ("bpe_save", 2, "LLM.bpe_save(tok_id, path) -> Bool"),
            ("bpe_from_gguf", 2, "LLM.bpe_from_gguf(gguf_id) -> tok_id"),
            (
                "bpe_encode",
                2,
                "LLM.bpe_encode(tok_id, text, [add_special]) -> Array<Int>",
            ),
            (
                "bpe_decode",
                2,
                "LLM.bpe_decode(tok_id, ids, [skip_special]) -> String",
            ),
            ("bpe_vocab_size", 2, "LLM.bpe_vocab_size(tok_id) -> Int"),
            ("load_gguf", 2, "LLM.load_gguf(path) -> gguf_id"),
            (
                "save_gguf",
                2,
                "LLM.save_gguf(path, metadata, tensors, [quant_mode]) -> Bool",
            ),
            ("gguf_metadata", 2, "LLM.gguf_metadata(gguf_id) -> Map"),
            (
                "gguf_tensor_names",
                2,
                "LLM.gguf_tensor_names(gguf_id) -> Array<String>",
            ),
            (
                "gguf_tensor_info",
                2,
                "LLM.gguf_tensor_info(gguf_id, name) -> Map",
            ),
            (
                "gguf_read_tensor",
                2,
                "LLM.gguf_read_tensor(gguf_id, name) -> Tensor",
            ),
            (
                "load_model_gguf",
                2,
                "LLM.load_model_gguf(path_or_id, [tok_id]) -> model_id",
            ),
            (
                "load_model_safetensors",
                2,
                "LLM.load_model_safetensors(path, config, [tok_id]) -> model_id",
            ),
            ("model_info", 2, "LLM.model_info(model_id) -> Map"),
            ("reset_cache", 2, "LLM.reset_cache(model_id) -> Bool"),
            (
                "forward_logits",
                2,
                "LLM.forward_logits(model_id, token_ids) -> Array<Float>",
            ),
            (
                "generate",
                2,
                "LLM.generate(model_id, prompt, [opts]) -> Map",
            ),
            (
                "embed",
                2,
                "LLM.embed(model_id, text_or_tokens, [pooling]) -> Array<Float>",
            ),
            (
                "cosine_similarity",
                2,
                "LLM.cosine_similarity(a, b) -> Float",
            ),
            (
                "generate_schema",
                2,
                "LLM.generate_schema(model_id, prompt, schema_map, [opts]) -> Map",
            ),
            (
                "generate_choices",
                2,
                "LLM.generate_choices(model_id, prompt, choices, [opts]) -> Map",
            ),
            (
                "snapshot_cache",
                2,
                "LLM.snapshot_cache(model_id) -> snap_id",
            ),
            (
                "restore_cache",
                2,
                "LLM.restore_cache(model_id, snap_id) -> Bool",
            ),
            ("cache_len", 2, "LLM.cache_len(model_id) -> Int"),
        ],
        "Sandbox" => &[
            ("run", 2, "Sandbox.run(source_code, [policy_map]) -> Map"),
            ("policy", 2, "Sandbox.policy() -> Map"),
            ("fuel_used", 2, "Sandbox.fuel_used() -> Int"),
        ],
        "Bundle" => &[
            ("is_standalone", 2, "Bundle.is_standalone() -> Bool"),
            ("modules", 2, "Bundle.modules() -> Array<String>"),
            ("assets", 2, "Bundle.assets() -> Array<String>"),
            ("has_asset", 2, "Bundle.has_asset(name) -> Bool"),
            ("read_text", 2, "Bundle.read_text(name) -> String"),
            (
                "extract_asset",
                2,
                "Bundle.extract_asset(name, dest) -> Bool",
            ),
        ],
        "GPU" => &[
            ("available", 2, "GPU.available() -> Bool"),
            ("info", 2, "GPU.info() -> Map"),
            (
                "set_tensor_accel",
                2,
                "GPU.set_tensor_accel(enabled) -> Bool",
            ),
            (
                "upload",
                2,
                "GPU.upload(tensor_or_array, [shape]) -> buf_id",
            ),
            ("download", 2, "GPU.download(buf_id) -> Tensor"),
            ("shape", 2, "GPU.shape(buf_id) -> Array<Int>"),
            ("free", 2, "GPU.free(buf_id) -> Bool"),
            ("matmul", 2, "GPU.matmul(a, b) -> Tensor"),
            ("matmul_buf", 2, "GPU.matmul_buf(a_id, b_id) -> out_id"),
            ("add", 2, "GPU.add(a, b) -> Tensor"),
            ("mul", 2, "GPU.mul(a, b) -> Tensor"),
            ("relu", 2, "GPU.relu(x) -> Tensor"),
            ("gelu", 2, "GPU.gelu(x) -> Tensor"),
            ("silu", 2, "GPU.silu(x) -> Tensor"),
            ("softmax", 2, "GPU.softmax(x) -> Tensor"),
            ("rmsnorm", 2, "GPU.rmsnorm(x, gamma, [eps]) -> Tensor"),
            (
                "layernorm",
                2,
                "GPU.layernorm(x, gamma, beta, [eps]) -> Tensor",
            ),
            ("attention", 2, "GPU.attention(q, k, v, [causal]) -> Tensor"),
            (
                "run_wgsl",
                2,
                "GPU.run_wgsl(src, inputs, out_len, workgroups, [entry]) -> Array<Float>",
            ),
            ("benchmark", 2, "GPU.benchmark([size]) -> Map"),
        ],
        "Matrix" => &[
            ("zeros", 2, "Matrix.zeros(shape) -> Tensor"),
            ("ones", 2, "Matrix.ones(shape) -> Tensor"),
            ("from_array", 2, "Matrix.from_array(arr, [shape]) -> Tensor"),
            ("arange", 2, "Matrix.arange(start, [stop, step]) -> Tensor"),
            ("linspace", 2, "Matrix.linspace(start, stop, num) -> Tensor"),
            ("matmul", 2, "Matrix.matmul(a, b) -> Tensor"),
            ("exp", 2, "Matrix.exp(t) -> Tensor"),
            ("log", 2, "Matrix.log(t) -> Tensor"),
            ("sqrt", 2, "Matrix.sqrt(t) -> Tensor"),
            ("gelu", 2, "Matrix.gelu(t) -> Tensor"),
            ("silu", 2, "Matrix.silu(t) -> Tensor"),
            ("argmax", 2, "Matrix.argmax(t, [axis]) -> Int | Tensor"),
            ("sum_axis", 2, "Matrix.sum_axis(t, axis) -> Tensor"),
            ("mean_axis", 2, "Matrix.mean_axis(t, axis) -> Tensor"),
        ],
        _ => &[],
    };
    entries
        .iter()
        .map(|(lbl, kind, detail)| json!({"label": lbl, "kind": kind, "detail": detail}))
        .collect()
}

fn format_publish_diagnostics(uri: &str, diags: &[LspDiagnostic]) -> String {
    let entries: Vec<JsonValue> = diags
        .iter()
        .map(|d| {
            json!({
                "range": {
                    "start": {"line": d.line, "character": d.col},
                    "end": {"line": d.line, "character": d.col + 5}
                },
                "severity": d.severity,
                "message": d.message
            })
        })
        .collect();
    json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": uri,
            "diagnostics": entries
        }
    })
    .to_string()
}

fn definition_location_json(
    id: &JsonValue,
    uri: &str,
    line: usize,
    col: usize,
    word_len: usize,
) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "uri": uri,
            "range": {
                "start": {"line": line, "character": col},
                "end": {"line": line, "character": col + word_len}
            }
        }
    })
    .to_string()
}

fn extract_json_position(params: &JsonValue) -> Option<(usize, usize)> {
    let pos = params.get("position").unwrap_or(params);
    let line = pos.get("line")?.as_u64()? as usize;
    let character = pos.get("character")?.as_u64()? as usize;
    Some((line, character))
}

fn get_word_at_position(doc: &str, target_line: usize, target_col: usize) -> Option<String> {
    let line = doc.lines().nth(target_line)?;
    let chars: Vec<char> = line.chars().collect();
    if target_col >= chars.len() {
        return None;
    }

    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    if !is_ident(chars[target_col]) {
        return None;
    }

    let mut start = target_col;
    while start > 0 && is_ident(chars[start - 1]) {
        start -= 1;
    }

    let mut end = target_col;
    while end < chars.len() && is_ident(chars[end]) {
        end += 1;
    }

    let word: String = chars[start..end].iter().collect();
    if word.is_empty() {
        None
    } else {
        Some(word)
    }
}

fn extract_document_symbols(doc: &str) -> Vec<(usize, String, usize)> {
    let mut out = Vec::new();
    for (line_idx, line) in doc.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed
            .strip_prefix("fn ")
            .or_else(|| trimmed.strip_prefix("def "))
        {
            let name = rest.split('(').next().unwrap_or("").trim();
            if !name.is_empty() {
                out.push((line_idx, name.to_string(), 12));
            }
        } else if let Some(rest) = trimmed
            .strip_prefix("struct ")
            .or_else(|| trimmed.strip_prefix("class "))
        {
            let name = rest.split([':', '{', '(']).next().unwrap_or("").trim();
            if !name.is_empty() {
                out.push((line_idx, name.to_string(), 23));
            }
        } else if let Some(rest) = trimmed.strip_prefix("enum ") {
            let name = rest.split(['<', '{', ':']).next().unwrap_or("").trim();
            if !name.is_empty() {
                out.push((line_idx, name.to_string(), 10));
            }
        } else if let Some(rest) = trimmed.strip_prefix("intent ") {
            let name = rest.split(['{', ':']).next().unwrap_or("").trim();
            if !name.is_empty() {
                out.push((line_idx, name.to_string(), 11));
            }
        }
    }
    out
}

fn find_definition_in_doc(doc: &str, word: &str) -> Option<(usize, usize)> {
    for (line_idx, line) in doc.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed
            .strip_prefix("fn ")
            .or_else(|| trimmed.strip_prefix("def "))
        {
            let func_name = rest.split('(').next().unwrap_or("").trim();
            if func_name == word {
                let col = line.find(word).unwrap_or(0);
                return Some((line_idx, col));
            }
        }
        if let Some(rest) = trimmed
            .strip_prefix("struct ")
            .or_else(|| trimmed.strip_prefix("class "))
            .or_else(|| trimmed.strip_prefix("enum "))
            .or_else(|| trimmed.strip_prefix("intent "))
        {
            let decl_name = rest
                .split([':', '{', '(', '<', ' '])
                .next()
                .unwrap_or("")
                .trim();
            if decl_name == word {
                let col = line.find(word).unwrap_or(0);
                return Some((line_idx, col));
            }
        }
        if let Some(rest) = trimmed.strip_prefix("let ") {
            let var_name = rest
                .split('=')
                .next()
                .unwrap_or("")
                .trim()
                .trim_start_matches("mut ")
                .trim();
            if var_name == word {
                let col = line.find(word).unwrap_or(0);
                return Some((line_idx, col));
            }
        }
    }
    None
}

fn collect_imported_ae_paths(current_uri: &str, doc: &str) -> Vec<PathBuf> {
    let base_dir = file_uri_to_path(current_uri)
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let mut paths = Vec::new();
    for line in doc.lines() {
        let trimmed = line.trim();
        let mod_name = if let Some(rest) = trimmed.strip_prefix("from ") {
            rest.split_whitespace().next()
        } else if let Some(rest) = trimmed.strip_prefix("import ") {
            rest.split([';', ' ']).next()
        } else {
            None
        };
        if let Some(name) = mod_name {
            let file_name = if name.ends_with(".ae") {
                name.to_string()
            } else {
                format!("{}.ae", name.trim_end_matches(';'))
            };
            for candidate in [
                base_dir.join(&file_name),
                Path::new("libraries").join(&file_name),
            ] {
                if candidate.is_file() {
                    paths.push(candidate);
                }
            }
        }
    }
    paths
}

fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let stripped = uri
        .strip_prefix("file:///")
        .or_else(|| uri.strip_prefix("file://"))?;
    Some(PathBuf::from(stripped))
}

fn path_to_file_uri(path: &Path) -> String {
    let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let norm = abs.to_string_lossy().replace('\\', "/");
    if norm.starts_with('/') {
        format!("file://{}", norm)
    } else {
        format!("file:///{}", norm.trim_start_matches("//?/"))
    }
}
