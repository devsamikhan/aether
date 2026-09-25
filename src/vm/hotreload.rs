// ==============================================================================
// AetherLive / AetherHotReload — Zero-Downtime Live Code Swapping Engine
// Erlang BEAM-Grade In-Place Bytecode Swapping & Hot State Preservation
// Pure Rust Standard Library (Zero Third-Party Crates)
// ==============================================================================

use super::compiler::BytecodeCompiler;
use super::interpreter::{with_current_vm, VM};
use super::value::{ClassDef, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

// ==============================================================================
// 1. Hot Reload Configuration & Reports
// ==============================================================================

#[derive(Clone, Debug)]
pub struct ReloadConfig {
    pub preserve_globals: bool,
    pub update_classes_in_place: bool,
    pub call_on_reload: bool,
}

impl Default for ReloadConfig {
    fn default() -> Self {
        Self {
            preserve_globals: true,
            update_classes_in_place: true,
            call_on_reload: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ReloadReport {
    pub version: u64,
    pub functions_updated: Vec<String>,
    pub functions_added: Vec<String>,
    pub classes_updated: Vec<String>,
    pub globals_preserved: Vec<String>,
    pub duration_us: u128,
    pub success: bool,
    pub error: Option<String>,
}

impl ReloadReport {
    pub fn to_value(&self) -> Value {
        let mut map = HashMap::new();
        map.insert("version".to_string(), Value::Int(self.version as i64));
        map.insert(
            "functions_updated".to_string(),
            Value::array(self.functions_updated.iter().map(Value::string).collect()),
        );
        map.insert(
            "functions_added".to_string(),
            Value::array(self.functions_added.iter().map(Value::string).collect()),
        );
        map.insert(
            "classes_updated".to_string(),
            Value::array(self.classes_updated.iter().map(Value::string).collect()),
        );
        map.insert(
            "globals_preserved".to_string(),
            Value::array(self.globals_preserved.iter().map(Value::string).collect()),
        );
        map.insert("duration_us".to_string(), Value::Int(self.duration_us as i64));
        map.insert("success".to_string(), Value::Bool(self.success));
        if let Some(err) = &self.error {
            map.insert("error".to_string(), Value::string(err.clone()));
        } else {
            map.insert("error".to_string(), Value::Nil);
        }
        Value::map(map)
    }
}

// ==============================================================================
// 2. Global Reload History & Telemetry
// ==============================================================================

static RELOAD_VERSION: AtomicU64 = AtomicU64::new(1);
static RELOAD_HISTORY: OnceLock<Mutex<Vec<ReloadReport>>> = OnceLock::new();

fn get_history() -> &'static Mutex<Vec<ReloadReport>> {
    RELOAD_HISTORY.get_or_init(|| Mutex::new(Vec::new()))
}

// ==============================================================================
// 3. Core Hot-Reload Engine
// ==============================================================================

/// Hot-reloads new source code into the provided active VM instance.
/// Atomic validation: parses and compiles first. If invalid, the VM state is 100% untouched.
pub fn hot_reload(
    vm: &mut VM,
    new_source: &str,
    config_opt: Option<ReloadConfig>,
) -> Result<ReloadReport, String> {
    let start_time = Instant::now();
    let config = config_opt.unwrap_or_default();

    // Step 1: Atomic AST Parse Validation
    let program = match crate::syntax::parse(new_source) {
        Ok(p) => p,
        Err((e, span)) => {
            let err_msg = format!("{}:{}: Syntax Error: {}", span.line, span.col, e);
            let report = ReloadReport {
                version: RELOAD_VERSION.load(Ordering::SeqCst),
                functions_updated: Vec::new(),
                functions_added: Vec::new(),
                classes_updated: Vec::new(),
                globals_preserved: Vec::new(),
                duration_us: start_time.elapsed().as_micros(),
                success: false,
                error: Some(err_msg.clone()),
            };
            get_history().lock().unwrap().push(report);
            return Err(err_msg);
        }
    };

    // Step 2: Atomic Bytecode Compilation Validation
    let compiler = BytecodeCompiler::new("<hotreload>", 0);
    let compiled_fn = match compiler.compile(&program) {
        Ok(f) => f,
        Err(e) => {
            let err_msg = format!("Compilation Error: {}", e);
            let report = ReloadReport {
                version: RELOAD_VERSION.load(Ordering::SeqCst),
                functions_updated: Vec::new(),
                functions_added: Vec::new(),
                classes_updated: Vec::new(),
                globals_preserved: Vec::new(),
                duration_us: start_time.elapsed().as_micros(),
                success: false,
                error: Some(err_msg.clone()),
            };
            get_history().lock().unwrap().push(report);
            return Err(err_msg);
        }
    };

    // Step 3: Snapshot Live User State & Existing Code Symbols
    let mut saved_state: HashMap<String, Value> = HashMap::new();
    let mut old_functions: HashSet<String> = HashSet::new();
    let mut old_classes: HashMap<String, Arc<ClassDef>> = HashMap::new();

    for (k, v) in vm.globals.iter() {
        match v {
            Value::Function(_) | Value::Closure { .. } => {
                old_functions.insert(k.clone());
            }
            Value::ClassDef(c) => {
                old_classes.insert(k.clone(), c.clone());
            }
            Value::Native(..) => {
                // Builtin native module or function, do not snapshot as user data
            }
            _ => {
                // Live runtime user variable (state, caches, connections, actors, counters)
                if config.preserve_globals {
                    saved_state.insert(k.clone(), v.clone());
                }
            }
        }
    }

    // Step 4: Execute Compiled Definitions in an Isolated Sub-VM
    let mut sub_vm = VM::with_globals(vm.globals.clone());
    sub_vm.modules_cache = vm.modules_cache.clone();
    if let Err(e) = sub_vm.interpret(compiled_fn) {
        let err_msg = format!("Runtime Execution Error during Hot-Swap: {}", e);
        let report = ReloadReport {
            version: RELOAD_VERSION.load(Ordering::SeqCst),
            functions_updated: Vec::new(),
            functions_added: Vec::new(),
            classes_updated: Vec::new(),
            globals_preserved: Vec::new(),
            duration_us: start_time.elapsed().as_micros(),
            success: false,
            error: Some(err_msg.clone()),
        };
        get_history().lock().unwrap().push(report);
        return Err(err_msg);
    }

    // Step 5: Merge Updated Symbols into Target VM
    let mut classes_updated = Vec::new();
    let mut functions_updated = Vec::new();
    let mut functions_added = Vec::new();

    for (k, v) in sub_vm.globals.into_iter() {
        match v {
            Value::Function(_) | Value::Closure { .. } => {
                if old_functions.contains(&k) {
                    functions_updated.push(k.clone());
                } else {
                    functions_added.push(k.clone());
                }
                vm.globals.insert(k, v);
            }
            Value::ClassDef(new_class_arc) => {
                if config.update_classes_in_place && old_classes.contains_key(&k) {
                    let old_class_arc = old_classes.get(&k).unwrap();
                    let new_methods = new_class_arc.methods.lock().clone();
                    let mut old_methods = old_class_arc.methods.lock();
                    for (m_name, m_fn) in new_methods {
                        old_methods.insert(m_name, m_fn);
                    }
                    vm.globals.insert(k.clone(), Value::ClassDef(old_class_arc.clone()));
                    classes_updated.push(k);
                } else {
                    vm.globals.insert(k, Value::ClassDef(new_class_arc));
                }
            }
            Value::Native(..) => {
                // Builtin native module or function
            }
            other => {
                // Top-level variable: only insert if it wasn't already in saved_state
                if config.preserve_globals && saved_state.contains_key(&k) {
                    // Do nothing, will be restored in Step 6
                } else {
                    vm.globals.insert(k, other);
                }
            }
        }
    }

    // Step 6: Restore Live State Variables
    let mut globals_preserved = Vec::new();
    if config.preserve_globals {
        for (k, v) in saved_state {
            vm.globals.insert(k.clone(), v);
            globals_preserved.push(k);
        }
    }

    functions_updated.sort();
    functions_added.sort();
    classes_updated.sort();
    globals_preserved.sort();

    // Step 7: Invoke Lifecycle Hook if defined
    if config.call_on_reload && vm.globals.contains_key("on_reload") {
        let _ = vm.call_global("on_reload", Vec::new());
    }

    let version = RELOAD_VERSION.fetch_add(1, Ordering::SeqCst);
    let report = ReloadReport {
        version,
        functions_updated,
        functions_added,
        classes_updated,
        globals_preserved,
        duration_us: start_time.elapsed().as_micros(),
        success: true,
        error: None,
    };

    get_history().lock().unwrap().push(report.clone());
    Ok(report)
}

/// Hot-reloads a source file from disk into the active VM.
pub fn hot_reload_file(
    vm: &mut VM,
    file_path: &str,
    config: Option<ReloadConfig>,
) -> Result<ReloadReport, String> {
    let source = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read source file '{}': {}", file_path, e))?;
    hot_reload(vm, &source, config)
}

// ==============================================================================
// 4. CLI Live Watcher Daemon
// ==============================================================================

/// Starts a zero-downtime file watcher process that auto-swaps bytecode when changed.
pub fn run_live_watcher(file_path: &str, poll_interval_ms: u64) -> Result<(), String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("File '{}' not found", file_path));
    }

    let mut last_modified = fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let initial_source = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read '{}': {}", file_path, e))?;

    let mut vm = VM::new();

    println!("================================================================================");
    println!("⚡ AETHERLIVE: ZERO-DOWNTIME LIVE CODE SWAPPING ENGINE (BEAM GRADE)");
    println!("Target File: {}", file_path);
    println!("Polling Interval: {}ms", poll_interval_ms);
    println!("Live State Preservation: ENABLED");
    println!("Status: Running initial script version...");
    println!("================================================================================\n");

    let program = crate::syntax::parse(&initial_source)
        .map_err(|(e, span)| format!("{}:{}: {}", span.line, span.col, e))?;
    let compiler = BytecodeCompiler::new("<main>", 0);
    let compiled_fn = compiler.compile(&program)?;

    if let Err(e) = vm.interpret(compiled_fn) {
        eprintln!("[AetherLive ⚠️] Initial execution warning: {}", e);
    }

    println!("\n[AetherLive 🟢] Initial code active. File watcher online.");
    println!("[AetherLive 💡] Edit and save '{}' to trigger instant hot code swapping.", file_path);
    println!("[AetherLive 💡] Press Ctrl+C to terminate.\n");

    let poll_dur = Duration::from_millis(poll_interval_ms.max(50));

    loop {
        std::thread::sleep(poll_dur);

        if let Ok(metadata) = fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                if modified > last_modified {
                    last_modified = modified;

                    // Allow brief flush window for file write completion
                    std::thread::sleep(Duration::from_millis(20));

                    match fs::read_to_string(path) {
                        Ok(new_source) => {
                            println!("[AetherLive ⚡] File change detected! Initiating hot-swap...");
                            match hot_reload(&mut vm, &new_source, None) {
                                Ok(report) => {
                                    println!(
                                        "[AetherLive ✅] Hot-Swap Successful (v{}, took {} µs)",
                                        report.version, report.duration_us
                                    );
                                    if !report.functions_updated.is_empty() {
                                        println!(
                                            "  • Functions Upgraded: {:?}",
                                            report.functions_updated
                                        );
                                    }
                                    if !report.functions_added.is_empty() {
                                        println!(
                                            "  • Functions Added: {:?}",
                                            report.functions_added
                                        );
                                    }
                                    if !report.classes_updated.is_empty() {
                                        println!(
                                            "  • Classes Patched In-Place: {:?}",
                                            report.classes_updated
                                        );
                                    }
                                    println!(
                                        "  • Live State Preserved: {} variables",
                                        report.globals_preserved.len()
                                    );
                                }
                                Err(err) => {
                                    eprintln!("[AetherLive ❌] Hot-Swap Aborted (Active State Preserved): {}", err);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[AetherLive ⚠️] Read error: {}", e);
                        }
                    }
                }
            }
        }
    }
}

// ==============================================================================
// 5. Standard Native Module Registration
// ==============================================================================

pub fn register_hotreload_module(globals: &mut HashMap<String, Value>) {
    let mut mod_map = HashMap::new();

    // 1. HotReload.reload(source_string, [preserve_state]) -> Map (Report)
    mod_map.insert(
        "reload".to_string(),
        Value::Native("HotReload.reload".into(), |args| {
            if args.is_empty() {
                return Err("HotReload.reload(source_string, [preserve_state]) requires source string".into());
            }
            let source = args[0].to_string();
            let preserve = if args.len() > 1 { args[1].is_truthy() } else { true };

            let config = ReloadConfig {
                preserve_globals: preserve,
                update_classes_in_place: true,
                call_on_reload: true,
            };

            let report = with_current_vm(|vm| hot_reload(vm, &source, Some(config)))?;
            Ok(report.to_value())
        }),
    );

    // 2. HotReload.reload_file(file_path, [preserve_state]) -> Map (Report)
    mod_map.insert(
        "reload_file".to_string(),
        Value::Native("HotReload.reload_file".into(), |args| {
            if args.is_empty() {
                return Err("HotReload.reload_file(file_path, [preserve_state]) requires file path".into());
            }
            let path = args[0].to_string();
            let preserve = if args.len() > 1 { args[1].is_truthy() } else { true };

            let config = ReloadConfig {
                preserve_globals: preserve,
                update_classes_in_place: true,
                call_on_reload: true,
            };

            let report = with_current_vm(|vm| hot_reload_file(vm, &path, Some(config)))?;
            Ok(report.to_value())
        }),
    );

    // 3. HotReload.version() -> Int
    mod_map.insert(
        "version".to_string(),
        Value::Native("HotReload.version".into(), |_| {
            let ver = RELOAD_VERSION.load(Ordering::SeqCst);
            Ok(Value::Int(ver as i64))
        }),
    );

    // 4. HotReload.history() -> Array of Reports
    mod_map.insert(
        "history".to_string(),
        Value::Native("HotReload.history".into(), |_| {
            let history = get_history().lock().unwrap();
            let arr = history.iter().map(|r| r.to_value()).collect();
            Ok(Value::array(arr))
        }),
    );

    let val = Value::map(mod_map);
    globals.insert("__native_hotreload".to_string(), val.clone());
    globals.insert("hotreload".to_string(), val.clone());
    globals.insert("aether_live".to_string(), val.clone());
    globals.insert("HotReload".to_string(), val);
}
