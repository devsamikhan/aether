use super::Type;

/// Subtyping variance classifications.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Variance {
    Covariant,
    Contravariant,
    Invariant,
}

/// Evaluates if t1 is a subtype of t2 (t1 <: t2).
/// Follows standard variance logic (contravariant parameters, covariant return types).
pub fn is_subtype(t1: &Type, t2: &Type) -> bool {
    match (t1, t2) {
        (Type::Int, Type::Int) => true,
        (Type::Float, Type::Float) => true,
        (Type::Bool, Type::Bool) => true,
        (Type::String, Type::String) => true,
        (Type::Char, Type::Char) => true,
        (Type::Unit, Type::Unit) => true,
        (Type::Array(inner1, size1), Type::Array(inner2, size2)) => {
            size1 == size2 && is_subtype(inner1, inner2)
        }
        (Type::Vec(inner1), Type::Vec(inner2)) => is_subtype(inner1, inner2),
        (
            Type::Function {
                params: p1,
                return_type: r1,
            },
            Type::Function {
                params: p2,
                return_type: r2,
            },
        ) => {
            if p1.len() != p2.len() {
                return false;
            }
            // Contravariant parameters, covariant returns
            let params_match = p1
                .iter()
                .zip(p2.iter())
                .all(|(param1, param2)| is_subtype(param2, param1));
            params_match && is_subtype(r1, r2)
        }
        _ => t1 == t2,
    }
}
