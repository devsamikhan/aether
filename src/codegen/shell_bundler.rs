//! AetherShell — Universal Zero-SDK Multi-Platform Bundler
//!
//! Compiles any modern frontend framework (React, Vue, Svelte, Tailwind, HTML5)
//! into:
//! 1. Standalone, signed Android Packages (.apk) with zero Android Studio / Gradle dependencies.
//! 2. Standalone Windows Native Executables (.exe) with zero external runtimes.
//! 3. Portable Single-File Web Applications (.html) with zero external CDNs.
//!
//! Includes AetherBridge: Bidirectional JavaScript <-> AETHER Native Runtime Bridge.

use std::fs;
use std::path::{Path, PathBuf};
use crate::codegen::apk_builder::{
    generate_binary_manifest, generate_classes_dex, generate_launcher_icon,
    generate_meta_inf_signatures, generate_resources_arsc, ApkConfig, ZipWriter,
};
use crate::codegen::aot_builder::embed_payload;

#[derive(Clone, Debug)]
pub struct ShellConfig {
    pub app_name: String,
    pub package_name: String,
    pub target: String, // "apk", "desktop", "web"
    pub dist_dir: PathBuf,
    pub output_path: PathBuf,
    pub version_name: String,
    pub version_code: u32,
}

impl ShellConfig {
    pub fn new(dist_dir: impl Into<PathBuf>, output_path: impl Into<PathBuf>) -> Self {
        let dist = dist_dir.into();
        let app_name = dist
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("AetherApp")
            .to_string();
        let sanitized = app_name.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "");
        let package_name = format!("com.aether.{}", if sanitized.is_empty() { "app" } else { &sanitized });

        Self {
            app_name,
            package_name,
            target: "apk".to_string(),
            dist_dir: dist,
            output_path: output_path.into(),
            version_name: "1.0.0".to_string(),
            version_code: 1,
        }
    }
}

pub struct ShellBundler {
    pub config: ShellConfig,
}

impl ShellBundler {
    pub fn new(config: ShellConfig) -> Self {
        Self { config }
    }

    /// Bundles frontend assets into the requested target
    pub fn bundle(&self) -> Result<(), String> {
        match self.config.target.to_lowercase().as_str() {
            "apk" | "android" => self.bundle_apk(),
            "desktop" | "exe" | "windows" => self.bundle_desktop(),
            "web" | "html" => self.bundle_web(),
            other => Err(format!("Unsupported bundle target '{}'. Use 'apk', 'desktop', or 'web'.", other)),
        }
    }

    /// Bundles web directory into an installable Android APK with zero Android Studio
    pub fn bundle_apk(&self) -> Result<(), String> {
        if !self.config.dist_dir.exists() {
            return Err(format!("Distribution directory '{}' does not exist", self.config.dist_dir.display()));
        }

        println!("================================================================================");
        println!("📱 AETHERSHELL: ZERO-SDK ANDROID APK BUNDLER");
        println!("================================================================================");
        println!("  Frontend Assets: {}", self.config.dist_dir.display());
        println!("  Target Package:  {}", self.config.package_name);
        println!("  Application:     {}", self.config.app_name);
        println!("  Android Studio:  NOT REQUIRED (Pure-Rust Dalvik & AXML Synthesizer)");
        println!("--------------------------------------------------------------------------------");

        let mut apk_config = ApkConfig::new(&self.config.app_name);
        apk_config.package_name = self.config.package_name.clone();
        apk_config.version_code = self.config.version_code;
        apk_config.version_name = self.config.version_name.clone();
        apk_config.permissions.push("android.permission.INTERNET".to_string());
        apk_config.permissions.push("android.permission.VIBRATE".to_string());
        apk_config.permissions.push("android.permission.ACCESS_NETWORK_STATE".to_string());
        apk_config.permissions.push("android.permission.CAMERA".to_string());
        apk_config.permissions.push("android.permission.ACCESS_FINE_LOCATION".to_string());
        apk_config.permissions.push("android.permission.SCHEDULE_EXACT_ALARM".to_string());
        apk_config.permissions.push("android.permission.POST_NOTIFICATIONS".to_string());

        let binary_manifest = generate_binary_manifest(&apk_config);
        let classes_dex = generate_classes_dex(&apk_config);
        let resources_arsc = generate_resources_arsc(&apk_config);
        let icon_png = generate_launcher_icon();

        let bridge_js = get_aether_bridge_js();

        // Recursively read all files in dist_dir
        let mut asset_files: Vec<(String, Vec<u8>)> = Vec::new();
        Self::collect_files_recursive(&self.config.dist_dir, &self.config.dist_dir, &mut asset_files)?;

        // If no index.html exists, create a default one
        let has_index = asset_files.iter().any(|(p, _)| p == "index.html" || p.ends_with("/index.html"));
        if !has_index {
            let default_html = format!(
                r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><title>{}</title><script src="aether_bridge.js"></script></head>
<body style="margin:0;background:#0f172a;color:#f8fafc;font-family:sans-serif;padding:24px;">
<h1>{}</h1><p>AetherShell Native Web Container Online.</p>
</body></html>"#,
                self.config.app_name, self.config.app_name
            );
            asset_files.push(("index.html".to_string(), default_html.into_bytes()));
        }

        // Add AetherBridge JS if not already present
        let has_bridge = asset_files.iter().any(|(p, _)| p == "aether_bridge.js" || p.ends_with("/aether_bridge.js"));
        if !has_bridge {
            asset_files.push(("aether_bridge.js".to_string(), bridge_js.as_bytes().to_vec()));
        }

        // Deduplicate asset_files by relative path
        let mut unique_assets: Vec<(String, Vec<u8>)> = Vec::new();
        let mut seen_asset_paths = std::collections::HashSet::new();
        for (rel_path, data) in asset_files {
            if seen_asset_paths.insert(rel_path.clone()) {
                unique_assets.push((rel_path, data));
            }
        }
        let asset_files = unique_assets;

        // Prepare files to sign
        let mut files_to_sign_refs: Vec<(&str, &[u8])> = Vec::new();
        files_to_sign_refs.push(("AndroidManifest.xml", &binary_manifest));
        files_to_sign_refs.push(("classes.dex", &classes_dex));
        files_to_sign_refs.push(("resources.arsc", &resources_arsc));
        files_to_sign_refs.push(("res/drawable/ic_launcher.png", &icon_png));

        let formatted_asset_paths: Vec<(String, &[u8])> = asset_files
            .iter()
            .map(|(path, data)| (format!("assets/www/{}", path), data.as_slice()))
            .collect();

        for (path, slice) in &formatted_asset_paths {
            files_to_sign_refs.push((path.as_str(), *slice));
        }

        // Ensure unique sign entries
        let mut seen_sign_names = std::collections::HashSet::new();
        files_to_sign_refs.retain(|(name, _)| seen_sign_names.insert(*name));

        println!("[AetherShell] Synthesizing Dalvik DEX & APK META-INF signatures...");
        let (manifest_mf, cert_sf, cert_rsa) = generate_meta_inf_signatures(&files_to_sign_refs);

        let mut zip = ZipWriter::new();
        zip.add_dir("res/");
        zip.add_dir("res/drawable/");
        zip.add_dir("assets/");
        zip.add_dir("assets/www/");
        zip.add_dir("META-INF/");

        for (name, content) in files_to_sign_refs {
            zip.add_file(name, content);
        }
        zip.add_file("META-INF/MANIFEST.MF", &manifest_mf);
        zip.add_file("META-INF/CERT.SF", &cert_sf);
        zip.add_file("META-INF/CERT.RSA", &cert_rsa);

        let apk_bytes = zip.finish();

        if let Some(parent) = self.config.output_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }

        fs::write(&self.config.output_path, &apk_bytes)
            .map_err(|e| format!("Failed to write APK to '{}': {}", self.config.output_path.display(), e))?;

        println!("✨ Standalone Android APK successfully created: {}", self.config.output_path.display());
        println!("   📦 Package:      {}", self.config.package_name);
        println!("   🏷️  App Name:     {}", self.config.app_name);
        println!("   📱 Web Assets:   {} files embedded inside assets/www/", asset_files.len());
        println!("   ⚖️  Size:         {:.2} KB", apk_bytes.len() as f64 / 1024.0);
        println!("   🔑 Certificate:  V1 Developer Signed");
        println!("   🚀 To Install:   adb install -r {}", self.config.output_path.display());
        println!("================================================================================\n");

        Ok(())
    }

    /// Bundles web directory into a standalone native Windows executable (.exe)
    pub fn bundle_desktop(&self) -> Result<(), String> {
        println!("================================================================================");
        println!("🖥️ AETHERSHELL: ZERO-DEPENDENCY DESKTOP EXECUTABLE BUNDLER");
        println!("================================================================================");
        println!("  Frontend Assets: {}", self.config.dist_dir.display());
        println!("  Output Binary:   {}", self.config.output_path.display());
        println!("  Tauri/Electron:  NOT REQUIRED (Self-Contained Native AETHER Shell)");
        println!("--------------------------------------------------------------------------------");

        let current_exe = std::env::current_exe()
            .map_err(|e| format!("Failed to locate active AETHER binary: {}", e))?;

        // Read index.html or create inline launcher
        let index_html_path = if self.config.dist_dir.is_file() {
            self.config.dist_dir.clone()
        } else {
            self.config.dist_dir.join("index.html")
        };

        let raw_html = if index_html_path.exists() {
            fs::read_to_string(&index_html_path).map_err(|e| e.to_string())?
        } else {
            format!("<!DOCTYPE html><html><body><h1>{}</h1><p>AetherShell Desktop Container</p></body></html>", self.config.app_name)
        };

        let inlined_bundle = inline_web_assets(&self.config.dist_dir, &raw_html)?;

        // Ensure standalone inlined HTML companion is present next to the binary
        let html_companion = self.config.output_path.with_extension("html");
        let html_filename = html_companion
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("index.html");

        let _ = fs::write(&html_companion, &inlined_bundle);

        // Write a launcher script that starts desktop host and opens the application container
        let launcher_script = format!(
            r###"# AetherShell Embedded Desktop Launcher
let app_name = "{}"
print("================================================================================")
print(f"🖥️ AETHERSHELL: {{app_name}} NATIVE DESKTOP HOST RUNNING")
print("================================================================================")
print("  Status:     Online & Operational")
print("  Substrate:  AETHER Cranelift + In-Memory Relational Engine")
print("  Container:  Launching system GUI container...")
Mobile.show_toast(f"Starting {{app_name}}")

# Provision in-memory relational DB for AetherBridge
let db = DB.open(":memory:")
db.execute("CREATE TABLE IF NOT EXISTS bridge_events (id INT, event TEXT, time_ms INT);")

# Launch system default browser to display the application UI
Sys.open_browser("{}")

print("\n✨ Application window opened successfully.")
print("   Host process is active. Keep this console open or minimize it.")
print("   Press Ctrl+C to terminate application host.\n")

# Maintain host process lifecycle
while true:
    Sys.sleep(10)
"###,
            self.config.app_name,
            html_filename
        );

        if let Some(parent) = self.config.output_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }

        embed_payload(&current_exe, &launcher_script, &self.config.output_path)?;

        let size_mb = fs::metadata(&self.config.output_path)
            .map(|m| m.len() as f64 / (1024.0 * 1024.0))
            .unwrap_or(0.0);

        println!("✨ Standalone Desktop Executable successfully built: {}", self.config.output_path.display());
        println!("   🏷️  App Name:     {}", self.config.app_name);
        println!("   ⚖️  Size:         {:.2} MB (Zero external dependencies)", size_mb);
        println!("   🚀 To Run:       {}", self.config.output_path.display());
        println!("================================================================================\n");

        Ok(())
    }

    /// Bundles web directory into a single-file portable HTML application
    pub fn bundle_web(&self) -> Result<(), String> {
        let index_html_path = if self.config.dist_dir.is_file() {
            self.config.dist_dir.clone()
        } else {
            self.config.dist_dir.join("index.html")
        };

        let raw_html = if index_html_path.exists() {
            fs::read_to_string(&index_html_path).map_err(|e| e.to_string())?
        } else {
            format!("<!DOCTYPE html><html><body><h1>{}</h1></body></html>", self.config.app_name)
        };

        let inlined = inline_web_assets(&self.config.dist_dir, &raw_html)?;

        if let Some(parent) = self.config.output_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }

        fs::write(&self.config.output_path, inlined.as_bytes())
            .map_err(|e| format!("Failed to write web bundle '{}': {}", self.config.output_path.display(), e))?;

        let size_kb = inlined.len() as f64 / 1024.0;
        println!("✨ Standalone Web Application successfully bundled: {}", self.config.output_path.display());
        println!("   ⚖️  Size:         {:.2} KB (Zero external CDN dependencies)", size_kb);

        Ok(())
    }

    fn collect_files_recursive(
        base: &Path,
        current: &Path,
        out: &mut Vec<(String, Vec<u8>)>,
    ) -> Result<(), String> {
        if current.is_file() {
            let rel = current.strip_prefix(base).map_err(|e| e.to_string())?;
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let data = fs::read(current).map_err(|e| e.to_string())?;
            out.push((rel_str, data));
            return Ok(());
        }

        if let Ok(entries) = fs::read_dir(current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::collect_files_recursive(base, &path, out)?;
                } else if path.is_file() {
                    let rel = path.strip_prefix(base).map_err(|e| e.to_string())?;
                    let rel_str = rel.to_string_lossy().replace('\\', "/");
                    let data = fs::read(&path).map_err(|e| e.to_string())?;
                    out.push((rel_str, data));
                }
            }
        }
        Ok(())
    }
}

/// Inlines CSS, JS, and AetherBridge into a single HTML document
pub fn inline_web_assets(dist_dir: &Path, html: &str) -> Result<String, String> {
    let mut output = html.to_string();

    let bridge_content = if dist_dir.join("aether_bridge.js").exists() {
        fs::read_to_string(dist_dir.join("aether_bridge.js")).unwrap_or_else(|_| get_aether_bridge_js().to_string())
    } else {
        get_aether_bridge_js().to_string()
    };

    let bridge_tag = format!("<script>\n{}\n</script>", bridge_content);

    // Strip external references to aether_bridge.js to avoid 404s in inlined bundle
    output = output.replace("<script src=\"aether_bridge.js\"></script>", "");
    output = output.replace("<script src=\"./aether_bridge.js\"></script>", "");

    if let Some(head_end) = output.find("</head>") {
        output.insert_str(head_end, &format!("{}\n", bridge_tag));
    } else {
        output = format!("{}\n{}", bridge_tag, output);
    }

    Ok(output)
}

/// Official AetherBridge JavaScript SDK
pub fn get_aether_bridge_js() -> &'static str {
    r###"// AETHER Universal Bridge for React, Vue, Svelte, Tailwind & Modern Web
(function(window) {
    if (window.Aether) return;

    const Aether = {
        version: "2.0.0",
        platform: typeof window !== "undefined" && window.AndroidBridge ? "android" : "desktop",

        // Invoke native AETHER backend RPC
        invoke: async function(command, payload = {}) {
            if (window.AndroidBridge && window.AndroidBridge.postMessage) {
                return new Promise((resolve) => {
                    const id = Date.now() + Math.random();
                    window.AndroidBridge.postMessage(JSON.stringify({ id, command, payload }));
                    resolve({ status: "ok", command, payload });
                });
            } else {
                try {
                    const res = await fetch("/api/bridge", {
                        method: "POST",
                        headers: { "Content-Type": "application/json" },
                        body: JSON.stringify({ command, payload })
                    });
                    return await res.json();
                } catch(e) {
                    return { status: "simulated", command, payload };
                }
            }
        },

        // Native Android Haptic Vibration
        vibrate: function(ms = 35) {
            if (window.AndroidBridge && window.AndroidBridge.vibrate) {
                window.AndroidBridge.vibrate(ms);
            } else if (navigator && navigator.vibrate) {
                navigator.vibrate(ms);
            }
            console.log("[Aether Haptics] Vibrated " + ms + " ms");
        },

        // Haptic feedback profile preset
        hapticFeedback: function(type = "light") {
            const ms = type === "heavy" ? 80 : (type === "medium" ? 45 : 25);
            this.vibrate(ms);
        },

        // Native Android Toast
        showToast: function(message) {
            if (window.AndroidBridge && window.AndroidBridge.showToast) {
                window.AndroidBridge.showToast(message);
            } else {
                const toast = document.createElement("div");
                toast.textContent = message;
                toast.style.cssText = "position:fixed;bottom:24px;left:50%;transform:translateX(-50%);background:#1f2937;color:#fff;padding:10px 22px;border-radius:24px;box-shadow:0 6px 16px rgba(0,0,0,0.4);z-index:999999;font-family:system-ui,sans-serif;font-size:14px;border:1px solid #374151;transition:opacity 0.3s;";
                document.body.appendChild(toast);
                setTimeout(() => { toast.style.opacity = "0"; setTimeout(() => toast.remove(), 300); }, 2200);
            }
            console.log("[Aether Toast] " + message);
        },

        // Native Hardware & Battery specs
        deviceInfo: async function() {
            return await this.invoke("device_info", {});
        },

        // Native AI Prompt Completion
        ai: async function(prompt) {
            return await this.invoke("ai_complete", { prompt });
        },

        // Native Relational SQL Query
        dbQuery: async function(sql, params = []) {
            return await this.invoke("db_query", { sql, params });
        },

        // Geolocation & GPS coordinates
        getLocation: async function() {
            if (window.AndroidBridge && window.AndroidBridge.getLocation) {
                return JSON.parse(window.AndroidBridge.getLocation());
            }
            return new Promise((resolve) => {
                if (navigator.geolocation) {
                    navigator.geolocation.getCurrentPosition(
                        (pos) => resolve({
                            latitude: pos.coords.latitude,
                            longitude: pos.coords.longitude,
                            accuracy: pos.coords.accuracy,
                            altitude: pos.coords.altitude
                        }),
                        (err) => resolve({ latitude: 33.6844, longitude: 73.0479, simulated: true, error: err.message })
                    );
                } else {
                    resolve({ latitude: 33.6844, longitude: 73.0479, simulated: true });
                }
            });
        },

        // Native Camera & QR / Barcode Scanner
        scanQR: async function() {
            if (window.AndroidBridge && window.AndroidBridge.scanQR) {
                return await this.invoke("scan_qr", {});
            }
            return new Promise((resolve) => {
                const mockCode = "AETHER-PAY-" + Math.floor(100000 + Math.random() * 900000);
                this.showToast("📷 Scanner Active: " + mockCode);
                resolve({ text: mockCode, format: "QR_CODE" });
            });
        },

        // Native Background Alarm & Notification Scheduler
        scheduleAlarm: async function(timestamp, title = "AETHER Alarm", sound = "default") {
            return await this.invoke("schedule_alarm", { timestamp, title, sound });
        },

        // System Push / Local Notification
        showNotification: async function(title, body = "") {
            if (window.AndroidBridge && window.AndroidBridge.showNotification) {
                window.AndroidBridge.showNotification(title, body);
            } else if (typeof Notification !== "undefined") {
                if (Notification.permission === "granted") {
                    new Notification(title, { body });
                } else if (Notification.permission !== "denied") {
                    Notification.requestPermission().then(p => {
                        if (p === "granted") new Notification(title, { body });
                    });
                }
            }
            this.showToast("🔔 " + title + (body ? ": " + body : ""));
        },

        // Native File & Media Picker
        pickFile: function(accept = "*/*") {
            return new Promise((resolve) => {
                const input = document.createElement("input");
                input.type = "file";
                input.accept = accept;
                input.onchange = (e) => {
                    const file = e.target.files[0];
                    if (!file) return resolve(null);
                    const reader = new FileReader();
                    reader.onload = (evt) => resolve({
                        name: file.name,
                        size: file.size,
                        type: file.type,
                        data: evt.target.result
                    });
                    reader.readAsDataURL(file);
                };
                input.click();
            });
        },

        // System Dark Mode Detection
        isDarkMode: function() {
            return window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches;
        }
    };

    window.Aether = Aether;

    // AETHER Universal Live Hot-Reload Engine
    (function initLiveHotReload() {
        if (typeof window === "undefined") return;
        const devHost = "http://127.0.0.1:8080";
        let currentVersion = 0;
        let isConnected = false;

        async function checkServer() {
            try {
                const res = await fetch(`${devHost}/api/live-status`, { cache: "no-store", mode: "cors" });
                if (res.ok) {
                    const data = await res.json();
                    if (!isConnected) {
                        isConnected = true;
                        currentVersion = data.version || 1;
                        if (window.Aether && window.Aether.showToast) {
                            window.Aether.showToast("⚡ AETHER Live Dev Connected");
                        }
                        poll();
                    }
                }
            } catch(e) {
                setTimeout(checkServer, 3500);
            }
        }

        async function poll() {
            try {
                const res = await fetch(`${devHost}/api/live-poll?v=${currentVersion}`, { cache: "no-store", mode: "cors" });
                if (res.ok) {
                    const data = await res.json();
                    if (data.reload && data.version > currentVersion) {
                        currentVersion = data.version;
                        if (window.location.protocol === "file:" && isConnected) {
                            window.location.href = `${devHost}/index.html`;
                        } else {
                            window.location.reload();
                        }
                        return;
                    }
                }
            } catch(e) {
                isConnected = false;
                setTimeout(checkServer, 2000);
                return;
            }
            setTimeout(poll, 150);
        }

        setTimeout(checkServer, 200);
    })();
})(typeof window !== "undefined" ? window : globalThis);
"###
}
