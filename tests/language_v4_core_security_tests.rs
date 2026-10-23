use aether::vm::run_source as execute_source;
use aether::vm::value::Value;

#[test]
fn test_capability_sandbox_fs_blocking_and_fuel_metering() {
    let src = r#"
let safe_val = Security.sandbox("let x = 21 * 2; x", {"fuel": 5000})

let fs_blocked = false
try {
    Security.sandbox("File.read('Cargo.toml')", {"fuel": 5000})
} catch as err {
    fs_blocked = true
}

let fuel_stopped = false
try {
    Security.sandbox("let i = 0; while true { i += 1 }", {"fuel": 300})
} catch as err {
    fuel_stopped = true
}

f"{safe_val}:{fs_blocked}:{fuel_stopped}"
"#;
    let res = execute_source(src).expect("Execution failed");
    assert_eq!(res, Value::string("42:true:true"));
}

#[test]
fn test_stack_overflow_guard_catchable_recursion() {
    let src = r#"
let overflow_caught = false
try {
    Security.sandbox("fn recurse(n) { return recurse(n + 1) }; recurse(1)", {"max_call_depth": 32, "fuel": 50000})
} catch as err {
    overflow_caught = true
}
overflow_caught
"#;
    let res = execute_source(src).expect("Execution failed");
    assert_eq!(res, Value::Bool(true));
}

#[test]
fn test_secret_redaction_and_controlled_exposure() {
    let src = r#"
let key = Secret("sk-aether-99887766")
let key_same = Secret("sk-aether-99887766")
let key_diff = Secret("sk-other-00000000")

let printed = f"key={key}"
let eq_same = (key == key_same)
let eq_diff = (key == key_diff)
let raw = Security.expose(key)

f"{printed}|{eq_same}|{eq_diff}|{raw}"
"#;
    let res = execute_source(src).expect("Execution failed");
    assert_eq!(
        res,
        Value::string("key=***REDACTED***|true|false|sk-aether-99887766")
    );
}

#[test]
fn test_static_security_taint_and_secret_scanner() {
    let src = r#"
let vulnerable_code = '
let api_key = "sk-1234567890"
let user_input = input("name")
Sql.execute("SELECT * FROM users WHERE name = " + user_input)
while true {
    let x = 1
}
'
let findings = Security.scan(vulnerable_code)
let c0 = findings[0]["category"]
let c1 = findings[1]["category"]
let c2 = findings[2]["category"]
f"{len(findings)}:{c0}:{c1}:{c2}"
"#;
    let res = execute_source(src).expect("Execution failed");
    assert_eq!(
        res,
        Value::string("3:HardcodedSecret:SqlInjectionTaint:UnboundedLoop")
    );
}

#[test]
fn test_modern_crypto_chacha20_aead_pbkdf2_and_constant_time() {
    let src = r#"
let master_key = "aether-v4-master-key"
let nonce = "unique-nonce-001"
let aad = "header:v4:authenticated"
let message = "Confidential Payload: Launch Sequence 42"

let ct = Crypto.encrypt_aead(master_key, nonce, message, aad)
let decrypted = Crypto.decrypt_aead(master_key, nonce, ct, aad)

let tamper_caught = false
try {
    Crypto.decrypt_aead(master_key, nonce, ct, "wrong-aad")
} catch as err {
    tamper_caught = true
}

let dk1 = Crypto.pbkdf2("MyStrongPass!", "salt-123", 1000, 32)
let dk2 = Crypto.pbkdf2("MyStrongPass!", "salt-123", 1000, 32)
let kdf_match = Crypto.constant_time_eq(dk1, dk2)
let rand_token = Crypto.random_hex(16)

f"{decrypted == message}:{tamper_caught}:{kdf_match}:{len( dk1 )}:{len(rand_token)}"
"#;
    let res = execute_source(src).expect("Execution failed");
    assert_eq!(res, Value::string("true:true:true:64:32"));
}

#[test]
fn test_cycle_detecting_gc_and_string_interner() {
    let src = r#"
let node = {"id": 101}
node["self_ref"] = node

let broken = GC.collect_cycles(node)
let after_ref = node["self_ref"]

let sym1 = GC.intern("aether_field_alpha")
let sym2 = GC.intern("aether_field_alpha")
let resolved = GC.resolve_symbol(sym1)

f"{broken}:{after_ref == nil}:{sym1 == sym2}:{resolved}"
"#;
    let res = execute_source(src).expect("Execution failed");
    assert_eq!(res, Value::string("1:true:true:aether_field_alpha"));
}

#[test]
fn test_precompiled_aec_bytecode_cache_roundtrip() {
    let src = r#"
let program_src = "fn square(n) { return n * n }\nsquare(12) + 6"
let aec_blob = Bytecode.compile_aec(program_src)
let result = Bytecode.run_aec(aec_blob, program_src)

let stale_caught = false
try {
    Bytecode.run_aec(aec_blob, "fn square(n) { return n * n }\nsquare(99)")
} catch as err {
    stale_caught = true
}

f"{result}:{stale_caught}"
"#;
    let res = execute_source(src).expect("Execution failed");
    assert_eq!(res, Value::string("150:true"));
}
