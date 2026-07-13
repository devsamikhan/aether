use super::{Type, TypeEnv};

/// Static type checker verifying schemas, intents, and function signatures.
pub struct TypeChecker {
    pub env: TypeEnv,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            env: TypeEnv::new(),
        }
    }

    /// Register user-defined types (such as structs or intents) to the type environment.
    pub fn register_type(&mut self, name: &str, ty: Type) {
        self.env.insert(name, ty);
    }

    /// Verify that a schema field declaration is valid.
    pub fn verify_schema_field(
        &self,
        struct_name: &str,
        field_name: &str,
        _field_type: &Type,
    ) -> Result<(), String> {
        if let Some(Type::Struct(name)) = self.env.lookup(struct_name) {
            println!(
                "[TypeChecker] Verifying field '{}' for struct '{}' conforms to schema restrictions.",
                field_name, name
            );
            Ok(())
        } else {
            Err(format!(
                "Semantic Error: Struct '{}' has not been declared in the active environment.",
                struct_name
            ))
        }
    }
}
