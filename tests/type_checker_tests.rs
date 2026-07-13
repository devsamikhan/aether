use aether::types::inference::{ASTExpr, InferenceContext};
use aether::types::subtyping::is_subtype;
use aether::types::unify::{apply_subst, unify};
use aether::types::{Type, TypeEnv};
use std::collections::HashMap;

#[test]
fn test_unify_primitives() {
    assert_eq!(unify(&Type::Int, &Type::Int), Ok(HashMap::new()));
    assert!(unify(&Type::Int, &Type::Bool).is_err());
}

#[test]
fn test_unify_type_vars() {
    let mut ctx = InferenceContext::new();
    let tv1 = ctx.next_type_var();

    let subst = unify(&tv1, &Type::Int).unwrap();
    assert_eq!(apply_subst(&tv1, &subst), Type::Int);

    // Occurs check (infinite recursive type)
    let err = unify(&tv1, &Type::Vec(Box::new(tv1.clone())));
    assert!(err.is_err());
}

#[test]
fn test_subtyping() {
    let f1 = Type::Function {
        params: vec![Type::Int],
        return_type: Box::new(Type::Int),
    };
    let f2 = Type::Function {
        params: vec![Type::Int],
        return_type: Box::new(Type::Int),
    };
    assert!(is_subtype(&f1, &f2));
}

#[test]
fn test_inference() {
    let mut ctx = InferenceContext::new();
    let mut env = TypeEnv::new();
    env.insert("x", Type::Int);

    let expr = ASTExpr::Binary(
        Box::new(ASTExpr::Var("x".to_string())),
        "+".to_string(),
        Box::new(ASTExpr::IntLiteral(10)),
    );
    let (ty, _) = ctx.infer(&expr, &env).unwrap();
    assert_eq!(ty, Type::Int);
}
