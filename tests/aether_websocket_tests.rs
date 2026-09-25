use aether::vm::{run_source, Value};

#[test]
fn test_rfc6455_sec_websocket_accept_test_vector() {
    // RFC 6455 section 4.2.2 standard test vector:
    // Key: "dGhlIHNhbXBsZSBub25jZQ=="
    // Accept: "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
    let key = "dGhlIHNhbXBsZSBub25jZQ==";
    let expected_accept = "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";
    let computed = aether::vm::crypto::sec_websocket_accept(key);
    assert_eq!(computed, expected_accept, "RFC 6455 Sec-WebSocket-Accept token mismatch");

    let code = r#"
token = Crypto.sec_websocket_accept("dGhlIHNhbXBsZSBub25jZQ==")
token
"#;
    let res = run_source(code).expect("Crypto.sec_websocket_accept failed");
    assert_eq!(res, Value::string(expected_accept));
}

#[test]
fn test_websocket_client_server_full_duplex_comm() {
    let server_handle = std::thread::spawn(|| {
        let server_code = r#"
import aether_ws as ws

server = ws.WebSocketServer("127.0.0.1", 19896)
server.listen()

conn = server.accept(4000)
if conn != nil:
    msg = conn.recv(3000)
    if msg != nil:
        conn.send_json({"server_status": "active", "received": msg["data"]})
    conn.recv(1000)
server.close()
"#;
        run_source(server_code).expect("Server script failed");
    });

    std::thread::sleep(std::time::Duration::from_millis(150));

    let client_code = r#"
import aether_ws as ws

client = ws.WebSocketClient.connect("ws://127.0.0.1:19896")
client.send("Hello from client")
resp = client.recv_json(3000)
client.close()

[
    resp["data"]["server_status"],
    resp["data"]["received"]
]
"#;
    let res = run_source(client_code).expect("Client script failed");
    server_handle.join().expect("Server thread panicked");

    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::string("active"));
        assert_eq!(items[1], Value::string("Hello from client"));
    } else {
        panic!("Expected Array result, got {:?}", res);
    }
}
