// ==============================================================================
// AetherRPC & Native Binary Serialization (AetherPack)
// Zero-Copy, Schema-Free High-Performance Binary Protocol & Microservice RPC
// ==============================================================================

use super::value::Value;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

// ==============================================================================
// 1. AetherPack Wire Specification
// ==============================================================================
// Magic Header: "AEPAC" (0xAE, 0x50, 0x41, 0x43) + Version (0x01)
pub const AETHERPACK_MAGIC: [u8; 4] = [0xAE, 0x50, 0x41, 0x43];
pub const AETHERPACK_VERSION: u8 = 0x01;

pub const TAG_NIL: u8 = 0x00;
pub const TAG_FALSE: u8 = 0x01;
pub const TAG_TRUE: u8 = 0x02;
pub const TAG_INT_POS: u8 = 0x03;
pub const TAG_INT_NEG: u8 = 0x04;
pub const TAG_FLOAT: u8 = 0x05;
pub const TAG_STRING: u8 = 0x06;
pub const TAG_BYTES: u8 = 0x07;
pub const TAG_ARRAY: u8 = 0x08;
pub const TAG_MAP: u8 = 0x09;
pub const TAG_TUPLE: u8 = 0x0A;
pub const TAG_SET: u8 = 0x0B;

pub struct AetherPack;

impl AetherPack {
    /// Serializes an AETHER Value into zero-copy binary bytes with standard magic header
    pub fn pack(val: &Value) -> Vec<u8> {
        let mut out = Vec::with_capacity(64);
        out.extend_from_slice(&AETHERPACK_MAGIC);
        out.push(AETHERPACK_VERSION);
        Self::pack_value(val, &mut out);
        out
    }

    /// Recursively serializes an individual value tag and payload
    fn pack_value(val: &Value, out: &mut Vec<u8>) {
        match val {
            Value::Nil => out.push(TAG_NIL),
            Value::Bool(b) => {
                out.push(if *b { TAG_TRUE } else { TAG_FALSE });
            }
            Value::Int(i) => {
                if *i >= 0 {
                    out.push(TAG_INT_POS);
                    out.extend_from_slice(&(*i as u64).to_le_bytes());
                } else {
                    out.push(TAG_INT_NEG);
                    let mag = (-*i) as u64;
                    out.extend_from_slice(&mag.to_le_bytes());
                }
            }
            Value::Float(f) => {
                out.push(TAG_FLOAT);
                out.extend_from_slice(&f.to_le_bytes());
            }
            Value::String(s) => {
                out.push(TAG_STRING);
                let bytes = s.as_bytes();
                out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                out.extend_from_slice(bytes);
            }
            Value::Array(arr) => {
                out.push(TAG_ARRAY);
                let items = arr.lock().clone();
                out.extend_from_slice(&(items.len() as u32).to_le_bytes());
                for item in items {
                    Self::pack_value(&item, out);
                }
            }
            Value::Tuple(tup) => {
                out.push(TAG_TUPLE);
                out.extend_from_slice(&(tup.len() as u32).to_le_bytes());
                for item in tup.iter() {
                    Self::pack_value(item, out);
                }
            }
            Value::Set(set) => {
                out.push(TAG_SET);
                let items = set.lock().clone();
                out.extend_from_slice(&(items.len() as u32).to_le_bytes());
                for item in items {
                    Self::pack_value(&item, out);
                }
            }
            Value::Map(m) => {
                out.push(TAG_MAP);
                let map = m.lock().clone();
                out.extend_from_slice(&(map.len() as u32).to_le_bytes());
                for (k, v) in map {
                    let k_bytes = k.as_bytes();
                    out.extend_from_slice(&(k_bytes.len() as u32).to_le_bytes());
                    out.extend_from_slice(k_bytes);
                    Self::pack_value(&v, out);
                }
            }
            Value::StructInstance(inst) => {
                out.push(TAG_MAP);
                let fields = inst.fields.lock().clone();
                out.extend_from_slice(&(fields.len() as u32).to_le_bytes());
                for (k, v) in fields {
                    let k_bytes = k.as_bytes();
                    out.extend_from_slice(&(k_bytes.len() as u32).to_le_bytes());
                    out.extend_from_slice(k_bytes);
                    Self::pack_value(&v, out);
                }
            }
            Value::ClassInstance(inst) => {
                out.push(TAG_MAP);
                let fields = inst.fields.lock().clone();
                out.extend_from_slice(&(fields.len() as u32).to_le_bytes());
                for (k, v) in fields {
                    let k_bytes = k.as_bytes();
                    out.extend_from_slice(&(k_bytes.len() as u32).to_le_bytes());
                    out.extend_from_slice(k_bytes);
                    Self::pack_value(&v, out);
                }
            }
            _ => {
                let s = val.to_string();
                out.push(TAG_STRING);
                let bytes = s.as_bytes();
                out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                out.extend_from_slice(bytes);
            }
        }
    }

    /// Deserializes binary bytes back into an AETHER Value
    pub fn unpack(bytes: &[u8]) -> Result<Value, String> {
        if bytes.len() < 5 {
            return Err("AetherPack payload too small (< 5 bytes)".to_string());
        }
        if bytes[0..4] != AETHERPACK_MAGIC {
            return Err("Invalid AetherPack magic header".to_string());
        }
        if bytes[4] != AETHERPACK_VERSION {
            return Err(format!("Unsupported AetherPack version: 0x{:02X}", bytes[4]));
        }

        let mut offset = 5;
        let res = Self::unpack_value(bytes, &mut offset)?;
        Ok(res)
    }

    fn unpack_value(bytes: &[u8], offset: &mut usize) -> Result<Value, String> {
        if *offset >= bytes.len() {
            return Err("Unexpected EOF while unpacking AetherPack value".to_string());
        }
        let tag = bytes[*offset];
        *offset += 1;

        match tag {
            TAG_NIL => Ok(Value::Nil),
            TAG_FALSE => Ok(Value::Bool(false)),
            TAG_TRUE => Ok(Value::Bool(true)),
            TAG_INT_POS => {
                if *offset + 8 > bytes.len() {
                    return Err("Unexpected EOF reading INT_POS".to_string());
                }
                let raw: [u8; 8] = bytes[*offset..*offset + 8].try_into().unwrap();
                *offset += 8;
                let val = u64::from_le_bytes(raw);
                Ok(Value::Int(val as i64))
            }
            TAG_INT_NEG => {
                if *offset + 8 > bytes.len() {
                    return Err("Unexpected EOF reading INT_NEG".to_string());
                }
                let raw: [u8; 8] = bytes[*offset..*offset + 8].try_into().unwrap();
                *offset += 8;
                let val = u64::from_le_bytes(raw);
                Ok(Value::Int(-(val as i64)))
            }
            TAG_FLOAT => {
                if *offset + 8 > bytes.len() {
                    return Err("Unexpected EOF reading FLOAT".to_string());
                }
                let raw: [u8; 8] = bytes[*offset..*offset + 8].try_into().unwrap();
                *offset += 8;
                let val = f64::from_le_bytes(raw);
                Ok(Value::Float(val))
            }
            TAG_STRING => {
                if *offset + 4 > bytes.len() {
                    return Err("Unexpected EOF reading STRING length".to_string());
                }
                let raw: [u8; 4] = bytes[*offset..*offset + 4].try_into().unwrap();
                *offset += 4;
                let len = u32::from_le_bytes(raw) as usize;
                if *offset + len > bytes.len() {
                    return Err("Unexpected EOF reading STRING payload".to_string());
                }
                let s = String::from_utf8_lossy(&bytes[*offset..*offset + len]).to_string();
                *offset += len;
                Ok(Value::string(s))
            }
            TAG_BYTES => {
                if *offset + 4 > bytes.len() {
                    return Err("Unexpected EOF reading BYTES length".to_string());
                }
                let raw: [u8; 4] = bytes[*offset..*offset + 4].try_into().unwrap();
                *offset += 4;
                let len = u32::from_le_bytes(raw) as usize;
                if *offset + len > bytes.len() {
                    return Err("Unexpected EOF reading BYTES payload".to_string());
                }
                let sub = &bytes[*offset..*offset + len];
                *offset += len;
                let arr: Vec<Value> = sub.iter().map(|b| Value::Int(*b as i64)).collect();
                Ok(Value::array(arr))
            }
            TAG_ARRAY => {
                if *offset + 4 > bytes.len() {
                    return Err("Unexpected EOF reading ARRAY count".to_string());
                }
                let raw: [u8; 4] = bytes[*offset..*offset + 4].try_into().unwrap();
                *offset += 4;
                let count = u32::from_le_bytes(raw) as usize;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    items.push(Self::unpack_value(bytes, offset)?);
                }
                Ok(Value::array(items))
            }
            TAG_TUPLE => {
                if *offset + 4 > bytes.len() {
                    return Err("Unexpected EOF reading TUPLE count".to_string());
                }
                let raw: [u8; 4] = bytes[*offset..*offset + 4].try_into().unwrap();
                *offset += 4;
                let count = u32::from_le_bytes(raw) as usize;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    items.push(Self::unpack_value(bytes, offset)?);
                }
                Ok(Value::tuple(items))
            }
            TAG_SET => {
                if *offset + 4 > bytes.len() {
                    return Err("Unexpected EOF reading SET count".to_string());
                }
                let raw: [u8; 4] = bytes[*offset..*offset + 4].try_into().unwrap();
                *offset += 4;
                let count = u32::from_le_bytes(raw) as usize;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    items.push(Self::unpack_value(bytes, offset)?);
                }
                Ok(Value::set(items))
            }
            TAG_MAP => {
                if *offset + 4 > bytes.len() {
                    return Err("Unexpected EOF reading MAP entry count".to_string());
                }
                let raw: [u8; 4] = bytes[*offset..*offset + 4].try_into().unwrap();
                *offset += 4;
                let count = u32::from_le_bytes(raw) as usize;
                let mut map = HashMap::with_capacity(count);
                for _ in 0..count {
                    if *offset + 4 > bytes.len() {
                        return Err("Unexpected EOF reading MAP key length".to_string());
                    }
                    let k_raw: [u8; 4] = bytes[*offset..*offset + 4].try_into().unwrap();
                    *offset += 4;
                    let k_len = u32::from_le_bytes(k_raw) as usize;
                    if *offset + k_len > bytes.len() {
                        return Err("Unexpected EOF reading MAP key content".to_string());
                    }
                    let key = String::from_utf8_lossy(&bytes[*offset..*offset + k_len]).to_string();
                    *offset += k_len;
                    let val = Self::unpack_value(bytes, offset)?;
                    map.insert(key, val);
                }
                Ok(Value::map(map))
            }
            _ => Err(format!("Unknown AetherPack tag: 0x{:02X} at offset {}", tag, *offset - 1)),
        }
    }
}

// ==============================================================================
// 2. Hex Helpers
// ==============================================================================

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    let clean = hex.trim();
    if clean.len() % 2 != 0 {
        return Err("Hex string length must be even".to_string());
    }
    let mut bytes = Vec::with_capacity(clean.len() / 2);
    for i in (0..clean.len()).step_by(2) {
        let b = u8::from_str_radix(&clean[i..i + 2], 16)
            .map_err(|e| format!("Invalid hex at index {}: {}", i, e))?;
        bytes.push(b);
    }
    Ok(bytes)
}

// ==============================================================================
// 3. Network Framing Primitives
// ==============================================================================

pub fn write_framed(stream: &mut TcpStream, payload: &[u8]) -> Result<(), String> {
    let len = payload.len() as u32;
    stream.write_all(&len.to_le_bytes()).map_err(|e| e.to_string())?;
    stream.write_all(payload).map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn read_framed(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > 64 * 1024 * 1024 {
        return Err(format!("Payload length {} exceeds 64MB limit", len));
    }
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload).map_err(|e| e.to_string())?;
    Ok(payload)
}

// ==============================================================================
// 4. In-Memory RPC Server & Client State
// ==============================================================================

struct RpcPendingCall {
    stream: Arc<Mutex<TcpStream>>,
    client_req_id: u64,
}

static PENDING_CALLS: OnceLock<Mutex<HashMap<u64, RpcPendingCall>>> = OnceLock::new();
static NEXT_CALL_ID: AtomicU64 = AtomicU64::new(1);

fn get_pending_calls() -> &'static Mutex<HashMap<u64, RpcPendingCall>> {
    PENDING_CALLS.get_or_init(|| Mutex::new(HashMap::new()))
}

struct RpcServerState {
    #[allow(dead_code)]
    pub id: u64,
    listener: TcpListener,
    clients: Vec<Arc<Mutex<TcpStream>>>,
}

static SERVERS: OnceLock<Mutex<HashMap<u64, Arc<Mutex<RpcServerState>>>>> = OnceLock::new();
static NEXT_SERVER_ID: AtomicU64 = AtomicU64::new(1);

fn get_servers() -> &'static Mutex<HashMap<u64, Arc<Mutex<RpcServerState>>>> {
    SERVERS.get_or_init(|| Mutex::new(HashMap::new()))
}

struct RpcClientState {
    #[allow(dead_code)]
    pub id: u64,
    stream: Arc<Mutex<TcpStream>>,
    next_req_id: AtomicU64,
}

static CLIENTS: OnceLock<Mutex<HashMap<u64, Arc<Mutex<RpcClientState>>>>> = OnceLock::new();
static NEXT_CLIENT_ID: AtomicU64 = AtomicU64::new(1);

fn get_clients() -> &'static Mutex<HashMap<u64, Arc<Mutex<RpcClientState>>>> {
    CLIENTS.get_or_init(|| Mutex::new(HashMap::new()))
}

// ==============================================================================
// 5. Native RPC Module Registration
// ==============================================================================

pub fn register_rpc_module(globals: &mut HashMap<String, Value>) {
    let mut mod_map = HashMap::new();

    // 1. Rpc.pack(value) -> array of byte ints
    mod_map.insert(
        "pack".to_string(),
        Value::Native("Rpc.pack".into(), |args| {
            if args.is_empty() {
                return Err("Rpc.pack(value) expects 1 argument".to_string());
            }
            let bytes = AetherPack::pack(&args[0]);
            let arr: Vec<Value> = bytes.into_iter().map(|b| Value::Int(b as i64)).collect();
            Ok(Value::array(arr))
        }),
    );

    // 2. Rpc.unpack(bytes_or_hex) -> Value
    mod_map.insert(
        "unpack".to_string(),
        Value::Native("Rpc.unpack".into(), |args| {
            if args.is_empty() {
                return Err("Rpc.unpack(bytes) expects 1 argument".to_string());
            }
            let bytes = match &args[0] {
                Value::Array(arr) => {
                    let items = arr.lock().clone();
                    let mut b_vec = Vec::with_capacity(items.len());
                    for it in items {
                        match it {
                            Value::Int(i) => b_vec.push(i as u8),
                            _ => return Err("Expected array of byte integers".to_string()),
                        }
                    }
                    b_vec
                }
                Value::String(s) => {
                    if s.starts_with("AEPAC") || s.len() >= 10 && &s[0..10] == "ae50414301" {
                        hex_to_bytes(s)?
                    } else if let Ok(h) = hex_to_bytes(s) {
                        h
                    } else {
                        s.as_bytes().to_vec()
                    }
                }
                _ => return Err("Rpc.unpack expects byte array or hex string".to_string()),
            };

            AetherPack::unpack(&bytes)
        }),
    );

    // 3. Rpc.pack_hex(value) -> hex string
    mod_map.insert(
        "pack_hex".to_string(),
        Value::Native("Rpc.pack_hex".into(), |args| {
            if args.is_empty() {
                return Err("Rpc.pack_hex(value) expects 1 argument".to_string());
            }
            let bytes = match &args[0] {
                Value::Array(arr) => {
                    let items = arr.lock().clone();
                    let mut is_byte_arr = true;
                    let mut b_vec = Vec::with_capacity(items.len());
                    for it in &items {
                        match it {
                            Value::Int(i) if *i >= 0 && *i <= 255 => b_vec.push(*i as u8),
                            _ => {
                                is_byte_arr = false;
                                break;
                            }
                        }
                    }
                    if is_byte_arr && b_vec.len() >= 5 && b_vec[0..4] == AETHERPACK_MAGIC {
                        b_vec
                    } else {
                        AetherPack::pack(&args[0])
                    }
                }
                _ => AetherPack::pack(&args[0]),
            };
            Ok(Value::string(bytes_to_hex(&bytes)))
        }),
    );

    // 4. Rpc.unpack_hex(hex_str) -> Value
    mod_map.insert(
        "unpack_hex".to_string(),
        Value::Native("Rpc.unpack_hex".into(), |args| {
            if args.is_empty() {
                return Err("Rpc.unpack_hex(hex_str) expects 1 argument".to_string());
            }
            let s = args[0].to_string();
            let bytes = hex_to_bytes(&s)?;
            AetherPack::unpack(&bytes)
        }),
    );

    // 5. Rpc.stats(value) -> map with size and compression stats
    mod_map.insert(
        "stats".to_string(),
        Value::Native("Rpc.stats".into(), |args| {
            if args.is_empty() {
                return Err("Rpc.stats(value) expects 1 argument".to_string());
            }
            let bytes = AetherPack::pack(&args[0]);
            let json_str = args[0].to_string(); // Approximate textual representation
            let mut map = HashMap::new();
            map.insert("pack_bytes".to_string(), Value::Int(bytes.len() as i64));
            map.insert("json_bytes".to_string(), Value::Int(json_str.len() as i64));
            let savings = if json_str.len() > 0 {
                let diff = (json_str.len() as f64 - bytes.len() as f64) / json_str.len() as f64;
                (diff * 100.0).round()
            } else {
                0.0
            };
            map.insert("savings_pct".to_string(), Value::Float(savings));
            map.insert("wire_tag".to_string(), Value::string(format!("0x{:02X}", bytes[5])));
            Ok(Value::map(map))
        }),
    );

    // 6. Rpc.server_create(port, [host]) -> server_id
    mod_map.insert(
        "server_create".to_string(),
        Value::Native("Rpc.server_create".into(), |args| {
            if args.is_empty() {
                return Err("Rpc.server_create(port, [host]) requires port".to_string());
            }
            let port = match args[0] {
                Value::Int(p) => p as u16,
                _ => return Err("Port must be integer".to_string()),
            };
            let host = if args.len() > 1 {
                args[1].to_string()
            } else {
                "127.0.0.1".to_string()
            };

            let listener = TcpListener::bind(format!("{}:{}", host, port))
                .map_err(|e| format!("Failed to bind RPC server on {}:{}: {}", host, port, e))?;
            listener.set_nonblocking(true).map_err(|e| e.to_string())?;

            let id = NEXT_SERVER_ID.fetch_add(1, Ordering::SeqCst);
            let state = RpcServerState {
                id,
                listener,
                clients: Vec::new(),
            };

            get_servers().lock().unwrap().insert(id, Arc::new(Mutex::new(state)));
            Ok(Value::Int(id as i64))
        }),
    );

    // 7. Rpc.server_poll(server_id, timeout_ms) -> incoming call map or nil
    mod_map.insert(
        "server_poll".to_string(),
        Value::Native("Rpc.server_poll".into(), |args| {
            if args.is_empty() {
                return Err("Rpc.server_poll(server_id, [timeout_ms]) expects server_id".to_string());
            }
            let server_id = match args[0] {
                Value::Int(id) => id as u64,
                _ => return Err("server_id must be integer".to_string()),
            };
            let timeout_ms = if args.len() > 1 {
                match args[1] {
                    Value::Int(ms) if ms > 0 => ms as u64,
                    _ => 200,
                }
            } else {
                200
            };

            let server_arc = {
                let servers = get_servers().lock().unwrap();
                servers.get(&server_id).cloned().ok_or_else(|| format!("Server {} not found", server_id))?
            };

            let start = Instant::now();
            let timeout = Duration::from_millis(timeout_ms);

            loop {
                // 1. Accept any pending client connection
                {
                    let mut server = server_arc.lock().unwrap();
                    if let Ok((stream, _)) = server.listener.accept() {
                        let _ = stream.set_nodelay(true);
                        let _ = stream.set_nonblocking(true);
                        server.clients.push(Arc::new(Mutex::new(stream)));
                    }
                }

                // 2. Poll all active connected clients for an incoming framed request
                let clients_snapshot = {
                    let server = server_arc.lock().unwrap();
                    server.clients.clone()
                };

                for client_arc in &clients_snapshot {
                    let mut stream = client_arc.lock().unwrap();
                    let mut len_buf = [0u8; 4];
                    match stream.read_exact(&mut len_buf) {
                        Ok(()) => {
                            let len = u32::from_le_bytes(len_buf) as usize;
                            let mut payload = vec![0u8; len];
                            let _ = stream.set_nonblocking(false);
                            if let Ok(()) = stream.read_exact(&mut payload) {
                                let _ = stream.set_nonblocking(true);
                                if let Ok(val) = AetherPack::unpack(&payload) {
                                    if let Value::Map(m) = val {
                                        let map = m.lock().clone();
                                        let client_req_id = match map.get("id") {
                                            Some(Value::Int(i)) => *i as u64,
                                            _ => 0,
                                        };
                                        let method = match map.get("method") {
                                            Some(v) => v.to_string(),
                                            None => "unknown".to_string(),
                                        };
                                        let call_args = match map.get("args") {
                                            Some(v) => v.clone(),
                                            None => Value::array(Vec::new()),
                                        };

                                        let call_id = NEXT_CALL_ID.fetch_add(1, Ordering::SeqCst);
                                        get_pending_calls().lock().unwrap().insert(
                                            call_id,
                                            RpcPendingCall {
                                                stream: Arc::clone(client_arc),
                                                client_req_id,
                                            },
                                        );

                                        let mut res_map = HashMap::new();
                                        res_map.insert("call_id".to_string(), Value::Int(call_id as i64));
                                        res_map.insert("client_id".to_string(), Value::Int(client_req_id as i64));
                                        res_map.insert("method".to_string(), Value::string(method));
                                        res_map.insert("args".to_string(), call_args);
                                        return Ok(Value::map(res_map));
                                    }
                                }
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // No data ready on this connection
                        }
                        Err(_) => {
                            // Client disconnected or connection error
                        }
                    }
                }

                if start.elapsed() >= timeout {
                    return Ok(Value::Nil);
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }),
    );

    // 8. Rpc.server_respond(call_id, status, result) -> bool
    mod_map.insert(
        "server_respond".to_string(),
        Value::Native("Rpc.server_respond".into(), |args| {
            if args.len() < 3 {
                return Err("Rpc.server_respond(call_id, status, result) requires 3 arguments".to_string());
            }
            let call_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("call_id must be integer".to_string()),
            };
            let status = args[1].to_string();
            let result_val = args[2].clone();

            let pending = get_pending_calls().lock().unwrap().remove(&call_id);
            if let Some(call) = pending {
                let mut resp_map = HashMap::new();
                resp_map.insert("id".to_string(), Value::Int(call.client_req_id as i64));
                resp_map.insert("status".to_string(), Value::string(status));
                resp_map.insert("result".to_string(), result_val);

                let payload = AetherPack::pack(&Value::map(resp_map));
                let mut stream = call.stream.lock().unwrap();
                let _ = stream.set_nonblocking(false);
                let write_res = write_framed(&mut stream, &payload);
                let _ = stream.set_nonblocking(true);
                Ok(Value::Bool(write_res.is_ok()))
            } else {
                Ok(Value::Bool(false))
            }
        }),
    );

    // 9. Rpc.server_close(server_id) -> bool
    mod_map.insert(
        "server_close".to_string(),
        Value::Native("Rpc.server_close".into(), |args| {
            if args.is_empty() {
                return Err("Rpc.server_close(server_id) expects server_id".to_string());
            }
            let server_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("server_id must be integer".to_string()),
            };
            let removed = get_servers().lock().unwrap().remove(&server_id);
            Ok(Value::Bool(removed.is_some()))
        }),
    );

    // 10. Rpc.client_connect(port, [host]) -> client_id
    mod_map.insert(
        "client_connect".to_string(),
        Value::Native("Rpc.client_connect".into(), |args| {
            if args.is_empty() {
                return Err("Rpc.client_connect(port, [host]) requires port".to_string());
            }
            let port = match args[0] {
                Value::Int(p) => p as u16,
                _ => return Err("Port must be integer".to_string()),
            };
            let host = if args.len() > 1 {
                args[1].to_string()
            } else {
                "127.0.0.1".to_string()
            };

            let stream = TcpStream::connect((host.as_str(), port))
                .map_err(|e| format!("Failed to connect to RPC server at {}:{}: {}", host, port, e))?;
            stream.set_nodelay(true).map_err(|e| e.to_string())?;

            let id = NEXT_CLIENT_ID.fetch_add(1, Ordering::SeqCst);
            let state = RpcClientState {
                id,
                stream: Arc::new(Mutex::new(stream)),
                next_req_id: AtomicU64::new(1),
            };

            get_clients().lock().unwrap().insert(id, Arc::new(Mutex::new(state)));
            Ok(Value::Int(id as i64))
        }),
    );

    // 11. Rpc.client_call(client_id, method, args, [timeout_ms]) -> result Value
    mod_map.insert(
        "client_call".to_string(),
        Value::Native("Rpc.client_call".into(), |args| {
            if args.len() < 3 {
                return Err("Rpc.client_call(client_id, method, args, [timeout_ms]) requires at least 3 arguments".to_string());
            }
            let client_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("client_id must be integer".to_string()),
            };
            let method = args[1].to_string();
            let call_args = args[2].clone();
            let timeout_ms = if args.len() > 3 {
                match args[3] {
                    Value::Int(ms) if ms > 0 => ms as u64,
                    _ => 5000,
                }
            } else {
                5000
            };

            let client_arc = {
                let clients = get_clients().lock().unwrap();
                clients.get(&client_id).cloned().ok_or_else(|| format!("Client {} not found", client_id))?
            };

            let client = client_arc.lock().unwrap();
            let req_id = client.next_req_id.fetch_add(1, Ordering::SeqCst);

            let mut req_map = HashMap::new();
            req_map.insert("id".to_string(), Value::Int(req_id as i64));
            req_map.insert("method".to_string(), Value::string(method.clone()));
            req_map.insert("args".to_string(), call_args);

            let payload = AetherPack::pack(&Value::map(req_map));
            let mut stream = client.stream.lock().unwrap();
            stream.set_read_timeout(Some(Duration::from_millis(timeout_ms))).map_err(|e| e.to_string())?;

            write_framed(&mut stream, &payload)
                .map_err(|e| format!("RPC dispatch error for '{}': {}", method, e))?;

            let resp_bytes = read_framed(&mut stream)
                .map_err(|e| format!("RPC response read error for '{}': {}", method, e))?;

            let resp_val = AetherPack::unpack(&resp_bytes)
                .map_err(|e| format!("RPC unpack error for '{}': {}", method, e))?;

            if let Value::Map(m) = resp_val {
                let map = m.lock().clone();
                let status = match map.get("status") {
                    Some(v) => v.to_string(),
                    None => "error".to_string(),
                };
                let result = map.get("result").cloned().unwrap_or(Value::Nil);
                if status == "ok" {
                    Ok(result)
                } else {
                    Err(format!("RPC Remote Error: {}", result.to_string()))
                }
            } else {
                Err("Invalid RPC response structure".to_string())
            }
        }),
    );

    // 12. Rpc.client_close(client_id) -> bool
    mod_map.insert(
        "client_close".to_string(),
        Value::Native("Rpc.client_close".into(), |args| {
            if args.is_empty() {
                return Err("Rpc.client_close(client_id) expects client_id".to_string());
            }
            let client_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("client_id must be integer".to_string()),
            };
            let removed = get_clients().lock().unwrap().remove(&client_id);
            Ok(Value::Bool(removed.is_some()))
        }),
    );

    globals.insert("Rpc".to_string(), Value::map(mod_map.clone()));
    globals.insert("__native_rpc".to_string(), Value::map(mod_map));
}
