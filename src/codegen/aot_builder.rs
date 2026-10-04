use std::fs;
use std::path::{Path, PathBuf};
use crate::syntax::parse;

pub struct AotBuilder {
    pub release: bool,
}

pub const MAGIC: &[u8] = b"__AETHER_EMBEDDED_PAYLOAD_V1__";

/// Extracts embedded AETHER script payload from executable bytes if present
pub fn extract_embedded_payload(bytes: &[u8]) -> Option<String> {
    if bytes.len() < MAGIC.len() * 2 + 8 {
        return None;
    }
    if &bytes[bytes.len() - MAGIC.len()..] != MAGIC {
        return None;
    }
    let search_slice = &bytes[..bytes.len() - MAGIC.len()];
    if let Some(start_idx) = search_slice.windows(MAGIC.len()).rposition(|w| w == MAGIC) {
        let content_start = start_idx + MAGIC.len() + 8;
        let content_end = bytes.len() - MAGIC.len();
        if content_start <= content_end {
            let len_bytes: [u8; 8] = search_slice[start_idx + MAGIC.len()..start_idx + MAGIC.len() + 8].try_into().ok()?;
            let expected_len = u64::from_le_bytes(len_bytes) as usize;
            if expected_len == content_end - content_start {
                return String::from_utf8(bytes[content_start..content_end].to_vec()).ok();
            }
        }
    }
    None
}

/// Embeds an AETHER script payload into a standalone native executable
pub fn embed_payload(exe_path: &Path, payload: &str, output_path: &Path) -> Result<(), String> {
    let mut exe_bytes = fs::read(exe_path)
        .map_err(|e| format!("Failed to read base executable '{}': {}", exe_path.display(), e))?;

    // Strip any existing payload if present
    if exe_bytes.len() >= MAGIC.len() * 2 + 8 && &exe_bytes[exe_bytes.len() - MAGIC.len()..] == MAGIC {
        let search_slice = &exe_bytes[..exe_bytes.len() - MAGIC.len()];
        if let Some(start_idx) = search_slice.windows(MAGIC.len()).rposition(|w| w == MAGIC) {
            exe_bytes.truncate(start_idx);
        }
    }

    let payload_bytes = payload.as_bytes();
    let payload_len = (payload_bytes.len() as u64).to_le_bytes();

    exe_bytes.extend_from_slice(MAGIC);
    exe_bytes.extend_from_slice(&payload_len);
    exe_bytes.extend_from_slice(payload_bytes);
    exe_bytes.extend_from_slice(MAGIC);

    fs::write(output_path, exe_bytes)
        .map_err(|e| format!("Failed to write standalone binary '{}': {}", output_path.display(), e))?;
    Ok(())
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

    /// Builds a standalone native executable for an AETHER script
    pub fn build_executable(&self, source_path: &Path, output_path: &Path) -> Result<(), String> {
        let source_content = fs::read_to_string(source_path)
            .map_err(|e| format!("Failed to read source file '{}': {}", source_path.display(), e))?;

        // 1. Verify syntax
        let program = parse(&source_content)
            .map_err(|(err, span)| format!("{}:{}: Syntax error: {}", span.line, span.col, err))?;

        println!("[AETHER Build] Parsed {} top-level statements successfully.", program.statements.len());
        println!("[AETHER Build] Packaging standalone zero-dependency executable...");

        // 2. Identify base runtime executable
        let current_exe = std::env::current_exe().unwrap_or_default();
        let target_dir = current_exe.parent().unwrap_or(Path::new("."));
        let release_exe = target_dir.join("aether.exe");

        let base_exe = if current_exe.exists() && current_exe.is_file() {
            current_exe
        } else if release_exe.exists() {
            release_exe
        } else {
            PathBuf::from("target").join("release").join("aether.exe")
        };

        if !base_exe.exists() {
            return Err(format!("Base AETHER executable runtime not found at '{}'", base_exe.display()));
        }

        embed_payload(&base_exe, &source_content, output_path)?;

        let metadata = fs::metadata(output_path).map_err(|e| e.to_string())?;
        println!("✨ Standalone executable successfully built: {}", output_path.display());
        println!("   Size: {:.2} MB (Zero external dependencies)", metadata.len() as f64 / (1024.0 * 1024.0));
        println!("   Ready to distribute and run directly on Windows!");

        Ok(())
    }
}
