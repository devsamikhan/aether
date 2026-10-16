// ==============================================================================
// AetherLive / AetherShell Live Hot-Reload Server (Pure Rust Zero-SDK)
// Sub-100ms Instant UI/UX Code Swapping for Mobile, Desktop & Web
// ==============================================================================

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime};

static LIVE_VERSION: AtomicU64 = AtomicU64::new(1);
static LAST_MODIFIED_FILE: OnceLock<Mutex<String>> = OnceLock::new();
static LAST_MODIFIED_TIME: AtomicU64 = AtomicU64::new(0);

fn get_last_modified_file() -> &'static Mutex<String> {
    LAST_MODIFIED_FILE.get_or_init(|| Mutex::new(String::from("initial")))
}

#[derive(Clone, Debug)]
pub struct LiveServerConfig {
    pub dist_dir: PathBuf,
    pub port: u16,
    pub poll_interval_ms: u64,
    pub enable_adb_reverse: bool,
    pub auto_open_browser: bool,
    pub desktop_app_mode: bool,
    pub window_width: u32,
    pub window_height: u32,
}

impl LiveServerConfig {
    pub fn new(dist_dir: impl Into<PathBuf>) -> Self {
        Self {
            dist_dir: dist_dir.into(),
            port: 8080,
            poll_interval_ms: 80,
            enable_adb_reverse: true,
            auto_open_browser: false,
            desktop_app_mode: false,
            window_width: 480,
            window_height: 850,
        }
    }
}

pub struct LiveDevServer {
    config: LiveServerConfig,
}

impl LiveDevServer {
    pub fn new(config: LiveServerConfig) -> Self {
        Self { config }
    }

    /// Starts the Live Hot-Reload Server daemon
    pub fn start(&self) -> Result<(), String> {
        let dist_dir = self.config.dist_dir.clone();
        if !dist_dir.exists() {
            return Err(format!("Frontend directory '{}' does not exist", dist_dir.display()));
        }

        let addr = format!("0.0.0.0:{}", self.config.port);
        let listener = TcpListener::bind(&addr)
            .map_err(|e| format!("Failed to bind Live Dev Server on {}: {}", addr, e))?;

        println!("================================================================================");
        println!("⚡ AETHERSHELL: REAL-TIME ZERO-SDK LIVE HOT-RELOAD SERVER");
        println!("================================================================================");
        println!("  📂 Root Assets:  {}", dist_dir.display());
        println!("  🌐 Local Host:   http://localhost:{}", self.config.port);
        println!("  📡 Network Host: http://{}:{}", get_local_ip().unwrap_or_else(|| "127.0.0.1".into()), self.config.port);
        println!("  ⏱️  Poll Cycle:  {} ms", self.config.poll_interval_ms);
        println!("  🔄 Hot-Reload:   Instant DOM & CSS Swapping (State Preserved)");

        // Setup ADB Reverse Port Forwarding for connected Android mobile phones
        if self.config.enable_adb_reverse {
            setup_adb_reverse(self.config.port);
        }

        // Open local browser or native desktop app container if requested
        if self.config.auto_open_browser || self.config.desktop_app_mode {
            let url = format!("http://localhost:{}", self.config.port);
            let is_desktop = self.config.desktop_app_mode;
            let width = self.config.window_width;
            let height = self.config.window_height;
            let title = self.config.dist_dir.file_name().and_then(|s| s.to_str()).unwrap_or("AetherApp").to_string();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(300));
                if is_desktop {
                    let _ = launch_desktop_window(&url, &title, width, height);
                } else if cfg!(windows) {
                    let _ = Command::new("cmd").args(["/C", "start", &url]).spawn();
                } else if cfg!(target_os = "macos") {
                    let _ = Command::new("open").arg(&url).spawn();
                } else {
                    let _ = Command::new("xdg-open").arg(&url).spawn();
                }
            });
        }

        // Spawn background file watcher thread
        let watcher_dir = dist_dir.clone();
        let poll_interval = self.config.poll_interval_ms;
        thread::spawn(move || {
            run_file_watcher(watcher_dir, poll_interval);
        });

        println!("--------------------------------------------------------------------------------");
        println!("✨ AetherLive is active and listening for connected clients.");
        println!("   Mobile phone and Desktop UI will update automatically on file save.");
        println!("   Press Ctrl+C to stop dev server.\n");

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let dir = dist_dir.clone();
                    let port = self.config.port;
                    thread::spawn(move || {
                        let _ = handle_http_request(stream, &dir, port);
                    });
                }
                Err(e) => eprintln!("[AetherLive ⚠️] Connection error: {}", e),
            }
        }

        Ok(())
    }
}

/// Automatically forwards device port to PC port over ADB USB
fn setup_adb_reverse(port: u16) {
    let adb_str = format!("tcp:{}", port);
    let output = Command::new("adb")
        .args(["reverse", &adb_str, &adb_str])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                println!("  📱 Mobile Link:  ADB Reverse Port {} -> Connected & Forwarded! 🟢", port);
            } else {
                println!("  📱 Mobile Link:  ADB available, waiting for phone connection (offline mode)");
            }
        }
        Err(_) => {
            println!("  📱 Mobile Link:  ADB not detected in PATH; Wi-Fi network host active");
        }
    }
}

/// Recursively scans and watches all files in target folder for modifications
fn run_file_watcher(dist_dir: PathBuf, poll_interval_ms: u64) {
    let mut file_timestamps: HashMap<String, SystemTime> = HashMap::new();

    // Initial populate
    collect_timestamps(&dist_dir, &dist_dir, &mut file_timestamps);

    let sleep_dur = Duration::from_millis(poll_interval_ms.max(40));

    loop {
        thread::sleep(sleep_dur);

        let mut current_timestamps: HashMap<String, SystemTime> = HashMap::new();
        collect_timestamps(&dist_dir, &dist_dir, &mut current_timestamps);

        let mut changed_file: Option<String> = None;

        for (rel_path, time) in &current_timestamps {
            if let Some(prev_time) = file_timestamps.get(rel_path) {
                if time > prev_time {
                    changed_file = Some(rel_path.clone());
                    break;
                }
            } else {
                changed_file = Some(rel_path.clone());
                break;
            }
        }

        if changed_file.is_none() {
            for rel_path in file_timestamps.keys() {
                if !current_timestamps.contains_key(rel_path) {
                    changed_file = Some(rel_path.clone());
                    break;
                }
            }
        }

        if let Some(file_name) = changed_file {
            let new_v = LIVE_VERSION.fetch_add(1, Ordering::SeqCst) + 1;
            let now_ms = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            LAST_MODIFIED_TIME.store(now_ms, Ordering::SeqCst);
            *get_last_modified_file().lock().unwrap() = file_name.clone();

            file_timestamps = current_timestamps;

            println!(
                "⚡ [AetherLive Hot-Reload] File change: '{}' (v{}, broadcast to clients)",
                file_name, new_v
            );
        }
    }
}

fn collect_timestamps(base: &Path, current: &Path, map: &mut HashMap<String, SystemTime>) {
    if let Ok(entries) = fs::read_dir(current) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_timestamps(base, &path, map);
            } else if path.is_file() {
                if let Ok(meta) = path.metadata() {
                    if let Ok(mtime) = meta.modified() {
                        let rel = path
                            .strip_prefix(base)
                            .map(|p| p.to_string_lossy().replace('\\', "/"))
                            .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"));
                        map.insert(rel, mtime);
                    }
                }
            }
        }
    }
}

/// Dispatches incoming HTTP requests
fn handle_http_request(mut stream: TcpStream, dist_dir: &Path, port: u16) -> std::io::Result<()> {
    let mut buffer = [0u8; 8192];
    let bytes_read = stream.read(&mut buffer)?;
    if bytes_read == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let mut lines = request.lines();
    let request_line = lines.next().unwrap_or("");
    let parts: Vec<&str> = request_line.split_whitespace().collect();

    if parts.len() < 2 {
        return send_http(&mut stream, 400, "text/plain", b"Bad Request", false);
    }

    let method = parts[0];
    let full_path = parts[1];
    let path = full_path.split('?').next().unwrap_or("/");

    // Handle CORS pre-flight OPTIONS
    if method == "OPTIONS" {
        let headers = "HTTP/1.1 204 No Content\r\n\
Access-Control-Allow-Origin: *\r\n\
Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
Access-Control-Allow-Headers: *\r\n\
Content-Length: 0\r\n\r\n";
        return stream.write_all(headers.as_bytes());
    }

    // 1. Live status check
    if path == "/api/live-status" {
        let ver = LIVE_VERSION.load(Ordering::SeqCst);
        let last_file = get_last_modified_file().lock().unwrap().clone();
        let last_time = LAST_MODIFIED_TIME.load(Ordering::SeqCst);
        let json = format!(
            "{{\"status\":\"online\",\"server\":\"AetherLive/2.0\",\"port\":{},\"version\":{},\"last_file\":\"{}\",\"timestamp\":{}}}",
            port, ver, last_file, last_time
        );
        return send_http(&mut stream, 200, "application/json", json.as_bytes(), true);
    }

    // 2. Live poll endpoint (v query param check)
    if path == "/api/live-poll" {
        let client_v: u64 = full_path
            .split('?')
            .nth(1)
            .and_then(|q| {
                q.split('&')
                    .find(|kv| kv.starts_with("v="))
                    .and_then(|kv| kv.split('=').nth(1))
                    .and_then(|v| v.parse().ok())
            })
            .unwrap_or(0);

        let current_v = LIVE_VERSION.load(Ordering::SeqCst);
        let last_file = get_last_modified_file().lock().unwrap().clone();
        let last_time = LAST_MODIFIED_TIME.load(Ordering::SeqCst);

        let reload = client_v > 0 && current_v > client_v;
        let json = format!(
            "{{\"reload\":{},\"version\":{},\"file\":\"{}\",\"timestamp\":{}}}",
            reload, current_v, last_file, last_time
        );
        return send_http(&mut stream, 200, "application/json", json.as_bytes(), true);
    }

    // 3. Serve Static Files from dist_dir
    let clean_path = path.trim_start_matches('/');
    let target_file = if clean_path.is_empty() || clean_path == "index.html" {
        dist_dir.join("index.html")
    } else {
        dist_dir.join(clean_path)
    };

    if target_file.exists() && target_file.is_file() {
        if let Ok(content) = fs::read(&target_file) {
            let mime = get_mime_type(&target_file);
            return send_http(&mut stream, 200, mime, &content, true);
        }
    }

    // Fallback 404
    send_http(&mut stream, 404, "text/plain", b"404 Not Found", true)
}

fn send_http(
    stream: &mut TcpStream,
    status_code: u16,
    content_type: &str,
    body: &[u8],
    no_cache: bool,
) -> std::io::Result<()> {
    let status_line = match status_code {
        200 => "200 OK",
        204 => "204 No Content",
        400 => "400 Bad Request",
        404 => "404 Not Found",
        _ => "500 Internal Server Error",
    };

    let cache_hdr = if no_cache {
        "Cache-Control: no-cache, no-store, must-revalidate\r\nPragma: no-cache\r\nExpires: 0\r\n"
    } else {
        ""
    };

    let headers = format!(
        "HTTP/1.1 {}\r\n\
Content-Type: {}\r\n\
Content-Length: {}\r\n\
Access-Control-Allow-Origin: *\r\n\
Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
Access-Control-Allow-Headers: *\r\n\
{}\
\r\n",
        status_line,
        content_type,
        body.len(),
        cache_hdr
    );

    stream.write_all(headers.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

fn get_mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    Some(socket.local_addr().ok()?.ip().to_string())
}

/// Spawns a dedicated standalone application window (Edge/Chrome App Mode, zero Electron / zero Tauri)
pub fn launch_desktop_window(
    url: &str,
    _title: &str,
    width: u32,
    height: u32,
) -> Result<(), String> {
    if cfg!(windows) {
        let edge_candidates = [
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        ];
        for path in &edge_candidates {
            if Path::new(path).exists() {
                let temp_profile = std::env::temp_dir().join("aether_desktop_edge_profile");
                let status = Command::new(path)
                    .args([
                        &format!("--app={}", url),
                        &format!("--window-size={},{}", width, height),
                        &format!("--user-data-dir={}", temp_profile.display()),
                        "--no-first-run",
                        "--no-default-browser-check",
                    ])
                    .spawn();
                if status.is_ok() {
                    return Ok(());
                }
            }
        }

        let chrome_candidates = [
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ];
        for path in &chrome_candidates {
            if Path::new(path).exists() {
                let temp_profile = std::env::temp_dir().join("aether_desktop_chrome_profile");
                let status = Command::new(path)
                    .args([
                        &format!("--app={}", url),
                        &format!("--window-size={},{}", width, height),
                        &format!("--user-data-dir={}", temp_profile.display()),
                        "--no-first-run",
                        "--no-default-browser-check",
                    ])
                    .spawn();
                if status.is_ok() {
                    return Ok(());
                }
            }
        }

        // Fallback to system browser
        Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()
            .map_err(|e| format!("Failed to open system browser: {}", e))?;
        Ok(())
    } else if cfg!(target_os = "macos") {
        let status = Command::new("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
            .args([&format!("--app={}", url), &format!("--window-size={},{}", width, height)])
            .spawn();
        if status.is_ok() {
            return Ok(());
        }
        Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to open macOS browser: {}", e))?;
        Ok(())
    } else {
        for browser in &["google-chrome", "chromium-browser", "chromium", "microsoft-edge"] {
            let status = Command::new(browser)
                .args([&format!("--app={}", url), &format!("--window-size={},{}", width, height)])
                .spawn();
            if status.is_ok() {
                return Ok(());
            }
        }
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to open Linux browser: {}", e))?;
        Ok(())
    }
}
