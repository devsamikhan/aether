use aether::vm::run_source;

#[test]
fn test_pdb_parsing_and_metrics() {
    let source = r#"
let macromol = Bio.Structure("data/kras_g12d_pocket.pdb")
assert_eq(macromol.atom_count(), 53)
assert_eq(macromol.residue_count(), 10)
assert_eq(macromol.chains(), ["A"])

let centroid = macromol.centroid()
assert(centroid[0] > 10.0)
assert(centroid[1] > 20.0)

let rg = macromol.radius_of_gyration()
assert(rg > 5.0)

let plddt = macromol.plddt_summary()
assert(plddt.mean > 85.0)
assert(plddt.confident_percent > 90.0)
assert_eq(plddt.confidence_category, "Very High (AlphaFold Reliable)")
"#;
    let res = run_source(source);
    assert!(res.is_ok(), "Error: {:?}", res.err());
}

#[test]
fn test_pdb_ligand_pocket_and_rmsd() {
    let source = r#"
let struct1 = Bio.Structure("data/kras_g12d_pocket.pdb")
let struct2 = Bio.Structure("data/kras_g12d_pocket.pdb")

// Self RMSD must be exactly 0.0
let self_rmsd = struct1.rmsd(struct2)
assert_eq(self_rmsd, 0.0)

// Find protein residues interacting within 4.0 Angstroms of MRTX1133 (MRT) inhibitor
let pocket = struct1.ligand_pocket("MRT", 4.0)
assert(pocket.len() > 0)

// Asp12 should be one of the closest contact residues in the G12D pocket!
let has_asp12 = false
for res in pocket {
    if res.res_seq == 12 && res.res_name == "ASP" {
        has_asp12 = true
        assert(res.distance < 3.5)
    }
}
assert(has_asp12)
"#;
    let res = run_source(source);
    assert!(res.is_ok(), "Error: {:?}", res.err());
}
