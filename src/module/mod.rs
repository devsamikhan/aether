pub mod import;
pub mod namespace;
pub mod resolver;

use std::path::PathBuf;

/// Visibility level of modules or items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
}

/// A structured module representation in AETHER compilation layout.
#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub path: PathBuf,
    pub visibility: Visibility,
    pub items: Vec<ModuleItem>,
}

#[derive(Debug, Clone)]
pub enum ModuleItem {
    Function(String),
    Intent(String),
    SubModule(Module),
}

impl Module {
    pub fn new(name: &str, path: PathBuf) -> Self {
        Self {
            name: name.to_string(),
            path,
            visibility: Visibility::Private,
            items: Vec::new(),
        }
    }
}
