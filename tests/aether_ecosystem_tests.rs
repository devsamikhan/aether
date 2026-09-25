// ==============================================================================
// AETHER 2.0 Ecosystem Integration Tests
// Verifies LSP Server, AetherPM Package Manager, Web Playground & MCP AI Server
// ==============================================================================

use aether::lsp::AetherLanguageServer;
use aether::mcp_server::AetherMcpServer;
use aether::package_manager::{compute_file_sha256, sha256_digest, AetherManifest, AetherPackageManager};

#[test]
fn test_lsp_diagnostics_and_hover() {
    let lsp = AetherLanguageServer::new();

    // 1. Valid code has 0 diagnostics
    let valid_code = "fn compute(x) { return x * 2; }";
    let diags = lsp.analyze(valid_code);
    assert!(diags.is_empty(), "Valid code should have zero diagnostics");

    // 2. Syntax error produces structured diagnostic
    let broken_code = "fn bad(x { return x;";
    let err_diags = lsp.analyze(broken_code);
    assert!(!err_diags.is_empty(), "Malformed code should produce diagnostic");
    assert_eq!(err_diags[0].severity, 1);

    // 3. Hover documentation lookup
    assert!(lsp.get_hover("intent").is_some());
    assert!(lsp.get_hover("QuantumSimulator").is_some());
    assert!(lsp.get_hover("Tensor").is_some());
    assert!(lsp.get_hover("Graph").is_some());
    assert!(lsp.get_hover("nonexistent").is_none());
}

#[test]
fn test_lsp_jsonrpc_lifecycle() {
    let mut lsp = AetherLanguageServer::new();

    // 1. Initialize request
    let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let resp = lsp.handle_jsonrpc(init_req);
    assert!(resp.is_some());
    assert!(resp.unwrap().contains("capabilities"));

    // 2. Hover request
    let hover_req = r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"word":"Tensor"}}"#;
    let hover_resp = lsp.handle_jsonrpc(hover_req);
    assert!(hover_resp.is_some());
    assert!(hover_resp.unwrap().contains("Tensor & Autograd"));
}

#[test]
fn test_package_manager_manifest_and_lock() {
    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let pm = AetherPackageManager::new(temp_dir.path());

    // 1. Create aether.toml
    let manifest_content = r#"
[package]
name = "test_vision"
version = "0.2.0"
authors = ["Test Engineer"]
description = "Vision processing library"

[dependencies]
math = "1.0.0"
"#;
    let manifest = AetherManifest::parse_toml(manifest_content).expect("Failed to parse manifest");
    assert_eq!(manifest.name, "test_vision");
    assert_eq!(manifest.version, "0.2.0");
    assert_eq!(manifest.dependencies.get("math"), Some(&"1.0.0".to_string()));

    // 2. Save manifest to disk
    manifest.save_to_file(pm.manifest_path()).expect("Failed to save manifest");
    assert!(pm.manifest_path().exists());

    // 3. Add dependency
    let dep = pm.add("neural_core@1.5.0").expect("Failed to add dependency");
    assert_eq!(dep.name, "neural_core");
    assert_eq!(dep.version, "1.5.0");
    assert!(dep.checksum.is_some());

    // Verify lockfile created
    assert!(pm.lock_path().exists());
    let lock_text = std::fs::read_to_string(pm.lock_path()).expect("Failed to read lock");
    assert!(lock_text.contains("neural_core 1.5.0"));

    // 4. Remove dependency
    pm.remove("neural_core").expect("Failed to remove dependency");
    let updated_manifest = AetherManifest::load_from_file(pm.manifest_path()).unwrap();
    assert!(!updated_manifest.dependencies.contains_key("neural_core"));

    // 5. Test publish
    let pub_msg = pm.publish().expect("Failed to publish package");
    assert!(pub_msg.contains("Successfully packaged 'test_vision'"));
}

#[test]
fn test_sha256_cryptographic_integrity() {
    // Known NIST/FIPS test vector: "" (empty string)
    assert_eq!(
        sha256_digest(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );

    // Known NIST/FIPS test vector: "hello"
    assert_eq!(
        sha256_digest(b"hello"),
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );

    // File hash verification
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), b"hello").unwrap();
    let file_hash = compute_file_sha256(temp_file.path()).unwrap();
    assert_eq!(
        file_hash,
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}

#[test]
fn test_mcp_ai_server_protocol() {
    let mcp = AetherMcpServer::new();

    // 1. Handshake initialize
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let resp = mcp.handle_request(init).unwrap();
    assert!(resp.contains("\"name\":\"aether-mcp\""));

    // 2. Tools list
    let tools_list = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
    let tools_resp = mcp.handle_request(tools_list).unwrap();
    assert!(tools_resp.contains("aether_run"));
    assert!(tools_resp.contains("aether_doctor"));

    // 3. Tool execution: aether_check
    let check_call = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"aether_check","code":"let a = 10;"}}"#;
    let check_resp = mcp.handle_request(check_call).unwrap();
    assert!(check_resp.contains("Syntax check PASSED"));

    // 4. Tool execution: aether_run
    let run_call = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"aether_run","code":"20 + 30"}}"#;
    let run_resp = mcp.handle_request(run_call).unwrap();
    assert!(run_resp.contains("Result: 50"));

    // 5. Resources list
    let res_list = r#"{"jsonrpc":"2.0","id":5,"method":"resources/list"}"#;
    let res_resp = mcp.handle_request(res_list).unwrap();
    assert!(res_resp.contains("aether://spec"));
}
