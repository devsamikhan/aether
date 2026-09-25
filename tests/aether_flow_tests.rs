// ==============================================================================
// AetherFlow Unit & Integration Test Suite
// Verifying Real-Time Reactive Streaming, Windowing & Aggregations
// Pure Rust Standard Library (Zero Third-Party Crates)
// ==============================================================================

use aether::vm::flow::{create_stream, get_stream};
use aether::vm::{run_source, Value};

#[test]
fn test_flow_tumbling_window_aggregations() {
    let sid = create_stream("sensor_stream", 100);
    let s_arc = get_stream(sid).expect("Failed to get stream");

    {
        let mut s = s_arc.lock().unwrap();
        s.emit_batch(
            vec![
                Value::Float(10.0),
                Value::Float(20.0),
                Value::Float(30.0),
                Value::Float(40.0),
                Value::Float(50.0),
                Value::Float(60.0),
            ],
            None,
        );
    }

    let s = s_arc.lock().unwrap();
    let wins = s.window_tumbling_count(3);
    assert_eq!(wins.len(), 2);
    assert_eq!(wins[0].len(), 3);
    assert_eq!(wins[1].len(), 3);

    let sums = aether::vm::flow::aggregate_windows(&wins, "sum");
    assert_eq!(sums[0], Value::Float(60.0));
    assert_eq!(sums[1], Value::Float(150.0));

    let avgs = aether::vm::flow::aggregate_windows(&wins, "mean");
    assert_eq!(avgs[0], Value::Float(20.0));
    assert_eq!(avgs[1], Value::Float(50.0));
}

#[test]
fn test_flow_sliding_window_aggregations() {
    let sid = create_stream("sliding_stream", 100);
    let s_arc = get_stream(sid).expect("Failed to get stream");

    {
        let mut s = s_arc.lock().unwrap();
        s.emit_batch(
            vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
                Value::Int(4),
                Value::Int(5),
            ],
            None,
        );
    }

    let s = s_arc.lock().unwrap();
    // Window size 3, slide 1 -> 3 overlapping windows
    let wins = s.window_sliding_count(3, 1);
    assert_eq!(wins.len(), 3);

    let sums = aether::vm::flow::aggregate_windows(&wins, "sum");
    assert_eq!(sums[0], Value::Float(6.0));  // 1+2+3
    assert_eq!(sums[1], Value::Float(9.0));  // 2+3+4
    assert_eq!(sums[2], Value::Float(12.0)); // 3+4+5
}

#[test]
fn test_flow_session_window_by_timeout() {
    let sid = create_stream("session_stream", 100);
    let s_arc = get_stream(sid).expect("Failed to get stream");

    {
        let mut s = s_arc.lock().unwrap();
        // Session 1: ts = 1000, 1100
        s.emit(Value::string("login"), Some(1000));
        s.emit(Value::string("click_item"), Some(1100));

        // Inactivity gap > 500ms
        // Session 2: ts = 2000, 2150
        s.emit(Value::string("checkout"), Some(2000));
        s.emit(Value::string("logout"), Some(2150));
    }

    let s = s_arc.lock().unwrap();
    let sessions = s.window_session(500);
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].len(), 2);
    assert_eq!(sessions[1].len(), 2);
    assert_eq!(sessions[0][0].value, Value::string("login"));
    assert_eq!(sessions[1][0].value, Value::string("checkout"));
}

#[test]
fn test_flow_ring_buffer_capacity_eviction() {
    let sid = create_stream("ring_buf", 3); // Max capacity 3
    let s_arc = get_stream(sid).expect("Failed to get stream");

    {
        let mut s = s_arc.lock().unwrap();
        s.emit(Value::Int(10), None);
        s.emit(Value::Int(20), None);
        s.emit(Value::Int(30), None);
        s.emit(Value::Int(40), None); // Evicts 10
        s.emit(Value::Int(50), None); // Evicts 20
    }

    let s = s_arc.lock().unwrap();
    assert_eq!(s.events.len(), 3);
    assert_eq!(s.events[0].value, Value::Int(30));
    assert_eq!(s.events[1].value, Value::Int(40));
    assert_eq!(s.events[2].value, Value::Int(50));
}

#[test]
fn test_flow_aether_stdlib_integration() {
    let src = r#"
from aether_flow import DataStream

let stream = DataStream.from_array([10.0, 25.0, 30.0, 45.0, 50.0, 65.0])
let filtered = stream.filter(lambda x: x >= 30.0)

let windowed = filtered.window_tumbling(2)
let sums = windowed.sum()
let counts = windowed.count()

assert(len(sums) == 2, "2 windows created")
assert(sums[0] == 75.0, "First window sum: 30 + 45 = 75")
assert(sums[1] == 115.0, "Second window sum: 50 + 65 = 115")
assert(counts[0] == 2, "Window size count is 2")
"#;

    let res = run_source(src);
    assert!(res.is_ok(), "Aether stdlib Flow test failed: {:?}", res.err());
}
