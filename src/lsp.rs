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
