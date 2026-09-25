use aether::vm::{run_source, Value};

#[test]
fn test_fiber_spawn_channel_communication() {
    let code = r#"
let ch = channel()

spawn:
    send(ch, 42)

let result = recv(ch)
result
"#;
    let res = run_source(code).expect("Fiber channel communication failed");
    assert_eq!(res, Value::Int(42));
}

#[test]
fn test_fiber_braced_block_syntax() {
    let code = r#"
let ch = channel()

spawn {
    let msg = "hello from fiber"
    send(ch, msg)
}

let result = recv(ch)
result
"#;
    let res = run_source(code).expect("Fiber braced block syntax failed");
    assert_eq!(res, Value::string("hello from fiber"));
}

#[test]
fn test_fiber_single_expression_spawn() {
    let code = r#"
let ch = channel()

fn worker(c):
    send(c, 100 * 5)

spawn worker(ch)

let result = recv(ch)
result
"#;
    let res = run_source(code).expect("Fiber single expression spawn failed");
    assert_eq!(res, Value::Int(500));
}

#[test]
fn test_debug_for() {
    let code = r#"
let mut total = 0
for i in range(5):
    total += i
total
"#;
    let res = run_source(code).expect("for failed");
    assert_eq!(res, Value::Int(10));
}

#[test]
fn test_multi_fiber_work_stealing_aggregation() {
    let code = r#"
let ch = channel()
let n = 5

for i in range(n):
    spawn:
        send(ch, i * 10)

let mut total = 0
for i in range(n):
    let val = recv(ch)
    total += val

total
"#;
    let res = run_source(code).expect("Multi-fiber work-stealing failed");
    // 0 + 10 + 20 + 30 + 40 = 100
    assert_eq!(res, Value::Int(100));
}

#[test]
fn test_fiber_channel_try_recv_and_sleep() {
    let code = r#"
let ch = channel()

let initial = try_recv(ch)

spawn:
    sleep_ms(10)
    send(ch, "delayed")

let res = recv(ch)
res
"#;
    let res = run_source(code).expect("try_recv and sleep_ms failed");
    assert_eq!(res, Value::string("delayed"));
}
