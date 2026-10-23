use aether::vm::run_source;

#[test]
fn test_bio_dna_packing_and_properties() {
    let source = r#"
let seq = Bio.dna("ATGCGATC")
assert_eq(seq.len, 8)
assert_eq(seq.raw_bytes, 2)
assert_eq(seq.gc_content(), 50.0)
assert_eq(seq.to_string(), "ATGCGATC")
"#;
    let res = run_source(source);
    assert!(res.is_ok(), "Error: {:?}", res.err());
}

#[test]
fn test_bio_reverse_complement_and_transcribe() {
    let source = r#"
let seq = Bio.dna("ATGC")
let rev = seq.reverse_complement()
assert_eq(rev.to_string(), "GCAT")

let rna = seq.transcribe()
assert_eq(rna, "AUGC")
"#;
    let res = run_source(source);
    assert!(res.is_ok(), "Error: {:?}", res.err());
}

#[test]
fn test_bio_translation_and_kmers() {
    let source = r#"
# ATG=M (Met), GCC=A (Ala), ATT=I (Ile), TAA=* (Stop)
let seq = Bio.dna("ATGGCCATTTAA")
let protein = seq.translate(0)
assert_eq(protein, "MAI*")

let kmers = seq.kmers(3)
assert_eq(kmers.len(), 10)
assert_eq(kmers[0], "ATG")
assert_eq(kmers[1], "TGG")

let counts = seq.count_kmers(3)
assert_eq(counts["ATG"], 1)
"#;
    let res = run_source(source);
    assert!(res.is_ok(), "Error: {:?}", res.err());
}

#[test]
fn test_bio_hamming_and_alignment() {
    let source = r#"
let d1 = Bio.dna("GATTACA")
let d2 = Bio.dna("GACTATA")
let dist = d1.hamming_distance(d2)
assert_eq(dist, 2)

let align_res = Bio.align("GATTACA", "GCATGCU")
assert(align_res.score != 0)
assert(align_res.identity_percent > 0.0)
"#;
    let res = run_source(source);
    assert!(res.is_ok(), "Error: {:?}", res.err());
}

#[test]
fn test_bio_fasta_parser_and_orfs() {
    let source = r#"
let fasta = ">gene1 Human Beta Globin\nATGGTGCATCTGACTCCT\n>gene2 Synthetic\nATGAAATAA"
let records = Bio.parse_fasta(fasta)
assert_eq(records.len(), 2)
assert_eq(records[0].id, "gene1")
assert_eq(records[0].sequence.len, 18)
assert_eq(records[1].id, "gene2")

let orfs = records[1].sequence.find_orfs(1)
assert_eq(orfs.len(), 1)
assert_eq(orfs[0].protein, "MK")
"#;
    let res = run_source(source);
    assert!(res.is_ok(), "Error: {:?}", res.err());
}

#[test]
fn test_chem_aspirin_properties() {
    let source = r#"
# Aspirin: Acetylsalicylic acid
let mol = Chem.from_smiles("CC(=O)OC1=CC=CC=C1C(=O)O")
assert_eq(mol.formula(), "C9H8O4")
assert(mol.weight() > 180.0 and mol.weight() < 180.2)
assert_eq(mol.heavy_atom_count(), 13)
assert_eq(mol.hbd(), 1)
assert_eq(mol.hba(), 4)

let r5 = mol.rule_of_five()
assert_eq(r5.is_drug_like, true)
assert_eq(r5.violations, 0)
assert_eq(r5.mw_pass, true)
assert_eq(r5.logp_pass, true)
"#;
    let res = run_source(source);
    assert!(res.is_ok(), "Error: {:?}", res.err());
}

#[test]
fn test_chem_fingerprint_and_tanimoto() {
    let source = r#"
let aspirin = Chem.from_smiles("CC(=O)OC1=CC=CC=C1C(=O)O")
let salicylic = Chem.from_smiles("OC1=CC=CC=C1C(=O)O")
let caffeine = Chem.from_smiles("CN1C=NC2=C1C(=O)N(C(=O)N2C)C")

let fp_asp = aspirin.fingerprint(512)
let fp_sal = salicylic.fingerprint(512)
let fp_caf = caffeine.fingerprint(512)

# Self similarity must be 1.0
let self_sim = Chem.tanimoto_similarity(fp_asp, fp_asp)
assert_eq(self_sim, 1.0)

# Aspirin and Salicylic acid share core benzene + carboxyl scaffold
let sim_asp_sal = Chem.tanimoto_similarity(fp_asp, fp_sal)
assert(sim_asp_sal > 0.4)

# Aspirin and Caffeine are chemically very different
let sim_asp_caf = Chem.tanimoto_similarity(fp_asp, fp_caf)
assert(sim_asp_sal > sim_asp_caf)
"#;
    let res = run_source(source);
    assert!(res.is_ok(), "Error: {:?}", res.err());
}

#[test]
fn test_chem_ai_binding_affinity_prediction() {
    let source = r#"
let mol = Chem.from_smiles("CC(=O)OC1=CC=CC=C1C(=O)O")
let fp = mol.fingerprint(512)

# Synthetic target receptor weights (512 dimensions)
let mut target_weights = []
for i in 0..512 {
    if i % 10 == 0 {
        target_weights.push(0.35)
    } else {
        target_weights.push(0.01)
    }
}

let affinity = Chem.predict_binding_affinity(fp, target_weights, 5.0)
assert(affinity.pic50 >= 2.0 and affinity.pic50 <= 12.0)
assert(affinity.kd_nm > 0.0)
assert(affinity.binding_class == "High Affinity" or affinity.binding_class == "Moderate Affinity" or affinity.binding_class == "Low/No Affinity")
"#;
    let res = run_source(source);
    assert!(res.is_ok(), "Error: {:?}", res.err());
}

#[test]
fn test_chem_safety_profile_and_toxicity_alerts() {
    let source = r#"
# Sotorasib contains reactive acrylamide warhead C(=O)C=C
let sotorasib = Chem.from_smiles("CC1CN(CCN1C2=NC(=O)N(C3=NC(=C(C=C32)F)C4=C(C=CC=C4F)O)C5=C(C=CN=C5C(C)C)C)C(=O)C=C")
let safe_soto = sotorasib.safety_profile()
assert_eq(safe_soto.is_safe, false)
assert(safe_soto.alert_count > 0)

# Aspirin has no reactive warheads or toxicophores
let aspirin = Chem.from_smiles("CC(=O)OC1=CC=CC=C1C(=O)O")
let safe_asp = aspirin.safety_profile()
assert_eq(safe_asp.pains_free, true)
assert_eq(safe_asp.herg_cardiotoxicity_risk, "Low")
"#;
    let res = run_source(source);
    assert!(res.is_ok(), "Error: {:?}", res.err());
}

#[test]
fn test_bio_visual_coloring_and_alignment_rendering() {
    let source = r#"
let colored = Bio.colorize("ATGC")
assert(colored.len() > 4) # ANSI escape tags added

let view = Bio.render_alignment("ATGGTGCAT", "ATGGTGCAC")
assert(view.len() > 10)
assert(view.contains("Alignment"))
"#;
    let res = run_source(source);
    assert!(res.is_ok(), "Error: {:?}", res.err());
}
