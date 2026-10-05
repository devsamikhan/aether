// ==============================================================================
// AETHER 2.0 In-Browser Interactive Playground HTTP Server
// Serves Web UI & Provides Native Compiler/VM Code Execution Bridge (/api/run)
// Pure Rust Standard Library (Zero Third-Party Crates)
// ==============================================================================

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Instant;

pub fn start_playground_server(port: u16, open_browser: bool) -> Result<(), String> {
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr)
        .map_err(|e| format!("Failed to bind playground server to {}: {}", addr, e))?;

    let url = format!("http://localhost:{}", port);
    println!("================================================================================");
    println!("⚡ AETHER 2.0 INTERACTIVE WEB PLAYGROUND");
    println!("================================================================================");
    println!("  URL:         {}", url);
    println!("  Mode:        Live WebAssembly Sandbox + Native Bytecode Bridge");
    println!("  Server:      Running on {}", addr);
    println!("  Press Ctrl+C to terminate the playground server.");
    println!("================================================================================");

    if open_browser {
        let _ = thread::spawn(move || {
            thread::sleep(std::time::Duration::from_millis(300));
            if cfg!(windows) {
                let _ = Command::new("cmd").args(["/C", "start", &url]).spawn();
            } else if cfg!(target_os = "macos") {
                let _ = Command::new("open").arg(&url).spawn();
            } else {
                let _ = Command::new("xdg-open").arg(&url).spawn();
            }
        });
    }

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| {
                    let _ = handle_client(stream);
                });
            }
            Err(e) => eprintln!("Playground Connection Error: {}", e),
        }
    }

    Ok(())
}

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = [0u8; 8192];
    let bytes_read = stream.read(&mut buffer)?;
    if bytes_read == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let mut lines = request.lines();
    let request_line = lines.next().unwrap_or("");
    let parts: Vec<&str> = request_line.split_whitespace().collect();

    if parts.len() < 2 {
        return send_response(&mut stream, 400, "text/plain", b"Bad Request");
    }

    let method = parts[0];
    let path = parts[1];

    if method == "POST" && path == "/api/run" {
        // Find body after double newline
        if let Some(body_start) = request.find("\r\n\r\n") {
            let body = &request[body_start + 4..];
            let code = extract_code_from_json(body).unwrap_or_default();

            let start = Instant::now();
            let (out_str, res_str) = match crate::vm::run_source(&code) {
                Ok(val) => ("Execution succeeded.".to_string(), val.to_string()),
                Err(e) => (format!("Error: {}", e), "None".to_string()),
            };
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

            let json_resp = format!(
                "{{\"output\":{},\"result\":{},\"duration_ms\":{:.2}}}",
                escape_json(&out_str),
                escape_json(&res_str),
                elapsed_ms
            );

            return send_response(
                &mut stream,
                200,
                "application/json",
                json_resp.as_bytes(),
            );
        }
    }

    // Static Asset Resolution
    let clean_path = if path == "/" || path == "/index.html" {
        "website/playground/index.html"
    } else if path == "/playground.css" {
        "website/playground/playground.css"
    } else if path == "/playground.js" {
        "website/playground/playground.js"
    } else {
        return send_response(&mut stream, 404, "text/plain", b"404 Not Found");
    };

    if let Ok(content) = fs::read(Path::new(clean_path)) {
        let content_type = if clean_path.ends_with(".html") {
            "text/html; charset=utf-8"
        } else if clean_path.ends_with(".css") {
            "text/css; charset=utf-8"
        } else if clean_path.ends_with(".js") {
            "application/javascript; charset=utf-8"
        } else {
            "application/octet-stream"
        };
        send_response(&mut stream, 200, content_type, &content)
    } else if clean_path.ends_with(".html") {
        send_response(&mut stream, 200, "text/html; charset=utf-8", get_fallback_studio_html().as_bytes())
    } else {
        send_response(&mut stream, 404, "text/plain", b"Asset not found.")
    }
}

fn get_fallback_studio_html() -> &'static str {
    r###"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>AETHER 2.0 Web Studio & Live Playground</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body { margin: 0; font-family: 'Segoe UI', system-ui, sans-serif; background: #0c0e12; color: #e1e2e8; display: flex; flex-direction: column; height: 100vh; }
        header { display: flex; align-items: center; justify-content: space-between; padding: 12px 24px; background: #161922; border-bottom: 1px solid #282d3c; }
        .logo { font-size: 20px; font-weight: 700; color: #00F5FF; display: flex; align-items: center; gap: 8px; }
        .btn { background: #00F5FF; color: #00363d; font-weight: 600; padding: 8px 18px; border: none; border-radius: 20px; cursor: pointer; }
        .main-container { display: flex; flex: 1; overflow: hidden; }
        .editor-pane, .output-pane { flex: 1; display: flex; flex-direction: column; border-right: 1px solid #282d3c; }
        textarea { flex: 1; background: #12141c; color: #a5d6ff; font-family: 'Consolas', monospace; font-size: 14px; padding: 16px; border: none; resize: none; outline: none; }
        .output-box { flex: 1; background: #090b0e; color: #00F5FF; font-family: 'Consolas', monospace; font-size: 13px; padding: 16px; overflow-y: auto; white-space: pre-wrap; }
        .toolbar { padding: 8px 16px; background: #1a1e28; display: flex; gap: 12px; align-items: center; font-size: 13px; }
        select { background: #222734; color: #fff; border: 1px solid #333b4e; padding: 4px 8px; border-radius: 6px; }
    </style>
</head>
<body>
    <header>
        <div class="logo">⚡ AETHER 2.0 Web Studio & Playground</div>
        <button class="btn" onclick="runCode()">▶ Run Script (Ctrl+Enter)</button>
    </header>
    <div class="main-container">
        <div class="editor-pane">
            <div class="toolbar">
                <label>Template: </label>
                <select id="tpl" onchange="loadTemplate()">
                    <option value="arcade">🕹️ 01. CyberRunner 2088 Arcade</option>
                    <option value="saas">🧠 02. AetherBrain AI SaaS</option>
                    <option value="db">🗄️ 03. Relational Database & SQL</option>
                    <option value="mobile">📱 04. Material 3 Mobile UI</option>
                    <option value="quantum">⚛️ 05. Quantum Teleportation</option>
                </select>
            </div>
            <textarea id="code"></textarea>
        </div>
        <div class="output-pane">
            <div class="toolbar">🖥️ Live Execution Terminal & Diagnostics</div>
            <div class="output-box" id="output">Click 'Run Script' to execute AETHER code in the bytecode VM...</div>
        </div>
    </div>
    <script>
        const TPLS = {
            arcade: "print('🕹️ Loading CyberRunner 2088 Engine...')\nMobile.vibrate(30)\nMobile.show_toast('Energy Core Collected!')\nlet theme = M3.theme('#00F5FF', true)\nprint('Theme Primary: ' + theme['primary'])",
            saas: "let db = DB.open(':memory:')\ndb.execute('CREATE TABLE audit_logs (id INT, event TEXT);')\ndb.insert('audit_logs', {'id': 1, 'event': 'UserLogin'})\nprint('Audit Count: ' + to_string(db.count('audit_logs')))\nlet agent = AI.agent('DocAI', 'Knowledge Assistant')\nprint(agent['name'] + ' is active!')",
            db: "let db = DB.open(':memory:')\ndb.execute('CREATE TABLE products (name TEXT, price FLOAT);')\ndb.insert('products', {'name': 'Aether Pro', 'price': 99.0})\nlet rows = db.query('SELECT * FROM products WHERE price > 50;')\nprint('Results: ' + to_string(rows))",
            mobile: "Mobile.show_toast('Hello Android!')\nlet batt = Mobile.battery_level()\nlet info = Mobile.device_info()\nprint('Battery: ' + to_string(batt) + '% on ' + info['brand'])",
            quantum: "let q = Quantum.circuit(2)\nq.h(0)\nq.cnot(0, 1)\nlet state = q.measure()\nprint('Entangled State: ' + to_string(state))"
        };
        function loadTemplate() {
            const k = document.getElementById('tpl').value;
            document.getElementById('code').value = TPLS[k] || '';
        }
        async function runCode() {
            const out = document.getElementById('output');
            out.innerText = '⚡ Executing in AETHER VM...';
            try {
                const res = await fetch('/api/run', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ code: document.getElementById('code').value })
                });
                const data = await res.json();
                out.innerText = data.output + '\\nResult: ' + data.result + '\\nDuration: ' + data.duration_ms + ' ms';
            } catch (e) {
                out.innerText = 'Network error: ' + e;
            }
        }
        document.addEventListener('keydown', e => {
            if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') runCode();
        });
        loadTemplate();
    </script>
</body>
</html>"###
}

fn send_response(
    stream: &mut TcpStream,
    status_code: u16,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let status_line = match status_code {
        200 => "HTTP/1.1 200 OK",
        400 => "HTTP/1.1 400 Bad Request",
        404 => "HTTP/1.1 404 Not Found",
        _ => "HTTP/1.1 500 Internal Server Error",
    };

    let header = format!(
        "{}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        status_line,
        content_type,
        body.len()
    );

    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

fn extract_code_from_json(json: &str) -> Option<String> {
    let pattern = "\"code\":";
    let start_idx = json.find(pattern)? + pattern.len();
    let rest = json[start_idx..].trim_start();
    if rest.starts_with('"') {
        let inside = &rest[1..];
        let end_idx = inside.find('"')?;
        Some(inside[..end_idx].replace("\\n", "\n").replace("\\\"", "\""))
    } else {
        None
    }
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
