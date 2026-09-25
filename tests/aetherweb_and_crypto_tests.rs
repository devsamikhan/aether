use aether::vm::{run_source, Value};

#[test]
fn test_crypto_sha256() {
    let code = r#"
from crypto import sha256

let h1 = sha256("hello")
let h2 = sha256("")
[h1, h2]
"#;
    let res = run_source(code).expect("SHA-256 test failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::string("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"));
        assert_eq!(items[1], Value::string("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"));
    } else {
        panic!("Expected array of hashes");
    }
}

#[test]
fn test_crypto_md5() {
    let code = r#"
from crypto import md5

let h = md5("hello")
h
"#;
    let res = run_source(code).expect("MD5 test failed");
    assert_eq!(res, Value::string("5d41402abc4b2a76b9719d911017c592"));
}

#[test]
fn test_crypto_base64_roundtrip() {
    let code = r#"
from crypto import base64_encode, base64_decode

let original = "Aether Web Services 2.0"
let encoded = base64_encode(original)
let decoded = base64_decode(encoded)
[encoded, decoded]
"#;
    let res = run_source(code).expect("Base64 test failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::string("QWV0aGVyIFdlYiBTZXJ2aWNlcyAyLjA="));
        assert_eq!(items[1], Value::string("Aether Web Services 2.0"));
    } else {
        panic!("Expected array");
    }
}

#[test]
fn test_crypto_hmac_sha256() {
    let code = r#"
from crypto import hmac_sha256

let sig = hmac_sha256("secret_key", "data_payload")
sig
"#;
    let res = run_source(code).expect("HMAC-SHA256 test failed");
    if let Value::String(s) = res {
        assert_eq!(s.len(), 64);
    } else {
        panic!("Expected 64-char hex string");
    }
}

#[test]
fn test_strings_split_join_and_starts_with() {
    let code = r#"
from strings import split, join, starts_with

let parts = split("users/102/profile", "/")
let joined = join("::", parts)
let sw = starts_with(":id", ":")
[parts, joined, sw]
"#;
    let res = run_source(code).expect("Strings test failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[1], Value::string("users::102::profile"));
        assert_eq!(items[2], Value::Bool(true));
    } else {
        panic!("Expected array");
    }
}

#[test]
fn test_http_listen_accept_and_respond() {
    let code = r#"
from http import listen, accept, respond, close, get
from sys import sleep

let port = 19876

spawn:
    let listener = listen(port)
    let req = accept(listener, 4000)
    if req != nil:
        respond(req["id"], 200, {"Content-Type": "application/json"}, '{"pong": true}')
    close(listener)

sleep(0.05)
let client_res = get("http://127.0.0.1:19876/ping")
[client_res["status"], client_res["ok"], client_res["body"]]
"#;
    let res = run_source(code).expect("HTTP listen/accept/respond failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(200));
        assert_eq!(items[1], Value::Bool(true));
        assert_eq!(items[2], Value::string("{\"pong\": true}"));
    } else {
        panic!("Expected array response");
    }
}

#[test]
fn test_aether_web_framework_routing_and_dispatch() {
    let code = r#"
from aether_web import App, Response

let app = App()

# Middleware adding a trace header
fn trace_mw(req):
    req.headers["x-trace-id"] = "trace-999"
    return nil

app.add_middleware(trace_mw)

fn health_handler(req):
    return Response.json({"status": "healthy"})

app.get("/health", health_handler)

fn user_handler(req):
    let user_id = req.params["id"]
    return Response.json({"user": user_id, "trace": req.headers["x-trace-id"]})

app.get("/api/users/:id", user_handler)

# 1. Simulate GET /health
let req1 = {"id": 1, "method": "GET", "path": "/health", "query": "", "headers": {}, "body": ""}
let res1 = app.handle(req1)

# 2. Simulate GET /api/users/42
let req2 = {"id": 2, "method": "GET", "path": "/api/users/42", "query": "", "headers": {}, "body": ""}
let res2 = app.handle(req2)

# 3. Simulate GET /missing
let req3 = {"id": 3, "method": "GET", "path": "/missing", "query": "", "headers": {}, "body": ""}
let res3 = app.handle(req3)

[res1.status, res1.body, res2.status, res2.body, res3.status]
"#;
    let res = run_source(code).expect("AetherWeb dispatch failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(200));
        assert_eq!(items[1], Value::string("{\"status\":\"healthy\"}"));
        assert_eq!(items[2], Value::Int(200));
        assert_eq!(items[3], Value::string("{\"trace\":\"trace-999\",\"user\":\"42\"}"));
        assert_eq!(items[4], Value::Int(404));
    } else {
        panic!("Expected array of results");
    }
}
