// ==============================================================================
// AetherTimeTravel Unit & Integration Test Suite
// Verifying Omniscient Time-Reversible VM & Execution History Replay
// ==============================================================================

use aether::vm::timetravel::TimeTravelSession;
use aether::vm::{run_source, Value};

#[test]
fn test_timetravel_record_mutation_and_state() {
    let mut sess = TimeTravelSession::new(1);
    assert_eq!(sess.current_step, 0);

    sess.record_mutation("account_balance", Value::Float(1000.0));
    sess.record_mutation("user_status", Value::string("active"));

    assert_eq!(sess.current_step, 2);
    assert_eq!(sess.current_state.get("account_balance"), Some(&Value::Float(1000.0)));
    assert_eq!(sess.current_state.get("user_status"), Some(&Value::string("active")));
}

#[test]
fn test_timetravel_checkpoint_and_restore() {
    let mut sess = TimeTravelSession::new(2);

    sess.record_mutation("inventory", Value::Int(50));
    sess.record_mutation("price", Value::Float(20.0));
    sess.create_checkpoint("initial_stock");

    // Simulate an incorrect update / anomaly
    sess.record_mutation("inventory", Value::Int(-999));
    sess.record_mutation("price", Value::Float(0.0));
    assert_eq!(sess.current_state.get("inventory"), Some(&Value::Int(-999)));

    // Restore to initial checkpoint
    sess.restore_checkpoint("initial_stock").unwrap();
    assert_eq!(sess.current_state.get("inventory"), Some(&Value::Int(50)));
    assert_eq!(sess.current_state.get("price"), Some(&Value::Float(20.0)));
}

#[test]
fn test_timetravel_rewind_step_by_step() {
    let mut sess = TimeTravelSession::new(3);

    sess.record_mutation("x", Value::Int(1)); // step 1
    sess.record_mutation("x", Value::Int(2)); // step 2
    sess.record_mutation("x", Value::Int(3)); // step 3
    sess.record_mutation("y", Value::Int(10)); // step 4

    assert_eq!(sess.current_state.get("x"), Some(&Value::Int(3)));
    assert_eq!(sess.current_state.get("y"), Some(&Value::Int(10)));

    // Rewind 2 steps: rolls back y=10 (y removed) and x=3 (x restored to 2)
    let rolled = sess.rewind_steps(2).unwrap();
    assert_eq!(rolled, 2);
    assert_eq!(sess.current_state.get("x"), Some(&Value::Int(2)));
    assert_eq!(sess.current_state.get("y"), None);
}

#[test]
fn test_timetravel_timeline_diff() {
    let mut sess = TimeTravelSession::new(4);

    sess.record_mutation("counter", Value::Int(10));
    sess.record_mutation("tag", Value::string("v1.0"));
    sess.create_checkpoint("v1");

    sess.record_mutation("counter", Value::Int(25));
    sess.record_mutation("new_feature", Value::Bool(true));
    sess.create_checkpoint("v2");

    let diffs = sess.diff_checkpoints("v1", "v2").unwrap();
    assert_eq!(diffs.len(), 2);
    assert_eq!(diffs.get("counter"), Some(&(Value::Int(10), Value::Int(25))));
    assert_eq!(diffs.get("new_feature"), Some(&(Value::Nil, Value::Bool(true))));
}

#[test]
fn test_timetravel_counterfactual_branching() {
    let mut sess = TimeTravelSession::new(5);

    sess.record_mutation("market_price", Value::Float(100.0));
    sess.create_checkpoint("pre_fork");

    // Fork counterfactual branch
    let branch_name = sess.branch_timeline("bull_case").unwrap();
    assert_eq!(branch_name, "bull_case");
    assert_eq!(sess.active_timeline, "bull_case");
}

#[test]
fn test_timetravel_aether_stdlib_script() {
    let code = r#"
from aether_timetravel import TimeMachine

# 1. Initialize Time Machine session
tm = TimeMachine()

# 2. Record clean baseline
tm["balance"] = 5000.0
tm["risk_level"] = "low"
tm.checkpoint("safe_state")

# 3. Trigger unintended mutation / bug
tm["balance"] = -15000.0
tm["risk_level"] = "critical"
let bad_balance = tm["balance"]

# 4. Rewind & Restore clean checkpoint
tm.restore("safe_state")
let restored_balance = tm["balance"]
let restored_risk = tm["risk_level"]

# 5. Branch into alternate timeline
tm.branch("remediation_timeline")
tm["balance"] = 7500.0
let branched_balance = tm["balance"]

[
    bad_balance,
    restored_balance,
    restored_risk,
    branched_balance
]
"#;

    let res = run_source(code).expect("TimeMachine script execution failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Float(-15000.0));
        assert_eq!(items[1], Value::Float(5000.0));
        assert_eq!(items[2], Value::string("low"));
        assert_eq!(items[3], Value::Float(7500.0));
    } else {
        panic!("Expected array, got {:?}", res);
    }
}
