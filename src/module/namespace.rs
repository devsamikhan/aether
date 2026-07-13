use std::collections::HashMap;

/// Namespace scope stack for identifier resolving and shadowing.
pub struct NamespaceManager {
    pub scopes: Vec<HashMap<String, String>>,
}

impl NamespaceManager {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    /// Register a local alias or name mapping in the active scope.
    pub fn register_alias(&mut self, alias: &str, absolute_name: &str) {
        if let Some(current) = self.scopes.last_mut() {
            current.insert(alias.to_string(), absolute_name.to_string());
        }
    }

    /// Resolve local name to absolute namespace representation.
    pub fn resolve_name(&self, name: &str) -> String {
        for scope in self.scopes.iter().rev() {
            if let Some(absolute) = scope.get(name) {
                return absolute.clone();
            }
        }
        name.to_string()
    }
}
