use aether::vm::run_source;

#[test]
fn test_end_to_end_precision_medicine_pipeline() {
    let source = r#"
# 1. Genomic Diagnostic: Reference vs Patient Biopsy
let wt_dna = Bio.dna("AAGATCACAGATTTTGGGCTGGCCAAACTGCTGGGTGCG")
let patient_dna = Bio.dna("AAGATCACAGATTTTGGGCTGGCCAAACGGCTGGGTGCG")

assert_eq(wt_dna.len, 39)
assert_eq(patient_dna.raw_bytes, 10)
assert_eq(wt_dna.hamming_distance(patient_dna), 1)

let align = Bio.align(wt_dna.to_string(), patient_dna.to_string())
assert(align.identity_percent > 95.0)

# 2. Peptide Translation & Mutation Signature
let wt_prot = wt_dna.translate(0)
let pt_prot = patient_dna.translate(0)
assert_eq(wt_prot, "KITDFGLAKLLGA")
assert_eq(pt_prot, "KITDFGLAKRLGA")

# 3. Pharmacological Screening Library
let osimertinib = Chem.from_smiles("CN(C)CC=CC(=O)NC1=CC(=C(C=C1NC2=NC=CC(=N2)C3=CN(C4=CC=CC=C34)C)OC)N(C)CCN(C)C")
let gefitinib = Chem.from_smiles("COC1=C(OCC2CCNCC2)C=C3C(=C1)N=CN=C3NC4=CC(=C(C=C4)F)Cl")
let caffeine = Chem.from_smiles("CN1C=NC2=C1C(=O)N(C(=O)N2C)C")

assert_eq(gefitinib.formula(), "C21H22ClFN4O2")
assert_eq(gefitinib.rule_of_five().is_drug_like, true)

# 4. Pocket Affinity Simulation
let mut pocket_weights = []
for i in 0..512 {
    if i % 16 == 0 {
        pocket_weights.push(0.55)
    } else {
        pocket_weights.push(0.01)
    }
}

let aff_osimer = Chem.predict_binding_affinity(osimertinib, pocket_weights, 4.5)
let aff_gefit = Chem.predict_binding_affinity(gefitinib, pocket_weights, 4.5)
let aff_caf = Chem.predict_binding_affinity(caffeine, pocket_weights, 4.5)

# Osimertinib and Gefitinib must show significantly higher affinity than caffeine
assert(aff_osimer.pic50 > aff_caf.pic50)
assert(aff_gefit.pic50 > aff_caf.pic50)
assert(aff_osimer.kd_nm < 100.0) # High potency
"#;

    let res = run_source(source);
    assert!(res.is_ok(), "Precision medicine pipeline failed: {:?}", res.err());
}
