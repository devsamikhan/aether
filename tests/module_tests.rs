use aether::module::namespace::NamespaceManager;
use aether::module::resolver::ModuleResolver;
use std::path::Path;

#[test]
fn test_module_resolution() {
    let mut resolver = ModuleResolver::new();
    let current_dir = Path::new("tests");
    let res = resolver
        .resolve_module_path(current_dir, "test_helper")
        .unwrap();
    assert!(res.to_string_lossy().contains("test_helper.aether"));
}

#[test]
fn test_circular_dependency() {
    let mut resolver = ModuleResolver::new();
    let current_dir = Path::new("tests");

    let path = resolver
        .resolve_module_path(current_dir, "cycle_a")
        .unwrap();
    // Resolving again should error
    let err = resolver.resolve_module_path(current_dir, "cycle_a");
    assert!(err.is_err());

    // Complete cycle_a
    resolver.complete_resolution(&path);
    let ok = resolver.resolve_module_path(current_dir, "cycle_a");
    assert!(ok.is_ok());
}

#[test]
fn test_namespace_shadowing() {
    let mut manager = NamespaceManager::new();
    manager.register_alias("x", "std::math::x");
    assert_eq!(manager.resolve_name("x"), "std::math::x");

    // Shadowing in inner scope
    manager.enter_scope();
    manager.register_alias("x", "local::x");
    assert_eq!(manager.resolve_name("x"), "local::x");

    manager.exit_scope();
    assert_eq!(manager.resolve_name("x"), "std::math::x");
}
