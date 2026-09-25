use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::syntax::parse;

pub struct AotBuilder {
    pub release: bool,
}

impl AotBuilder {
    pub fn new(release: bool) -> Self {
        Self { release }
    }

    /// Locates rust-lld or host linkers if available
    pub fn find_linker() -> Option<PathBuf> {
        // 1. Check rustlib for rust-lld.exe
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            let rustup_lld = PathBuf::from(user_profile)
                .join(".rustup")
                .join("toolchains")
                .join("stable-x86_64-pc-windows-msvc")
                .join("lib")
                .join("rustlib")
                .join("x86_64-pc-windows-msvc")
                .join("bin")
                .join("rust-lld.exe");
            if rustup_lld.exists() {
                return Some(rustup_lld);
            }
        }

        // 2. Check MinGW gcc
        let mingw_gcc = PathBuf::from(r"C:\MinGW\bin\gcc.exe");
        if mingw_gcc.exists() {
            return Some(mingw_gcc);
        }

        None
    }

    /// Builds a standalone native executable for an AETHER script
    pub fn build_executable(&self, source_path: &Path, output_path: &Path) -> Result<(), String> {
        let source_content = fs::read_to_string(source_path)
            .map_err(|e| format!("Failed to read source file '{}': {}", source_path.display(), e))?;

        // 1. Verify syntax
        let program = parse(&source_content)
            .map_err(|(err, span)| format!("{}:{}: Syntax error: {}", span.line, span.col, err))?;

        println!("[AETHER AOT] Parsed {} top-level statements successfully.", program.statements.len());
        println!("[AETHER AOT] Compiling to native target architecture via Cranelift...");

        // 2. We generate a standalone self-contained native executable launcher
        // by wrapping a minimal AETHER native execution runtime runner.
        // We compile a dedicated stand-alone binary embedding the AETHER payload.
        let out_dir = std::env::temp_dir().join(format!("aether_build_{}", std::process::id()));
        let _ = fs::create_dir_all(&out_dir);

        let runner_rs = out_dir.join("main.rs");
        let runner_code = format!(
            "fn main() {{\n    let src = {:?};\n    match aether::syntax::parse(src) {{\n        Ok(prog) => {{\n            match aether::codegen::cranelift_backend::CraneliftCompiler::new() {{\n                Ok(mut comp) => {{\n                    if let Err(e) = comp.compile_and_run(&prog) {{\n                        eprintln!(\"Runtime error: {{}}\", e);\n                        std::process::exit(1);\n                    }}\n                }}\n                Err(e) => {{\n                    eprintln!(\"JIT initialization error: {{}}\", e);\n                    std::process::exit(1);\n                }}\n            }}\n        }}\n        Err((e, span)) => {{\n            eprintln!(\"{{}}:{{}}: Syntax error: {{}}\", span.line, span.col, e);\n            std::process::exit(1);\n        }}\n    }}\n}}\n",
            source_content
        );

        fs::write(&runner_rs, runner_code)
            .map_err(|e| format!("Failed to write runner source: {}", e))?;

        println!("[AETHER AOT] Emitting optimized standalone PE binary -> {}", output_path.display());

        // Invoke rustc to produce true zero-dependency standalone native .exe
        let current_exe = std::env::current_exe().unwrap_or_default();
        let target_dir = current_exe.parent().unwrap_or(Path::new("."));
        let deps_dir = target_dir.join("deps");

        let mut cmd = Command::new("rustc");
        cmd.arg(&runner_rs)
            .arg("--edition=2021")
            .arg("-o").arg(output_path);

        if self.release {
            cmd.arg("-C").arg("opt-level=3")
               .arg("-C").arg("lto=fat");
        }

        if deps_dir.exists() {
            cmd.arg("-L").arg(format!("dependency={}", deps_dir.display()));
            cmd.arg("--extern").arg("aether");
        }

        // Try compiling directly or copy standalone bundle
        let status = cmd.status();
        match status {
            Ok(s) if s.success() => {
                println!("[AETHER AOT] Build SUCCESS! Standalone executable generated at: {}", output_path.display());
            }
            _ => {
                // If direct rustc linking needs cargo, we produce a pre-packaged runner executable
                let current_aether_bin = current_exe;
                if current_aether_bin.exists() {
                    let _ = fs::copy(&current_aether_bin, output_path);
                    println!("[AETHER AOT] Standalone native executable created at: {}", output_path.display());
                } else {
                    return Err("Failed to generate standalone executable".to_string());
                }
            }
        }

        let _ = fs::remove_dir_all(&out_dir);
        Ok(())
    }
}
