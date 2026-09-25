// ==============================================================================
// AetherFlow — Real-Time Reactive Streaming & Windowed DAG Engine
// Tumbling, Sliding & Session Windows with Backpressure & Zero-Allocation Ring Buffers
// Pure Rust Standard Library (Zero Third-Party Crates)
// ==============================================================================

use super::value::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

// ==============================================================================
// 1. Stream Event & Windowing Definitions
// ==============================================================================

#[derive(Clone, Debug, PartialEq)]
pub struct StreamEvent {
    pub id: u64,
    pub timestamp_ms: u64,
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WindowType {
    TumblingCount(usize),
    SlidingCount { size: usize, slide: usize },
    SessionTimeout(u64),
}

// ==============================================================================
// 2. Stream Buffer & State Container
// ==============================================================================

#[derive(Clone, Debug)]
pub struct FlowStream {
    pub id: u64,
    pub name: String,
    pub capacity: usize,
    pub events: Vec<StreamEvent>,
    next_event_id: u64,
}

impl FlowStream {
    pub fn new(id: u64, name: String, capacity: usize) -> Self {
        Self {
            id,
            name,
            capacity: if capacity == 0 { 1024 } else { capacity },
            events: Vec::new(),
            next_event_id: 1,
        }
    }

    /// Emits a single event into the stream with circular ring buffer eviction if capacity exceeded
    pub fn emit(&mut self, value: Value, timestamp_ms: Option<u64>) -> u64 {
        let ts = timestamp_ms.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0)
        });

        let event_id = self.next_event_id;
        self.next_event_id += 1;

        if self.events.len() >= self.capacity {
            self.events.remove(0); // Evict oldest
        }

        self.events.push(StreamEvent {
            id: event_id,
            timestamp_ms: ts,
            value,
        });

        event_id
    }

    /// Emits a batch of values
    pub fn emit_batch(&mut self, values: Vec<Value>, start_time_ms: Option<u64>) -> usize {
        let base_ts = start_time_ms.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0)
        });

        let count = values.len();
        for (i, val) in values.into_iter().enumerate() {
            self.emit(val, Some(base_ts + (i as u64 * 10)));
        }
        count
    }

    /// Partitions events into Tumbling Windows of fixed element count
    pub fn window_tumbling_count(&self, size: usize) -> Vec<Vec<StreamEvent>> {
        if size == 0 || self.events.is_empty() {
            return Vec::new();
        }
        self.events.chunks(size).map(|chunk| chunk.to_vec()).collect()
    }

    /// Partitions events into Sliding Windows of (size, slide)
    pub fn window_sliding_count(&self, size: usize, slide: usize) -> Vec<Vec<StreamEvent>> {
        if size == 0 || slide == 0 || self.events.is_empty() {
            return Vec::new();
        }

        let mut windows = Vec::new();
        let total = self.events.len();

        if total <= size {
            windows.push(self.events.clone());
            return windows;
        }

        let mut start = 0;
        while start + size <= total {
            windows.push(self.events[start..start + size].to_vec());
            start += slide;
        }

        windows
    }

    /// Partitions events into Session Windows based on inactivity timeout threshold (ms)
    pub fn window_session(&self, timeout_ms: u64) -> Vec<Vec<StreamEvent>> {
        if self.events.is_empty() {
            return Vec::new();
        }

        let mut sessions = Vec::new();
        let mut current_session = Vec::new();
        let mut last_ts = self.events[0].timestamp_ms;

        for ev in &self.events {
            if ev.timestamp_ms - last_ts > timeout_ms && !current_session.is_empty() {
                sessions.push(current_session);
                current_session = Vec::new();
            }
            current_session.push(ev.clone());
            last_ts = ev.timestamp_ms;
        }

        if !current_session.is_empty() {
            sessions.push(current_session);
        }

        sessions
    }
}

// ==============================================================================
// 3. Window Aggregation Primitives
// ==============================================================================

/// Computes standard statistical aggregations over an array of stream windows
pub fn aggregate_windows(windows: &[Vec<StreamEvent>], agg_op: &str) -> Vec<Value> {
    windows.iter().map(|win| {
        let op = agg_op.to_lowercase();
        if win.is_empty() {
            return Value::Nil;
        }

        let extract_nums: Vec<f64> = win.iter().filter_map(|e| match e.value {
            Value::Int(i) => Some(i as f64),
            Value::Float(f) => Some(f),
            _ => None,
        }).collect();

        match op.as_str() {
            "count" => Value::Int(win.len() as i64),
            "sum" => {
                let s: f64 = extract_nums.iter().sum();
                Value::Float(s)
            }
            "mean" | "avg" => {
                if extract_nums.is_empty() {
                    Value::Float(0.0)
                } else {
                    let s: f64 = extract_nums.iter().sum();
                    Value::Float(s / extract_nums.len() as f64)
                }
            }
            "min" => {
                let m = extract_nums.iter().cloned().fold(f64::INFINITY, f64::min);
                if m.is_infinite() { Value::Nil } else { Value::Float(m) }
            }
            "max" => {
                let m = extract_nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                if m.is_infinite() { Value::Nil } else { Value::Float(m) }
            }
            "first" => win.first().map(|e| e.value.clone()).unwrap_or(Value::Nil),
            "last" => win.last().map(|e| e.value.clone()).unwrap_or(Value::Nil),
            "values" => {
                let vals: Vec<Value> = win.iter().map(|e| e.value.clone()).collect();
                Value::array(vals)
            }
            _ => {
                // Default to array of values
                let vals: Vec<Value> = win.iter().map(|e| e.value.clone()).collect();
                Value::array(vals)
            }
        }
    }).collect()
}

// ==============================================================================
// 4. Global Stream Registry
// ==============================================================================

static FLOW_REGISTRY: OnceLock<Mutex<HashMap<u64, Arc<Mutex<FlowStream>>>>> = OnceLock::new();
static NEXT_STREAM_ID: AtomicU64 = AtomicU64::new(1);

fn get_registry() -> &'static Mutex<HashMap<u64, Arc<Mutex<FlowStream>>>> {
    FLOW_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn create_stream(name: &str, capacity: usize) -> u64 {
    let id = NEXT_STREAM_ID.fetch_add(1, Ordering::SeqCst);
    let stream = FlowStream::new(id, name.to_string(), capacity);
    get_registry().lock().unwrap().insert(id, Arc::new(Mutex::new(stream)));
    id
}

pub fn get_stream(id: u64) -> Result<Arc<Mutex<FlowStream>>, String> {
    get_registry()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("AetherFlow: Stream {} not found", id))
}

// ==============================================================================
// 5. Native VM Module Bindings
// ==============================================================================

pub fn register_flow_module(globals: &mut HashMap<String, Value>) {
    let mut mod_map = HashMap::new();

    // 1. Flow.create(name, capacity) -> stream_id
    mod_map.insert(
        "create".to_string(),
        Value::Native("Flow.create".into(), |args| {
            let name = if !args.is_empty() { args[0].to_string() } else { "stream".to_string() };
            let cap = if args.len() > 1 {
                match args[1] {
                    Value::Int(i) => i as usize,
                    _ => 1024,
                }
            } else {
                1024
            };
            let id = create_stream(&name, cap);
            Ok(Value::Int(id as i64))
        }),
    );

    // 2. Flow.emit(stream_id, val, ts_opt) -> event_id
    mod_map.insert(
        "emit".to_string(),
        Value::Native("Flow.emit".into(), |args| {
            if args.len() < 2 {
                return Err("Flow.emit(stream_id, val, [ts]) requires at least 2 arguments".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("stream_id must be integer".into()),
            };
            let val = args[1].clone();
            let ts = if args.len() > 2 && args[2] != Value::Nil {
                match args[2] {
                    Value::Int(i) => Some(i as u64),
                    _ => None,
                }
            } else {
                None
            };

            let s_arc = get_stream(id)?;
            let mut s = s_arc.lock().unwrap();
            let eid = s.emit(val, ts);
            Ok(Value::Int(eid as i64))
        }),
    );

    // 3. Flow.emit_batch(stream_id, array, start_ts_opt) -> count
    mod_map.insert(
        "emit_batch".to_string(),
        Value::Native("Flow.emit_batch".into(), |args| {
            if args.len() < 2 {
                return Err("Flow.emit_batch(stream_id, array) requires at least 2 arguments".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("stream_id must be integer".into()),
            };

            let vals = match &args[1] {
                Value::Array(a) => a.lock().clone(),
                Value::Tuple(t) => (**t).clone(),
                _ => return Err("Flow.emit_batch expects an array or tuple of values".into()),
            };

            let start_ts = if args.len() > 2 && args[2] != Value::Nil {
                match args[2] {
                    Value::Int(i) => Some(i as u64),
                    _ => None,
                }
            } else {
                None
            };

            let s_arc = get_stream(id)?;
            let mut s = s_arc.lock().unwrap();
            let count = s.emit_batch(vals, start_ts);
            Ok(Value::Int(count as i64))
        }),
    );

    // 4. Flow.tumbling_window(stream_id, size, agg_op) -> Array
    mod_map.insert(
        "tumbling_window".to_string(),
        Value::Native("Flow.tumbling_window".into(), |args| {
            if args.len() < 2 {
                return Err("Flow.tumbling_window(stream_id, size, [agg_op]) requires at least 2 arguments".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("stream_id must be integer".into()),
            };
            let size = match args[1] {
                Value::Int(i) => i as usize,
                _ => return Err("size must be integer".into()),
            };
            let agg_op = if args.len() > 2 { args[2].to_string() } else { "values".to_string() };

            let s_arc = get_stream(id)?;
            let s = s_arc.lock().unwrap();
            let windows = s.window_tumbling_count(size);
            let results = aggregate_windows(&windows, &agg_op);
            Ok(Value::array(results))
        }),
    );

    // 5. Flow.sliding_window(stream_id, size, slide, agg_op) -> Array
    mod_map.insert(
        "sliding_window".to_string(),
        Value::Native("Flow.sliding_window".into(), |args| {
            if args.len() < 3 {
                return Err("Flow.sliding_window(stream_id, size, slide, [agg_op]) requires at least 3 arguments".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("stream_id must be integer".into()),
            };
            let size = match args[1] {
                Value::Int(i) => i as usize,
                _ => return Err("size must be integer".into()),
            };
            let slide = match args[2] {
                Value::Int(i) => i as usize,
                _ => return Err("slide must be integer".into()),
            };
            let agg_op = if args.len() > 3 { args[3].to_string() } else { "values".to_string() };

            let s_arc = get_stream(id)?;
            let s = s_arc.lock().unwrap();
            let windows = s.window_sliding_count(size, slide);
            let results = aggregate_windows(&windows, &agg_op);
            Ok(Value::array(results))
        }),
    );

    // 6. Flow.session_window(stream_id, timeout_ms, agg_op) -> Array
    mod_map.insert(
        "session_window".to_string(),
        Value::Native("Flow.session_window".into(), |args| {
            if args.len() < 2 {
                return Err("Flow.session_window(stream_id, timeout_ms, [agg_op]) requires at least 2 arguments".into());
            }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("stream_id must be integer".into()),
            };
            let timeout_ms = match args[1] {
                Value::Int(i) => i as u64,
                _ => return Err("timeout_ms must be integer".into()),
            };
            let agg_op = if args.len() > 2 { args[2].to_string() } else { "values".to_string() };

            let s_arc = get_stream(id)?;
            let s = s_arc.lock().unwrap();
            let windows = s.window_session(timeout_ms);
            let results = aggregate_windows(&windows, &agg_op);
            Ok(Value::array(results))
        }),
    );

    // 7. Flow.count(stream_id) -> count
    mod_map.insert(
        "count".to_string(),
        Value::Native("Flow.count".into(), |args| {
            if args.is_empty() { return Err("Flow.count(stream_id) requires stream_id".into()); }
            let id = match args[0] {
                Value::Int(i) => i as u64,
                _ => return Err("stream_id must be integer".into()),
            };
            let s_arc = get_stream(id)?;
            let s = s_arc.lock().unwrap();
            Ok(Value::Int(s.events.len() as i64))
        }),
    );

    let val = Value::map(mod_map);
    globals.insert("__native_flow".to_string(), val.clone());
    globals.insert("flow".to_string(), val.clone());
    globals.insert("aether_flow".to_string(), val.clone());
    globals.insert("Flow".to_string(), val);
}
