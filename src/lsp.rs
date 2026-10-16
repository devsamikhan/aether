// ==============================================================================
// AETHER 2.0 Language Server Protocol (LSP) Engine
// JSON-RPC 2.0 Language Server for VS Code, Cursor & Modern IDEs
// Pure Rust Standard Library (Zero Third-Party Crates)
// ==============================================================================

use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};

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

        // 1. Syntax Parsing Pass
        match crate::syntax::parse(source) {
            Ok(_) => {
                // Syntax valid - can perform semantic and contract checks
            }
            Err((err_msg, span)) => {
                diagnostics.push(LspDiagnostic {
                    line: if span.line > 0 { span.line - 1 } else { 0 },
                    col: if span.col > 0 { span.col - 1 } else { 0 },
                    message: format!("Syntax error: {}", err_msg),
                    severity: 1, // Error
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
                * `AI.chat(prompt)`: Chats with default on-device agent".to_string(),
            ),
            "AetherBridge" => Some(
                "### AetherBridge Native Mobile & Web Substrate\n\
                Bidirectional hardware bridge connecting frontends directly to mobile hardware:\n\
                * `AetherBridge.getLocation()`: Real-time GPS coordinates\n\
                * `AetherBridge.scanQR()`: Camera viewfinder barcode/QR scanner\n\
                * `AetherBridge.scheduleAlarm(title, timeMs)`: Native exact OS alarm\n\
                * `AetherBridge.showNotification(title, body)`: System push notifications\n\
                * `AetherBridge.pickFile(accept)`: Native file picker\n\
                * `AetherBridge.isDarkMode()`: System theme detection".to_string(),
            ),
            "DB" => Some(
                "### AETHER Embedded Relational SQL Engine\n\
                In-memory or file-backed ACID relational tables with high-speed query execution:\n\
                * `DB.open(path)`: Opens SQLite or in-memory DB\n\
                * `db.execute(sql)`: Executes DDL/DML statements\n\
                * `db.query(sql)`: Returns array of result rows as maps".to_string(),
            ),
            "fn" | "def" => Some(
                "### Function Declaration\n\
                Defines a function with optional typed parameters and trailing expression returns.\n\
                ```aether\n\
                fn calculate_total(price: Float, qty: Int) -> Float:\n\
                    price * qty\n\
                ```".to_string(),
            ),
            "let" => Some(
                "### Variable Binding\n\
                Binds an immutable variable. Use `let mut` for mutable variables.".to_string(),
            ),
            "intent" => Some(
                "### AETHER Intent Contract\n\
                Declarative specification block bounding execution constraints and invariant schemas.\n\
                ```aether\n\
                intent TransferFunds {\n\
                    schema { amount: Float }\n\
                    invariants { require(amount > 0.0); }\n\
                }\n\
                ```"
                .to_string(),
            ),
            "require" => Some(
                "### Intent Precondition Invariant\n\
                Asserts constraints that must hold prior to intent function execution.".to_string(),
            ),
            "ensure" => Some(
                "### Intent Postcondition Invariant\n\
                Asserts constraints that must hold upon function return.".to_string(),
            ),
            "M3" => Some(
                "### Google Material 3 UI Engine\n\
                Generates dynamic color harmonies, surface elevations, and responsive card widgets.".to_string(),
            ),
            "QuantumSimulator" | "qubit" => Some(
                "### AETHER Quantum State Vector Simulator\n\
                Native Unitary Matrix simulation engine with Bloch sphere projections.\n\
                * `sim.hadamard(target)`: Applies Hadamard gate (Superposition)\n\
                * `sim.cnot(control, target)`: Entangles two qubits (Bell State)\n\
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
                BEAM-grade live code swapping preserving process heap state and active socket connections."
                    .to_string(),
            ),
            "spawn" | "fiber" => Some(
                "### Fiber Concurrency\n\
                Lightweight M:N green thread scheduled across work-stealing threadpools."
                    .to_string(),
            ),
            _ => None,
        }
    }

    /// Runs the standard JSON-RPC 2.0 LSP event loop over stdin/stdout
    pub fn run_stdio_server(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut stdout = io::stdout();

        loop {
            // Read HTTP-like Header: Content-Length: <N>\r\n\r\n
            let mut line = String::new();
            let mut content_length: usize = 0;

            loop {
                line.clear();
                if handle.read_line(&mut line)? == 0 {
                    return Ok(()); // EOF
                }
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    break;
                }
                if let Some(stripped) = trimmed.strip_prefix("Content-Length:") {
                    content_length = stripped.trim().parse::<usize>().unwrap_or(0);
                }
            }

            if content_length == 0 {
                continue;
            }

            // Read JSON Payload
            let mut buffer = vec![0u8; content_length];
            handle.read_exact(&mut buffer)?;
            let payload = String::from_utf8_lossy(&buffer);

            // Handle JSON-RPC Message
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

    /// Parses and routes JSON-RPC 2.0 messages
    pub fn handle_jsonrpc(&mut self, payload: &str) -> Option<String> {
        let (id, method) = parse_jsonrpc_request(payload)?;

        match method.as_str() {
            "initialize" => {
                let result = "{\"capabilities\":{\
                    \"textDocumentSync\":1,\
                    \"hoverProvider\":true,\
                    \"definitionProvider\":true,\
                    \"completionProvider\":{\"triggerCharacters\":[\".\",\":\"]}\
                }}";
                Some(format_jsonrpc_response(id, result))
            }
            "initialized" => None,
            "shutdown" => Some(format_jsonrpc_response(id, "null")),
            "textDocument/didOpen" => {
                if let Some((uri, text)) = extract_document_open(payload) {
                    let diags = self.analyze(&text);
                    self.documents.insert(uri.clone(), text);
                    Some(format_publish_diagnostics(&uri, &diags))
                } else {
                    None
                }
            }
            "textDocument/didChange" => {
                if let Some((uri, text)) = extract_document_change(payload) {
                    let diags = self.analyze(&text);
                    self.documents.insert(uri.clone(), text);
                    Some(format_publish_diagnostics(&uri, &diags))
                } else {
                    None
                }
            }
            "textDocument/hover" => {
                let mut word_opt = extract_hover_word(payload);
                if word_opt.is_none() {
                    if let Some(uri) = extract_json_string(payload, "uri") {
                        if let Some(doc) = self.documents.get(&uri) {
                            if let Some((line, char_idx)) = extract_position(payload) {
                                word_opt = get_word_at_position(doc, line, char_idx);
                            }
                        }
                    }
                }

                if let Some(word) = word_opt {
                    if let Some(docs) = self.get_hover(&word) {
                        let res = format!(
                            "{{\"contents\":{{\"kind\":\"markdown\",\"value\":{}}}}}",
                            escape_json(&docs)
                        );
                        return Some(format_jsonrpc_response(id, &res));
                    }
                }
                Some(format_jsonrpc_response(id, "{\"contents\":[]}"))
            }
            "textDocument/definition" => {
                if let Some(uri) = extract_json_string(payload, "uri") {
                    if let Some(doc) = self.documents.get(&uri) {
                        if let Some((line, char_idx)) = extract_position(payload) {
                            if let Some(word) = get_word_at_position(doc, line, char_idx) {
                                if let Some((def_line, def_col)) = find_definition_in_doc(doc, &word) {
                                    let res = format!(
                                        "{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":{},\"character\":{}}},\"end\":{{\"line\":{},\"character\":{}}}}}}}",
                                        uri, def_line, def_col, def_line, def_col + word.len()
                                    );
                                    return Some(format_jsonrpc_response(id, &res));
                                }
                            }
                        }
                    }
                }
                Some(format_jsonrpc_response(id, "null"))
            }
            "textDocument/completion" => {
                let mut items: Vec<String> = Vec::new();

                // Check for module member completion (e.g. AI. or AetherBridge.)
                if payload.contains("\"AI.\"") || payload.contains("AI.") {
                    items.push("{\"label\":\"embed\",\"kind\":2,\"detail\":\"AI.embed(text) -> 128-D Dense Vector\"}".to_string());
                    items.push("{\"label\":\"similarity\",\"kind\":2,\"detail\":\"AI.similarity(v1, v2) -> Float Cosine\"}".to_string());
                    items.push("{\"label\":\"rag_search\",\"kind\":2,\"detail\":\"AI.rag_search(query, docs, top_k) -> Array\"}".to_string());
                    items.push("{\"label\":\"chat\",\"kind\":2,\"detail\":\"AI.chat(prompt) -> String Offline LLM Reply\"}".to_string());
                    items.push("{\"label\":\"Agent\",\"kind\":7,\"detail\":\"AI.Agent(name, role) -> Local Intelligent Agent\"}".to_string());
                } else if payload.contains("AetherBridge.") {
                    items.push("{\"label\":\"getLocation\",\"kind\":2,\"detail\":\"AetherBridge.getLocation() -> Promise<Location>\"}".to_string());
                    items.push("{\"label\":\"scanQR\",\"kind\":2,\"detail\":\"AetherBridge.scanQR() -> Promise<QRResult>\"}".to_string());
                    items.push("{\"label\":\"scheduleAlarm\",\"kind\":2,\"detail\":\"AetherBridge.scheduleAlarm(title, timeMs)\"}".to_string());
                    items.push("{\"label\":\"showNotification\",\"kind\":2,\"detail\":\"AetherBridge.showNotification(title, body)\"}".to_string());
                    items.push("{\"label\":\"pickFile\",\"kind\":2,\"detail\":\"AetherBridge.pickFile(accept) -> Promise<File>\"}".to_string());
                    items.push("{\"label\":\"isDarkMode\",\"kind\":2,\"detail\":\"AetherBridge.isDarkMode() -> Promise<Bool>\"}".to_string());
                } else if payload.contains("DB.") {
                    items.push("{\"label\":\"open\",\"kind\":2,\"detail\":\"DB.open(path) -> DatabaseHandle\"}".to_string());
                    items.push("{\"label\":\"execute\",\"kind\":2,\"detail\":\"db.execute(sql) -> Result\"}".to_string());
                    items.push("{\"label\":\"query\",\"kind\":2,\"detail\":\"db.query(sql) -> Array<Map>\"}".to_string());
                } else {
                    // Core Keywords
                    let kws = [
                        ("fn", 14, "Function Declaration"),
                        ("let", 14, "Immutable Variable"),
                        ("mut", 14, "Mutable Variable Modifier"),
                        ("intent", 14, "Formal Contract Invariant Block"),
                        ("require", 14, "Intent Precondition Requirement"),
                        ("ensure", 14, "Intent Postcondition Guarantee"),
                        ("struct", 7, "Custom Composite Struct Type"),
                        ("match", 14, "Pattern Matching Expression"),
                        ("spawn", 14, "Spawn Concurrent M:N Green Fiber"),
                        ("channel", 14, "Thread-Safe Channel Primitive"),
                        ("return", 14, "Return Value from Function"),
                        ("AI", 9, "Native On-Device AI & Embeddings Module"),
                        ("AetherBridge", 9, "Hardware Bridge & Native Mobile API"),
                        ("DB", 9, "Embedded Relational SQL & DataFrame Engine"),
                        ("M3", 9, "Google Material 3 Dynamic UI Engine"),
                        ("Sys", 9, "Operating System Primitives"),
                        ("Mobile", 9, "Mobile Haptics & Notification Substrate"),
                    ];
                    for (lbl, kind, detail) in kws {
                        items.push(format!("{{\"label\":\"{}\",\"kind\":{},\"detail\":\"{}\"}}", lbl, kind, detail));
                    }

                    // Dynamically scan cached documents for user functions & variables
                    for doc in self.documents.values() {
                        for line in doc.lines() {
                            let trimmed = line.trim();
                            if let Some(rest) = trimmed.strip_prefix("fn ").or_else(|| trimmed.strip_prefix("def ")) {
                                let name = rest.split('(').next().unwrap_or("").trim();
                                if !name.is_empty() {
                                    items.push(format!("{{\"label\":\"{}\",\"kind\":3,\"detail\":\"User Function\"}}", name));
                                }
                            } else if let Some(rest) = trimmed.strip_prefix("let ") {
                                let name = rest.split('=').next().unwrap_or("").trim().trim_start_matches("mut ").trim();
                                if !name.is_empty() {
                                    items.push(format!("{{\"label\":\"{}\",\"kind\":6,\"detail\":\"Local Variable\"}}", name));
                                }
                            }
                        }
                    }
                }

                let completions = format!("{{\"isIncomplete\":false,\"items\":[{}]}}", items.join(","));
                Some(format_jsonrpc_response(id, &completions))
            }
            _ => None,
        }
    }
}

// ==============================================================================
// Pure JSON Helpers (Zero External Dependency)
// ==============================================================================

fn parse_jsonrpc_request(payload: &str) -> Option<(String, String)> {
    let method = extract_json_string(payload, "method")?;
    let id = extract_json_raw(payload, "id").unwrap_or_else(|| "null".to_string());
    Some((id, method))
}

fn format_jsonrpc_response(id: String, result_json: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{}}}",
        id, result_json
    )
}

fn format_publish_diagnostics(uri: &str, diags: &[LspDiagnostic]) -> String {
    let mut items = Vec::new();
    for d in diags {
        items.push(format!(
            "{{\"range\":{{\"start\":{{\"line\":{},\"character\":{}}},\"end\":{{\"line\":{},\"character\":{}}}}},\"severity\":{},\"message\":{}}}",
            d.line, d.col, d.line, d.col + 5, d.severity, escape_json(&d.message)
        ));
    }
    format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/publishDiagnostics\",\"params\":{{\"uri\":\"{}\",\"diagnostics\":[{}]}}}}",
        uri, items.join(",")
    )
}

fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":", key);
    let start_idx = json.find(&pattern)? + pattern.len();
    let rest = json[start_idx..].trim_start();
    if rest.starts_with('"') {
        let inside = &rest[1..];
        let end_idx = inside.find('"')?;
        Some(inside[..end_idx].to_string())
    } else {
        None
    }
}

fn extract_json_raw(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":", key);
    let start_idx = json.find(&pattern)? + pattern.len();
    let rest = json[start_idx..].trim_start();
    let mut end = 0;
    for c in rest.chars() {
        if c == ',' || c == '}' || c == ']' {
            break;
        }
        end += c.len_utf8();
    }
    Some(rest[..end].trim().to_string())
}

fn extract_document_open(payload: &str) -> Option<(String, String)> {
    let uri = extract_json_string(payload, "uri")?;
    let text = extract_json_string(payload, "text")?;
    Some((uri, text))
}

fn extract_document_change(payload: &str) -> Option<(String, String)> {
    let uri = extract_json_string(payload, "uri")?;
    let text = extract_json_string(payload, "text")?;
    Some((uri, text))
}

fn extract_hover_word(payload: &str) -> Option<String> {
    // If exact word is present in params or fallback
    for kw in &["intent", "QuantumSimulator", "Tensor", "Graph", "LiveReloader", "spawn"] {
        if payload.contains(kw) {
            return Some((*kw).to_string());
        }
    }
    None
}

fn escape_json(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn extract_position(payload: &str) -> Option<(usize, usize)> {
    let line_str = extract_json_raw(payload, "line")?;
    let char_str = extract_json_raw(payload, "character")?;
    let line = line_str.parse::<usize>().ok()?;
    let character = char_str.parse::<usize>().ok()?;
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

fn find_definition_in_doc(doc: &str, word: &str) -> Option<(usize, usize)> {
    for (line_idx, line) in doc.lines().enumerate() {
        let trimmed = line.trim();
        // fn foo(...) or def foo(...)
        if let Some(rest) = trimmed.strip_prefix("fn ").or_else(|| trimmed.strip_prefix("def ")) {
            let func_name = rest.split('(').next().unwrap_or("").trim();
            if func_name == word {
                let col = line.find(word).unwrap_or(0);
                return Some((line_idx, col));
            }
        }
        // struct Foo
        if let Some(rest) = trimmed.strip_prefix("struct ") {
            let struct_name = rest.split_whitespace().next().unwrap_or("").trim_end_matches('{').trim();
            if struct_name == word {
                let col = line.find(word).unwrap_or(0);
                return Some((line_idx, col));
            }
        }
        // let foo = ... or let mut foo = ...
        if let Some(rest) = trimmed.strip_prefix("let ") {
            let var_name = rest.split('=').next().unwrap_or("").trim().trim_start_matches("mut ").trim();
            if var_name == word {
                let col = line.find(word).unwrap_or(0);
                return Some((line_idx, col));
            }
        }
    }
    None
}

