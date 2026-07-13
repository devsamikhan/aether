pub mod checker;
pub mod inference;
pub mod subtyping;
pub mod unify;

use std::collections::HashMap;

/// Representation of AETHER types for static compilation validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    // Primitives
    Int,
    Float,
    Bool,
    String,
    Char,
    Unit,

    // Compound
    Array(Box<Type>, usize),
    Vec(Box<Type>),
    HashMap(Box<Type>, Box<Type>),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),

    // User-defined
    Intent(String),
    Struct(String),
    Enum(String),

    // Function
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },

    // Generic
    TypeVar(String),

    // Special
    Unknown,
    Error,
}

/// Type environment tracking active variable names to their inferred or declared types.
#[derive(Debug, Clone)]
pub struct TypeEnv {
    pub bindings: HashMap<String, Type>,
    pub parent: Option<Box<TypeEnv>>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            parent: None,
        }
    }

    pub fn child(parent: TypeEnv) -> Self {
        Self {
            bindings: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn insert(&mut self, name: &str, ty: Type) {
        self.bindings.insert(name.to_string(), ty);
    }

    pub fn lookup(&self, name: &str) -> Option<Type> {
        if let Some(ty) = self.bindings.get(name) {
            Some(ty.clone())
        } else if let Some(ref parent) = self.parent {
            parent.lookup(name)
        } else {
            None
        }
    }
}
