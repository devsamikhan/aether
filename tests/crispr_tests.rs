use aether::vm::run_source;

#[test]
fn test_crispr_target_scan_and_scoring() {
    let source = r#"
// Sequence containing target spacer (20nt) + NGG (CGG) PAM
let seq = Bio.dna("ATGACGTAGCTAGCTAGCTAACGGTA")
let targets = seq.crispr_targets("NGG", 20)

assert(targets.len() >= 1)
let t0 = targets[0]
assert_eq(t0.spacer.len(), 20)
assert_eq(t0.pam, "CGG")
assert_eq(t0.strand, "+")
assert(t0.gc_content >= 30.0)
assert(t0.on_target_score > 0.40)
assert_eq(t0.has_poly_t_terminator, false)
"#;
    let res = run_source(source);
    assert!(res.is_ok(), "Error: {:?}", res.err());
}

#[test]
fn test_crispr_poly_t_penalty_and_off_target() {
    let source = r#"
// Spacer with TTTT poly-T terminator tract
let bad_seq = Bio.dna("ATGTTTTAGCTAGCTAGCTAACGGTA")
let targets = Bio.crispr_find_targets(bad_seq, "NGG", 20)
assert(targets.len() >= 1)
assert_eq(targets[0].has_poly_t_terminator, true)
assert(targets[0].on_target_score < 0.60)

// Test off-target calculation:
let spacer = "ACGTAGCTAGCTAGCTAACG"
let genome = "ACGTAGCTAGCTAGCTAACGTGGATGACGTAGCTAGCTAACTAACGG"
let off_target_score = Bio.crispr_off_target_score(spacer, genome, "NGG")
assert(off_target_score > 0.0)
assert(off_target_score <= 1.0)
"#;
    let res = run_source(source);
    assert!(res.is_ok(), "Error: {:?}", res.err());
}
