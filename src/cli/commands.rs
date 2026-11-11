use super::diagnostics::format_error_with_source;
use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub fn run_script(file: Option<&str>, jit: bool) -> Result<(), String> {
    let target = match file {
        Some(f) => PathBuf::from(f),
        None => {
            let main_ae = PathBuf::from("src/main.ae");
            if main_ae.is_file() {
                main_ae
            } else {
                return Err(
                    "No script specified and `src/main.ae` was not found in the current directory."
                        .to_string(),
                );
            }
        }
    };

    let src = std::fs::read_to_string(&target)
        .map_err(|e| format!("Failed to read '{}': {}", target.display(), e))?;

    if jit {
        let prog = aether::syntax::parse(&src).map_err(|(e, span)| {
            aether::syntax::diagnostics::render_diagnostic_snippet(
                &src, span.line, span.col, &e, None,
            )
        })?;
        let mut compiler = aether::codegen::cranelift_backend::CraneliftCompiler::new()?;
        return compiler.compile_and_run(&prog).map(|val| {
            println!("JIT Result: {}", val);
        });
    }

    aether::vm::run_source(&src)
        .map(|_| ())
        .map_err(|e| format_error_with_source(&src, &e))
}

pub fn run_jit(file: &str) -> Result<(), String> {
    run_script(Some(file), true)
}

pub fn run_repl() -> Result<(), String> {
    println!("AETHER Interactive REPL");
    println!("Commands: :help, :vars, :type <expr>, :disasm <code>, :lint <code>, :clear, exit\n");
    let mut vm = aether::vm::VM::new();
    let builtin_keys: HashSet<String> = vm.globals.keys().cloned().collect();
    let stdin = io::stdin();
    let mut buffer = String::new();
    let mut brace_depth: i32 = 0;

    loop {
        let prompt = if buffer.is_empty() {
            "aether> "
        } else {
            "   ...> "
        };
        print!("{}", prompt);
        let _ = io::stdout().flush();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let trimmed = line.trim();
        if buffer.is_empty() {
            if matches!(trimmed, "exit" | "quit" | ":quit" | ":q") {
                break;
            }
            if trimmed.is_empty() {
                continue;
            }
            if handle_repl_meta(trimmed, &mut vm, &builtin_keys) {
                continue;
            }
        }

        for ch in line.chars() {
            match ch {
                '{' | '(' | '[' => brace_depth += 1,
                '}' | ')' | ']' => brace_depth = (brace_depth - 1).max(0),
                _ => {}
            }
        }
        buffer.push_str(&line);

        let needs_more = brace_depth > 0
            || trimmed.ends_with(':')
            || (!buffer.trim().is_empty() && line.starts_with("  ") && !trimmed.is_empty());
        if needs_more {
            continue;
        }

        let src_to_run = std::mem::take(&mut buffer);
        brace_depth = 0;

        match aether::syntax::parse(&src_to_run) {
            Ok(raw_prog) => {
                let (prog, _) = aether::vm::optimizer::optimize_program(&raw_prog);
                let compiler = aether::vm::BytecodeCompiler::new("<repl>", 0);
                match compiler.compile(&prog) {
                    Ok(compiled_fn) => match vm.interpret(compiled_fn) {
                        Ok(val) => {
                            if val != aether::vm::Value::Nil {
                                println!("=> {}", val);
                            }
                        }
                        Err(e) => eprintln!("{}", format_error_with_source(&src_to_run, &e)),
                    },
                    Err(e) => eprintln!("Compiler Error: {}", e),
                }
            }
            Err((e, span)) => eprintln!(
                "{}",
                aether::syntax::diagnostics::render_diagnostic_snippet(
                    &src_to_run,
                    span.line,
                    span.col,
                    &e,
                    None
                )
            ),
        }
    }
    Ok(())
}

fn handle_repl_meta(
    trimmed: &str,
    vm: &mut aether::vm::VM,
    builtin_keys: &HashSet<String>,
) -> bool {
    match trimmed {
        ":help" => {
            println!("REPL Meta-Commands:");
            println!("  :vars           Show user-defined global variables in session");
            println!("  :type <expr>    Evaluate expression and display its runtime type");
            println!("  :disasm <code>  Compile and disassemble AETHER expression/statement");
            println!("  :lint <code>    Run static linter on AETHER snippet");
            println!("  :clear          Reset VM state");
            println!("  exit / :q       Exit the REPL");
            true
        }
        ":clear" => {
            *vm = aether::vm::VM::new();
            println!("REPL state cleared.");
            true
        }
        ":vars" => {
            let mut count = 0;
            for (k, v) in &vm.globals {
                if !builtin_keys.contains(k) {
                    println!("  {} : {} = {}", k, v.type_name(), v);
                    count += 1;
                }
            }
            if count == 0 {
                println!("  (no user-defined globals)");
            }
            true
        }
        _ if trimmed.starts_with(":type ") => {
            let expr_src = &trimmed[6..];
            if let Ok(prog) = aether::syntax::parse(expr_src) {
                let compiler = aether::vm::BytecodeCompiler::new("<repl>", 0);
                if let Ok(compiled_fn) = compiler.compile(&prog) {
                    match vm.interpret(compiled_fn) {
                        Ok(val) => println!("Type: {} (Value: {})", val.type_name(), val),
                        Err(e) => eprintln!("{}", format_error_with_source(expr_src, &e)),
                    }
                }
            }
            true
        }
        _ if trimmed.starts_with(":disasm ") => {
            match aether::vm::bytecode::disassemble_source(&trimmed[8..]) {
                Ok(dis) => print!("{}", dis),
                Err(e) => eprintln!("Disassembly Error: {}", e),
            }
            true
        }
        _ if trimmed.starts_with(":lint ") => {
            match aether::syntax::diagnostics::lint_source(&trimmed[6..]) {
                Ok(warnings) => {
                    if warnings.is_empty() {
                        println!("No lint warnings.");
                    } else {
                        for w in warnings {
                            println!("  [{}] Line {}: {}", w.code, w.line, w.message);
                        }
                    }
                }
                Err(e) => eprintln!("Lint Error: {}", e),
            }
            true
        }
        _ => false,
    }
}

pub fn run_replay(file: &str) -> Result<(), String> {
    let source = std::fs::read_to_string(file)
        .map_err(|e| format!("Failed to read script '{}': {}", file, e))?;
    let session_id = aether::vm::timetravel::create_session();
    let session_arc = aether::vm::timetravel::get_session(session_id)?;
    aether::vm::run_source(&source).map_err(|e| format_error_with_source(&source, &e))?;
    println!("Script executed. Launching Interactive TimeTravel TUI Debugger...");
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut stdout = io::stdout();
    let _ = aether::vm::timetravel_tui::run_interactive_tui(session_arc, &mut reader, &mut stdout);
    Ok(())
}

pub fn run_verify(file: &str) -> Result<(), String> {
    let source = std::fs::read_to_string(file)
        .map_err(|e| format!("Failed to read file '{}': {}", file, e))?;
    println!("=== AETHER Formal Verification & Symbolic Prover ===");
    println!("Analyzing: {}\n", file);

    let program = aether::syntax::parse(&source).map_err(|(e, span)| {
        aether::syntax::diagnostics::render_diagnostic_snippet(
            &source, span.line, span.col, &e, None,
        )
    })?;
    let mut verifier = aether::vm::proof::SymbolicVerifier::new();
    let reports = verifier.verify_program(&program);

    let mut all_pass = true;
    for r in reports {
        if r.is_verified {
            println!(
                "  [PROVED] Intent Function '{}' verified across {} paths.",
                r.function_name, r.paths_explored
            );
        } else {
            all_pass = false;
            println!(
                "  [FAILED] Function '{}' verification failed!",
                r.function_name
            );
            println!("    Diagnostic: {}", r.diagnostic);
            if let Some(ce) = &r.counter_example {
                println!("    Counter-Example Input: {:?}", ce);
            }
        }
        for v in &r.vulnerabilities {
            println!("    Vulnerability: {}", v);
        }
    }

    if all_pass {
        println!("\nVerification Result: ALL CONTRACTS PROVED SOUND");
        Ok(())
    } else {
        Err("Formal contract verification failed".to_string())
    }
}

pub fn run_check(file: &str) -> Result<(), String> {
    let src =
        std::fs::read_to_string(file).map_err(|e| format!("Failed to read '{}': {}", file, e))?;
    println!(
        "=== AETHER Static Type, Lint & Security Audit: {} ===",
        file
    );

    let prog = aether::syntax::parse(&src).map_err(|(e, span)| {
        aether::syntax::diagnostics::render_diagnostic_snippet(&src, span.line, span.col, &e, None)
    })?;

    let mut tc = aether::types::checker::TypeChecker::new();
    let type_errors: Vec<String> = match tc.check_program(&prog) {
        Ok(()) => Vec::new(),
        Err(errs) => errs,
    };
    let lint_warnings = aether::syntax::diagnostics::lint_source(&src).unwrap_or_default();
    let sec_findings = aether::vm::security::scan_source_security(&src).unwrap_or_default();

    for te in &type_errors {
        println!("  [TypeError] {}", te);
    }
    for w in &lint_warnings {
        println!(
            "{}",
            aether::syntax::diagnostics::render_diagnostic_snippet(
                &src,
                w.line,
                1,
                &format!("[Lint:{}] {}", w.code, w.message),
                None
            )
        );
    }
    for sf in &sec_findings {
        println!(
            "{}",
            aether::syntax::diagnostics::render_diagnostic_snippet(
                &src,
                sf.line,
                1,
                &format!("[Security:{}:{}] {}", sf.severity, sf.category, sf.message),
                None
            )
        );
    }
    println!(
        "Summary: {} type errors, {} lint warnings, {} security findings",
        type_errors.len(),
        lint_warnings.len(),
        sec_findings.len()
    );

    if type_errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} type error(s) detected in '{}'",
            type_errors.len(),
            file
        ))
    }
}

pub fn run_lint(file: &str) -> Result<(), String> {
    let src =
        std::fs::read_to_string(file).map_err(|e| format!("Failed to read '{}': {}", file, e))?;
    let warnings = aether::syntax::diagnostics::lint_source(&src)?;
    if warnings.is_empty() {
        println!("No lint warnings in {}", file);
    } else {
        println!(
            "=== AETHER Linter: {} ({} warnings) ===",
            file,
            warnings.len()
        );
        for w in warnings {
            let msg = format!("[{}] {}", w.code, w.message);
            println!(
                "{}",
                aether::syntax::diagnostics::render_diagnostic_snippet(&src, w.line, 1, &msg, None)
            );
        }
    }
    Ok(())
}

pub fn run_fmt(file: &str, write_in_place: bool) -> Result<(), String> {
    let src =
        std::fs::read_to_string(file).map_err(|e| format!("Failed to read '{}': {}", file, e))?;
    let formatted = aether::syntax::diagnostics::format_source(&src);
    if write_in_place {
        std::fs::write(file, &formatted)
            .map_err(|e| format!("Failed to write '{}': {}", file, e))?;
        println!("Formatted {}", file);
    } else {
        print!("{}", formatted);
    }
    Ok(())
}

pub fn run_disasm(file: &str) -> Result<(), String> {
    let src =
        std::fs::read_to_string(file).map_err(|e| format!("Failed to read '{}': {}", file, e))?;
    let dis = aether::vm::bytecode::disassemble_source(&src)?;
    print!("{}", dis);
    Ok(())
}

pub fn run_build(
    source: Option<&str>,
    out: Option<&str>,
    target: Option<&str>,
    name: Option<&str>,
    package: Option<&str>,
    release: bool,
) -> Result<(), String> {
    let src_path = match source {
        Some(s) => PathBuf::from(s),
        None => {
            let default_main = PathBuf::from("src/main.ae");
            if default_main.is_file() {
                default_main
            } else {
                return Err("Usage: aether build <script.ae> [-o <out>] [--target web|apk|android-project|exe] [--release]".to_string());
            }
        }
    };
    let output_path = out.map(PathBuf::from);
    let target_lower = target.map(str::to_lowercase);

    let is_web = target_lower.as_deref() == Some("web")
        || target_lower.as_deref() == Some("html")
        || output_path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|s| s.to_str())
            == Some("html");

    if is_web {
        let app = name.map(str::to_string).unwrap_or_else(|| {
            src_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("App")
                .to_string()
        });
        let out_html = output_path.unwrap_or_else(|| src_path.with_extension("html"));
        let builder = aether::codegen::web_builder::WebBuilder::new(&app);
        return builder.build_web_app(&src_path, &out_html);
    }

    let is_android_project = target_lower.as_deref() == Some("android-project");
    let is_apk = target_lower.as_deref() == Some("apk")
        || target_lower.as_deref() == Some("android")
        || output_path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|s| s.to_str())
            == Some("apk");

    if is_apk || is_android_project {
        let mut config = aether::codegen::apk_builder::ApkConfig::from_source_file(&src_path);
        if let Some(pkg) = package {
            config.package_name = pkg.to_string();
        }
        if let Some(app_name) = name {
            config.app_name = app_name.to_string();
        }
        let builder = aether::codegen::apk_builder::ApkBuilder::new(config);
        return if is_android_project {
            let out_dir = output_path.unwrap_or_else(|| src_path.with_extension("android"));
            builder.export_android_project(&src_path, &out_dir)
        } else {
            let out_apk = output_path.unwrap_or_else(|| src_path.with_extension("apk"));
            builder.build_apk(&src_path, &out_apk)
        };
    }

    let out_exe = output_path.unwrap_or_else(|| {
        if cfg!(windows) {
            src_path.with_extension("exe")
        } else {
            src_path.with_extension("out")
        }
    });
    let builder = aether::codegen::aot_builder::AotBuilder::new(release);
    builder.build_executable(&src_path, &out_exe)
}

pub fn run_compile(script: &str, out: Option<&str>) -> Result<(), String> {
    let script_path = Path::new(script);
    let stem = script_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = if cfg!(windows) { "obj" } else { "o" };
    let out_path = out
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("{}.{}", stem, ext)));

    println!("=== AETHER Ahead-of-Time (AOT) Native Compiler ===");
    println!("Input Source:  {}", script_path.display());
    println!("Target Object: {}", out_path.display());
    aether::codegen::aot::compile_file_to_object(script_path, &out_path)?;
    println!(
        "Successfully compiled native object file: {}",
        out_path.display()
    );
    Ok(())
}

pub fn run_wasm(file: &str, out: Option<&str>) -> Result<(), String> {
    let out_path = out
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}.wasm", file.trim_end_matches(".ae")));
    let src = std::fs::read_to_string(file)
        .map_err(|e| format!("Failed to read file '{}': {}", file, e))?;
    let prog = aether::syntax::parse(&src).map_err(|(e, span)| {
        aether::syntax::diagnostics::render_diagnostic_snippet(&src, span.line, span.col, &e, None)
    })?;
    let mut compiler = aether::codegen::wasm::WasmCompiler::new();
    let wasm_bytes = compiler.compile_program(&prog)?;
    std::fs::write(&out_path, &wasm_bytes)
        .map_err(|e| format!("Failed to write WASM binary '{}': {}", out_path, e))?;
    println!(
        "[AetherWasm] Compiled {} -> {} ({} bytes)",
        file,
        out_path,
        wasm_bytes.len()
    );
    Ok(())
}

pub fn run_edge(wasm_file: &str, func_name: &str, args: &[String]) -> Result<(), String> {
    let bytes =
        std::fs::read(wasm_file).map_err(|e| format!("Failed to read '{}': {}", wasm_file, e))?;
    let module = aether::vm::wasm_runtime::WasmModule::parse(&bytes)?;
    let mut instance = aether::vm::wasm_runtime::WasmInstance::new(module);
    let call_args: Vec<i64> = args.iter().map(|s| s.parse::<i64>().unwrap_or(0)).collect();
    let result = instance.invoke(func_name, &call_args, 1_000_000)?;
    println!("{}", result);
    Ok(())
}

pub fn run_web(source: &str, out: Option<&str>, name: Option<&str>) -> Result<(), String> {
    let source_path = Path::new(source);
    let out_path = out
        .map(PathBuf::from)
        .unwrap_or_else(|| source_path.with_extension("html"));
    let app = name.map(str::to_string).unwrap_or_else(|| {
        source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("App")
            .to_string()
    });
    aether::codegen::web_builder::WebBuilder::new(&app).build_web_app(source_path, &out_path)
}

pub fn run_apk(
    source: &str,
    out: Option<&str>,
    package: Option<&str>,
    name: Option<&str>,
) -> Result<(), String> {
    let source_path = Path::new(source);
    let out_path = out
        .map(PathBuf::from)
        .unwrap_or_else(|| source_path.with_extension("apk"));
    let mut config = aether::codegen::apk_builder::ApkConfig::from_source_file(source_path);
    if let Some(pkg) = package {
        config.package_name = pkg.to_string();
    }
    if let Some(app_name) = name {
        config.app_name = app_name.to_string();
    }
    aether::codegen::apk_builder::ApkBuilder::new(config).build_apk(source_path, &out_path)
}

pub fn run_bundle(
    dist_dir: &str,
    out: Option<&str>,
    target: &str,
    name: Option<&str>,
    package: Option<&str>,
) -> Result<(), String> {
    let dist_path = PathBuf::from(dist_dir);
    let target_lower = target.to_lowercase();
    let final_ext = match target_lower.as_str() {
        "desktop" | "exe" | "windows" => "exe",
        "web" | "html" => "html",
        _ => "apk",
    };
    let final_output = out
        .map(PathBuf::from)
        .unwrap_or_else(|| dist_path.with_extension(final_ext));

    let mut config = aether::codegen::shell_bundler::ShellConfig::new(dist_path, final_output);
    config.target = target_lower;
    if let Some(app_name) = name {
        config.app_name = app_name.to_string();
    }
    if let Some(pkg) = package {
        config.package_name = pkg.to_string();
    }
    aether::codegen::shell_bundler::ShellBundler::new(config).bundle()
}

pub fn run_deploy(
    input: &str,
    device: Option<&str>,
    port: u16,
    package: Option<&str>,
    no_launch: bool,
) -> Result<(), String> {
    let input_path = PathBuf::from(input);
    let target_apk = if input_path.extension().and_then(|s| s.to_str()) == Some("apk") {
        input_path
    } else {
        let apk_out = input_path.join("app.apk");
        let mut config =
            aether::codegen::shell_bundler::ShellConfig::new(input_path, apk_out.clone());
        config.target = "apk".to_string();
        aether::codegen::shell_bundler::ShellBundler::new(config).bundle()?;
        apk_out
    };

    let detected_pkg = package.map(str::to_string).unwrap_or_else(|| {
        let fname = target_apk
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        if fname.contains("pocket") {
            "com.aether.pocketai".to_string()
        } else if fname.contains("chrono") {
            "com.aether.chronopulse".to_string()
        } else {
            "com.aether.frontend".to_string()
        }
    });

    let rep = aether::codegen::adb_deployer::AdbDeployer::deploy(
        device,
        &target_apk,
        &detected_pkg,
        Some("com.aether.runtime.MainActivity"),
        !no_launch,
        Some(port),
    )?;
    println!("Deployment finished successfully to {}!", rep.device.model);
    Ok(())
}

pub fn run_devices() -> Result<(), String> {
    let devs = aether::codegen::adb_deployer::AdbDeployer::list_devices()?;
    println!("Connected Android Devices (ADB):");
    if devs.is_empty() {
        println!("  No devices detected.");
    } else {
        for d in devs {
            println!(
                "  * {} | Model: {} | Product: {} [{}]",
                d.id, d.model, d.product, d.status
            );
        }
    }
    Ok(())
}

pub fn run_vsix(source: &str, out: &str) -> Result<(), String> {
    let path =
        aether::codegen::vsix_builder::VsixBuilder::new(PathBuf::from(source), PathBuf::from(out))
            .package()?;
    println!("Packaged VSIX extension: {}", path.display());
    Ok(())
}

pub fn run_serve(
    target: Option<&str>,
    port: u16,
    host: &str,
    threads: usize,
    open: bool,
) -> Result<(), String> {
    let mut static_dir = None;
    let mut script_path = None;

    if let Some(arg_path) = target {
        let p = PathBuf::from(arg_path);
        if p.is_dir() {
            static_dir = Some(p);
        } else if p.is_file() {
            if matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("ae") | Some("aether")
            ) {
                script_path = Some(p);
            } else if matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("html") | Some("htm")
            ) {
                static_dir = p.parent().map(|d| d.to_path_buf());
            }
        }
    } else if Path::new("frontend").exists() {
        static_dir = Some(PathBuf::from("frontend"));
    } else if Path::new("dist").exists() {
        static_dir = Some(PathBuf::from("dist"));
    } else if Path::new("index.html").exists() {
        static_dir = Some(PathBuf::from("."));
    }

    let config = aether::codegen::edge_server::EdgeServerConfig {
        host: host.to_string(),
        port,
        static_dir,
        script_path,
        thread_pool_size: threads,
        auto_open_browser: open,
    };
    aether::codegen::edge_server::EdgeServer::new(config).start()
}

pub fn run_live(
    target: &str,
    port: u16,
    interval: u64,
    open: bool,
    desktop: bool,
) -> Result<(), String> {
    let target_path = PathBuf::from(target);
    let is_script = matches!(
        target_path.extension().and_then(|e| e.to_str()),
        Some("ae") | Some("aether")
    );
    if (target_path.is_dir() || target_path.join("index.html").exists() || !is_script)
        && target_path.exists()
    {
        let mut config = aether::codegen::live_server::LiveServerConfig::new(target_path);
        config.port = port;
        config.poll_interval_ms = interval;
        config.auto_open_browser = open && !desktop;
        config.desktop_app_mode = desktop;
        return aether::codegen::live_server::LiveDevServer::new(config).start();
    }

    aether::vm::hotreload::run_live_watcher(target, interval)
}

pub fn run_desktop(
    target: Option<&str>,
    port: u16,
    width: u32,
    height: u32,
    interval: u64,
) -> Result<(), String> {
    let target_path = if let Some(p) = target {
        PathBuf::from(p)
    } else if Path::new("frontend").exists() {
        PathBuf::from("frontend")
    } else if Path::new("index.html").exists() {
        PathBuf::from(".")
    } else if Path::new("src/main.ae").exists() {
        PathBuf::from("src/main.ae")
    } else {
        PathBuf::from(".")
    };

    let serving_dir = if target_path.is_file()
        && matches!(
            target_path.extension().and_then(|e| e.to_str()),
            Some("ae") | Some("aether")
        ) {
        let temp_dir = std::env::temp_dir().join("aether_desktop_app");
        let _ = std::fs::create_dir_all(&temp_dir);
        let out_html = temp_dir.join("index.html");
        let app_name = target_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("AetherApp");
        if let Err(e) = aether::codegen::web_builder::WebBuilder::new(app_name)
            .build_web_app(&target_path, &out_html)
        {
            eprintln!("[AetherDesktop] Web compile warning: {}", e);
        }
        temp_dir
    } else {
        target_path
    };

    let mut config = aether::codegen::live_server::LiveServerConfig::new(serving_dir);
    config.port = port;
    config.poll_interval_ms = interval;
    config.auto_open_browser = false;
    config.desktop_app_mode = true;
    config.window_width = width;
    config.window_height = height;
    aether::codegen::live_server::LiveDevServer::new(config).start()
}

pub fn run_lsp(check: Option<&str>) -> Result<(), String> {
    if let Some(file_path) = check {
        let code = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Could not read '{}': {}", file_path, e))?;
        let srv = aether::lsp::AetherLanguageServer::new();
        let diags = srv.analyze(&code);
        if diags.is_empty() {
            println!("LSP Diagnostics: OK (0 errors)");
            Ok(())
        } else {
            for d in &diags {
                eprintln!("{}:{}: Error: {}", d.line + 1, d.col + 1, d.message);
            }
            Err(format!("{} LSP diagnostic error(s) found", diags.len()))
        }
    } else {
        let mut srv = aether::lsp::AetherLanguageServer::new();
        srv.run_stdio_server().map_err(|e| e.to_string())
    }
}

pub fn run_studio(port: u16) -> Result<(), String> {
    aether::playground_server::start_playground_server(port, true)
}

pub fn run_mcp() -> Result<(), String> {
    aether::mcp_server::AetherMcpServer::new()
        .run_stdio_server()
        .map_err(|e| e.to_string())
}

pub fn run_ai_context() -> Result<(), String> {
    println!("{}", aether::mcp_server::generate_ai_system_context());
    Ok(())
}

pub fn run_swarm(port: u16, duration: u64) -> Result<(), String> {
    let cfg = aether::vm::swarm::SwarmConfig {
        swarm_port: port,
        ..Default::default()
    };
    let node = aether::vm::swarm::SwarmNode::new(cfg);
    node.start().map_err(|e| format!("Swarm Error: {}", e))?;
    node.run_cli_monitor(duration);
    node.stop();
    Ok(())
}

pub fn run_rpc_call(target: &str, method: &str, args: &[String]) -> Result<(), String> {
    let (host, port) = if let Some((h, p)) = target.split_once(':') {
        (h, p.parse::<u16>().unwrap_or(19890))
    } else {
        ("127.0.0.1", target.parse::<u16>().unwrap_or(19890))
    };

    let call_args: Vec<aether::vm::Value> = args
        .iter()
        .map(|s| {
            if let Ok(i) = s.parse::<i64>() {
                aether::vm::Value::Int(i)
            } else if let Ok(f) = s.parse::<f64>() {
                aether::vm::Value::Float(f)
            } else if s == "true" {
                aether::vm::Value::Bool(true)
            } else if s == "false" {
                aether::vm::Value::Bool(false)
            } else {
                aether::vm::Value::string(s.clone())
            }
        })
        .collect();

    let mut req_map = std::collections::HashMap::new();
    req_map.insert("id".to_string(), aether::vm::Value::Int(1));
    req_map.insert(
        "method".to_string(),
        aether::vm::Value::string(method.to_string()),
    );
    req_map.insert("args".to_string(), aether::vm::Value::array(call_args));

    let payload = aether::vm::rpc::AetherPack::pack(&aether::vm::Value::map(req_map));
    let mut stream = std::net::TcpStream::connect((host, port)).map_err(|e| {
        format!(
            "Failed to connect to RPC server at {}:{}: {}",
            host, port, e
        )
    })?;
    let _ = stream.set_nodelay(true);
    aether::vm::rpc::write_framed(&mut stream, &payload)?;
    let resp_bytes = aether::vm::rpc::read_framed(&mut stream)?;
    let resp_val = aether::vm::rpc::AetherPack::unpack(&resp_bytes)?;

    if let aether::vm::Value::Map(m) = resp_val {
        let map = m.lock().clone();
        let status = map.get("status").map(|s| s.to_string()).unwrap_or_default();
        let result = map.get("result").cloned().unwrap_or(aether::vm::Value::Nil);
        if status == "ok" {
            println!("{}", result);
            Ok(())
        } else {
            Err(format!("RPC Remote Error: {}", result))
        }
    } else {
        println!("{}", resp_val);
        Ok(())
    }
}

pub fn run_df(csv_path: &str, mode: &str, rows_n: usize) -> Result<(), String> {
    let content = std::fs::read_to_string(csv_path)
        .map_err(|e| format!("Failed to read '{}': {}", csv_path, e))?;
    let df = aether::vm::dataframe::NativeDataFrame::from_csv(&content)?;
    println!("=== AETHER Columnar DataFrame ===");
    println!("Source: {}", csv_path);
    let (rows, cols) = df.shape();
    println!("Shape: [{} rows x {} columns]", rows, cols);
    println!("Schema: {:?}", df.column_names);

    if mode == "head" {
        let preview = df.head(rows_n);
        println!("\n[First {} Rows Preview]\n{}", rows_n, preview.to_csv());
    } else {
        println!("\n[Columnar Statistical Summary]");
        for (k, v) in df.describe() {
            println!("  * {}: {}", k, v);
        }
    }
    Ok(())
}

pub fn run_sql(csv_path: &str, query: &str) -> Result<(), String> {
    let content = std::fs::read_to_string(csv_path)
        .map_err(|e| format!("Failed to read '{}': {}", csv_path, e))?;
    let df = aether::vm::dataframe::NativeDataFrame::from_csv(&content)?;
    let mut tables = std::collections::HashMap::new();
    let table_name = Path::new(csv_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("data")
        .to_lowercase();
    tables.insert(table_name, Arc::new(Mutex::new(df.clone())));
    tables.insert("data".to_string(), Arc::new(Mutex::new(df)));

    let result_df = aether::vm::sql::execute_sql(query, &tables)?;
    println!("=== AetherSQL Execution Result ===");
    println!("Query: {}", query);
    println!(
        "Shape: [{} rows x {} columns]\n",
        result_df.row_count,
        result_df.column_names.len()
    );
    println!("{}", result_df.to_csv());
    Ok(())
}

pub fn run_quantum() -> Result<(), String> {
    println!("--- [Quantum Circuit Simulator Verification] ---");
    let verified = aether::quantum::run_bell_state_verification()
        .map_err(|e| format!("Quantum Error: {}", e))?;
    println!(
        "[Quantum] Bell State verification: {}",
        if verified { "SUCCESS" } else { "FAILED" }
    );
    let result =
        aether::quantum::simulate_grover(3, 5).map_err(|e| format!("Quantum Error: {}", e))?;
    println!(
        "[Quantum] Grover's search collapsed to: |{}> (Target: |5>)",
        result
    );
    Ok(())
}

pub fn run_complexity() -> Result<(), String> {
    println!("--- [Computational Complexity Analyzer] ---");
    aether::complexity::ComplexityAnalyzer::print_speedup_comparison();
    Ok(())
}

pub fn run_new(name: &str, template: &str) -> Result<(), String> {
    let path = aether::toolchain::scaffold_project(name, template)?;
    println!("Created AETHER project '{}' at {}", name, path.display());
    println!("\nNext steps:");
    println!("  cd {}", name);
    println!("  aether run src/main.ae");
    Ok(())
}

pub fn run_add(pkg: &str) -> Result<(), String> {
    let pm = aether::package_manager::AetherPackageManager::new(".");
    let dep = pm.add(pkg)?;
    println!("Added dependency: {} (v{})", dep.name, dep.version);
    if let Some(src) = dep.source {
        println!("  Source: {}", src);
    }
    if let Some(csum) = dep.checksum {
        println!("  Checksum: {}", csum);
    }
    Ok(())
}

pub fn run_remove(pkg: &str) -> Result<(), String> {
    let pm = aether::package_manager::AetherPackageManager::new(".");
    pm.remove(pkg)?;
    println!("Removed dependency: {}", pkg);
    Ok(())
}

pub fn run_publish() -> Result<(), String> {
    let msg = aether::package_manager::AetherPackageManager::new(".").publish()?;
    println!("{}", msg);
    Ok(())
}

pub fn run_install(pkg: Option<&str>, system: bool) -> Result<(), String> {
    if system
        || (pkg.is_none()
            && !Path::new("aether.toml").exists()
            && !Path::new("Aether.toml").exists())
    {
        return run_install_system();
    }

    let pm = aether::package_manager::AetherPackageManager::new(".");
    if let Some(spec) = pkg {
        let dep = pm.install(spec)?;
        println!("Installed package: {} (v{})", dep.name, dep.version);
        if let Some(cs) = dep.checksum {
            println!("  SHA-256: {}", cs);
        }
        return Ok(());
    }

    let deps = pm.install_all()?;
    println!("Installed {} package(s) from manifest:", deps.len());
    for d in deps {
        println!("  * {} (v{})", d.name, d.version);
    }
    Ok(())
}

pub fn run_install_system() -> Result<(), String> {
    let rep = aether::toolchain::install_to_system()?;
    println!("=== AETHER Global Toolchain Installation ===");
    println!("Installed Binary: {}", rep.target_bin.display());
    println!("Standard Libraries Copied: {}", rep.libraries_copied);
    if rep.already_in_path {
        println!("PATH Status: Already registered in User PATH");
    } else if rep.path_updated {
        println!("PATH Status: Added to User PATH (restart shell to refresh PATH)");
    } else {
        println!(
            "PATH Status: Add '{}' to your system PATH",
            aether::toolchain::get_aether_bin_dir().display()
        );
    }
    Ok(())
}

pub fn run_list() -> Result<(), String> {
    let lib_dir = aether::toolchain::get_aether_lib_dir();
    println!(
        "AETHER Standard & Installed Libraries ({}):",
        lib_dir.display()
    );
    let mut entries = Vec::new();
    for dir in [lib_dir.as_path(), Path::new("libraries")] {
        if let Ok(read) = std::fs::read_dir(dir) {
            for entry in read.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) == Some("ae") {
                    if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                        entries.push(format!("  - {} ({})", name, dir.display()));
                    }
                }
            }
        }
    }
    if entries.is_empty() {
        println!("  (no libraries found)");
    } else {
        entries.sort();
        entries.dedup();
        for line in entries {
            println!("{}", line);
        }
    }
    Ok(())
}

pub fn run_search(query: &str) -> Result<(), String> {
    let q = query.to_lowercase();
    let index = [
        (
            "aether_tensor",
            "Multi-dimensional tensors and linear algebra",
        ),
        (
            "aether_autograd",
            "Automatic differentiation & neural networks",
        ),
        ("aether_df", "Columnar DataFrame analytics & CSV processing"),
        ("aether_sql", "Hybrid relational + vector SQL query engine"),
        (
            "aether_test",
            "Unit testing assertions and benchmarking harness",
        ),
        ("aether_web", "HTTP microservices and REST routing library"),
        (
            "aether_ws",
            "WebSocket client and broadcast server primitives",
        ),
        (
            "aether_rpc",
            "Binary framed RPC transport & AetherPack codec",
        ),
        ("aether_flow", "Reactive stream windows and event pipelines"),
        (
            "aether_graph",
            "In-memory property graph & traversal algorithms",
        ),
        (
            "aether_proof",
            "Formal verification & symbolic contract helpers",
        ),
        ("aether_vector", "Cosine similarity, dot product, L2 search"),
    ];
    println!("Libraries matching '{}':", query);
    let mut matches = 0usize;
    for (name, desc) in index {
        if name.contains(&q) || desc.to_lowercase().contains(&q) {
            println!("  * {} - {}", name, desc);
            matches += 1;
        }
    }
    if matches == 0 {
        println!("  (no matching libraries in index)");
    }
    Ok(())
}

pub fn run_doctor() -> Result<(), String> {
    let doc = aether::toolchain::run_doctor();
    println!("=== AETHER Toolchain Doctor ===");
    println!("Version: {} ({})", doc.version, doc.channel);
    println!("Host OS: {} ({})", doc.os, doc.arch);
    println!("Binary:  {}", doc.bin_location.display());
    println!("Home:    {}", doc.aether_home.display());
    println!("\nDiagnostics Checklist:");
    for check in doc.checks {
        let mark = if check.passed { "OK" } else { "WARN" };
        println!("  [{}] {}: {}", mark, check.name, check.detail);
    }
    Ok(())
}

pub fn run_bench() -> Result<(), String> {
    aether::toolchain::run_production_benchmarks()
}

pub fn run_self_update() -> Result<(), String> {
    let rep = aether::toolchain::perform_self_update()?;
    println!(
        "Toolchain updated: {} -> {}",
        rep.current_version, rep.new_version
    );
    Ok(())
}

pub fn run_tour(lesson: Option<&str>) -> Result<(), String> {
    match lesson {
        Some("list") | Some("--list") | None => {
            aether::tour::list_tour_lessons();
            Ok(())
        }
        Some(arg) => {
            let id = arg
                .parse::<usize>()
                .map_err(|_| format!("Invalid tour lesson id '{}'", arg))?;
            aether::tour::run_tour_lesson(id)
        }
    }
}

pub fn print_version() -> Result<(), String> {
    println!(
        "AETHER Language Compiler {}",
        aether::toolchain::AETHER_VERSION
    );
    println!("Channel: {}", aether::toolchain::AETHER_RELEASE_CHANNEL);
    println!("Runtime: Bytecode VM + Cranelift AOT/JIT");
    Ok(())
}
