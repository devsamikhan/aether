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
    } else {
        send_response(&mut stream, 404, "text/plain", b"Asset not found.")
    }
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
