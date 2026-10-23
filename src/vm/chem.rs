use super::value::Value;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

static NEXT_CHEM_ID: AtomicU64 = AtomicU64::new(1);
static CHEM_REGISTRY: OnceLock<Mutex<HashMap<u64, Arc<Molecule>>>> = OnceLock::new();

fn get_chem_registry() -> &'static Mutex<HashMap<u64, Arc<Molecule>>> {
    CHEM_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Clone, Debug)]
pub struct ChemAtom {
    pub symbol: String,
    pub is_aromatic: bool,
    pub formal_charge: i8,
    pub implicit_h: usize,
    pub neighbors: Vec<(usize, f32)>, // (neighbor_atom_idx, bond_order)
}

#[derive(Clone, Debug)]
pub struct Molecule {
    pub smiles: String,
    pub atoms: Vec<ChemAtom>,
}

impl Molecule {
    pub fn from_smiles(smiles: &str) -> Result<Self, String> {
        let clean = smiles.trim();
        if clean.is_empty() {
            return Err("ChemError: Empty SMILES string".into());
        }

        let mut atoms: Vec<ChemAtom> = Vec::new();
        let mut branch_stack: Vec<usize> = Vec::new();
        let mut rings: HashMap<u32, (usize, f32)> = HashMap::new();
        let mut last_atom_idx: Option<usize> = None;
        let mut pending_bond_order: Option<f32> = None;

        let chars: Vec<char> = clean.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let ch = chars[i];

            match ch {
                '(' => {
                    if let Some(last) = last_atom_idx {
                        branch_stack.push(last);
                    }
                    i += 1;
                }
                ')' => {
                    if let Some(prev) = branch_stack.pop() {
                        last_atom_idx = Some(prev);
                    }
                    i += 1;
                }
                '-' => {
                    pending_bond_order = Some(1.0);
                    i += 1;
                }
                '=' => {
                    pending_bond_order = Some(2.0);
                    i += 1;
                }
                '#' => {
                    pending_bond_order = Some(3.0);
                    i += 1;
                }
                ':' => {
                    pending_bond_order = Some(1.5);
                    i += 1;
                }
                '/' | '\\' => {
                    // Cis/Trans stereochemical bond markers in SMILES
                    i += 1;
                }
                '1'..='9' => {
                    let ring_num = ch.to_digit(10).unwrap();
                    let bond = pending_bond_order.take().unwrap_or(1.0);
                    if let Some(current_idx) = last_atom_idx {
                        if let Some((prev_idx, prev_bond)) = rings.remove(&ring_num) {
                            let final_bond = if bond != 1.0 { bond } else { prev_bond };
                            atoms[current_idx].neighbors.push((prev_idx, final_bond));
                            atoms[prev_idx].neighbors.push((current_idx, final_bond));
                        } else {
                            rings.insert(ring_num, (current_idx, bond));
                        }
                    }
                    i += 1;
                }
                '[' => {
                    // Bracket atom: e.g. [NH4+], [OH-], [C@@H]
                    let mut bracket_content = String::new();
                    i += 1;
                    while i < chars.len() && chars[i] != ']' {
                        bracket_content.push(chars[i]);
                        i += 1;
                    }
                    if i < chars.len() && chars[i] == ']' {
                        i += 1;
                    }

                    let (sym, aromatic, charge, exp_h) = parse_bracket_atom(&bracket_content)?;
                    let new_atom_idx = atoms.len();
                    atoms.push(ChemAtom {
                        symbol: sym,
                        is_aromatic: aromatic,
                        formal_charge: charge,
                        implicit_h: exp_h,
                        neighbors: Vec::new(),
                    });

                    if let Some(prev) = last_atom_idx {
                        let bond = pending_bond_order.take().unwrap_or(1.0);
                        atoms[prev].neighbors.push((new_atom_idx, bond));
                        atoms[new_atom_idx].neighbors.push((prev, bond));
                    }
                    last_atom_idx = Some(new_atom_idx);
                }
                _ => {
                    // Standard atom symbol
                    let (sym, aromatic, adv) = parse_atom_symbol(&chars[i..])?;
                    i += adv;

                    let new_atom_idx = atoms.len();
                    atoms.push(ChemAtom {
                        symbol: sym.clone(),
                        is_aromatic: aromatic,
                        formal_charge: 0,
                        implicit_h: 0,
                        neighbors: Vec::new(),
                    });

                    if let Some(prev) = last_atom_idx {
                        let default_order = if aromatic && atoms[prev].is_aromatic {
                            1.5
                        } else {
                            1.0
                        };
                        let bond = pending_bond_order.take().unwrap_or(default_order);
                        atoms[prev].neighbors.push((new_atom_idx, bond));
                        atoms[new_atom_idx].neighbors.push((prev, bond));
                    }
                    last_atom_idx = Some(new_atom_idx);
                }
            }
        }

        // Calculate implicit Hydrogens based on standard chemical valences
        for atom in &mut atoms {
            let bond_sum: f32 = atom.neighbors.iter().map(|(_, b)| *b).sum();
            let target_valence: f32 = match atom.symbol.as_str() {
                "C" => {
                    if atom.is_aromatic {
                        3.0
                    } else {
                        4.0
                    }
                }
                "N" => {
                    if atom.is_aromatic {
                        if atom.neighbors.len() == 2 {
                            2.0 // pyridine-like
                        } else {
                            3.0 // pyrrole-like
                        }
                    } else {
                        3.0
                    }
                }
                "O" => 2.0,
                "S" => {
                    if bond_sum > 4.0 {
                        6.0
                    } else if bond_sum > 2.0 {
                        4.0
                    } else {
                        2.0
                    }
                }
                "P" => {
                    if bond_sum > 3.0 {
                        5.0
                    } else {
                        3.0
                    }
                }
                "F" | "Cl" | "Br" | "I" => 1.0,
                "H" => 1.0,
                _ => 4.0,
            };

            let remaining = target_valence - bond_sum;
            atom.implicit_h = if remaining > 0.0 {
                remaining.round() as usize
            } else {
                0
            };
        }

        Ok(Self {
            smiles: clean.to_string(),
            atoms,
        })
    }

    pub fn molecular_weight(&self) -> f64 {
        let mut total_weight = 0.0;
        for atom in &self.atoms {
            let w = match atom.symbol.as_str() {
                "H" => 1.008,
                "C" => 12.011,
                "N" => 14.007,
                "O" => 15.999,
                "F" => 18.998,
                "P" => 30.974,
                "S" => 32.06,
                "Cl" => 35.45,
                "Br" => 79.904,
                "I" => 126.904,
                _ => 12.011,
            };
            total_weight += w + (atom.implicit_h as f64 * 1.008);
        }
        (total_weight * 1000.0).round() / 1000.0
    }

    pub fn formula(&self) -> String {
        let mut counts: HashMap<String, usize> = HashMap::new();
        let mut total_h = 0usize;

        for atom in &self.atoms {
            if atom.symbol == "H" {
                total_h += 1;
            } else {
                *counts.entry(atom.symbol.clone()).or_insert(0) += 1;
                total_h += atom.implicit_h;
            }
        }

        let mut res = String::new();
        // Hill system: C first, H second, then alphabetical
        if let Some(&c) = counts.get("C") {
            res.push('C');
            if c > 1 {
                res.push_str(&c.to_string());
            }
            counts.remove("C");
        }
        if total_h > 0 {
            res.push('H');
            if total_h > 1 {
                res.push_str(&total_h.to_string());
            }
        }

        let mut remaining_elems: Vec<String> = counts.keys().cloned().collect();
        remaining_elems.sort();
        for elem in remaining_elems {
            let cnt = counts[&elem];
            res.push_str(&elem);
            if cnt > 1 {
                res.push_str(&cnt.to_string());
            }
        }

        res
    }

    pub fn heavy_atom_count(&self) -> usize {
        self.atoms.iter().filter(|a| a.symbol != "H").count()
    }

    /// Hydrogen Bond Donors (HBD): count of OH, NH groups
    pub fn hbd(&self) -> usize {
        let mut count = 0;
        for atom in &self.atoms {
            if (atom.symbol == "O" || atom.symbol == "N") && atom.implicit_h > 0 {
                count += atom.implicit_h;
            }
        }
        count
    }

    /// Hydrogen Bond Acceptors (HBA): count of N and O atoms
    pub fn hba(&self) -> usize {
        self.atoms
            .iter()
            .filter(|a| a.symbol == "O" || a.symbol == "N")
            .count()
    }

    /// Rotatable Bonds count: single non-ring bonds between non-terminal heavy atoms
    pub fn rotatable_bonds(&self) -> usize {
        let mut count = 0;
        for (i, atom) in self.atoms.iter().enumerate() {
            if atom.symbol == "H" {
                continue;
            }
            for &(neighbor_idx, bond_order) in &atom.neighbors {
                if neighbor_idx > i && (bond_order - 1.0).abs() < 0.1 {
                    let neighbor = &self.atoms[neighbor_idx];
                    if neighbor.symbol != "H"
                        && atom.neighbors.len() > 1
                        && neighbor.neighbors.len() > 1
                    {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    /// Crippen LogP (Octanol-Water Partition Coefficient)
    pub fn logp(&self) -> f64 {
        let mut total = 0.0;
        for atom in &self.atoms {
            let base_contrib = match atom.symbol.as_str() {
                "C" => {
                    if atom.is_aromatic {
                        0.29
                    } else {
                        0.14
                    }
                }
                "N" => {
                    if atom.is_aromatic {
                        -0.45
                    } else if atom.implicit_h > 0 {
                        -0.67
                    } else {
                        -0.30
                    }
                }
                "O" => {
                    if atom.implicit_h > 0 {
                        -0.47 // -OH
                    } else {
                        -0.05 // -O- or =O
                    }
                }
                "F" => 0.31,
                "Cl" => 0.69,
                "Br" => 0.92,
                "I" => 1.25,
                "S" => 0.45,
                "P" => 0.20,
                _ => 0.0,
            };
            total += base_contrib + (atom.implicit_h as f64 * 0.11);
        }
        (total * 100.0).round() / 100.0
    }

    /// Topological Polar Surface Area (TPSA in Å²)
    pub fn tpsa(&self) -> f64 {
        let mut total: f64 = 0.0;
        for atom in &self.atoms {
            match atom.symbol.as_str() {
                "O" => {
                    if atom.implicit_h > 0 {
                        total += 20.23; // -OH
                    } else if atom.neighbors.iter().any(|(_, b)| *b >= 1.9) {
                        total += 17.07; // Carbonyl =O
                    } else {
                        total += 9.23; // Ether/Ester -O-
                    }
                }
                "N" => {
                    if atom.is_aromatic {
                        total += 12.89;
                    } else if atom.implicit_h >= 2 {
                        total += 26.02; // -NH2
                    } else if atom.implicit_h == 1 {
                        total += 12.03; // -NH-
                    } else {
                        total += 3.24; // >N-
                    }
                }
                _ => {}
            }
        }
        (total * 100.0).round() / 100.0
    }

    /// Lipinski's Rule of Five for oral drug-likeness
    pub fn rule_of_five(&self) -> HashMap<String, Value> {
        let mw = self.molecular_weight();
        let logp = self.logp();
        let hbd = self.hbd();
        let hba = self.hba();
        let rotb = self.rotatable_bonds();
        let tpsa = self.tpsa();

        let mw_pass = mw <= 500.0;
        let logp_pass = logp <= 5.0;
        let hbd_pass = hbd <= 5;
        let hba_pass = hba <= 10;
        let rotb_pass = rotb <= 10;
        let tpsa_pass = tpsa <= 140.0;

        let mut violations = 0;
        if !mw_pass {
            violations += 1;
        }
        if !logp_pass {
            violations += 1;
        }
        if !hbd_pass {
            violations += 1;
        }
        if !hba_pass {
            violations += 1;
        }

        let is_drug_like = violations <= 1; // Standard Lipinski allows 0 or 1 violation

        let mut res = HashMap::new();
        res.insert("is_drug_like".to_string(), Value::Bool(is_drug_like));
        res.insert("violations".to_string(), Value::Int(violations as i64));
        res.insert("molecular_weight".to_string(), Value::Float(mw));
        res.insert("logp".to_string(), Value::Float(logp));
        res.insert("hbd".to_string(), Value::Int(hbd as i64));
        res.insert("hba".to_string(), Value::Int(hba as i64));
        res.insert("rotatable_bonds".to_string(), Value::Int(rotb as i64));
        res.insert("tpsa".to_string(), Value::Float(tpsa));
        res.insert("mw_pass".to_string(), Value::Bool(mw_pass));
        res.insert("logp_pass".to_string(), Value::Bool(logp_pass));
        res.insert("hbd_pass".to_string(), Value::Bool(hbd_pass));
        res.insert("hba_pass".to_string(), Value::Bool(hba_pass));
        res.insert("rotb_pass".to_string(), Value::Bool(rotb_pass));
        res.insert("tpsa_pass".to_string(), Value::Bool(tpsa_pass));

        res
    }

    /// Comprehensive Safety, Toxicity Alerts & PAINS Screening
    pub fn safety_profile(&self) -> HashMap<String, Value> {
        let mut alerts = Vec::new();
        let logp = self.logp();
        let s = &self.smiles;

        // 1. Reactive warhead / Electrophilic Michael Acceptor (Covalent modifier alert)
        if s.contains("C(=O)C=C") || s.contains("C=CC(=O)") {
            alerts.push(Value::string("Reactive Electrophile (Michael Acceptor / Acrylamide)"));
        }

        // 2. Aromatic Nitro (Ames Mutagenicity Alert)
        if s.contains("N(=O)=O") || s.contains("[N+](=O)[O-]") {
            alerts.push(Value::string("Aromatic Nitro (Potential Mutagenicity / Carcinogenicity)"));
        }

        // 3. Aliphatic Alkyl Halide (DNA Alkylation Risk)
        let has_aliphatic_halide = self.atoms.iter().any(|a| {
            matches!(a.symbol.as_str(), "Cl" | "Br" | "I")
                && a.neighbors.iter().any(|(n_idx, _)| !self.atoms[*n_idx].is_aromatic && self.atoms[*n_idx].symbol == "C")
        });
        if has_aliphatic_halide {
            alerts.push(Value::string("Aliphatic Alkyl Halide (DNA Alkylating Potential)"));
        }

        // 4. Quinone / Catechol (PAINS Redox Cycling Alert)
        if s.contains("C1=CC(=O)C=CC1=O") || s.contains("Oc1c(O)cccc1") {
            alerts.push(Value::string("PAINS Alert: Quinone / Catechol (Redox Cycling & Protein Aggregation)"));
        }

        // 5. Rhodanine core (PAINS Assay Interferer)
        if s.contains("SC(=S)N") || s.contains("C=C1SC(=O)NC1=O") {
            alerts.push(Value::string("PAINS Alert: Rhodanine / Thiazolidinedione (Assay Interference)"));
        }

        // 6. hERG Cardiotoxicity Risk (High lipophilicity + basic aliphatic nitrogen)
        let has_basic_nitrogen = self.atoms.iter().any(|a| {
            a.symbol == "N" && !a.is_aromatic && a.neighbors.len() >= 2 && a.neighbors.iter().all(|(_, b)| *b <= 1.1)
        });
        let herg_risk = if logp > 3.8 && has_basic_nitrogen {
            "High (hERG Potassium Channel Block Risk)"
        } else if logp > 3.0 && has_basic_nitrogen {
            "Moderate (Monitor QT Interval)"
        } else {
            "Low"
        };

        // 7. Hepatotoxicity Risk
        let hep_risk = if logp > 4.5 && self.heavy_atom_count() > 35 {
            "High (Lipophilicity & High Mass)"
        } else if logp > 3.5 {
            "Moderate"
        } else {
            "Low"
        };

        let pains_free = !alerts.iter().any(|a| a.to_string().contains("PAINS"));
        let is_safe = alerts.is_empty() && (herg_risk == "Low" || herg_risk.starts_with("Moderate")) && (hep_risk == "Low" || hep_risk == "Moderate");

        let mut res = HashMap::new();
        res.insert("is_safe".to_string(), Value::Bool(is_safe));
        res.insert("pains_free".to_string(), Value::Bool(pains_free));
        res.insert("alert_count".to_string(), Value::Int(alerts.len() as i64));
        res.insert("alerts".to_string(), Value::array(alerts));
        res.insert("herg_cardiotoxicity_risk".to_string(), Value::string(herg_risk));
        res.insert("hepatotoxicity_risk".to_string(), Value::string(hep_risk));

        res
    }

    /// Extended Connectivity Fingerprint (ECFP/Morgan style)
    pub fn fingerprint(&self, n_bits: usize) -> Vec<f64> {
        let n_bits = if n_bits == 0 { 512 } else { n_bits };
        let mut bits = vec![0.0f64; n_bits];

        // Radius 0 hashes
        let atom_hashes: Vec<u64> = self
            .atoms
            .iter()
            .map(|a| {
                let mut h = 14695981039346656037u64; // FNV-1a
                for b in a.symbol.as_bytes() {
                    h ^= *b as u64;
                    h = h.wrapping_mul(1099511628211);
                }
                h ^= (a.implicit_h as u64).wrapping_add(1);
                h = h.wrapping_mul(1099511628211);
                h ^= if a.is_aromatic { 13 } else { 7 };
                h = h.wrapping_mul(1099511628211);
                h ^= (a.formal_charge as i64 as u64).wrapping_add(3);
                h
            })
            .collect();

        for &h in &atom_hashes {
            let bit_idx = (h % (n_bits as u64)) as usize;
            bits[bit_idx] = 1.0;
        }

        // Radius 1 hashes
        let mut radius1_hashes = Vec::with_capacity(self.atoms.len());
        for (i, atom) in self.atoms.iter().enumerate() {
            let mut nhash = atom_hashes[i];
            let mut neighbor_pairs = Vec::new();
            for &(n_idx, bond) in &atom.neighbors {
                neighbor_pairs.push((atom_hashes[n_idx], (bond * 10.0) as u64));
            }
            neighbor_pairs.sort();
            for (nh, b) in neighbor_pairs {
                nhash ^= nh;
                nhash = nhash.wrapping_mul(1099511628211);
                nhash ^= b;
                nhash = nhash.wrapping_mul(1099511628211);
            }
            radius1_hashes.push(nhash);
            let bit_idx = (nhash % (n_bits as u64)) as usize;
            bits[bit_idx] = 1.0;
        }

        // Radius 2 hashes
        for (i, atom) in self.atoms.iter().enumerate() {
            let mut nhash = radius1_hashes[i];
            for &(n_idx, _) in &atom.neighbors {
                nhash ^= radius1_hashes[n_idx];
                nhash = nhash.wrapping_mul(1099511628211);
            }
            let bit_idx = (nhash % (n_bits as u64)) as usize;
            bits[bit_idx] = 1.0;
        }

        bits
    }
}

fn parse_bracket_atom(content: &str) -> Result<(String, bool, i8, usize), String> {
    let mut symbol = String::new();
    let mut charge = 0i8;
    let mut aromatic = false;
    let mut explicit_h = 0usize;

    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;
    while i < chars.len() && chars[i].is_alphabetic() {
        if i == 0 && chars[i].is_ascii_lowercase() {
            aromatic = true;
            symbol.push(chars[i].to_ascii_uppercase());
        } else {
            symbol.push(chars[i]);
        }
        i += 1;
    }

    if symbol.is_empty() {
        return Err(format!("ChemError: Empty atom symbol in '[{}]'", content));
    }

    while i < chars.len() {
        if chars[i] == '+' {
            charge += 1;
            i += 1;
        } else if chars[i] == '-' {
            charge -= 1;
            i += 1;
        } else if chars[i] == 'H' {
            i += 1;
            if i < chars.len() && chars[i].is_digit(10) {
                explicit_h = chars[i].to_digit(10).unwrap_or(1) as usize;
                i += 1;
            } else {
                explicit_h = 1;
            }
        } else {
            // Skips '@' chiral indicators, isotopic numbers, etc.
            i += 1;
        }
    }

    Ok((symbol, aromatic, charge, explicit_h))
}

fn parse_atom_symbol(slice: &[char]) -> Result<(String, bool, usize), String> {
    if slice.is_empty() {
        return Err("ChemError: Unexpected end of SMILES string".into());
    }

    // Two-character element: Cl, Br
    if slice.len() >= 2 {
        if (slice[0] == 'C' && slice[1] == 'l') || (slice[0] == 'B' && slice[1] == 'r') {
            return Ok((format!("{}{}", slice[0], slice[1]), false, 2));
        }
    }

    let ch = slice[0];
    if ch.is_ascii_lowercase() && matches!(ch, 'c' | 'n' | 'o' | 's' | 'p') {
        return Ok((ch.to_ascii_uppercase().to_string(), true, 1));
    }

    if matches!(ch, 'C' | 'N' | 'O' | 'S' | 'P' | 'F' | 'I' | 'H' | 'B') {
        return Ok((ch.to_string(), false, 1));
    }

    Err(format!("ChemError: Unsupported atom token '{}' in SMILES", ch))
}

pub fn tanimoto_similarity(fp1: &[f64], fp2: &[f64]) -> f64 {
    if fp1.len() != fp2.len() || fp1.is_empty() {
        return 0.0;
    }
    let mut a_and_b = 0.0;
    let mut a_count = 0.0;
    let mut b_count = 0.0;

    for i in 0..fp1.len() {
        let b1 = fp1[i] > 0.5;
        let b2 = fp2[i] > 0.5;
        if b1 {
            a_count += 1.0;
        }
        if b2 {
            b_count += 1.0;
        }
        if b1 && b2 {
            a_and_b += 1.0;
        }
    }

    let denom: f64 = a_count + b_count - a_and_b;
    if denom <= 0.0 {
        1.0
    } else {
        ((a_and_b / denom) * 10000.0).round() / 10000.0
    }
}

pub fn wrap_molecule_handle(mol: Molecule) -> Value {
    let id = NEXT_CHEM_ID.fetch_add(1, Ordering::SeqCst);
    let smiles = mol.smiles.clone();
    let heavy_atoms = mol.heavy_atom_count();

    let mol_arc = Arc::new(mol);
    get_chem_registry().lock().insert(id, mol_arc);

    let mut map = HashMap::new();
    map.insert("_chem_mol".to_string(), Value::Int(id as i64));
    map.insert("_type".to_string(), Value::string("Chem.Molecule"));
    map.insert("smiles".to_string(), Value::string(smiles));
    map.insert("heavy_atoms".to_string(), Value::Int(heavy_atoms as i64));
    Value::map(map)
}

pub fn extract_molecule(val: &Value) -> Result<Arc<Molecule>, String> {
    match val {
        Value::Map(m) => {
            let map = m.lock();
            if let Some(Value::Int(id)) = map.get("_chem_mol") {
                let reg = get_chem_registry().lock();
                reg.get(&(*id as u64)).cloned().ok_or_else(|| {
                    format!("ChemError: Molecule handle #{} not found or deallocated", id)
                })
            } else {
                Err("ChemError: Expected a Chem.Molecule object".into())
            }
        }
        Value::String(s) => {
            let mol = Molecule::from_smiles(s)?;
            Ok(Arc::new(mol))
        }
        other => Err(format!(
            "ChemError: Expected Molecule object or SMILES string, got {}",
            other.type_name()
        )),
    }
}

pub fn extract_float_vector(val: &Value) -> Result<Vec<f64>, String> {
    match val {
        Value::Array(arr) => {
            let items = arr.lock();
            let mut vec = Vec::with_capacity(items.len());
            for item in items.iter() {
                match item {
                    Value::Int(i) => vec.push(*i as f64),
                    Value::Float(f) => vec.push(*f),
                    _ => return Err("ChemError: Vector elements must be numeric".into()),
                }
            }
            Ok(vec)
        }
        Value::Map(m) => {
            // Check if it's an AETHER Tensor handle
            let map = m.lock();
            if let Some(Value::Int(_id)) = map.get("_tensor_id") {
                // Read tensor data flat
                drop(map);
                let t_data = super::tensor::get_tensor_data(val)?;
                Ok(t_data.data)
            } else {
                Err("ChemError: Expected array or Tensor".into())
            }
        }
        other => Err(format!(
            "ChemError: Expected vector or Tensor, got {}",
            other.type_name()
        )),
    }
}

pub fn register_chem_module(globals: &mut HashMap<String, Value>) {
    let mut chem = HashMap::new();

    // Chem.molecule(smiles) / Chem.from_smiles(smiles)
    let make_mol = Value::Native("Chem.from_smiles".into(), |args| {
        if args.is_empty() {
            return Err("Chem.from_smiles(smiles_str) requires a SMILES string".into());
        }
        let s = format!("{}", args[0]);
        let mol = Molecule::from_smiles(&s)?;
        Ok(wrap_molecule_handle(mol))
    });
    chem.insert("from_smiles".to_string(), make_mol.clone());
    chem.insert("molecule".to_string(), make_mol);

    // Chem.formula(mol)
    chem.insert(
        "formula".to_string(),
        Value::Native("Chem.formula".into(), |args| {
            if args.is_empty() {
                return Err("Chem.formula(mol) requires 1 argument".into());
            }
            let mol = extract_molecule(&args[0])?;
            Ok(Value::string(mol.formula()))
        }),
    );

    // Chem.weight(mol) / Chem.molecular_weight(mol)
    let get_mw = Value::Native("Chem.weight".into(), |args| {
        if args.is_empty() {
            return Err("Chem.weight(mol) requires 1 argument".into());
        }
        let mol = extract_molecule(&args[0])?;
        Ok(Value::Float(mol.molecular_weight()))
    });
    chem.insert("weight".to_string(), get_mw.clone());
    chem.insert("molecular_weight".to_string(), get_mw);

    // Chem.heavy_atom_count(mol)
    chem.insert(
        "heavy_atom_count".to_string(),
        Value::Native("Chem.heavy_atom_count".into(), |args| {
            if args.is_empty() {
                return Err("Chem.heavy_atom_count(mol) requires 1 argument".into());
            }
            let mol = extract_molecule(&args[0])?;
            Ok(Value::Int(mol.heavy_atom_count() as i64))
        }),
    );

    // Chem.hbd(mol)
    chem.insert(
        "hbd".to_string(),
        Value::Native("Chem.hbd".into(), |args| {
            if args.is_empty() {
                return Err("Chem.hbd(mol) requires 1 argument".into());
            }
            let mol = extract_molecule(&args[0])?;
            Ok(Value::Int(mol.hbd() as i64))
        }),
    );

    // Chem.hba(mol)
    chem.insert(
        "hba".to_string(),
        Value::Native("Chem.hba".into(), |args| {
            if args.is_empty() {
                return Err("Chem.hba(mol) requires 1 argument".into());
            }
            let mol = extract_molecule(&args[0])?;
            Ok(Value::Int(mol.hba() as i64))
        }),
    );

    // Chem.rotatable_bonds(mol)
    chem.insert(
        "rotatable_bonds".to_string(),
        Value::Native("Chem.rotatable_bonds".into(), |args| {
            if args.is_empty() {
                return Err("Chem.rotatable_bonds(mol) requires 1 argument".into());
            }
            let mol = extract_molecule(&args[0])?;
            Ok(Value::Int(mol.rotatable_bonds() as i64))
        }),
    );

    // Chem.logp(mol)
    chem.insert(
        "logp".to_string(),
        Value::Native("Chem.logp".into(), |args| {
            if args.is_empty() {
                return Err("Chem.logp(mol) requires 1 argument".into());
            }
            let mol = extract_molecule(&args[0])?;
            Ok(Value::Float(mol.logp()))
        }),
    );

    // Chem.tpsa(mol)
    chem.insert(
        "tpsa".to_string(),
        Value::Native("Chem.tpsa".into(), |args| {
            if args.is_empty() {
                return Err("Chem.tpsa(mol) requires 1 argument".into());
            }
            let mol = extract_molecule(&args[0])?;
            Ok(Value::Float(mol.tpsa()))
        }),
    );

    // Chem.rule_of_five(mol)
    chem.insert(
        "rule_of_five".to_string(),
        Value::Native("Chem.rule_of_five".into(), |args| {
            if args.is_empty() {
                return Err("Chem.rule_of_five(mol) requires 1 argument".into());
            }
            let mol = extract_molecule(&args[0])?;
            Ok(Value::map(mol.rule_of_five()))
        }),
    );

    // Chem.safety_profile(mol)
    chem.insert(
        "safety_profile".to_string(),
        Value::Native("Chem.safety_profile".into(), |args| {
            if args.is_empty() {
                return Err("Chem.safety_profile(mol) requires 1 argument".into());
            }
            let mol = extract_molecule(&args[0])?;
            Ok(Value::map(mol.safety_profile()))
        }),
    );

    // Chem.fingerprint(mol, [n_bits=512])
    chem.insert(
        "fingerprint".to_string(),
        Value::Native("Chem.fingerprint".into(), |args| {
            if args.is_empty() {
                return Err("Chem.fingerprint(mol, [n_bits]) requires at least 1 argument".into());
            }
            let mol = extract_molecule(&args[0])?;
            let n_bits = if args.len() > 1 {
                match &args[1] {
                    Value::Int(n) => *n as usize,
                    _ => 512,
                }
            } else {
                512
            };
            let fp = mol.fingerprint(n_bits);
            let vals: Vec<Value> = fp.into_iter().map(Value::Float).collect();
            Ok(Value::array(vals))
        }),
    );

    // Chem.tanimoto_similarity(fp1, fp2)
    chem.insert(
        "tanimoto_similarity".to_string(),
        Value::Native("Chem.tanimoto_similarity".into(), |args| {
            if args.len() < 2 {
                return Err("Chem.tanimoto_similarity(fp1, fp2) requires 2 arguments".into());
            }
            let fp1 = extract_float_vector(&args[0])?;
            let fp2 = extract_float_vector(&args[1])?;
            let sim = tanimoto_similarity(&fp1, &fp2);
            Ok(Value::Float(sim))
        }),
    );

    // Chem.predict_binding_affinity(mol_or_fp, target_weights_or_tensor, [bias=5.5])
    // AI / Drug Screening target affinity prediction:
    // Calculates score = bias + dot_product(fp, weights)
    // pIC50 = score clamped to reasonable biological range [2.0, 12.0]
    // Kd (nM) = 10^(9 - pIC50)
    chem.insert(
        "predict_binding_affinity".to_string(),
        Value::Native("Chem.predict_binding_affinity".into(), |args| {
            if args.len() < 2 {
                return Err(
                    "Chem.predict_binding_affinity(mol_or_fp, target_weights, [bias]) requires at least 2 arguments"
                        .into(),
                );
            }

            let fp = match &args[0] {
                Value::Map(m) if m.lock().contains_key("_chem_mol") => {
                    let mol = extract_molecule(&args[0])?;
                    mol.fingerprint(512)
                }
                Value::String(s) => {
                    let mol = Molecule::from_smiles(s)?;
                    mol.fingerprint(512)
                }
                _ => extract_float_vector(&args[0])?,
            };

            let weights = extract_float_vector(&args[1])?;
            let bias = if args.len() > 2 {
                match &args[2] {
                    Value::Float(f) => *f,
                    Value::Int(i) => *i as f64,
                    _ => 5.5,
                }
            } else {
                5.5
            };

            let min_len = fp.len().min(weights.len());
            let mut dot = 0.0;
            for i in 0..min_len {
                dot += fp[i] * weights[i];
            }

            let raw_score = bias + dot;
            let pic50 = raw_score.max(2.0).min(12.0);
            let kd_nm = (10.0f64).powf(9.0 - pic50);

            let binding_class = if kd_nm < 100.0 {
                "High Affinity"
            } else if kd_nm <= 10000.0 {
                "Moderate Affinity"
            } else {
                "Low/No Affinity"
            };

            let mut res = HashMap::new();
            res.insert("raw_score".to_string(), Value::Float(raw_score));
            res.insert("pic50".to_string(), Value::Float((pic50 * 100.0).round() / 100.0));
            res.insert("kd_nm".to_string(), Value::Float((kd_nm * 100.0).round() / 100.0));
            res.insert("binding_class".to_string(), Value::string(binding_class));

            Ok(Value::map(res))
        }),
    );

    globals.insert("Chem".to_string(), Value::map(chem));
}
