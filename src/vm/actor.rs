use super::value::Value;
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::Duration;

#[derive(Clone)]
pub struct ActorMessage {
    pub payload: Value,
    pub reply_sender: Option<Arc<Mutex<Sender<Value>>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorStatus {
    Running,
    Stopped,
    Failed,
}

pub struct ActorState {
    pub id: u64,
    pub name: String,
    pub status: ActorStatus,
    pub restart_count: usize,
    pub max_retries: usize,
    pub strategy: String,
    pub mailbox: Arc<Mutex<VecDeque<ActorMessage>>>,
    pub cv: Arc<Condvar>,
    pub initial_state: Value,
}

static NEXT_ACTOR_ID: AtomicU64 = AtomicU64::new(1);
static ACTORS: OnceLock<Mutex<HashMap<u64, Arc<Mutex<ActorState>>>>> = OnceLock::new();
static ACTOR_NAMES: OnceLock<Mutex<HashMap<String, u64>>> = OnceLock::new();

fn get_actors() -> &'static Mutex<HashMap<u64, Arc<Mutex<ActorState>>>> {
    ACTORS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn get_actor_names() -> &'static Mutex<HashMap<String, u64>> {
    ACTOR_NAMES.get_or_init(|| Mutex::new(HashMap::new()))
}

// ==============================================================================
// Cluster Node Registry & TCP Routing
// ==============================================================================

pub struct ClusterNodeInfo {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub stream: Option<Arc<Mutex<TcpStream>>>,
}

static CLUSTER_NODES: OnceLock<Mutex<HashMap<String, ClusterNodeInfo>>> = OnceLock::new();
static LOCAL_NODE_NAME: OnceLock<Mutex<String>> = OnceLock::new();

fn get_cluster_nodes() -> &'static Mutex<HashMap<String, ClusterNodeInfo>> {
    CLUSTER_NODES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn get_local_node_name() -> &'static Mutex<String> {
    LOCAL_NODE_NAME.get_or_init(|| Mutex::new("local-node".to_string()))
}

// ==============================================================================
// Native Actor Module Registration
// ==============================================================================

pub fn register_actor_module(globals: &mut HashMap<String, Value>) {
    let mut mod_map = HashMap::new();

    // 1. Actor.spawn(name, [initial_state]) -> actor_id
    mod_map.insert(
        "spawn".to_string(),
        Value::Native("Actor.spawn".into(), |args| {
            let id = NEXT_ACTOR_ID.fetch_add(1, Ordering::SeqCst);
            let name = if !args.is_empty() {
                args[0].to_string()
            } else {
                format!("actor-{}", id)
            };
            let initial_state = if args.len() > 1 {
                args[1].clone()
            } else {
                Value::Nil
            };

            let state = ActorState {
                id,
                name: name.clone(),
                status: ActorStatus::Running,
                restart_count: 0,
                max_retries: 3,
                strategy: "one_for_one".to_string(),
                mailbox: Arc::new(Mutex::new(VecDeque::new())),
                cv: Arc::new(Condvar::new()),
                initial_state,
            };

            get_actors().lock().unwrap().insert(id, Arc::new(Mutex::new(state)));
            get_actor_names().lock().unwrap().insert(name, id);

            Ok(Value::Int(id as i64))
        }),
    );

    // 2. Actor.lookup(name) -> actor_id or nil
    mod_map.insert(
        "lookup".to_string(),
        Value::Native("Actor.lookup".into(), |args| {
            if args.is_empty() {
                return Err("Actor.lookup(name) expects name".into());
            }
            let name = args[0].to_string();
            let names = get_actor_names().lock().unwrap();
            if let Some(id) = names.get(&name) {
                Ok(Value::Int(*id as i64))
            } else {
                Ok(Value::Nil)
            }
        }),
    );

    // 3. Actor.send(actor_id, message) -> bool
    mod_map.insert(
        "send".to_string(),
        Value::Native("Actor.send".into(), |args| {
            if args.len() < 2 {
                return Err("Actor.send(actor_id, message) expects 2 arguments".into());
            }
            let actor_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("actor_id must be an integer".into()),
            };
            let msg = ActorMessage {
                payload: args[1].clone(),
                reply_sender: None,
            };

            let actors = get_actors().lock().unwrap();
            if let Some(actor_arc) = actors.get(&actor_id) {
                let actor = actor_arc.lock().unwrap();
                if actor.status == ActorStatus::Running {
                    actor.mailbox.lock().unwrap().push_back(msg);
                    actor.cv.notify_one();
                    return Ok(Value::Bool(true));
                }
            }
            Ok(Value::Bool(false))
        }),
    );

    // 4. Actor.ask(actor_id, message, [timeout_ms]) -> response or nil
    mod_map.insert(
        "ask".to_string(),
        Value::Native("Actor.ask".into(), |args| {
            if args.len() < 2 {
                return Err("Actor.ask(actor_id, message, [timeout_ms]) expects at least 2 arguments".into());
            }
            let actor_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("actor_id must be an integer".into()),
            };
            let timeout_ms = if args.len() > 2 {
                match args[2] {
                    Value::Int(ms) if ms > 0 => ms as u64,
                    _ => 3000,
                }
            } else {
                3000
            };

            let (tx, rx) = channel();
            let msg = ActorMessage {
                payload: args[1].clone(),
                reply_sender: Some(Arc::new(Mutex::new(tx))),
            };

            {
                let actors = get_actors().lock().unwrap();
                let actor_arc = actors.get(&actor_id).ok_or_else(|| format!("Actor {} not found", actor_id))?;
                let actor = actor_arc.lock().unwrap();
                if actor.status != ActorStatus::Running {
                    return Err(format!("Actor {} is not in Running state", actor_id));
                }
                actor.mailbox.lock().unwrap().push_back(msg);
                actor.cv.notify_one();
            }

            match rx.recv_timeout(Duration::from_millis(timeout_ms)) {
                Ok(val) => Ok(val),
                Err(_) => Ok(Value::Nil),
            }
        }),
    );

    // 5. Actor.recv(actor_id, [timeout_ms]) -> map {payload, has_reply, reply_channel} or nil
    mod_map.insert(
        "recv".to_string(),
        Value::Native("Actor.recv".into(), |args| {
            if args.is_empty() {
                return Err("Actor.recv(actor_id, [timeout_ms]) expects actor_id".into());
            }
            let actor_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("actor_id must be an integer".into()),
            };
            let timeout_ms = if args.len() > 1 {
                match args[1] {
                    Value::Int(ms) if ms > 0 => Some(Duration::from_millis(ms as u64)),
                    _ => None,
                }
            } else {
                None
            };

            let (mailbox_arc, cv_arc) = {
                let actors = get_actors().lock().unwrap();
                let actor_arc = actors.get(&actor_id).ok_or_else(|| format!("Actor {} not found", actor_id))?;
                let actor = actor_arc.lock().unwrap();
                (Arc::clone(&actor.mailbox), Arc::clone(&actor.cv))
            };

            let mut q = mailbox_arc.lock().unwrap();
            if q.is_empty() {
                if let Some(timeout) = timeout_ms {
                    let (res_q, wait_res) = cv_arc.wait_timeout(q, timeout).unwrap();
                    q = res_q;
                    if wait_res.timed_out() && q.is_empty() {
                        return Ok(Value::Nil);
                    }
                } else {
                    q = cv_arc.wait(q).unwrap();
                }
            }

            if let Some(msg) = q.pop_front() {
                let mut map = HashMap::new();
                map.insert("payload".to_string(), msg.payload);
                if let Some(tx_arc) = msg.reply_sender {
                    map.insert("has_reply".to_string(), Value::Bool(true));
                    // Store sender in global ephemeral map
                    let reply_id = NEXT_ACTOR_ID.fetch_add(1, Ordering::SeqCst);
                    REPLY_CHANNELS.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap().insert(reply_id, tx_arc);
                    map.insert("reply_id".to_string(), Value::Int(reply_id as i64));
                } else {
                    map.insert("has_reply".to_string(), Value::Bool(false));
                    map.insert("reply_id".to_string(), Value::Nil);
                }
                Ok(Value::map(map))
            } else {
                Ok(Value::Nil)
            }
        }),
    );

    // 6. Actor.reply(reply_id, value) -> bool
    mod_map.insert(
        "reply".to_string(),
        Value::Native("Actor.reply".into(), |args| {
            if args.len() < 2 {
                return Err("Actor.reply(reply_id, value) expects 2 arguments".into());
            }
            let reply_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("reply_id must be integer".into()),
            };
            let reply_map = REPLY_CHANNELS.get_or_init(|| Mutex::new(HashMap::new()));
            if let Some(tx_arc) = reply_map.lock().unwrap().remove(&reply_id) {
                let res = tx_arc.lock().unwrap().send(args[1].clone());
                Ok(Value::Bool(res.is_ok()))
            } else {
                Ok(Value::Bool(false))
            }
        }),
    );

    // 7. Actor.status(actor_id) -> string ("running" | "stopped" | "failed")
    mod_map.insert(
        "status".to_string(),
        Value::Native("Actor.status".into(), |args| {
            if args.is_empty() {
                return Err("Actor.status(actor_id) expects actor_id".into());
            }
            let actor_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("actor_id must be integer".into()),
            };
            let actors = get_actors().lock().unwrap();
            if let Some(actor_arc) = actors.get(&actor_id) {
                let actor = actor_arc.lock().unwrap();
                let s = match actor.status {
                    ActorStatus::Running => "running",
                    ActorStatus::Stopped => "stopped",
                    ActorStatus::Failed => "failed",
                };
                Ok(Value::string(s))
            } else {
                Ok(Value::string("not_found"))
            }
        }),
    );

    // 8. Actor.fail(actor_id) -> bool (simulates controlled crash / exception)
    mod_map.insert(
        "fail".to_string(),
        Value::Native("Actor.fail".into(), |args| {
            if args.is_empty() {
                return Err("Actor.fail(actor_id) expects actor_id".into());
            }
            let actor_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("actor_id must be integer".into()),
            };
            let actors = get_actors().lock().unwrap();
            if let Some(actor_arc) = actors.get(&actor_id) {
                let mut actor = actor_arc.lock().unwrap();
                actor.status = ActorStatus::Failed;
                Ok(Value::Bool(true))
            } else {
                Ok(Value::Bool(false))
            }
        }),
    );

    // 9. Actor.restart(actor_id) -> bool
    mod_map.insert(
        "restart".to_string(),
        Value::Native("Actor.restart".into(), |args| {
            if args.is_empty() {
                return Err("Actor.restart(actor_id) expects actor_id".into());
            }
            let actor_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("actor_id must be integer".into()),
            };
            let actors = get_actors().lock().unwrap();
            if let Some(actor_arc) = actors.get(&actor_id) {
                let mut actor = actor_arc.lock().unwrap();
                actor.restart_count += 1;
                actor.status = ActorStatus::Running;
                actor.mailbox.lock().unwrap().clear();
                Ok(Value::Bool(true))
            } else {
                Ok(Value::Bool(false))
            }
        }),
    );

    // 10. Actor.restarts(actor_id) -> int
    mod_map.insert(
        "restarts".to_string(),
        Value::Native("Actor.restarts".into(), |args| {
            if args.is_empty() {
                return Err("Actor.restarts(actor_id) expects actor_id".into());
            }
            let actor_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("actor_id must be integer".into()),
            };
            let actors = get_actors().lock().unwrap();
            if let Some(actor_arc) = actors.get(&actor_id) {
                let actor = actor_arc.lock().unwrap();
                Ok(Value::Int(actor.restart_count as i64))
            } else {
                Ok(Value::Int(0))
            }
        }),
    );

    // 11. Actor.stop(actor_id) -> bool
    mod_map.insert(
        "stop".to_string(),
        Value::Native("Actor.stop".into(), |args| {
            if args.is_empty() {
                return Err("Actor.stop(actor_id) expects actor_id".into());
            }
            let actor_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("actor_id must be integer".into()),
            };
            let actors = get_actors().lock().unwrap();
            if let Some(actor_arc) = actors.get(&actor_id) {
                let mut actor = actor_arc.lock().unwrap();
                actor.status = ActorStatus::Stopped;
                actor.cv.notify_all();
                Ok(Value::Bool(true))
            } else {
                Ok(Value::Bool(false))
            }
        }),
    );

    // 12. Actor.count() -> int
    mod_map.insert(
        "count".to_string(),
        Value::Native("Actor.count".into(), |_args| {
            let actors = get_actors().lock().unwrap();
            Ok(Value::Int(actors.len() as i64))
        }),
    );

    // -------------------------------------------------------------------------
    // Cluster Networking Primitives
    // -------------------------------------------------------------------------

    // 13. Cluster.init(node_name)
    mod_map.insert(
        "cluster_init".to_string(),
        Value::Native("Actor.cluster_init".into(), |args| {
            if args.is_empty() {
                return Err("cluster_init(node_name) expects node name".into());
            }
            let name = args[0].to_string();
            *get_local_node_name().lock().unwrap() = name;
            Ok(Value::Bool(true))
        }),
    );

    // 14. Cluster.join(node_name, host, port) -> bool
    mod_map.insert(
        "cluster_join".to_string(),
        Value::Native("Actor.cluster_join".into(), |args| {
            if args.len() < 3 {
                return Err("cluster_join(node_name, host, port) requires 3 arguments".into());
            }
            let node_name = args[0].to_string();
            let host = args[1].to_string();
            let port = match args[2] {
                Value::Int(p) => p as u16,
                _ => return Err("port must be integer".into()),
            };

            let stream = TcpStream::connect((host.as_str(), port))
                .map_err(|e| format!("Failed to connect to cluster node {}: {}", node_name, e))?;
            stream.set_nodelay(true).map_err(|e| e.to_string())?;

            let info = ClusterNodeInfo {
                name: node_name.clone(),
                host,
                port,
                stream: Some(Arc::new(Mutex::new(stream))),
            };

            get_cluster_nodes().lock().unwrap().insert(node_name, info);
            Ok(Value::Bool(true))
        }),
    );

    // 15. Cluster.send_remote(node_name, target_actor, message) -> bool
    mod_map.insert(
        "cluster_send".to_string(),
        Value::Native("Actor.cluster_send".into(), |args| {
            if args.len() < 3 {
                return Err("cluster_send(node_name, target_actor, message) requires 3 arguments".into());
            }
            let node_name = args[0].to_string();
            let target_actor = args[1].to_string();
            let msg_payload = args[2].to_string();

            let nodes = get_cluster_nodes().lock().unwrap();
            let node = nodes.get(&node_name).ok_or_else(|| format!("Cluster node {} not registered", node_name))?;

            if let Some(ref stream_arc) = node.stream {
                let frame = format!("{}::{}::{}\n", target_actor, get_local_node_name().lock().unwrap(), msg_payload);
                let mut stream = stream_arc.lock().unwrap();
                stream.write_all(frame.as_bytes()).map_err(|e| e.to_string())?;
                stream.flush().map_err(|e| e.to_string())?;
                Ok(Value::Bool(true))
            } else {
                Err(format!("No open TCP stream to node {}", node_name))
            }
        }),
    );

    // 16. Cluster.listen(port) -> listener_id
    mod_map.insert(
        "cluster_listen".to_string(),
        Value::Native("Actor.cluster_listen".into(), |args| {
            if args.is_empty() {
                return Err("cluster_listen(port) requires port".into());
            }
            let port = match args[0] {
                Value::Int(p) => p as u16,
                _ => return Err("port must be integer".into()),
            };

            let listener = TcpListener::bind(("127.0.0.1", port))
                .map_err(|e| format!("Cluster bind on port {} failed: {}", port, e))?;
            listener.set_nonblocking(true).map_err(|e| e.to_string())?;

            let listener_id = NEXT_ACTOR_ID.fetch_add(1, Ordering::SeqCst);
            CLUSTER_LISTENERS.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap().insert(listener_id, Arc::new(listener));

            Ok(Value::Int(listener_id as i64))
        }),
    );

    // 17. Cluster.poll(listener_id) -> map {target_actor, sender_node, payload} or nil
    mod_map.insert(
        "cluster_poll".to_string(),
        Value::Native("Actor.cluster_poll".into(), |args| {
            if args.is_empty() {
                return Err("cluster_poll(listener_id) requires listener_id".into());
            }
            let listener_id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("listener_id must be integer".into()),
            };

            let listener_arc = {
                let listeners = CLUSTER_LISTENERS.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
                listeners.get(&listener_id).cloned().ok_or_else(|| format!("Listener {} not found", listener_id))?
            };

            // 1. Accept any incoming connections without blocking
            while let Ok((stream, _)) = listener_arc.accept() {
                let _ = stream.set_nonblocking(true);
                let _ = stream.set_nodelay(true);
                get_accepted_streams().lock().unwrap().entry(listener_id).or_default().push(stream);
            }

            // 2. Poll across all active streams for next newline-delimited frame
            let mut streams_map = get_accepted_streams().lock().unwrap();
            let streams = streams_map.entry(listener_id).or_default();
            let mut disconnected = Vec::new();
            let mut result = None;

            for (idx, stream) in streams.iter_mut().enumerate() {
                let mut buf = Vec::new();
                let mut byte = [0u8; 1];
                let mut got_line = false;
                loop {
                    match stream.read(&mut byte) {
                        Ok(1) => {
                            if byte[0] == b'\n' {
                                got_line = true;
                                break;
                            }
                            buf.push(byte[0]);
                            if buf.len() > 65536 {
                                got_line = true;
                                break;
                            }
                        }
                        Ok(0) => {
                            disconnected.push(idx);
                            break;
                        }
                        Ok(_) => {}
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            break;
                        }
                        Err(_) => {
                            disconnected.push(idx);
                            break;
                        }
                    }
                }

                if got_line {
                    let raw_str = String::from_utf8_lossy(&buf);
                    let parts: Vec<&str> = raw_str.trim().split("::").collect();
                    if parts.len() >= 3 {
                        let mut map = HashMap::new();
                        map.insert("target_actor".to_string(), Value::string(parts[0]));
                        map.insert("sender_node".to_string(), Value::string(parts[1]));
                        map.insert("payload".to_string(), Value::string(parts[2]));
                        result = Some(Value::map(map));
                        break;
                    }
                }
            }

            for idx in disconnected.into_iter().rev() {
                if idx < streams.len() {
                    streams.remove(idx);
                }
            }

            if let Some(res) = result {
                Ok(res)
            } else {
                Ok(Value::Nil)
            }
        }),
    );

    globals.insert("Actor".to_string(), Value::map(mod_map.clone()));
    globals.insert("__native_actor".to_string(), Value::map(mod_map));
}

static REPLY_CHANNELS: OnceLock<Mutex<HashMap<u64, Arc<Mutex<Sender<Value>>>>>> = OnceLock::new();
static CLUSTER_LISTENERS: OnceLock<Mutex<HashMap<u64, Arc<TcpListener>>>> = OnceLock::new();
static ACCEPTED_STREAMS: OnceLock<Mutex<HashMap<u64, Vec<TcpStream>>>> = OnceLock::new();

fn get_accepted_streams() -> &'static Mutex<HashMap<u64, Vec<TcpStream>>> {
    ACCEPTED_STREAMS.get_or_init(|| Mutex::new(HashMap::new()))
}
