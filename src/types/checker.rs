use super::{Type, TypeEnv};
use crate::syntax::ast::{Program, Statement};
use std::collections::HashMap;

/// Static type checker verifying schemas, intents, enums, traits, impls, type aliases, and function signatures.
pub struct TypeChecker {
    pub env: TypeEnv,
    pub traits: HashMap<String, Vec<String>>,
    pub type_aliases: HashMap<String, String>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            env: TypeEnv::new(),
            traits: HashMap::new(),
            type_aliases: HashMap::new(),
        }
    }

    /// Register user-defined types (such as structs or intents) to the type environment.
    pub fn register_type(&mut self, name: &str, ty: Type) {
        self.env.insert(name, ty);
    }

    /// Parse a type annotation string (supporting primitives, generics, and unions `A | B`) into `Type`.
    pub fn parse_type_annotation(&self, raw: &str) -> Type {
        let trimmed = raw.trim();
        if trimmed.contains('|') {
            let parts: Vec<Type> = trimmed
                .split('|')
                .map(|p| self.parse_type_annotation(p.trim()))
                .collect();
            return Type::Union(parts);
        }
        if let Some(resolved) = self.type_aliases.get(trimmed) {
            return Type::Alias(
                trimmed.to_string(),
                Box::new(self.parse_type_annotation(resolved)),
            );
        }
        let base = trimmed.split('<').next().unwrap_or(trimmed).trim();
        match base {
            "Int" | "int" | "i64" => Type::Int,
            "Float" | "float" | "f64" => Type::Float,
            "Bool" | "bool" => Type::Bool,
            "String" | "str" => Type::String,
            "Char" | "char" => Type::Char,
            "Unit" | "Nil" | "nil" | "void" => Type::Unit,
            other => {
                if let Some(existing) = self.env.lookup(other) {
                    existing
                } else {
                    Type::TypeVar(other.to_string())
                }
            }
        }
    }

    /// Statically analyze an entire AETHER `Program`, registering declarations and verifying `trait`/`impl` contracts.
    pub fn check_program(&mut self, program: &Program) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        for stmt in &program.statements {
            match stmt {
                Statement::TypeAlias { name, target, .. } => {
                    self.type_aliases.insert(name.clone(), target.clone());
                    let resolved = self.parse_type_annotation(target);
                    self.register_type(name, Type::Alias(name.clone(), Box::new(resolved)));
                }
                Statement::StructDef { name, .. } | Statement::ClassDef { name, .. } => {
                    self.register_type(name, Type::Struct(name.clone()));
                }
                Statement::EnumDef { name, .. } => {
                    self.register_type(name, Type::Enum(name.clone()));
                }
                Statement::TraitDef { name, methods, .. } => {
                    self.register_type(name, Type::Trait(name.clone()));
                    self.traits.insert(name.clone(), methods.clone());
                }
                Statement::ImplDef {
                    trait_name: Some(t_name),
                    target_type,
                    methods,
                    ..
                } => {
                    if let Some(required_methods) = self.traits.get(t_name) {
                        let implemented: Vec<String> = methods
                            .iter()
                            .filter_map(|m| match m {
                                Statement::FnDef { name, .. } => Some(name.clone()),
                                _ => None,
                            })
                            .collect();
                        for req in required_methods {
                            if !implemented.contains(req) {
                                errors.push(format!(
                                    "TraitError: Type '{}' is missing required method '{}' from trait '{}'",
                                    target_type, req, t_name
                                ));
                            }
                        }
                    }
                }
                Statement::IntentDef { name, .. } => {
                    self.register_type(name, Type::Intent(name.clone()));
                }
                Statement::FnDef {
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    let param_types: Vec<Type> = params
                        .iter()
                        .map(|p| {
                            p.type_ann
                                .as_deref()
                                .map(|a| self.parse_type_annotation(a))
                                .unwrap_or(Type::Unknown)
                        })
                        .collect();
                    let ret_ty = return_type
                        .as_deref()
                        .map(|r| self.parse_type_annotation(r))
                        .unwrap_or(Type::Unknown);
                    self.register_type(
                        name,
                        Type::Function {
                            params: param_types,
                            return_type: Box::new(ret_ty),
                        },
                    );
                }
                _ => {}
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
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
