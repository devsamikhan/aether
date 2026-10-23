use aether::vm::run_source;

#[test]
fn test_kras_g12d_real_data_workflow() {
    let source = r#"
# 1. Ingest real FASTA files
let wt_raw = File.read("data/kras_human_wt.fasta")
let pt_raw = File.read("data/kras_patient_g12d.fasta")

let wt_records = Bio.parse_fasta(wt_raw)
let pt_records = Bio.parse_fasta(pt_raw)

assert_eq(wt_records[0].sequence.len, 219)
assert_eq(pt_records[0].sequence.len, 219)

# 2. Pinpoint G12D single nucleotide mutation
let align = Bio.align(wt_records[0].sequence.to_string(), pt_records[0].sequence.to_string())
assert_eq(align.score, 435)
assert(align.identity_percent > 99.0)

let pt_prot = pt_records[0].sequence.translate(0)
assert_eq(pt_prot[11], "D") # 12th residue is Aspartate (G12D)

# 3. Read lightweight real ChEMBL bioactivity dataset
let json_str = File.read("data/kras_real_bioactivity.json")
let compounds = Json.parse(json_str)
assert_eq(compounds.len(), 9)

# 4. Verify MRTX1133 selective non-covalent inhibitor
let mrtx_smiles = compounds[0]["smiles"]
let mrtx_mol = Chem.from_smiles(mrtx_smiles)
assert_eq(mrtx_mol.formula(), "C33H31F3N6O2")
assert_eq(mrtx_mol.heavy_atoms, 44)

# 5. Verify Sotorasib covalent incompatibility on G12D
let soto_smiles = compounds[2]["smiles"]
let soto_mol = Chem.from_smiles(soto_smiles)
assert_eq(soto_mol.formula(), "C30H30F2N6O3")
assert(soto_smiles.contains("C(=O)C=C"))
"#;

    let res = run_source(source);
    assert!(res.is_ok(), "KRAS G12D real discovery test failed: {:?}", res.err());
}
