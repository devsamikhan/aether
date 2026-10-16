use std::fs;
use std::path::PathBuf;
use aether::codegen::shell_bundler::{ShellBundler, ShellConfig, get_aether_bridge_js, inline_web_assets};
use aether::toolchain::scaffold_project;

#[test]
fn test_aether_bridge_js_contains_core_api() {
    let bridge_js = get_aether_bridge_js();
    assert!(bridge_js.contains("window.Aether"));
    assert!(bridge_js.contains("vibrate"));
    assert!(bridge_js.contains("showToast"));
    assert!(bridge_js.contains("deviceInfo"));
    assert!(bridge_js.contains("ai"));
    assert!(bridge_js.contains("dbQuery"));
    assert!(bridge_js.contains("invoke"));
    assert!(bridge_js.contains("hapticFeedback"));
}

#[test]
fn test_inline_web_assets() {
    let html = "<!DOCTYPE html><html><head><title>App</title></head><body><h1>Hello</h1></body></html>";
    let inlined = inline_web_assets(&PathBuf::from("."), html).expect("Inlining should succeed");
    assert!(inlined.contains("window.Aether"));
    assert!(inlined.contains("<script>"));
    assert!(inlined.contains("</head>"));
}

#[test]
fn test_bundle_web_distribution() {
    let temp_dist = PathBuf::from("target/test_web_dist");
    let _ = fs::remove_dir_all(&temp_dist);
    fs::create_dir_all(&temp_dist).unwrap();

    let index_file = temp_dist.join("index.html");
    fs::write(
        &index_file,
        r#"<!DOCTYPE html><html><head><title>Test App</title></head><body><div id="root">Aether UI</div></body></html>"#,
    ).unwrap();

    let output_html = PathBuf::from("target/test_bundle_output.html");
    let mut config = ShellConfig::new(temp_dist.clone(), output_html.clone());
    config.target = "web".to_string();
    config.app_name = "Test React App".to_string();

    let bundler = ShellBundler::new(config);
    bundler.bundle_web().expect("Web bundle should succeed");

    assert!(output_html.exists());
    let content = fs::read_to_string(&output_html).unwrap();
    assert!(content.contains("Aether UI"));
    assert!(content.contains("window.Aether"));

    let _ = fs::remove_file(output_html);
    let _ = fs::remove_dir_all(temp_dist);
}

#[test]
fn test_bundle_apk_distribution() {
    let temp_dist = PathBuf::from("target/test_react_dist");
    let _ = fs::remove_dir_all(&temp_dist);
    fs::create_dir_all(&temp_dist).unwrap();

    let index_file = temp_dist.join("index.html");
    fs::write(
        &index_file,
        r#"<!DOCTYPE html><html><head><title>React Omni</title></head><body><button onclick="Aether.vibrate(50)">Click</button></body></html>"#,
    ).unwrap();

    let app_css = temp_dist.join("style.css");
    fs::write(&app_css, "body { background: #000; color: #fff; }").unwrap();

    let output_apk = PathBuf::from("target/test_react_app.apk");
    let mut config = ShellConfig::new(temp_dist.clone(), output_apk.clone());
    config.target = "apk".to_string();
    config.app_name = "React Omni Test".to_string();
    config.package_name = "com.test.reactomni".to_string();

    let bundler = ShellBundler::new(config);
    bundler.bundle_apk().expect("APK bundle should succeed");

    assert!(output_apk.exists());
    let apk_bytes = fs::read(&output_apk).unwrap();
    assert!(apk_bytes.len() > 1024);
    // Verify ZIP magic bytes
    assert_eq!(&apk_bytes[0..4], &[0x50, 0x4B, 0x03, 0x04]);

    // Verify all required APK components and web assets are embedded in the ZIP
    let expected_entries = [
        "AndroidManifest.xml",
        "classes.dex",
        "resources.arsc",
        "res/drawable/ic_launcher.png",
        "assets/www/index.html",
        "assets/www/aether_bridge.js",
        "assets/www/style.css",
        "META-INF/MANIFEST.MF",
        "META-INF/CERT.SF",
        "META-INF/CERT.RSA",
    ];

    for expected in &expected_entries {
        assert!(
            apk_bytes.windows(expected.len()).any(|w| w == expected.as_bytes()),
            "Expected entry '{}' missing in bundled APK",
            expected
        );
    }

    let _ = fs::remove_file(output_apk);
    let _ = fs::remove_dir_all(temp_dist);
}

#[test]
fn test_scaffold_frontend_templates() {
    let templates = ["react", "vue", "svelte", "3d", "fullstack"];
    for tpl in &templates {
        let proj_name = format!("target/test_proj_{}", tpl);
        let _ = fs::remove_dir_all(&proj_name);

        scaffold_project(&proj_name, tpl).expect("Scaffolding should succeed");
        let proj_path = PathBuf::from(&proj_name);

        assert!(proj_path.join("aether.toml").exists());
        assert!(proj_path.join("src/main.ae").exists());
        assert!(proj_path.join("frontend/index.html").exists());
        assert!(proj_path.join("frontend/aether_bridge.js").exists());
        assert!(proj_path.join("frontend/icon.svg").exists());

        // Verify HTML content has framework signatures
        let index_html = fs::read_to_string(proj_path.join("frontend/index.html")).unwrap();
        match *tpl {
            "react" => {
                assert!(index_html.contains("React") || index_html.contains("react"));
                assert!(index_html.contains("useState") || index_html.contains("AETHER React"));
            }
            "vue" => {
                assert!(index_html.contains("Vue") || index_html.contains("vue"));
                assert!(index_html.contains("createApp") || index_html.contains("AETHER Vue"));
            }
            "svelte" => {
                assert!(index_html.contains("Svelte") || index_html.contains("svelte"));
                assert!(index_html.contains("Store") || index_html.contains("Signals"));
            }
            "3d" => {
                assert!(index_html.contains("WebGL") || index_html.contains("Canvas") || index_html.contains("Universe"));
                assert!(index_html.contains("requestAnimationFrame"));
            }
            "fullstack" => {
                assert!(index_html.contains("Fullstack") || index_html.contains("Relational"));
            }
            _ => {}
        }

        // Test that it can be bundled into standalone HTML with zero external tools
        let bundle_out = proj_path.join(format!("{}.html", tpl));
        let mut config = ShellConfig::new(proj_path.join("frontend"), bundle_out.clone());
        config.target = "web".to_string();
        let bundler = ShellBundler::new(config);
        assert!(bundler.bundle_web().is_ok());
        assert!(bundle_out.exists());

        let _ = fs::remove_dir_all(proj_path);
    }
}

#[test]
fn test_flagship_07_execution_and_artifacts() {
    let script_path = std::path::Path::new("flagship_projects/07_aether_omni_react_app/main.ae");
    assert!(script_path.exists(), "Flagship 07 main.ae must exist");
    let source = std::fs::read_to_string(script_path).unwrap();
    let res = aether::vm::run_source(&source);
    assert!(res.is_ok(), "Flagship 07 execution failed: {:?}", res.err());

    let apk = std::path::Path::new("flagship_projects/07_aether_omni_react_app/AetherOmni.apk");
    assert!(apk.exists(), "AetherOmni.apk must exist");

    let exe = std::path::Path::new("flagship_projects/07_aether_omni_react_app/AetherOmni.exe");
    assert!(exe.exists(), "AetherOmni.exe must exist");

    let html = std::path::Path::new("flagship_projects/07_aether_omni_react_app/AetherOmni.html");
    assert!(html.exists(), "AetherOmni.html must exist");
}

#[test]
fn test_flagship_08_chronopulse_alarm_execution_and_artifacts() {
    let script_path = std::path::Path::new("flagship_projects/08_aether_chronopulse_alarm/main.ae");
    assert!(script_path.exists(), "Flagship 08 main.ae must exist");
    let source = std::fs::read_to_string(script_path).unwrap();
    let res = aether::vm::run_source(&source);
    assert!(res.is_ok(), "Flagship 08 execution failed: {:?}", res.err());

    let apk = std::path::Path::new("flagship_projects/08_aether_chronopulse_alarm/ChronoPulse.apk");
    assert!(apk.exists(), "ChronoPulse.apk must exist");

    let exe = std::path::Path::new("flagship_projects/08_aether_chronopulse_alarm/ChronoPulse.exe");
    assert!(exe.exists(), "ChronoPulse.exe must exist");

    let html = std::path::Path::new("flagship_projects/08_aether_chronopulse_alarm/ChronoPulse.html");
    assert!(html.exists(), "ChronoPulse.html must exist");
}

#[test]
fn test_live_hotreload_server_and_bridge() {
    let bridge = get_aether_bridge_js();
    assert!(bridge.contains("initLiveHotReload"), "Default bridge must have Live Hot-Reload engine");
    assert!(bridge.contains("/api/live-status"), "Default bridge must query /api/live-status");
    assert!(bridge.contains("/api/live-poll"), "Default bridge must query /api/live-poll");

    let custom_bridge_path = std::path::Path::new("flagship_projects/08_aether_chronopulse_alarm/frontend/aether_bridge.js");
    let custom_bridge = std::fs::read_to_string(custom_bridge_path).unwrap();
    assert!(custom_bridge.contains("initLiveHotReload"), "ChronoPulse bridge must have Live Hot-Reload engine");
    assert!(custom_bridge.contains("aether-live-hud-pill"), "ChronoPulse bridge must have live HUD pill");

    let config = aether::codegen::live_server::LiveServerConfig::new("flagship_projects/08_aether_chronopulse_alarm/frontend");
    assert_eq!(config.port, 8080);
    assert_eq!(config.poll_interval_ms, 80);
}

#[test]
fn test_adb_deployer_device_discovery() {
    use aether::codegen::adb_deployer::AdbDeployer;
    let devices = AdbDeployer::list_devices();
    assert!(devices.is_ok(), "AdbDeployer::list_devices() must run without error");
    let list = devices.unwrap();
    println!("Found {} connected ADB device(s): {:?}", list.len(), list);
}

#[test]
fn test_vsix_extension_packager() {
    use aether::codegen::vsix_builder::VsixBuilder;
    let out_dir = std::path::Path::new("target/test_vsix_out");
    let _ = std::fs::create_dir_all(out_dir);
    let vsix_path = out_dir.join("aether-language-support-2.0.0.vsix");
    let builder = VsixBuilder::new("editors/vscode", &vsix_path);
    let res = builder.package();
    assert!(res.is_ok(), "VsixBuilder::package() should succeed: {:?}", res.err());
    assert!(vsix_path.exists(), "Packaged .vsix extension must exist on disk");
    let size = std::fs::metadata(&vsix_path).unwrap().len();
    assert!(size > 500, "VSIX file must contain valid zipped package contents");
    let _ = std::fs::remove_dir_all(out_dir);
}

#[test]
fn test_native_ai_engine_rag_and_agent() {
    use aether::vm::ai_engine::{generate_embedding, cosine_similarity, VectorRagStore, LocalAgent};

    let v1 = generate_embedding("AETHER native fast ahead of time compiler");
    let v2 = generate_embedding("AETHER compiler performance benchmark");
    let v3 = generate_embedding("banana apple cooking recipes");

    assert_eq!(v1.len(), 128);
    let sim_related = cosine_similarity(&v1, &v2);
    let sim_unrelated = cosine_similarity(&v1, &v3);
    assert!(sim_related > sim_unrelated, "Semantic similarity between related phrases must exceed unrelated phrases");

    let mut store = VectorRagStore::new();
    store.add("AETHER compiles directly to native binary code via Cranelift.");
    store.add("AETHER features embedded zero-dependency SQLite and DataFrame engine.");
    let hits = store.search("Cranelift native compiler", 1);
    assert_eq!(hits.len(), 1);
    assert!(hits[0].2.contains("Cranelift"));

    let mut agent = LocalAgent::new("Nova", "System Architect");
    let reply = agent.chat("Who are you?");
    assert!(reply.contains("Nova"));

    // Test AETHER VM script integration with AI module
    let script = "let emb = AI.embed(\"aether on-device ai\")\nlet bot = AI.Agent(\"Alpha\", \"Autonomous Copilot\")\nbot.add_knowledge(\"AETHER 2.0 has zero external dependencies.\")\nlet res = bot.chat(\"Tell me about AETHER\")\nres\n";
    let vm_res = aether::vm::run_source(script);
    assert!(vm_res.is_ok(), "VM script with AI module must succeed: {:?}", vm_res.err());
}

#[test]
fn test_p2p_swarm_node_and_beacons() {
    use aether::vm::swarm::{SwarmConfig, SwarmNode, DEFAULT_SWARM_PORT};

    let config = SwarmConfig::default();
    let node = SwarmNode::new(config);
    assert_eq!(node.config.swarm_port, DEFAULT_SWARM_PORT);
    assert_eq!(node.get_active_peers().len(), 0);
}

#[test]
fn test_expanded_mobile_hardware_apis_in_bridge() {
    let bridge = get_aether_bridge_js();
    assert!(bridge.contains("getLocation"), "Bridge must have getLocation API");
    assert!(bridge.contains("scanQR"), "Bridge must have scanQR API");
    assert!(bridge.contains("scheduleAlarm"), "Bridge must have scheduleAlarm API");
    assert!(bridge.contains("showNotification"), "Bridge must have showNotification API");
    assert!(bridge.contains("pickFile"), "Bridge must have pickFile API");
    assert!(bridge.contains("isDarkMode"), "Bridge must have isDarkMode API");
}

