use aether::module::namespace::NamespaceManager;
use aether::types::inference::{ASTExpr, InferenceContext};
use aether::types::{Type, TypeEnv};

#[test]
fn test_integration_flow() {
    let mut ctx = InferenceContext::new();
    let mut env = TypeEnv::new();
    let mut namespace = NamespaceManager::new();

    // Map module method types
    namespace.register_alias("math::add", "std::math::add");
    env.insert(
        "std::math::add",
        Type::Function {
            params: vec![Type::Int, Type::Int],
            return_type: Box::new(Type::Int),
        },
    );

    let resolved_name = namespace.resolve_name("math::add");
    assert_eq!(resolved_name, "std::math::add");

    // Call math::add(10, 20)
    let call_expr = ASTExpr::Call(
        resolved_name,
        vec![ASTExpr::IntLiteral(10), ASTExpr::IntLiteral(20)],
    );

    let (inferred_type, _) = ctx.infer(&call_expr, &env).unwrap();
    assert_eq!(inferred_type, Type::Int);
}
