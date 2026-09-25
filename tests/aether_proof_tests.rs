// ==============================================================================
// AetherProof Unit & Integration Test Suite
// Verifying Automated Formal Verification, Theorem Proving & SMT-Style Analysis
// Pure Rust Standard Library (Zero Third-Party Crates)
// ==============================================================================

use aether::syntax::parse;
use aether::vm::proof::SymbolicVerifier;
use aether::vm::run_source;

#[test]
fn test_proof_valid_intent_contract() {
    let src = r#"
intent safe_increment(x):
    require: x >= 0
    ensure: result > 0
    body:
        return x + 1
"#;

    let program = parse(src).expect("Parse failed");
    let mut verifier = SymbolicVerifier::new();
    let reports = verifier.verify_program(&program);

    assert_eq!(reports.len(), 1);
    assert!(reports[0].is_verified, "Contract should be mathematically proved");
    assert_eq!(reports[0].function_name, "safe_increment");
    assert!(reports[0].counter_example.is_none());
}

#[test]
fn test_proof_detected_contract_violation_with_counterexample() {
    let src = r#"
intent bad_decrement(x):
    require: x >= 0
    ensure: result > 0
    body:
        return x - 1
"#;

    let program = parse(src).expect("Parse failed");
    let mut verifier = SymbolicVerifier::new();
    let reports = verifier.verify_program(&program);

    assert_eq!(reports.len(), 1);
    assert!(!reports[0].is_verified, "Contract violation should be caught");
    assert!(reports[0].counter_example.is_some(), "Counter-example should be generated");
    let ce = reports[0].counter_example.as_ref().unwrap();
    assert_eq!(*ce.get("x").unwrap(), 0, "Counter-example input x=0 produces result=-1 violating > 0");
}

#[test]
fn test_proof_division_by_zero_vulnerability() {
    let src = r#"
def unsafe_calc(x):
    let bad = 100 / 0
    return bad
"#;

    let program = parse(src).expect("Parse failed");
    let mut verifier = SymbolicVerifier::new();
    let reports = verifier.verify_program(&program);

    assert_eq!(reports.len(), 1);
    assert!(!reports[0].vulnerabilities.is_empty(), "Should detect division by zero vulnerability");
    assert!(reports[0].vulnerabilities[0].contains("Division by Zero"));
}

#[test]
fn test_proof_aether_stdlib_integration() {
    let src = r#"
from aether_proof import Verifier

let code = "intent safe_scale(x):\n    require: x >= 1\n    ensure: result >= 2\n    body:\n        return x * 2\n"
let reports = Verifier.verify(code)

assert(len(reports) == 1, "1 intent function analyzed")
assert(reports[0].is_valid(), "safe_scale is mathematically verified")
assert(reports[0].function == "safe_scale", "Function name matches")
"#;

    let res = run_source(src);
    assert!(res.is_ok(), "Aether stdlib Proof test failed: {:?}", res.err());
}
