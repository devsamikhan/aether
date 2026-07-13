use super::Type;
use std::collections::HashMap;

pub type Substitution = HashMap<String, Type>;

/// Apply substitutions recursively to a Type structure.
pub fn apply_subst(ty: &Type, subst: &Substitution) -> Type {
    match ty {
        Type::TypeVar(name) => {
            if let Some(resolved) = subst.get(name) {
                apply_subst(resolved, subst)
            } else {
                ty.clone()
            }
        }
        Type::Array(inner, size) => Type::Array(Box::new(apply_subst(inner, subst)), *size),
        Type::Vec(inner) => Type::Vec(Box::new(apply_subst(inner, subst))),
        Type::HashMap(k, v) => Type::HashMap(
            Box::new(apply_subst(k, subst)),
            Box::new(apply_subst(v, subst)),
        ),
        Type::Option(inner) => Type::Option(Box::new(apply_subst(inner, subst))),
        Type::Result(ok, err) => Type::Result(
            Box::new(apply_subst(ok, subst)),
            Box::new(apply_subst(err, subst)),
        ),
        Type::Function {
            params,
            return_type,
        } => {
            let next_params = params.iter().map(|p| apply_subst(p, subst)).collect();
            Type::Function {
                params: next_params,
                return_type: Box::new(apply_subst(return_type, subst)),
            }
        }
        _ => ty.clone(),
    }
}

/// Helper occurs check to prevent infinite type loops.
pub fn occurs_check(var: &str, ty: &Type) -> bool {
    match ty {
        Type::TypeVar(name) => var == name,
        Type::Array(inner, _) => occurs_check(var, inner),
        Type::Vec(inner) => occurs_check(var, inner),
        Type::HashMap(k, v) => occurs_check(var, k) || occurs_check(var, v),
        Type::Option(inner) => occurs_check(var, inner),
        Type::Result(ok, err) => occurs_check(var, ok) || occurs_check(var, err),
        Type::Function {
            params,
            return_type,
        } => params.iter().any(|p| occurs_check(var, p)) || occurs_check(var, return_type),
        _ => false,
    }
}

/// Unifies two types, returning a Substitution map if successful.
pub fn unify(t1: &Type, t2: &Type) -> Result<Substitution, String> {
    match (t1, t2) {
        (Type::TypeVar(v1), Type::TypeVar(v2)) if v1 == v2 => Ok(HashMap::new()),
        (Type::TypeVar(var), other) | (other, Type::TypeVar(var)) => {
            if occurs_check(var, other) {
                return Err(format!(
                    "Occurs check failed: infinite type recursion detected for type variable '{}'",
                    var
                ));
            }
            let mut s = HashMap::new();
            s.insert(var.clone(), other.clone());
            Ok(s)
        }
        (Type::Int, Type::Int) => Ok(HashMap::new()),
        (Type::Float, Type::Float) => Ok(HashMap::new()),
        (Type::Bool, Type::Bool) => Ok(HashMap::new()),
        (Type::String, Type::String) => Ok(HashMap::new()),
        (Type::Char, Type::Char) => Ok(HashMap::new()),
        (Type::Unit, Type::Unit) => Ok(HashMap::new()),
        (Type::Array(i1, s1), Type::Array(i2, s2)) if s1 == s2 => unify(i1, i2),
        (Type::Vec(i1), Type::Vec(i2)) => unify(i1, i2),
        (Type::HashMap(k1, v1), Type::HashMap(k2, v2)) => {
            let mut s1 = unify(k1, k2)?;
            let s2 = unify(&apply_subst(v1, &s1), &apply_subst(v2, &s1))?;
            s1.extend(s2);
            Ok(s1)
        }
        (Type::Option(i1), Type::Option(i2)) => unify(i1, i2),
        (Type::Result(o1, e1), Type::Result(o2, e2)) => {
            let mut s1 = unify(o1, o2)?;
            let s2 = unify(&apply_subst(e1, &s1), &apply_subst(e2, &s1))?;
            s1.extend(s2);
            Ok(s1)
        }
        (Type::Intent(n1), Type::Intent(n2)) if n1 == n2 => Ok(HashMap::new()),
        (Type::Struct(n1), Type::Struct(n2)) if n1 == n2 => Ok(HashMap::new()),
        (Type::Enum(n1), Type::Enum(n2)) if n1 == n2 => Ok(HashMap::new()),
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
                return Err("Function parameter count mismatch".to_string());
            }
            let mut subst = HashMap::new();
            for (param1, param2) in p1.iter().zip(p2.iter()) {
                let s = unify(&apply_subst(param1, &subst), &apply_subst(param2, &subst))?;
                subst.extend(s);
            }
            let s_ret = unify(&apply_subst(r1, &subst), &apply_subst(r2, &subst))?;
            subst.extend(s_ret);
            Ok(subst)
        }
        _ => Err(format!(
            "Type mismatch: expected type {:?}, but found type {:?}",
            t1, t2
        )),
    }
}
