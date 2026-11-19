use crate::syntax::ast::{Block, Program, Statement};
use crate::syntax::parse;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

pub struct AotBuilder {
    pub release: bool,
}

pub const MAGIC: &[u8] = b"__AETHER_EMBEDDED_PAYLOAD_V1__";
pub const BUNDLE_MAGIC: &[u8] = b"__AETHER_EMBEDDED_BUNDLE_V2__";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedBundle {
    pub entry_source: String,
    pub modules: HashMap<String, String>,
    pub assets: HashMap<String, Vec<u8>>,
}

/// Extracts legacy V1 embedded AETHER script payload from executable bytes if present.
pub fn extract_embedded_payload(bytes: &[u8]) -> Option<String> {
    extract_framed_bytes(bytes, MAGIC).and_then(|raw| String::from_utf8(raw).ok())
}

/// Extracts V2 multi-module + asset standalone bundle from executable bytes if present.
pub fn extract_embedded_bundle(bytes: &[u8]) -> Option<EmbeddedBundle> {
    let raw = extract_framed_bytes(bytes, BUNDLE_MAGIC)?;
    serde_json::from_slice(&raw).ok()
}

fn extract_framed_bytes(bytes: &[u8], magic: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() < magic.len() * 2 + 8 {
        return None;
    }
    if &bytes[bytes.len() - magic.len()..] != magic {
        return None;
    }
    let search_slice = &bytes[..bytes.len() - magic.len()];
    if let Some(start_idx) = search_slice.windows(magic.len()).rposition(|w| w == magic) {
        let content_start = start_idx + magic.len() + 8;
        let content_end = bytes.len() - magic.len();
        if content_start <= content_end {
            let len_bytes: [u8; 8] = search_slice
                [start_idx + magic.len()..start_idx + magic.len() + 8]
                .try_into()
                .ok()?;
            let expected_len = u64::from_le_bytes(len_bytes) as usize;
            if expected_len == content_end - content_start {
                return Some(bytes[content_start..content_end].to_vec());
            }
        }
    }
    None
}

fn strip_existing_trailer(exe_bytes: &mut Vec<u8>) {
    for magic in [BUNDLE_MAGIC, MAGIC] {
        if exe_bytes.len() >= magic.len() * 2 + 8
            && &exe_bytes[exe_bytes.len() - magic.len()..] == magic
        {
            let search_slice = &exe_bytes[..exe_bytes.len() - magic.len()];
            if let Some(start_idx) = search_slice.windows(magic.len()).rposition(|w| w == magic) {
                exe_bytes.truncate(start_idx);
            }
        }
    }
}

fn write_framed_executable(
    exe_path: &Path,
    magic: &[u8],
    payload_bytes: &[u8],
    output_path: &Path,
) -> Result<(), String> {
    let mut exe_bytes = fs::read(exe_path).map_err(|e| {
        format!(
            "Failed to read base executable '{}': {}",
            exe_path.display(),
            e
        )
    })?;

    strip_existing_trailer(&mut exe_bytes);

    let payload_len = (payload_bytes.len() as u64).to_le_bytes();
    exe_bytes.extend_from_slice(magic);
    exe_bytes.extend_from_slice(&payload_len);
    exe_bytes.extend_from_slice(payload_bytes);
    exe_bytes.extend_from_slice(magic);

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }

    fs::write(output_path, exe_bytes).map_err(|e| {
        format!(
            "Failed to write standalone binary '{}': {}",
            output_path.display(),
            e
        )
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(output_path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(output_path, perms);
        }
    }

    Ok(())
}

/// Embeds a legacy V1 single-script payload into a standalone native executable.
pub fn embed_payload(exe_path: &Path, payload: &str, output_path: &Path) -> Result<(), String> {
    write_framed_executable(exe_path, MAGIC, payload.as_bytes(), output_path)
}

/// Embeds a V2 multi-module + asset bundle into a standalone native executable.
pub fn embed_bundle(
    exe_path: &Path,
    bundle: &EmbeddedBundle,
    output_path: &Path,
) -> Result<(), String> {
    let serialized = serde_json::to_vec(bundle)
        .map_err(|e| format!("Failed to serialize standalone bundle: {}", e))?;
    write_framed_executable(exe_path, BUNDLE_MAGIC, &serialized, output_path)
}

fn collect_imports_from_block(block: &Block, out: &mut Vec<String>) {
    for stmt in &block.statements {
        collect_imports_from_stmt(stmt, out);
    }
}

fn collect_imports_from_stmt(stmt: &Statement, out: &mut Vec<String>) {
    match stmt {
        Statement::Import { module, .. } | Statement::FromImport { module, .. } => {
            out.push(module.clone());
        }
        Statement::FnDef { body, .. }
        | Statement::While { body, .. }
        | Statement::For { body, .. }
        | Statement::With { body, .. }
        | Statement::Sandbox { body, .. }
        | Statement::Spawn { body, .. } => collect_imports_from_block(body, out),
        Statement::TryCatch {
            try_block,
            handlers,
            finally_block,
            ..
        } => {
            collect_imports_from_block(try_block, out);
            for h in handlers {
                collect_imports_from_block(&h.body, out);
            }
            if let Some(fb) = finally_block {
                collect_imports_from_block(fb, out);
            }
        }
        Statement::ClassDef { body, .. } => {
            for m in body {
                collect_imports_from_stmt(m, out);
            }
        }
        Statement::ImplDef { methods, .. } => {
            for m in methods {
                collect_imports_from_stmt(m, out);
            }
        }
        _ => {}
    }
}

fn collect_imports(program: &Program) -> Vec<String> {
    let mut out = Vec::new();
    for stmt in &program.statements {
        collect_imports_from_stmt(stmt, &mut out);
    }
    out
}

fn resolve_module_file(mod_name: &str, base_dir: &Path) -> Option<PathBuf> {
    let clean = mod_name.to_lowercase();
    let mut candidates = vec![
        base_dir.join(mod_name),
        base_dir.join(format!("{}.ae", mod_name)),
        base_dir.join(format!("{}.aether", mod_name)),
        base_dir.join(format!("libraries/{}.ae", mod_name)),
        PathBuf::from(mod_name),
        PathBuf::from(format!("{}.ae", mod_name)),
        PathBuf::from(format!("{}.aether", mod_name)),
        PathBuf::from(format!("libraries/{}.ae", mod_name)),
        PathBuf::from(format!("libraries/{}.aether", mod_name)),
    ];
    if mod_name.contains('.') && !mod_name.ends_with(".ae") && !mod_name.ends_with(".aether") {
        let slash = mod_name.replace('.', "/");
        candidates.push(base_dir.join(format!("{}.ae", slash)));
        candidates.push(PathBuf::from(format!("{}.ae", slash)));
        candidates.push(PathBuf::from(format!("libraries/{}.ae", slash)));
    }
    if clean.starts_with("aether") && !clean.starts_with("aether_") {
        let suffix = &clean["aether".len()..];
        candidates.push(PathBuf::from(format!("libraries/aether_{}.ae", suffix)));
    }
    candidates.into_iter().find(|p| p.is_file())
}

/// Recursively collects all imported `.ae` modules (local + `libraries/*.ae`) for standalone bundling.
pub fn collect_transitive_modules(
    source_path: &Path,
    entry_program: &Program,
) -> HashMap<String, String> {
    let base_dir = source_path.parent().unwrap_or_else(|| Path::new("."));
    let mut modules = HashMap::new();
    let mut visited = HashSet::new();
    let mut queue: Vec<String> = collect_imports(entry_program);

    while let Some(mod_name) = queue.pop() {
        if !visited.insert(mod_name.clone()) {
            continue;
        }
        if let Some(mod_path) = resolve_module_file(&mod_name, base_dir) {
            if let Ok(src) = fs::read_to_string(&mod_path) {
                if let Ok(prog) = parse(&src) {
                    for dep in collect_imports(&prog) {
                        if !visited.contains(&dep) {
                            queue.push(dep);
                        }
                    }
                }
                modules.insert(mod_name, src);
            }
        }
    }
    modules
}

impl AotBuilder {
    pub fn new(release: bool) -> Self {
        Self { release }
    }

    /// Locates rust-lld or host linkers if available
    pub fn find_linker() -> Option<PathBuf> {
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

        let mingw_gcc = PathBuf::from(r"C:\MinGW\bin\gcc.exe");
        if mingw_gcc.exists() {
            return Some(mingw_gcc);
        }

        None
    }

    /// Builds a standalone native executable bundling the script + all imported modules.
    pub fn build_executable(&self, source_path: &Path, output_path: &Path) -> Result<(), String> {
        self.build_executable_with_assets(source_path, output_path, &[])
    }

    /// Builds a standalone native executable bundling the script, all transitive modules, and optional binary/text assets.
    pub fn build_executable_with_assets(
        &self,
        source_path: &Path,
        output_path: &Path,
        asset_paths: &[String],
    ) -> Result<(), String> {
        let source_content = fs::read_to_string(source_path).map_err(|e| {
            format!(
                "Failed to read source file '{}': {}",
                source_path.display(),
                e
            )
        })?;

        // 1. Verify syntax & collect transitive modules
        let program = parse(&source_content)
            .map_err(|(err, span)| format!("{}:{}: Syntax error: {}", span.line, span.col, err))?;
        let modules = collect_transitive_modules(source_path, &program);

        // 2. Load embedded assets (weights, configs, tokenizers, etc.)
        let mut assets = HashMap::new();
        for asset_spec in asset_paths {
            let p = PathBuf::from(asset_spec);
            let bytes = fs::read(&p)
                .map_err(|e| format!("Failed to read asset '{}': {}", p.display(), e))?;
            let key = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(asset_spec)
                .to_string();
            assets.insert(key, bytes);
        }

        println!(
            "[AETHER Build] Parsed {} statements, bundled {} transitive modules and {} assets.",
            program.statements.len(),
            modules.len(),
            assets.len()
        );
        println!("[AETHER Build] Packaging standalone zero-dependency executable...");

        // 3. Identify base runtime executable
        let base_exe = resolve_base_runtime_exe()?;
        let bundle = EmbeddedBundle {
            entry_source: source_content,
            modules,
            assets,
        };
        embed_bundle(&base_exe, &bundle, output_path)?;

        let metadata = fs::metadata(output_path).map_err(|e| e.to_string())?;
        println!(
            "[AETHER Build] Standalone executable created: {} ({:.2} MB)",
            output_path.display(),
            metadata.len() as f64 / (1024.0 * 1024.0)
        );
        Ok(())
    }
}

fn resolve_base_runtime_exe() -> Result<PathBuf, String> {
    let exe_name = if cfg!(windows) {
        "aether.exe"
    } else {
        "aether"
    };
    let current_exe = std::env::current_exe().unwrap_or_default();
    let is_test_harness = current_exe
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.contains("tests") || n.contains("deps"));

    let mut candidates = Vec::new();
    if !is_test_harness && current_exe.is_file() {
        candidates.push(current_exe.clone());
    }
    if let Some(parent) = current_exe.parent() {
        candidates.push(parent.join(exe_name));
        if let Some(grand) = parent.parent() {
            candidates.push(grand.join(exe_name));
        }
    }
    candidates.push(PathBuf::from("target").join("release").join(exe_name));
    candidates.push(PathBuf::from("target").join("debug").join(exe_name));

    for cand in candidates {
        if cand.is_file() {
            return Ok(cand);
        }
    }
    Err(format!(
        "Base AETHER runtime executable '{}' not found; run `cargo build` first",
        exe_name
    ))
}
