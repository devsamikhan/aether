use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Resolves module imports, tracking recursive references to prevent circular loops.
pub struct ModuleResolver {
    pub active_resolutions: HashSet<PathBuf>,
}

impl ModuleResolver {
    pub fn new() -> Self {
        Self {
            active_resolutions: HashSet::new(),
        }
    }

    /// Resolves a module reference to a file path. Detects circular imports.
    pub fn resolve_module_path(
        &mut self,
        current_dir: &Path,
        module_name: &str,
    ) -> Result<PathBuf, String> {
        // Resolve absolute namespace shortcuts or relative files
        let path = if module_name.starts_with("std::") {
            PathBuf::from("libraries")
                .join(module_name.replace("::", "/"))
                .with_extension("aether")
        } else {
            current_dir
                .join(module_name.replace("::", "/"))
                .with_extension("aether")
        };

        if self.active_resolutions.contains(&path) {
            return Err(format!(
                "Circular Dependency Error: Module dependency loop detected for namespace path: {:?}",
                path
            ));
        }

        self.active_resolutions.insert(path.clone());
        Ok(path)
    }

    /// Removes path from active list once compilation completes.
    pub fn complete_resolution(&mut self, path: &Path) {
        self.active_resolutions.remove(path);
    }
}
