use aether::vm::{run_source, Value};

#[test]
fn test_actor_basic_async_messaging() {
    let code = r#"
from aether_actor import ActorSystem

system = ActorSystem("test-sys")

# Spawn an actor without background fiber for direct mailbox testing
aref = system.spawn_actor(nil, "inbox_worker")

# Send 3 messages asynchronously
aref.send("msg_1")
aref.send("msg_2")
aref.send("msg_3")

# Pop from mailbox
m1 = aref.recv(100)["payload"]
m2 = aref.recv(100)["payload"]
m3 = aref.recv(100)["payload"]

[
    aref.name,
    aref.status(),
    m1,
    m2,
    m3
]
"#;
    let res = run_source(code).expect("Basic actor async test failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0].to_string(), "inbox_worker");
        assert_eq!(items[1].to_string(), "running");
        assert_eq!(items[2].to_string(), "msg_1");
        assert_eq!(items[3].to_string(), "msg_2");
        assert_eq!(items[4].to_string(), "msg_3");
    } else {
        panic!("Expected Array, got {:?}", res);
    }
}

#[test]
fn test_actor_ask_request_response() {
    let code = r#"
from aether_actor import ActorSystem

system = ActorSystem("ask-sys")

# Worker function that responds to ping or arithmetic
fn echo_handler(msg):
    if msg == "ping":
        return "pong"
    elif type(msg) == "array":
        return msg[0] + msg[1]
    return "unknown"

worker = system.spawn_actor(echo_handler, "echo_service")

# Give fiber a moment to start
sleep_ms(30)

res_ping = worker.ask("ping", 2000)
res_calc = worker.ask([100, 250], 2000)

worker.stop()

[
    res_ping,
    res_calc,
    worker.status()
]
"#;
    let res = run_source(code).expect("Actor ask request-response test failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0].to_string(), "pong");
        assert_eq!(items[1].to_string(), "350");
        assert_eq!(items[2].to_string(), "stopped");
    } else {
        panic!("Expected Array, got {:?}", res);
    }
}

#[test]
fn test_actor_supervision_one_for_one_auto_restart() {
    let code = r#"
from aether_actor import ActorSystem, Supervisor

system = ActorSystem("sup-sys")
supervisor = Supervisor("one_for_one", 3)

worker = system.spawn_actor(nil, "critical_worker")
supervisor.supervise(worker)

# 1. Initial status
s0 = worker.status()
r0 = worker.restarts()

# 2. Simulate worker failure
worker.fail()
s_failed = worker.status()

# 3. Supervisor detects failure and heals
healed = supervisor.check_and_heal()
s_healed = worker.status()
r_after = worker.restarts()

[
    s0,
    r0,
    s_failed,
    healed,
    s_healed,
    r_after
]
"#;
    let res = run_source(code).expect("Actor supervision test failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0].to_string(), "running");
        assert_eq!(items[1].to_string(), "0");
        assert_eq!(items[2].to_string(), "failed");
        assert_eq!(items[3].to_string(), "1");
        assert_eq!(items[4].to_string(), "running");
        assert_eq!(items[5].to_string(), "1");
    } else {
        panic!("Expected Array, got {:?}", res);
    }
}

#[test]
fn test_cluster_node_cross_machine_messaging() {
    let code = r#"
from aether_actor import ClusterNode

port = 19688

# Node A: Server listener
node_a = ClusterNode("node-alpha", "127.0.0.1", port)
node_a.listen()

# Node B: Client node joining cluster
node_b = ClusterNode("node-beta", "127.0.0.1", 19689)
node_b.join("node-alpha", "127.0.0.1", port)

# Node B dispatches message to target actor on Node A
node_b.send("node-alpha", "payment_worker", "PROCESS_ORDER_77")

sleep_ms(50)

# Node A polls incoming frame
frame = node_a.poll()

[
    frame["target_actor"],
    frame["sender_node"],
    frame["payload"]
]
"#;
    let res = run_source(code).expect("Cluster cross-node messaging test failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0].to_string(), "payment_worker");
        assert_eq!(items[1].to_string(), "node-beta");
        assert_eq!(items[2].to_string(), "PROCESS_ORDER_77");
    } else {
        panic!("Expected Array, got {:?}", res);
    }
}
