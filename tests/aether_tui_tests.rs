// ==============================================================================
// AetherTimeTravel TUI Unit & Integration Test Suite
// Verifying Interactive Timeline Visualization & State Navigation
// Pure Rust Standard Library (Zero Third-Party Crates)
// ==============================================================================

use aether::vm::timetravel::{create_session, get_session};
use aether::vm::timetravel_tui::{
    compute_state_at_step, compute_step_diff, render_tui_frame, run_interactive_tui,
};
use aether::vm::{run_source, Value};
use std::io::Cursor;

#[test]
fn test_tui_state_reconstruction_across_steps() {
    let sess_id = create_session();
    let sess_arc = get_session(sess_id).expect("Failed to get session");

    {
        let mut s = sess_arc.lock().unwrap();
        s.record_mutation("x", Value::Int(10));
        s.record_mutation("y", Value::Float(20.5));
        s.record_mutation("x", Value::Int(99)); // update x at step 3
    }

    let s = sess_arc.lock().unwrap();
    let state_at_1 = compute_state_at_step(&s, 1);
    assert_eq!(state_at_1.get("x"), Some(&Value::Int(10)));
    assert_eq!(state_at_1.get("y"), None);

    let state_at_2 = compute_state_at_step(&s, 2);
    assert_eq!(state_at_2.get("x"), Some(&Value::Int(10)));
    assert_eq!(state_at_2.get("y"), Some(&Value::Float(20.5)));

    let state_at_3 = compute_state_at_step(&s, 3);
    assert_eq!(state_at_3.get("x"), Some(&Value::Int(99)));
    assert_eq!(state_at_3.get("y"), Some(&Value::Float(20.5)));
}

#[test]
fn test_tui_diff_computation() {
    let sess_id = create_session();
    let sess_arc = get_session(sess_id).expect("Failed to get session");

    {
        let mut s = sess_arc.lock().unwrap();
        s.record_mutation("balance", Value::Float(100.0));
        s.record_mutation("balance", Value::Float(150.0));
        s.record_mutation("flag", Value::Bool(true));
    }

    let s = sess_arc.lock().unwrap();
    let diff = compute_step_diff(&s, 1, 2);
    assert_eq!(diff.len(), 1);
    let (old_v, new_v) = diff.get("balance").unwrap();
    assert_eq!(old_v.as_ref().unwrap(), &Value::Float(100.0));
    assert_eq!(new_v.as_ref().unwrap(), &Value::Float(150.0));
}

#[test]
fn test_tui_frame_rendering() {
    let sess_id = create_session();
    let sess_arc = get_session(sess_id).expect("Failed to get session");

    {
        let mut s = sess_arc.lock().unwrap();
        s.record_mutation("server", Value::string("prod-eu-1"));
        s.record_mutation("replicas", Value::Int(3));
        s.create_checkpoint("initial_cluster");
    }

    let s = sess_arc.lock().unwrap();
    let frame = render_tui_frame(&s, 2, true);

    // Frame assertions
    assert!(frame.contains("AETHER TIME-TRAVEL OMNISCIENT DEBUGGER"));
    assert!(frame.contains("TIMELINE RIBBON"));
    assert!(frame.contains("VARIABLE INSPECTOR"));
    assert!(frame.contains("server"));
    assert!(frame.contains("replicas"));
    assert!(frame.contains("CONTROLS:"));
}

#[test]
fn test_tui_interactive_controller_loop() {
    let sess_id = create_session();
    let sess_arc = get_session(sess_id).expect("Failed to get session");

    {
        let mut s = sess_arc.lock().unwrap();
        s.record_mutation("alpha", Value::Int(1));
        s.record_mutation("alpha", Value::Int(2));
    }

    // Simulate user typing: 'n' (next), 'p' (previous), 'd' (toggle diff), 'q' (quit)
    let fake_input = b"n\np\nd\nq\n";
    let mut reader = Cursor::new(&fake_input[..]);
    let mut output = Vec::new();

    let res = run_interactive_tui(sess_arc, &mut reader, &mut output);
    assert!(res.is_ok(), "Interactive TUI failed: {:?}", res.err());
    let out_str = String::from_utf8_lossy(&output);
    assert!(out_str.contains("Exiting TimeTravel Debugger"));
}

#[test]
fn test_tui_aether_stdlib_integration() {
    let src = r#"
from aether_timetravel import TimeMachine
from aether_tui import TimeTravelTUI

let tm = TimeMachine()
tm["status"] = "INITIALIZING"
tm["retries"] = 0
tm.checkpoint("init_cp")
tm["status"] = "RUNNING"
tm["retries"] = 1

let tui = TimeTravelTUI(tm)
let frame = tui.render()
assert(len(frame) > 50, "TUI frame rendered successfully")

let st = tui.state_at(1)
assert(st["status"] == "INITIALIZING", "State at step 1 matches")
"#;

    let res = run_source(src);
    assert!(res.is_ok(), "Aether stdlib TUI test failed: {:?}", res.err());
}
