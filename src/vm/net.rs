use super::value::Value;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

static NEXT_LISTENER_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

fn with_listeners<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<u64, TcpListener>) -> R,
{
    static LISTENERS: Mutex<Option<HashMap<u64, TcpListener>>> = Mutex::new(None);
    let mut guard = LISTENERS.lock();
    let map = guard.get_or_insert_with(HashMap::new);
    f(map)
}

fn with_streams<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<u64, TcpStream>) -> R,
{
    static STREAMS: Mutex<Option<HashMap<u64, TcpStream>>> = Mutex::new(None);
    let mut guard = STREAMS.lock();
    let map = guard.get_or_insert_with(HashMap::new);
    f(map)
}

pub struct WsConnection {
    pub stream: Arc<Mutex<TcpStream>>,
    pub is_client: bool,
}

static NEXT_WS_ID: AtomicU64 = AtomicU64::new(1);

fn with_websockets<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<u64, WsConnection>) -> R,
{
    static WEBSOCKETS: Mutex<Option<HashMap<u64, WsConnection>>> = Mutex::new(None);
    let mut guard = WEBSOCKETS.lock();
    let map = guard.get_or_insert_with(HashMap::new);
    f(map)
}

pub fn register_net_module(globals: &mut HashMap<String, Value>) {
    let mut http_module = HashMap::new();

    // 1. Http.get(url) -> map
    http_module.insert("get".to_string(), Value::Native("Http.get".into(), |args| {
        if args.is_empty() {
            return Err("Http.get(url) expects URL string".to_string());
        }
        let url_str = args[0].to_string();
        execute_http_client_request("GET", &url_str, "", None)
    }));

    // 2. Http.post(url, body) -> map
    http_module.insert("post".to_string(), Value::Native("Http.post".into(), |args| {
        if args.len() < 2 {
            return Err("Http.post(url, body) expects url and body string".to_string());
        }
        let url_str = args[0].to_string();
        let post_body = args[1].to_string();
        execute_http_client_request("POST", &url_str, &post_body, None)
    }));

    // 3. Http.request(method, url, body?, headers?) -> map
    http_module.insert("request".to_string(), Value::Native("Http.request".into(), |args| {
        if args.len() < 2 {
            return Err("Http.request(method, url, body?, headers?) expects at least method and url".to_string());
        }
        let method = args[0].to_string().to_uppercase();
        let url_str = args[1].to_string();
        let body_str = if args.len() > 2 { args[2].to_string() } else { String::new() };
        let custom_headers = if args.len() > 3 {
            match &args[3] {
                Value::Map(m) => {
                    let mut h = HashMap::new();
                    for (k, v) in m.lock().iter() {
                        h.insert(k.clone(), v.to_string());
                    }
                    Some(h)
                }
                _ => None,
            }
        } else {
            None
        };
        execute_http_client_request(&method, &url_str, &body_str, custom_headers)
    }));

    // 4. Http.serve_response(port, response_body) -> serves one static HTTP response
    http_module.insert("serve_response".to_string(), Value::Native("Http.serve_response".into(), |args| {
        let port = if !args.is_empty() {
            match args[0] {
                Value::Int(p) => p as u16,
                _ => 8080,
            }
        } else {
            8080
        };

        let response_body = if args.len() > 1 {
            args[1].to_string()
        } else {
            "Hello from Project AETHER Native HTTP Server!".to_string()
        };

        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
            .map_err(|e| format!("Failed to bind port {}: {}", port, e))?;
        listener.set_nonblocking(false).map_err(|e| e.to_string())?;

        let (mut stream, _) = listener.accept().map_err(|e| e.to_string())?;
        let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);

        let mut req_line = String::new();
        reader.read_line(&mut req_line).map_err(|e| e.to_string())?;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        stream.write_all(response.as_bytes()).map_err(|e| e.to_string())?;
        stream.flush().map_err(|e| e.to_string())?;

        let mut res = HashMap::new();
        res.insert("served".to_string(), Value::Bool(true));
        res.insert("request_line".to_string(), Value::string(req_line.trim()));
        res.insert("port".to_string(), Value::Int(port as i64));

        Ok(Value::map(res))
    }));

    // 5. Http.listen(port | host, port) -> listener_id
    http_module.insert("listen".to_string(), Value::Native("Http.listen".into(), |args| {
        let (host, port) = if args.len() >= 2 {
            let h = args[0].to_string();
            let p = match args[1] {
                Value::Int(p) => p as u16,
                _ => 8080,
            };
            (h, p)
        } else if !args.is_empty() {
            match &args[0] {
                Value::Int(p) => ("127.0.0.1".to_string(), *p as u16),
                Value::String(hp) => {
                    if let Some((h, p)) = hp.split_once(':') {
                        (h.to_string(), p.parse::<u16>().unwrap_or(8080))
                    } else {
                        (hp.as_str().to_string(), 8080)
                    }
                }
                _ => ("127.0.0.1".to_string(), 8080),
            }
        } else {
            ("127.0.0.1".to_string(), 8080)
        };

        let listener = TcpListener::bind(format!("{}:{}", host, port))
            .map_err(|e| format!("Failed to bind to {}:{}: {}", host, port, e))?;
        
        let id = NEXT_LISTENER_ID.fetch_add(1, Ordering::SeqCst);
        with_listeners(|map| map.insert(id, listener));

        Ok(Value::Int(id as i64))
    }));

    // 6. Http.accept(listener_id, timeout_ms?) -> request map
    http_module.insert("accept".to_string(), Value::Native("Http.accept".into(), |args| {
        if args.is_empty() {
            return Err("Http.accept(listener_id, timeout_ms?) expects listener id".to_string());
        }
        let listener_id = match args[0] {
            Value::Int(id) => id as u64,
            _ => return Err("Listener ID must be an integer".to_string()),
        };

        let timeout_opt = if args.len() > 1 {
            match args[1] {
                Value::Int(ms) if ms > 0 => Some(Duration::from_millis(ms as u64)),
                _ => None,
            }
        } else {
            None
        };

        let listener_clone = with_listeners(|map| {
            map.get(&listener_id).and_then(|l| l.try_clone().ok())
        }).ok_or_else(|| format!("Invalid or closed listener id: {}", listener_id))?;

        if let Some(t) = timeout_opt {
            listener_clone.set_nonblocking(true).map_err(|e| e.to_string())?;
            let start = std::time::Instant::now();
            loop {
                match listener_clone.accept() {
                    Ok((stream, _)) => {
                        let _ = listener_clone.set_nonblocking(false);
                        let _ = stream.set_nonblocking(false);
                        return parse_incoming_http_request(stream);
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if start.elapsed() >= t {
                            let _ = listener_clone.set_nonblocking(false);
                            return Ok(Value::Nil);
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(e) => {
                        let _ = listener_clone.set_nonblocking(false);
                        return Err(format!("Accept error: {}", e));
                    }
                }
            }
        } else {
            listener_clone.set_nonblocking(false).map_err(|e| e.to_string())?;
            let (stream, _) = listener_clone.accept().map_err(|e| format!("Accept error: {}", e))?;
            let _ = stream.set_nonblocking(false);
            parse_incoming_http_request(stream)
        }
    }));

    // 7. Http.respond(req_id, status_code, headers_map?, body_str?) -> bool
    http_module.insert("respond".to_string(), Value::Native("Http.respond".into(), |args| {
        if args.is_empty() {
            return Err("Http.respond(req_id, status, headers?, body?) expects request id".to_string());
        }
        let req_id = match args[0] {
            Value::Int(id) => id as u64,
            _ => return Err("Request ID must be an integer".to_string()),
        };

        let status = if args.len() > 1 {
            match args[1] {
                Value::Int(s) => s as u16,
                _ => 200,
            }
        } else {
            200
        };

        let custom_headers = if args.len() > 2 {
            match &args[2] {
                Value::Map(m) => {
                    let mut h = HashMap::new();
                    for (k, v) in m.lock().iter() {
                        h.insert(k.clone(), v.to_string());
                    }
                    h
                }
                _ => HashMap::new(),
            }
        } else {
            HashMap::new()
        };

        let body = if args.len() > 3 {
            args[3].to_string()
        } else {
            String::new()
        };

        let stream_opt = with_streams(|map| map.remove(&req_id));
        let mut stream = stream_opt.ok_or_else(|| format!("Request ID {} not found or already responded", req_id))?;

        let status_text = match status {
            200 => "OK",
            201 => "Created",
            202 => "Accepted",
            204 => "No Content",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            500 => "Internal Server Error",
            _ => "Status",
        };

        let mut header_lines = String::new();
        let mut has_content_type = false;
        for (k, v) in custom_headers {
            if k.eq_ignore_ascii_case("content-type") {
                has_content_type = true;
            }
            header_lines.push_str(&format!("{}: {}\r\n", k, v));
        }

        if !has_content_type {
            header_lines.push_str("Content-Type: application/json; charset=utf-8\r\n");
        }

        let response = format!(
            "HTTP/1.1 {} {}\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
            status,
            status_text,
            header_lines,
            body.len(),
            body
        );

        stream.write_all(response.as_bytes()).map_err(|e| e.to_string())?;
        stream.flush().map_err(|e| e.to_string())?;

        Ok(Value::Bool(true))
    }));

    // 8. Http.close(listener_id) -> bool
    http_module.insert("close".to_string(), Value::Native("Http.close".into(), |args| {
        if args.is_empty() {
            return Err("Http.close(listener_id) expects listener id".to_string());
        }
        let listener_id = match args[0] {
            Value::Int(id) => id as u64,
            _ => return Err("Listener ID must be an integer".to_string()),
        };

        let closed = with_listeners(|map| map.remove(&listener_id).is_some());
        Ok(Value::Bool(closed))
    }));

    let mut ws_module = HashMap::new();

    // 1. WebSocket.upgrade(req_or_id, [key]) -> ws_id
    ws_module.insert("upgrade".to_string(), Value::Native("WebSocket.upgrade".into(), |args| {
        if args.is_empty() {
            return Err("WebSocket.upgrade expects request map or request id".to_string());
        }
        let (req_id, ws_key) = if let Value::Map(m) = &args[0] {
            let guard = m.lock();
            let id = match guard.get("id") {
                Some(Value::Int(i)) => *i as u64,
                _ => return Err("WebSocket.upgrade expects request map with 'id'".to_string()),
            };
            let key = match guard.get("headers") {
                Some(Value::Map(hm)) => {
                    let hg = hm.lock();
                    match hg.get("sec-websocket-key") {
                        Some(Value::String(k)) => (**k).clone(),
                        _ => return Err("WebSocket.upgrade request does not have Sec-WebSocket-Key header".to_string()),
                    }
                }
                _ => return Err("WebSocket.upgrade request missing headers map".to_string()),
            };
            (id, key)
        } else if let Value::Int(id) = args[0] {
            if args.len() < 2 {
                return Err("WebSocket.upgrade(req_id, key) expects key string when passing req_id directly".to_string());
            }
            (id as u64, args[1].to_string())
        } else {
            return Err("WebSocket.upgrade expects request map or (req_id, key)".to_string());
        };

        let stream_opt = with_streams(|map| map.remove(&req_id));
        let mut stream = stream_opt.ok_or_else(|| format!("Request ID {} not found", req_id))?;

        let accept_key = crate::vm::crypto::sec_websocket_accept(&ws_key);
        let response = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
            accept_key
        );
        stream.write_all(response.as_bytes()).map_err(|e| e.to_string())?;
        stream.flush().map_err(|e| e.to_string())?;
        let _ = stream.set_nonblocking(false);
        stream.set_read_timeout(Some(Duration::from_millis(50))).map_err(|e| e.to_string())?;

        let ws_id = NEXT_WS_ID.fetch_add(1, Ordering::SeqCst);
        let stream = Arc::new(Mutex::new(stream));
        with_websockets(|map| map.insert(ws_id, WsConnection { stream, is_client: false }));
        Ok(Value::Int(ws_id as i64))
    }));

    // 2. WebSocket.connect(url) -> ws_id
    ws_module.insert("connect".to_string(), Value::Native("WebSocket.connect".into(), |args| {
        if args.is_empty() {
            return Err("WebSocket.connect(url) expects URL string".to_string());
        }
        let url_str = args[0].to_string();
        let (host, port, path) = parse_url(&url_str)?;

        let mut stream = TcpStream::connect((host.as_str(), port))
            .map_err(|e| format!("WebSocket connect to {}:{} failed: {}", host, port, e))?;

        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut key_bytes = [0u8; 16];
        let mut s = nanos;
        for b in &mut key_bytes {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            *b = (s >> 32) as u8;
        }
        let sec_key = crate::vm::crypto::base64_encode(&key_bytes);

        let handshake_req = format!(
            "GET {} HTTP/1.1\r\nHost: {}:{}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {}\r\nSec-WebSocket-Version: 13\r\n\r\n",
            path, host, port, sec_key
        );

        stream.write_all(handshake_req.as_bytes()).map_err(|e| e.to_string())?;
        stream.flush().map_err(|e| e.to_string())?;

        stream.set_read_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
        let (status_line, _resp_headers) = read_http_headers(&mut stream)?;

        if !status_line.contains("101") {
            return Err(format!("WebSocket handshake rejected with status: {}", status_line.trim()));
        }

        stream.set_read_timeout(Some(Duration::from_millis(50))).map_err(|e| e.to_string())?;
        let ws_id = NEXT_WS_ID.fetch_add(1, Ordering::SeqCst);
        let stream = Arc::new(Mutex::new(stream));
        with_websockets(|map| map.insert(ws_id, WsConnection { stream, is_client: true }));
        Ok(Value::Int(ws_id as i64))
    }));

    // 3. WebSocket.send(ws_id, data, [is_binary]) -> bool
    ws_module.insert("send".to_string(), Value::Native("WebSocket.send".into(), |args| {
        if args.len() < 2 {
            return Err("WebSocket.send(ws_id, data) expects ws_id and data".to_string());
        }
        let ws_id = match args[0] {
            Value::Int(id) => id as u64,
            _ => return Err("ws_id must be an integer".to_string()),
        };
        let payload_str = args[1].to_string();
        let opcode = if args.len() > 2 && args[2].is_truthy() { 0x2 } else { 0x1 };

        let (stream_arc, is_client) = with_websockets(|map| -> Result<(Arc<Mutex<TcpStream>>, bool), String> {
            let conn = map.get(&ws_id).ok_or_else(|| format!("WebSocket {} not found", ws_id))?;
            Ok((Arc::clone(&conn.stream), conn.is_client))
        })?;

        let frame = encode_ws_frame(payload_str.as_bytes(), opcode, is_client);
        let mut stream = stream_arc.lock();
        stream.write_all(&frame).map_err(|e| e.to_string())?;
        stream.flush().map_err(|e| e.to_string())?;
        Ok(Value::Bool(true))
    }));

    // 4. WebSocket.recv(ws_id, [timeout_ms]) -> map or nil
    ws_module.insert("recv".to_string(), Value::Native("WebSocket.recv".into(), |args| {
        if args.is_empty() {
            return Err("WebSocket.recv(ws_id, [timeout_ms]) expects ws_id".to_string());
        }
        let ws_id = match args[0] {
            Value::Int(id) => id as u64,
            _ => return Err("ws_id must be an integer".to_string()),
        };
        let timeout_ms = if args.len() > 1 {
            match args[1] {
                Value::Int(ms) if ms > 0 => Some(Duration::from_millis(ms as u64)),
                _ => None,
            }
        } else {
            Some(Duration::from_millis(50))
        };

        let (stream_arc, is_client) = with_websockets(|map| -> Result<(Arc<Mutex<TcpStream>>, bool), String> {
            let conn = match map.get(&ws_id) {
                Some(c) => c,
                None => return Err(format!("WebSocket {} not found", ws_id)),
            };
            Ok((Arc::clone(&conn.stream), conn.is_client))
        })?;

        let frame_res = {
            let mut stream = stream_arc.lock();
            let _ = stream.set_nonblocking(false);
            if let Some(to) = timeout_ms {
                let _ = stream.set_read_timeout(Some(to));
            }
            decode_ws_frame(&mut stream)
        };

        match frame_res {
            WsFrameResult::TimedOut => Ok(Value::Nil),
            WsFrameResult::Closed => {
                with_websockets(|map| map.remove(&ws_id));
                let mut map = HashMap::new();
                map.insert("type".to_string(), Value::string("close"));
                map.insert("code".to_string(), Value::Int(1000));
                map.insert("data".to_string(), Value::string(""));
                Ok(Value::map(map))
            }
            WsFrameResult::Frame { opcode, payload } => {
                if opcode == 0x8 {
                    with_websockets(|map| map.remove(&ws_id));
                    let mut map = HashMap::new();
                    map.insert("type".to_string(), Value::string("close"));
                    let code = if payload.len() >= 2 {
                        u16::from_be_bytes([payload[0], payload[1]]) as i64
                    } else {
                        1000
                    };
                    map.insert("code".to_string(), Value::Int(code));
                    map.insert("data".to_string(), Value::string(""));
                    Ok(Value::map(map))
                } else if opcode == 0x9 {
                    let pong = encode_ws_frame(&payload, 0xA, is_client);
                    let mut stream = stream_arc.lock();
                    let _ = stream.write_all(&pong);
                    let _ = stream.flush();

                    let mut map = HashMap::new();
                    map.insert("type".to_string(), Value::string("ping"));
                    map.insert("data".to_string(), Value::string(String::from_utf8_lossy(&payload)));
                    Ok(Value::map(map))
                } else if opcode == 0xA {
                    let mut map = HashMap::new();
                    map.insert("type".to_string(), Value::string("pong"));
                    map.insert("data".to_string(), Value::string(String::from_utf8_lossy(&payload)));
                    Ok(Value::map(map))
                } else if opcode == 0x2 {
                    let mut map = HashMap::new();
                    map.insert("type".to_string(), Value::string("binary"));
                    map.insert("data".to_string(), Value::string(String::from_utf8_lossy(&payload)));
                    Ok(Value::map(map))
                } else {
                    let mut map = HashMap::new();
                    map.insert("type".to_string(), Value::string("text"));
                    map.insert("data".to_string(), Value::string(String::from_utf8_lossy(&payload)));
                    Ok(Value::map(map))
                }
            }
        }
    }));

    // 5. WebSocket.close(ws_id) -> bool
    ws_module.insert("close".to_string(), Value::Native("WebSocket.close".into(), |args| {
        if args.is_empty() {
            return Err("WebSocket.close(ws_id) expects ws_id".to_string());
        }
        let ws_id = match args[0] {
            Value::Int(id) => id as u64,
            _ => return Err("ws_id must be an integer".to_string()),
        };
        let conn_opt = with_websockets(|map| map.remove(&ws_id));
        if let Some(conn) = conn_opt {
            let close_frame = encode_ws_frame(&1000u16.to_be_bytes(), 0x8, conn.is_client);
            let mut stream = conn.stream.lock();
            let _ = stream.write_all(&close_frame);
            let _ = stream.flush();
            Ok(Value::Bool(true))
        } else {
            Ok(Value::Bool(false))
        }
    }));

    // 6. WebSocket.is_open(ws_id) -> bool
    ws_module.insert("is_open".to_string(), Value::Native("WebSocket.is_open".into(), |args| {
        if args.is_empty() {
            return Err("WebSocket.is_open(ws_id) expects ws_id".to_string());
        }
        let ws_id = match args[0] {
            Value::Int(id) => id as u64,
            _ => return Err("ws_id must be an integer".to_string()),
        };
        let open = with_websockets(|map| map.contains_key(&ws_id));
        Ok(Value::Bool(open))
    }));

    // 7. WebSocket.broadcast(ids, message) -> count
    ws_module.insert("broadcast".to_string(), Value::Native("WebSocket.broadcast".into(), |args| {
        if args.len() < 2 {
            return Err("WebSocket.broadcast(ids, message) expects ids list and message string".to_string());
        }
        let ids = match &args[0] {
            Value::Array(l) => {
                let guard = l.lock();
                let mut res = Vec::new();
                for item in guard.iter() {
                    if let Value::Int(id) = item {
                        res.push(*id as u64);
                    }
                }
                res
            }
            _ => return Err("First argument to WebSocket.broadcast must be a list of integer IDs".to_string()),
        };
        let message = args[1].to_string();

        let targets = with_websockets(|map| {
            let mut list = Vec::new();
            for id in &ids {
                if let Some(conn) = map.get(id) {
                    list.push((*id, Arc::clone(&conn.stream), conn.is_client));
                }
            }
            list
        });

        let mut count = 0;
        let mut failed_ids = Vec::new();

        for (id, stream_arc, is_client) in targets {
            let frame = encode_ws_frame(message.as_bytes(), 0x1, is_client);
            let mut stream = stream_arc.lock();
            if stream.write_all(&frame).is_ok() && stream.flush().is_ok() {
                count += 1;
            } else {
                failed_ids.push(id);
            }
        }

        if !failed_ids.is_empty() {
            with_websockets(|map| {
                for failed in failed_ids {
                    map.remove(&failed);
                }
            });
        }

        Ok(Value::Int(count))
    }));

    globals.insert("Http".to_string(), Value::map(http_module.clone()));
    globals.insert("Net".to_string(), Value::map(http_module.clone()));
    globals.entry("__native_http".to_string()).or_insert_with(|| Value::map(http_module));
    globals.entry("WebSocket".to_string()).or_insert_with(|| Value::map(ws_module.clone()));
    globals.entry("__native_ws".to_string()).or_insert_with(|| Value::map(ws_module));
}

fn read_http_headers(stream: &mut TcpStream) -> Result<(String, HashMap<String, String>), String> {
    let mut header_bytes = Vec::new();
    let mut buf = [0u8; 1];
    while !header_bytes.ends_with(b"\r\n\r\n") {
        match stream.read(&mut buf) {
            Ok(1) => header_bytes.push(buf[0]),
            Ok(_) => return Err("Unexpected EOF while reading HTTP headers".into()),
            Err(e) => return Err(format!("Error reading HTTP headers: {}", e)),
        }
        if header_bytes.len() > 65536 {
            return Err("HTTP header too large".into());
        }
    }

    let header_str = String::from_utf8_lossy(&header_bytes).to_string();
    let mut lines = header_str.split("\r\n");
    let status_or_req_line = lines.next().unwrap_or("").to_string();

    let mut headers = HashMap::new();
    for line in lines {
        if line.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_lowercase(), v.trim().to_string());
        }
    }

    Ok((status_or_req_line, headers))
}

fn parse_incoming_http_request(mut stream: TcpStream) -> Result<Value, String> {
    stream.set_read_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;

    let (req_line, headers_str_map) = read_http_headers(&mut stream)?;

    let parts: Vec<&str> = req_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Malformed HTTP request line".to_string());
    }

    let method = parts[0].to_uppercase();
    let full_path = parts[1];

    let (path, query) = if let Some((p, q)) = full_path.split_once('?') {
        (p.to_string(), q.to_string())
    } else {
        (full_path.to_string(), String::new())
    };

    let mut headers = HashMap::new();
    let mut content_length: usize = 0;
    for (k, v) in headers_str_map {
        if k == "content-length" {
            if let Ok(len) = v.parse::<usize>() {
                content_length = len;
            }
        }
        headers.insert(k, Value::string(v));
    }

    let mut body = String::new();
    if content_length > 0 {
        let mut body_bytes = vec![0u8; content_length];
        stream.read_exact(&mut body_bytes).map_err(|e| format!("Failed to read body: {}", e))?;
        body = String::from_utf8_lossy(&body_bytes).to_string();
    }

    let req_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::SeqCst);
    with_streams(|map| map.insert(req_id, stream));

    let mut req_map = HashMap::new();
    req_map.insert("id".to_string(), Value::Int(req_id as i64));
    req_map.insert("method".to_string(), Value::string(method));
    req_map.insert("path".to_string(), Value::string(path));
    req_map.insert("query".to_string(), Value::string(query));
    req_map.insert("headers".to_string(), Value::map(headers));
    req_map.insert("body".to_string(), Value::string(body));

    Ok(Value::map(req_map))
}

fn execute_http_client_request(
    method: &str,
    url_str: &str,
    post_body: &str,
    custom_headers: Option<HashMap<String, String>>,
) -> Result<Value, String> {
    let (host, port, path) = parse_url(url_str)?;

    let stream_res = TcpStream::connect_timeout(
        &format!("{}:{}", host, port).parse().map_err(|e| format!("Invalid address {}:{}: {}", host, port, e))?,
        Duration::from_secs(5),
    );

    let mut stream = stream_res.map_err(|e| format!("Http client connection error to {}:{}: {}", host, port, e))?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;

    let mut headers_str = format!("Host: {}:{}\r\nUser-Agent: Aether/2.0\r\nConnection: close\r\n", host, port);

    let mut has_content_type = false;
    let mut has_content_length = false;

    if let Some(ch) = custom_headers {
        for (k, v) in ch {
            if k.eq_ignore_ascii_case("content-type") { has_content_type = true; }
            if k.eq_ignore_ascii_case("content-length") { has_content_length = true; }
            headers_str.push_str(&format!("{}: {}\r\n", k, v));
        }
    }

    if !post_body.is_empty() {
        if !has_content_type {
            headers_str.push_str("Content-Type: application/json\r\n");
        }
        if !has_content_length {
            headers_str.push_str(&format!("Content-Length: {}\r\n", post_body.len()));
        }
    }

    let request = format!("{} {} HTTP/1.1\r\n{}\r\n{}", method, path, headers_str, post_body);
    stream.write_all(request.as_bytes()).map_err(|e| e.to_string())?;

    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line).map_err(|e| e.to_string())?;

    let status_code = parse_status_code(&status_line).unwrap_or(200);

    let mut headers = HashMap::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_lowercase(), Value::string(v.trim()));
        }
    }

    let mut body_bytes = Vec::new();
    let _ = reader.read_to_end(&mut body_bytes);
    let body = String::from_utf8_lossy(&body_bytes).to_string();

    let mut result = HashMap::new();
    result.insert("status".to_string(), Value::Int(status_code));
    result.insert("ok".to_string(), Value::Bool(status_code >= 200 && status_code < 300));
    result.insert("body".to_string(), Value::string(body));
    result.insert("headers".to_string(), Value::map(headers));

    Ok(Value::map(result))
}

fn parse_url(url: &str) -> Result<(String, u16, String), String> {
    let (without_proto, default_port) = if let Some(stripped) = url.strip_prefix("ws://") {
        (stripped, 80)
    } else if let Some(stripped) = url.strip_prefix("wss://") {
        (stripped, 443)
    } else if let Some(stripped) = url.strip_prefix("http://") {
        (stripped, 80)
    } else if let Some(stripped) = url.strip_prefix("https://") {
        (stripped, 443)
    } else {
        (url, 80)
    };

    let (host_port, path) = if let Some((hp, p)) = without_proto.split_once('/') {
        (hp, format!("/{}", p))
    } else {
        (without_proto, "/".to_string())
    };

    let (host, port) = if let Some((h, p)) = host_port.split_once(':') {
        let port_num = p.parse::<u16>().map_err(|_| format!("Invalid port in url: {}", p))?;
        (h.to_string(), port_num)
    } else {
        (host_port.to_string(), default_port)
    };

    Ok((host, port, path))
}

fn parse_status_code(status_line: &str) -> Option<i64> {
    let parts: Vec<&str> = status_line.split_whitespace().collect();
    if parts.len() >= 2 {
        parts[1].parse::<i64>().ok()
    } else {
        None
    }
}

enum WsFrameResult {
    Frame { opcode: u8, payload: Vec<u8> },
    TimedOut,
    Closed,
}

fn encode_ws_frame(payload: &[u8], opcode: u8, mask: bool) -> Vec<u8> {
    let mut frame = Vec::new();
    let b0 = 0x80 | (opcode & 0x0F);
    frame.push(b0);

    let len = payload.len();
    let mask_bit = if mask { 0x80 } else { 0x00 };

    if len <= 125 {
        frame.push(mask_bit | (len as u8));
    } else if len <= 65535 {
        frame.push(mask_bit | 126);
        frame.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        frame.push(mask_bit | 127);
        frame.extend_from_slice(&(len as u64).to_be_bytes());
    }

    if mask {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let mask_key = [
            (nanos & 0xFF) as u8 ^ 0x5A,
            ((nanos >> 8) & 0xFF) as u8 ^ 0xA5,
            ((nanos >> 16) & 0xFF) as u8 ^ 0x3C,
            ((nanos >> 24) & 0xFF) as u8 ^ 0xC3,
        ];
        frame.extend_from_slice(&mask_key);
        for (i, &b) in payload.iter().enumerate() {
            frame.push(b ^ mask_key[i % 4]);
        }
    } else {
        frame.extend_from_slice(payload);
    }

    frame
}

fn decode_ws_frame(stream: &mut TcpStream) -> WsFrameResult {
    let mut header = [0u8; 2];
    match stream.read_exact(&mut header) {
        Ok(()) => {}
        Err(e) => {
            if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut {
                return WsFrameResult::TimedOut;
            }
            return WsFrameResult::Closed;
        }
    }

    let b0 = header[0];
    let b1 = header[1];

    let opcode = b0 & 0x0F;
    let is_masked = (b1 & 0x80) != 0;
    let len_code = b1 & 0x7F;

    let payload_len: usize = if len_code <= 125 {
        len_code as usize
    } else if len_code == 126 {
        let mut len_bytes = [0u8; 2];
        if stream.read_exact(&mut len_bytes).is_err() {
            return WsFrameResult::Closed;
        }
        u16::from_be_bytes(len_bytes) as usize
    } else {
        let mut len_bytes = [0u8; 8];
        if stream.read_exact(&mut len_bytes).is_err() {
            return WsFrameResult::Closed;
        }
        u64::from_be_bytes(len_bytes) as usize
    };

    let mask_key = if is_masked {
        let mut mk = [0u8; 4];
        if stream.read_exact(&mut mk).is_err() {
            return WsFrameResult::Closed;
        }
        Some(mk)
    } else {
        None
    };

    let mut payload = vec![0u8; payload_len];
    if stream.read_exact(&mut payload).is_err() {
        return WsFrameResult::Closed;
    }

    if let Some(mk) = mask_key {
        for (i, b) in payload.iter_mut().enumerate() {
            *b ^= mk[i % 4];
        }
    }

    WsFrameResult::Frame { opcode, payload }
}
