pub mod commands;
pub mod diagnostics;
pub mod report;
pub mod test_runner;

use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "aether",
    about = "AETHER Programming Language Compiler, Bytecode VM & Scientific Toolchain",
    disable_version_flag = true,
    arg_required_else_help = false
)]
struct Cli {
    /// Print compiler version and runtime details
    #[arg(short = 'v', long = "version")]
    version: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Execute an AETHER source file on the VM or Cranelift JIT
    Run {
        file: Option<String>,
        #[arg(long, alias = "native")]
        jit: bool,
    },
    /// Execute an AETHER source file directly using Cranelift JIT
    Jit { file: String },
    /// Start an interactive AETHER REPL session
    Repl,
    /// Run a script and launch the TimeTravel TUI debugger
    #[command(visible_alias = "timetravel")]
    Replay { file: String },
    /// Watch an AETHER script and hot-reload bytecode on changes
    Hotreload {
        file: String,
        #[arg(default_value_t = 250)]
        interval_ms: u64,
    },
    /// Run `.ae` test suites on the VM (`aether test [file|dir] [--all]`)
    Test {
        target: Option<String>,
        #[arg(long)]
        all: bool,
    },
    /// Formally verify intent contracts with the symbolic prover
    #[command(visible_alias = "prove")]
    Verify { file: String },
    /// Run static type checking, linting, and security scanning
    #[command(visible_alias = "audit")]
    Check { file: String },
    /// Run the static linter on an AETHER file
    Lint { file: String },
    /// Format an AETHER source file
    #[command(visible_alias = "format")]
    Fmt {
        file: String,
        #[arg(short = 'w', long)]
        write: bool,
    },
    /// Disassemble AETHER source into bytecode instructions
    #[command(visible_alias = "disassemble")]
    Disasm { file: String },
    /// Execute an AETHER script and generate a dynamic HTML execution report
    #[command(visible_alias = "html-report")]
    Report {
        file: String,
        #[arg(long = "html", short = 'o', value_name = "OUTPUT.html")]
        html: Option<String>,
        #[arg(value_name = "OUTPUT.html")]
        out_positional: Option<String>,
    },
    /// Build a standalone executable, web app, or Android APK from an `.ae` script
    Build {
        source: Option<String>,
        #[arg(short = 'o', long)]
        out: Option<String>,
        #[arg(short = 't', long)]
        target: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        package: Option<String>,
        #[arg(long)]
        release: bool,
    },
    /// Compile an `.ae` script into a native AOT object file (`.obj` / `.o`)
    #[command(visible_alias = "aot")]
    Compile {
        file: String,
        #[arg(short = 'o', long)]
        out: Option<String>,
    },
    /// Compile an `.ae` script into a WebAssembly module (`.wasm`)
    Wasm {
        file: String,
        #[arg(short = 'o', long)]
        out: Option<String>,
    },
    /// Execute a function inside a compiled `.wasm` module
    Edge {
        wasm_file: String,
        function: String,
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Build a standalone single-file HTML/Web application
    Web {
        source: String,
        #[arg(short = 'o', long)]
        out: Option<String>,
        #[arg(long)]
        name: Option<String>,
    },
    /// Build a signed Android APK from an `.ae` script
    Apk {
        source: String,
        #[arg(short = 'o', long)]
        out: Option<String>,
        #[arg(long)]
        package: Option<String>,
        #[arg(long)]
        name: Option<String>,
    },
    /// Bundle a frontend distribution directory into APK, desktop, or web output
    Bundle {
        dist_dir: String,
        #[arg(short = 'o', long)]
        out: Option<String>,
        #[arg(short = 't', long, default_value = "apk")]
        target: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        package: Option<String>,
    },
    /// Deploy an APK or frontend directory to a connected Android device via ADB
    #[command(visible_alias = "run-mobile")]
    Deploy {
        input: String,
        #[arg(short = 'd', long)]
        device: Option<String>,
        #[arg(short = 'p', long, default_value_t = 8080)]
        port: u16,
        #[arg(long)]
        package: Option<String>,
        #[arg(long)]
        no_launch: bool,
    },
    /// List connected Android devices via ADB
    Devices,
    /// Package the VS Code AETHER extension into a `.vsix` archive
    #[command(visible_alias = "vscode")]
    Vsix {
        #[arg(short = 's', long, default_value = "editors/vscode")]
        source: String,
        #[arg(
            short = 'o',
            long,
            default_value = "editors/vscode/aether-language-support-2.0.0.vsix"
        )]
        out: String,
    },
    /// Start the HTTP edge/microservice server
    #[command(visible_aliases = ["serverless", "microservice"])]
    Serve {
        target: Option<String>,
        #[arg(short = 'p', long, default_value_t = 3000)]
        port: u16,
        #[arg(short = 'H', long, default_value = "0.0.0.0")]
        host: String,
        #[arg(short = 't', long, default_value_t = 8)]
        threads: usize,
        #[arg(short = 'o', long)]
        open: bool,
    },
    /// Start live dev server for web assets or hot-reload for `.ae` scripts
    #[command(visible_aliases = ["dev", "watch"])]
    Live {
        target: String,
        #[arg(short = 'p', long, default_value_t = 8080)]
        port: u16,
        #[arg(short = 'i', long, default_value_t = 80)]
        interval: u64,
        #[arg(short = 'o', long)]
        open: bool,
        #[arg(short = 'd', long)]
        desktop: bool,
    },
    /// Launch standalone desktop GUI app container
    Desktop {
        target: Option<String>,
        #[arg(short = 'p', long, default_value_t = 8080)]
        port: u16,
        #[arg(short = 'w', long, default_value_t = 480)]
        width: u32,
        #[arg(short = 'H', long, default_value_t = 850)]
        height: u32,
        #[arg(short = 'i', long, default_value_t = 80)]
        interval: u64,
    },
    /// Run the Language Server Protocol (LSP) daemon or check a single file
    Lsp {
        #[arg(long)]
        check: Option<String>,
    },
    /// Launch the local browser IDE playground server
    #[command(visible_alias = "playground")]
    Studio {
        #[arg(default_value_t = 8080)]
        port: u16,
    },
    /// Run the Model Context Protocol (MCP) stdio server for AI assistants
    Mcp,
    /// Print system context prompt for AI coding assistants
    #[command(visible_alias = "ai-context")]
    Ai,
    /// Start an AETHER distributed swarm node
    #[command(visible_alias = "mesh")]
    Swarm {
        #[arg(short = 'p', long, default_value_t = aether::vm::swarm::DEFAULT_SWARM_PORT)]
        port: u16,
        #[arg(short = 'd', long, default_value_t = 60)]
        duration: u64,
    },
    /// Call a remote AETHER RPC endpoint over AetherPack
    #[command(visible_alias = "rpc-call")]
    Rpc {
        target: String,
        method: String,
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Inspect schema, head, or statistics of a CSV file as a NativeDataFrame
    Df {
        file: String,
        #[arg(default_value = "summary")]
        mode: String,
        #[arg(default_value_t = 5)]
        rows: usize,
    },
    /// Run a hybrid SQL query directly against a CSV file
    Sql { file: String, query: String },
    /// Execute a quick head preview of a CSV file
    Head {
        file: String,
        #[arg(default_value_t = 5)]
        rows: usize,
    },
    /// Run built-in quantum state and Grover simulation checks
    Quantum,
    /// Print computational complexity class speedup reference
    Complexity,
    /// Scaffold a new AETHER project directory
    #[command(visible_aliases = ["init", "create"])]
    New {
        name: String,
        #[arg(short = 't', long, default_value = "default")]
        template: String,
    },
    /// Add a dependency package to `aether.toml`
    Add { package: String },
    /// Remove a dependency package from `aether.toml`
    Remove { package: String },
    /// Install a package or the global AETHER toolchain (`--system`)
    Install {
        package: Option<String>,
        #[arg(short = 's', long, visible_aliases = ["global"])]
        system: bool,
        #[arg(short = 'g', hide = true)]
        global_short: bool,
    },
    /// Install the AETHER binary and stdlib into `~/.aether` and User PATH
    #[command(visible_alias = "setup")]
    InstallSystem,
    /// List installed libraries and bundled `.ae` standard modules
    List,
    /// Search available AETHER standard/ecosystem libraries
    Search { query: String },
    /// Package current project for AetherPM distribution
    Publish,
    /// Diagnose local toolchain health and environment configuration
    Doctor,
    /// Run production micro-benchmarks on the VM
    Bench,
    /// Perform a safety-checked toolchain self-update
    #[command(visible_aliases = ["update", "upgrade"])]
    SelfUpdate {
        /// Optional release manifest or binary URL override
        #[arg(long)]
        url: Option<String>,
        /// Only check if a newer release is available without modifying binaries
        #[arg(long)]
        check: bool,
    },
    /// Run interactive guided language tour lessons
    Tour { lesson: Option<String> },
    /// Show AETHER compiler version information
    Version,
}

pub fn run() -> Result<(), String> {
    if check_embedded_aot_payload()? {
        return Ok(());
    }

    let raw_args: Vec<String> = std::env::args().collect();

    // Direct script shorthand: `aether my_script.ae`
    if raw_args.len() >= 2 && is_script_path(&raw_args[1]) {
        let jit = raw_args.iter().any(|a| a == "--jit" || a == "--native");
        return commands::run_script(Some(&raw_args[1]), jit);
    }

    if raw_args.len() <= 1 {
        let mut cmd = Cli::command();
        cmd.print_help().map_err(|e| e.to_string())?;
        println!();
        return Ok(());
    }

    let cli = Cli::parse_from(raw_args);
    if cli.version {
        return commands::print_version();
    }

    match cli.command {
        None => {
            let mut cmd = Cli::command();
            cmd.print_help().map_err(|e| e.to_string())?;
            println!();
            Ok(())
        }
        Some(cmd) => dispatch(cmd),
    }
}

fn dispatch(cmd: Commands) -> Result<(), String> {
    match cmd {
        Commands::Run { file, jit } => commands::run_script(file.as_deref(), jit),
        Commands::Jit { file } => commands::run_jit(&file),
        Commands::Repl => commands::run_repl(),
        Commands::Replay { file } => commands::run_replay(&file),
        Commands::Hotreload { file, interval_ms } => {
            aether::vm::hotreload::run_live_watcher(&file, interval_ms)
        }
        Commands::Test { target, all } => test_runner::run_test_command(target.as_deref(), all),
        Commands::Verify { file } => commands::run_verify(&file),
        Commands::Check { file } => commands::run_check(&file),
        Commands::Lint { file } => commands::run_lint(&file),
        Commands::Fmt { file, write } => commands::run_fmt(&file, write),
        Commands::Disasm { file } => commands::run_disasm(&file),
        Commands::Report {
            file,
            html,
            out_positional,
        } => {
            let out = html.as_deref().or(out_positional.as_deref());
            report::run_report_command(&file, out)
        }
        Commands::Build {
            source,
            out,
            target,
            name,
            package,
            release,
        } => commands::run_build(
            source.as_deref(),
            out.as_deref(),
            target.as_deref(),
            name.as_deref(),
            package.as_deref(),
            release,
        ),
        Commands::Compile { file, out } => commands::run_compile(&file, out.as_deref()),
        Commands::Wasm { file, out } => commands::run_wasm(&file, out.as_deref()),
        Commands::Edge {
            wasm_file,
            function,
            args,
        } => commands::run_edge(&wasm_file, &function, &args),
        Commands::Web { source, out, name } => {
            commands::run_web(&source, out.as_deref(), name.as_deref())
        }
        Commands::Apk {
            source,
            out,
            package,
            name,
        } => commands::run_apk(&source, out.as_deref(), package.as_deref(), name.as_deref()),
        Commands::Bundle {
            dist_dir,
            out,
            target,
            name,
            package,
        } => commands::run_bundle(
            &dist_dir,
            out.as_deref(),
            &target,
            name.as_deref(),
            package.as_deref(),
        ),
        Commands::Deploy {
            input,
            device,
            port,
            package,
            no_launch,
        } => commands::run_deploy(
            &input,
            device.as_deref(),
            port,
            package.as_deref(),
            no_launch,
        ),
        Commands::Devices => commands::run_devices(),
        Commands::Vsix { source, out } => commands::run_vsix(&source, &out),
        Commands::Serve {
            target,
            port,
            host,
            threads,
            open,
        } => commands::run_serve(target.as_deref(), port, &host, threads, open),
        Commands::Live {
            target,
            port,
            interval,
            open,
            desktop,
        } => commands::run_live(&target, port, interval, open, desktop),
        Commands::Desktop {
            target,
            port,
            width,
            height,
            interval,
        } => commands::run_desktop(target.as_deref(), port, width, height, interval),
        Commands::Lsp { check } => commands::run_lsp(check.as_deref()),
        Commands::Studio { port } => commands::run_studio(port),
        Commands::Mcp => commands::run_mcp(),
        Commands::Ai => commands::run_ai_context(),
        Commands::Swarm { port, duration } => commands::run_swarm(port, duration),
        Commands::Rpc {
            target,
            method,
            args,
        } => commands::run_rpc_call(&target, &method, &args),
        Commands::Df { file, mode, rows } => commands::run_df(&file, &mode, rows),
        Commands::Sql { file, query } => commands::run_sql(&file, &query),
        Commands::Head { file, rows } => commands::run_df(&file, "head", rows),
        Commands::Quantum => commands::run_quantum(),
        Commands::Complexity => commands::run_complexity(),
        Commands::New { name, template } => commands::run_new(&name, &template),
        Commands::Add { package } => commands::run_add(&package),
        Commands::Remove { package } => commands::run_remove(&package),
        Commands::Install {
            package,
            system,
            global_short,
        } => commands::run_install(package.as_deref(), system || global_short),
        Commands::InstallSystem => commands::run_install_system(),
        Commands::List => commands::run_list(),
        Commands::Search { query } => commands::run_search(&query),
        Commands::Publish => commands::run_publish(),
        Commands::Doctor => commands::run_doctor(),
        Commands::Bench => commands::run_bench(),
        Commands::SelfUpdate { url, check } => commands::run_self_update(url.as_deref(), check),
        Commands::Tour { lesson } => commands::run_tour(lesson.as_deref()),
        Commands::Version => commands::print_version(),
    }
}

fn is_script_path(arg: &str) -> bool {
    !arg.starts_with('-') && (arg.ends_with(".ae") || arg.ends_with(".aether"))
}

fn check_embedded_aot_payload() -> Result<bool, String> {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Ok(bytes) = std::fs::read(&exe_path) {
            if let Some(payload_str) =
                aether::codegen::aot_builder::extract_embedded_payload(&bytes)
            {
                aether::vm::run_source(&payload_str)
                    .map(|_| ())
                    .map_err(|e| diagnostics::format_error_with_source(&payload_str, &e))?;
                return Ok(true);
            }
        }
    }
    Ok(false)
}
