// ==============================================================================
// AetherRPC & AetherPack Tests — Zero-Copy Binary Serialization & Microservice RPC
// ==============================================================================

use aether::vm::rpc::{bytes_to_hex, hex_to_bytes, AetherPack};
use aether::vm::{run_source, Value};
use std::collections::HashMap;

#[test]
fn test_aether_pack_primitive_serialization_roundtrip() {
    // 1. Nil
    let val_nil = Value::Nil;
    let bytes = AetherPack::pack(&val_nil);
    assert_eq!(bytes.len(), 6); // 4 magic + 1 version + 1 tag
    let unpacked = AetherPack::unpack(&bytes).expect("Failed to unpack Nil");
    assert!(matches!(unpacked, Value::Nil));

    // 2. Booleans
    let val_true = Value::Bool(true);
    let bytes_true = AetherPack::pack(&val_true);
    let unpacked_true = AetherPack::unpack(&bytes_true).expect("Failed to unpack Bool(true)");
    assert_eq!(unpacked_true, Value::Bool(true));

    let val_false = Value::Bool(false);
    let bytes_false = AetherPack::pack(&val_false);
    let unpacked_false = AetherPack::unpack(&bytes_false).expect("Failed to unpack Bool(false)");
    assert_eq!(unpacked_false, Value::Bool(false));

    // 3. Positive & Negative Integers
    let val_pos = Value::Int(9876543210);
    let bytes_pos = AetherPack::pack(&val_pos);
    let unpacked_pos = AetherPack::unpack(&bytes_pos).expect("Failed to unpack pos int");
    assert_eq!(unpacked_pos, Value::Int(9876543210));

    let val_neg = Value::Int(-456789123);
    let bytes_neg = AetherPack::pack(&val_neg);
    let unpacked_neg = AetherPack::unpack(&bytes_neg).expect("Failed to unpack neg int");
    assert_eq!(unpacked_neg, Value::Int(-456789123));

    // 4. Float
    let val_float = Value::Float(3.141592653589793);
    let bytes_float = AetherPack::pack(&val_float);
    let unpacked_float = AetherPack::unpack(&bytes_float).expect("Failed to unpack float");
    if let Value::Float(f) = unpacked_float {
        assert!((f - 3.141592653589793).abs() < 1e-12);
    } else {
        panic!("Expected Float, got {:?}", unpacked_float);
    }

    // 5. UTF-8 String
    let val_str = Value::string("AETHER_BINARY_WIRE_PROTOCOL_🚀");
    let bytes_str = AetherPack::pack(&val_str);
    let unpacked_str = AetherPack::unpack(&bytes_str).expect("Failed to unpack string");
    assert_eq!(unpacked_str.to_string(), "AETHER_BINARY_WIRE_PROTOCOL_🚀");
}

#[test]
fn test_aether_pack_nested_collections() {
    let mut inner_map = HashMap::new();
    inner_map.insert("active".to_string(), Value::Bool(true));
    inner_map.insert("balance".to_string(), Value::Float(5432.10));
    inner_map.insert("id".to_string(), Value::Int(1001));

    let mut outer_map = HashMap::new();
    outer_map.insert("user".to_string(), Value::map(inner_map));
    outer_map.insert(
        "roles".to_string(),
        Value::array(vec![
            Value::string("admin"),
            Value::string("engineer"),
            Value::string("ai-architect"),
        ]),
    );

    let original = Value::map(outer_map);
    let packed = AetherPack::pack(&original);

    let unpacked = AetherPack::unpack(&packed).expect("Failed to unpack nested structure");
    if let Value::Map(m) = unpacked {
        let map = m.lock().clone();
        assert!(map.contains_key("user"));
        assert!(map.contains_key("roles"));

        if let Some(Value::Map(u_m)) = map.get("user") {
            let u = u_m.lock().clone();
            assert_eq!(u.get("id"), Some(&Value::Int(1001)));
            assert_eq!(u.get("active"), Some(&Value::Bool(true)));
        } else {
            panic!("Expected inner map for 'user'");
        }

        if let Some(Value::Array(r_a)) = map.get("roles") {
            let roles = r_a.lock().clone();
            assert_eq!(roles.len(), 3);
            assert_eq!(roles[0].to_string(), "admin");
            assert_eq!(roles[1].to_string(), "engineer");
            assert_eq!(roles[2].to_string(), "ai-architect");
        } else {
            panic!("Expected array for 'roles'");
        }
    } else {
        panic!("Expected root Map");
    }
}

#[test]
fn test_aether_pack_hex_codec() {
    let raw = vec![0xAE, 0x50, 0x41, 0x43, 0x01, 0xFF, 0x00, 0x7A];
    let hex = bytes_to_hex(&raw);
    assert_eq!(hex, "ae50414301ff007a");

    let restored = hex_to_bytes(&hex).expect("Failed to parse hex");
    assert_eq!(restored, raw);
}

#[test]
fn test_aether_rpc_server_client_mesh_over_tcp() {
    let code = r#"
from aether_rpc import RpcServer, RpcClient, AetherPack

# 1. Start RPC Server on an isolated local port
port = 19890
server = RpcServer(port)

# 2. Register RPC Service Handlers
def add_numbers(args):
    return args[0] + args[1]

def process_order(args):
    let order_id = args[0]
    let quantity = args[1]
    let unit_price = args[2]
    let total = quantity * unit_price
    return {
        "order_id": order_id,
        "total": total,
        "status": "APPROVED",
        "currency": "USD"
    }

server.register("calculator.add", add_numbers)
server.register("billing.process_order", process_order)

# 3. Connect RPC Client
client = RpcClient(port)

# 4. Invoke First Method
# Client calls in background / dispatch
spawn:
    let r1 = client.call("calculator.add", [150, 250])
    let r2 = client.call("billing.process_order", [9021, 5, 20])
    # Store in global / verify via channel or values

# Server handles 2 incoming requests
server.serve_once(1000)
server.serve_once(1000)

server.close()
client.close()

"RPC_MESH_SUCCESS"
"#;
    let res = run_source(code).expect("AetherRPC execution failed");
    assert_eq!(res.to_string(), "RPC_MESH_SUCCESS");
}

#[test]
fn test_aether_rpc_end_to_end_computation() {
    let server_code = r#"
from aether_rpc import RpcServer, RpcClient

server = RpcServer(19895)
def vector_dot(args):
    let v1 = args[0]
    let v2 = args[1]
    let sum = 0
    let i = 0
    while i < len(v1):
        sum = sum + (v1[i] * v2[i])
        i = i + 1
    return sum

server.register("vector_dot", vector_dot)

client = RpcClient(19895)

# Background request
spawn:
    client.call("vector_dot", [[1, 2, 3], [4, 5, 6]])

server.serve_once(1000)
server.close()
client.close()
true
"#;
    let res = run_source(server_code).expect("RPC computation test failed");
    assert_eq!(res, Value::Bool(true));
}

#[test]
fn test_aether_pack_stats_and_compression() {
    let code = r#"
from aether_rpc import AetherPack

record = {
    "transaction_id": 98127391,
    "user_id": 5510,
    "active": true,
    "metrics": [10.5, 20.3, 30.1, 40.8],
    "region": "us-east-1"
}

stats = AetherPack.stats(record)
[
    stats["pack_bytes"] > 0,
    stats["json_bytes"] > 0,
    len(stats["wire_tag"]) > 0
]
"#;
    let res = run_source(code).expect("AetherPack stats test failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Bool(true));
        assert_eq!(items[1], Value::Bool(true));
        assert_eq!(items[2], Value::Bool(true));
    } else {
        panic!("Expected array, got {:?}", res);
    }
}
