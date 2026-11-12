// ==============================================================================
// AETHER 2.0 Ecosystem Integration Tests
// Verifies LSP Server, AetherPM Package Manager, Web Playground & MCP AI Server
// ==============================================================================

use aether::lsp::AetherLanguageServer;
use aether::mcp_server::AetherMcpServer;
use aether::package_manager::{
    compute_file_sha256, sha256_digest, AetherManifest, AetherPackageManager,
};

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
    assert!(
        !err_diags.is_empty(),
        "Malformed code should produce diagnostic"
    );
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
    let hover_req =
        r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"word":"Tensor"}}"#;
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
    assert_eq!(
        manifest.dependencies.get("math"),
        Some(&"1.0.0".to_string())
    );

    // 2. Save manifest to disk
    manifest
        .save_to_file(pm.manifest_path())
        .expect("Failed to save manifest");
    assert!(pm.manifest_path().exists());

    // 3. Add dependency
    let dep = pm
        .add("neural_core@1.5.0")
        .expect("Failed to add dependency");
    assert_eq!(dep.name, "neural_core");
    assert_eq!(dep.version, "1.5.0");
    assert!(dep.checksum.is_some());

    // Verify lockfile created
    assert!(pm.lock_path().exists());
    let lock_text = std::fs::read_to_string(pm.lock_path()).expect("Failed to read lock");
    assert!(lock_text.contains("neural_core 1.5.0"));

    // 4. Remove dependency
    pm.remove("neural_core")
        .expect("Failed to remove dependency");
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

#[test]
fn test_lsp_exact_spans_symbols_bio_chem_and_cross_file_definition() {
    use aether::lsp::AetherLanguageServer;
    use serde_json::Value;

    let mut lsp = AetherLanguageServer::new();
    let _ = lsp.handle_jsonrpc(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);

    let lib_uri = "file:///workspace/helpers.ae";
    let lib_open = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": lib_uri,
                "languageId": "aether",
                "version": 1,
                "text": "def compute_gc(seq):\n    return Bio.gc_content(seq)\n"
            }
        }
    });
    let lib_open_diag = lsp
        .handle_jsonrpc(&lib_open.to_string())
        .expect("didOpen publishes initial diagnostics");
    let lib_diag_json: Value = serde_json::from_str(&lib_open_diag).unwrap();
    assert!(lib_diag_json["params"]["diagnostics"]
        .as_array()
        .unwrap()
        .is_empty());

    let main_uri = "file:///workspace/main.ae";
    let main_src = concat!(
        "from helpers import compute_gc\n",
        "let m = Chem.from_smiles(\"CCO\")\n",
        "let x = compute_gc(\"ATGC\")\n",
        "Bio.\n",
        "Chem.\n"
    );
    let main_open = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": main_uri,
                "languageId": "aether",
                "version": 1,
                "text": main_src
            }
        }
    });
    let diag_raw = lsp
        .handle_jsonrpc(&main_open.to_string())
        .expect("didOpen publishes diagnostics");
    let diag_json: Value = serde_json::from_str(&diag_raw).unwrap();
    assert_eq!(diag_json["method"], "textDocument/publishDiagnostics");

    // Hover on Bio and Chem
    let bio_hover_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/hover",
        "params": { "textDocument": { "uri": main_uri }, "position": { "line": 3, "character": 1 } }
    });
    let bio_hover: Value =
        serde_json::from_str(&lsp.handle_jsonrpc(&bio_hover_req.to_string()).unwrap()).unwrap();
    assert!(bio_hover["result"]["contents"]["value"]
        .as_str()
        .unwrap_or("")
        .contains("Bio"));

    // Completions after "Bio." and "Chem."
    let bio_comp_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "textDocument/completion",
        "params": { "textDocument": { "uri": main_uri }, "position": { "line": 3, "character": 4 } }
    });
    let bio_comp: Value =
        serde_json::from_str(&lsp.handle_jsonrpc(&bio_comp_req.to_string()).unwrap()).unwrap();
    let bio_labels: Vec<&str> = bio_comp["result"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["label"].as_str())
        .collect();
    assert!(bio_labels.contains(&"gc_content"));
    assert!(bio_labels.contains(&"parse_fasta"));
    assert!(bio_labels.contains(&"parse_fastq"));
    assert!(bio_labels.contains(&"orf_finder"));

    let chem_comp_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 31,
        "method": "textDocument/completion",
        "params": { "textDocument": { "uri": main_uri }, "position": { "line": 4, "character": 5 } }
    });
    let chem_comp: Value =
        serde_json::from_str(&lsp.handle_jsonrpc(&chem_comp_req.to_string()).unwrap()).unwrap();
    let chem_labels: Vec<&str> = chem_comp["result"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["label"].as_str())
        .collect();
    assert!(chem_labels.contains(&"from_smiles"));
    assert!(chem_labels.contains(&"parse_sdf"));
    assert!(chem_labels.contains(&"substructure_match"));

    // Document symbols
    let sym_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "textDocument/documentSymbol",
        "params": { "textDocument": { "uri": lib_uri } }
    });
    let sym_resp: Value =
        serde_json::from_str(&lsp.handle_jsonrpc(&sym_req.to_string()).unwrap()).unwrap();
    let sym_names: Vec<&str> = sym_resp["result"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s["name"].as_str())
        .collect();
    assert!(sym_names.contains(&"compute_gc"));

    // Cross-document definition lookup from main.ae line 2 ("compute_gc") -> helpers.ae
    let def_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "textDocument/definition",
        "params": { "textDocument": { "uri": main_uri }, "position": { "line": 2, "character": 10 } }
    });
    let def_resp: Value =
        serde_json::from_str(&lsp.handle_jsonrpc(&def_req.to_string()).unwrap()).unwrap();
    assert_eq!(def_resp["result"]["uri"], lib_uri);

    // Exact span check for TypeError diagnostic (mismatch on line 2 -> 0-indexed line 1)
    let err_uri = "file:///workspace/type_err.ae";
    let err_open = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": err_uri,
                "languageId": "aether",
                "version": 1,
                "text": "let ok = 10\ntrait Printable:\n    def show()\nstruct Empty {}\nimpl Printable for Empty {}\n"
            }
        }
    });
    let err_diag_raw = lsp.handle_jsonrpc(&err_open.to_string()).unwrap();
    let err_diag: Value = serde_json::from_str(&err_diag_raw).unwrap();
    let diags = err_diag["params"]["diagnostics"].as_array().unwrap();
    assert!(
        !diags.is_empty(),
        "Expected missing trait method diagnostic"
    );
    assert_eq!(diags[0]["range"]["start"]["line"], 4);

    // Close clears diagnostics cleanly
    let close_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didClose",
        "params": { "textDocument": { "uri": err_uri } }
    });
    let close_resp: Value =
        serde_json::from_str(&lsp.handle_jsonrpc(&close_req.to_string()).unwrap()).unwrap();
    assert!(close_resp["params"]["diagnostics"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn test_package_manager_http_fetch_local_and_checksum_verification() {
    use aether::package_manager::AetherPackageManager;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    // 1. Ephemeral HTTP server serving an .ae package
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let remote_code = "def greet(name):\n    return \"Hello \" + name\n";

    let server_handle = std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut req_buf = [0u8; 1024];
            let _ = stream.read(&mut req_buf);
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                remote_code.len(),
                remote_code
            );
            let _ = stream.write_all(resp.as_bytes());
        }
    });

    let proj_dir = tempfile::tempdir().unwrap();
    let pm = AetherPackageManager::new(proj_dir.path());

    // Bundled stdlib package copy (`libraries/aether_tensor.ae`)
    let stdlib_dep = pm
        .add("aether_tensor@1.0.0")
        .expect("Bundled library copy should succeed");
    assert_eq!(stdlib_dep.name, "aether_tensor");

    // HTTP source fetch via parse_package_spec ("<url>@<version>")
    let remote_spec = format!("http://127.0.0.1:{}/hello_http.ae@1.2.3", port);
    let remote_dep = pm
        .add(&remote_spec)
        .expect("HTTP package fetch should succeed");
    assert_eq!(remote_dep.name, "hello_http");
    assert_eq!(remote_dep.version, "1.2.3");

    // Install from lockfile passes with matching checksum
    let installed = pm.install_all().expect("Lockfile verification should pass");
    assert!(installed.iter().any(|p| p.name == "hello_http"));

    // Tamper lockfile checksum and verify install_all detects corruption!
    let lock_path = pm.lock_path();
    let lock_orig = std::fs::read_to_string(&lock_path).unwrap();
    let mut mutated = lock_orig.clone();
    if let Some(pos) = mutated.find("sha256 = \"") {
        let replace_at = pos + 10;
        let replacement_char = if &mutated[replace_at..replace_at + 1] == "0" {
            "1"
        } else {
            "0"
        };
        mutated.replace_range(replace_at..replace_at + 1, replacement_char);
        std::fs::write(&lock_path, mutated).unwrap();
        let corrupt_res = pm.install_all();
        assert!(
            corrupt_res.is_err(),
            "Expected SHA-256 checksum mismatch error"
        );
        std::fs::write(&lock_path, lock_orig).unwrap();
    }

    // Local directory package copy test
    let local_pkg_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        local_pkg_dir.path().join("lib.ae"),
        "def square(n):\n    return n * n\n",
    )
    .unwrap();
    let local_spec = format!("{}@1.0.0", local_pkg_dir.path().display());
    let local_dep = pm
        .add(&local_spec)
        .expect("Local path package add should succeed");
    assert!(!local_dep.name.is_empty());

    let _ = server_handle.join();
}
