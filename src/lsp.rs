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
                if let Some(word) = extract_hover_word(payload) {
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
            "textDocument/completion" => {
                let completions = "{\"isIncomplete\":false,\"items\":[\
                    {\"label\":\"intent\",\"kind\":14,\"detail\":\"Intent Contract Block\"},\
                    {\"label\":\"Tensor\",\"kind\":7,\"detail\":\"Tensor Autograd Engine\"},\
                    {\"label\":\"Graph\",\"kind\":7,\"detail\":\"AetherGraph Knowledge Engine\"},\
                    {\"label\":\"QuantumSimulator\",\"kind\":7,\"detail\":\"Quantum Matrix Simulator\"},\
                    {\"label\":\"LiveReloader\",\"kind\":7,\"detail\":\"Zero-Downtime Hot Reload\"},\
                    {\"label\":\"spawn\",\"kind\":14,\"detail\":\"Spawn Green Fiber\"}\
                ]}";
                Some(format_jsonrpc_response(id, completions))
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
