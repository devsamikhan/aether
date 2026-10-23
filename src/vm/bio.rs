use super::value::Value;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

static NEXT_DNA_ID: AtomicU64 = AtomicU64::new(1);
static DNA_REGISTRY: OnceLock<Mutex<HashMap<u64, Arc<PackedDna>>>> = OnceLock::new();

fn get_dna_registry() -> &'static Mutex<HashMap<u64, Arc<PackedDna>>> {
    DNA_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

static NEXT_STRUCT_ID: AtomicU64 = AtomicU64::new(1);
static STRUCT_REGISTRY: OnceLock<Mutex<HashMap<u64, Arc<Macromolecule>>>> = OnceLock::new();

fn get_struct_registry() -> &'static Mutex<HashMap<u64, Arc<Macromolecule>>> {
    STRUCT_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Clone, Debug, PartialEq)]
pub struct PdbAtom {
    pub serial: usize,
    pub name: String,
    pub res_name: String,
    pub chain_id: String,
    pub res_seq: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub occupancy: f64,
    pub b_factor: f64, // pLDDT in AlphaFold models
    pub element: String,
    pub is_hetero: bool,
}

#[derive(Clone, Debug)]
pub struct Macromolecule {
    pub title: String,
    pub atoms: Vec<PdbAtom>,
}

impl Macromolecule {
    pub fn atom_count(&self) -> usize {
        self.atoms.len()
    }

    pub fn residue_count(&self) -> usize {
        let mut seen = std::collections::HashSet::new();
        for a in &self.atoms {
            if !a.is_hetero {
                seen.insert((a.chain_id.clone(), a.res_seq));
            }
        }
        seen.len()
    }

    pub fn chains(&self) -> Vec<String> {
        let mut chains = Vec::new();
        for a in &self.atoms {
            if !chains.contains(&a.chain_id) {
                chains.push(a.chain_id.clone());
            }
        }
        chains
    }

    pub fn centroid(&self) -> (f64, f64, f64) {
        if self.atoms.is_empty() {
            return (0.0, 0.0, 0.0);
        }
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut sz = 0.0;
        for a in &self.atoms {
            sx += a.x;
            sy += a.y;
            sz += a.z;
        }
        let n = self.atoms.len() as f64;
        (
            (sx / n * 100.0).round() / 100.0,
            (sy / n * 100.0).round() / 100.0,
            (sz / n * 100.0).round() / 100.0,
        )
    }

    pub fn radius_of_gyration(&self) -> f64 {
        if self.atoms.is_empty() {
            return 0.0;
        }
        let (cx, cy, cz) = self.centroid();
        let sum_sq: f64 = self
            .atoms
            .iter()
            .map(|a| (a.x - cx).powi(2) + (a.y - cy).powi(2) + (a.z - cz).powi(2))
            .sum();
        ((sum_sq / self.atoms.len() as f64).sqrt() * 100.0).round() / 100.0
    }

    pub fn plddt_summary(&self) -> HashMap<String, Value> {
        let ca_atoms: Vec<&PdbAtom> = self.atoms.iter().filter(|a| a.name == "CA").collect();
        let target_atoms = if !ca_atoms.is_empty() {
            ca_atoms
        } else {
            self.atoms.iter().collect()
        };

        if target_atoms.is_empty() {
            let mut map = HashMap::new();
            map.insert("mean".to_string(), Value::Float(0.0));
            map.insert("min".to_string(), Value::Float(0.0));
            map.insert("max".to_string(), Value::Float(0.0));
            map.insert("confident_percent".to_string(), Value::Float(0.0));
            map.insert("confidence_category".to_string(), Value::string("Unknown"));
            return map;
        }

        let mut min_val = f64::MAX;
        let mut max_val = f64::MIN;
        let mut sum_val = 0.0;
        let mut confident_count = 0usize;

        for a in &target_atoms {
            if a.b_factor < min_val {
                min_val = a.b_factor;
            }
            if a.b_factor > max_val {
                max_val = a.b_factor;
            }
            sum_val += a.b_factor;
            if a.b_factor >= 70.0 {
                confident_count += 1;
            }
        }

        let n = target_atoms.len() as f64;
        let mean = sum_val / n;
        let conf_pct = (confident_count as f64 / n) * 100.0;
        let category = if mean >= 90.0 {
            "Very High (AlphaFold Reliable)"
        } else if mean >= 70.0 {
            "Confident (AlphaFold Well-modeled)"
        } else if mean >= 50.0 {
            "Low Confidence"
        } else {
            "Very Low / Disordered"
        };

        let mut map = HashMap::new();
        map.insert("mean".to_string(), Value::Float((mean * 10.0).round() / 10.0));
        map.insert("min".to_string(), Value::Float((min_val * 10.0).round() / 10.0));
        map.insert("max".to_string(), Value::Float((max_val * 10.0).round() / 10.0));
        map.insert(
            "confident_percent".to_string(),
            Value::Float((conf_pct * 10.0).round() / 10.0),
        );
        map.insert("confidence_category".to_string(), Value::string(category));
        map
    }

    pub fn rmsd(&self, other: &Macromolecule) -> Result<f64, String> {
        let ca1: Vec<&PdbAtom> = self.atoms.iter().filter(|a| a.name == "CA").collect();
        let ca2: Vec<&PdbAtom> = other.atoms.iter().filter(|a| a.name == "CA").collect();
        let n = ca1.len().min(ca2.len());
        if n == 0 {
            return Err("BioError: No matching CA atoms found for RMSD calculation".to_string());
        }
        let mut sum_sq = 0.0;
        for i in 0..n {
            sum_sq += (ca1[i].x - ca2[i].x).powi(2)
                + (ca1[i].y - ca2[i].y).powi(2)
                + (ca1[i].z - ca2[i].z).powi(2);
        }
        Ok(((sum_sq / n as f64).sqrt() * 1000.0).round() / 1000.0)
    }

    pub fn binding_pocket(&self, cx: f64, cy: f64, cz: f64, radius: f64) -> Vec<Value> {
        let mut res_map: HashMap<(String, i32), (String, f64)> = HashMap::new();
        for a in &self.atoms {
            if !a.is_hetero {
                let dist = ((a.x - cx).powi(2) + (a.y - cy).powi(2) + (a.z - cz).powi(2)).sqrt();
                if dist <= radius {
                    let key = (a.chain_id.clone(), a.res_seq);
                    let entry = res_map.entry(key).or_insert((a.res_name.clone(), dist));
                    if dist < entry.1 {
                        entry.1 = dist;
                    }
                }
            }
        }
        let mut results = Vec::new();
        for ((chain, res_seq), (res_name, min_dist)) in res_map {
            let mut r = HashMap::new();
            r.insert("chain".to_string(), Value::string(chain));
            r.insert("res_seq".to_string(), Value::Int(res_seq as i64));
            r.insert("res_name".to_string(), Value::string(res_name));
            r.insert("distance".to_string(), Value::Float((min_dist * 100.0).round() / 100.0));
            results.push(Value::map(r));
        }
        results.sort_by(|a, b| {
            let d_a = match a {
                Value::Map(m) => match m.lock().get("distance") {
                    Some(Value::Float(f)) => *f,
                    _ => 0.0,
                },
                _ => 0.0,
            };
            let d_b = match b {
                Value::Map(m) => match m.lock().get("distance") {
                    Some(Value::Float(f)) => *f,
                    _ => 0.0,
                },
                _ => 0.0,
            };
            d_a.partial_cmp(&d_b).unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    pub fn ligand_pocket(&self, ligand_name: &str, radius: f64) -> Vec<Value> {
        let ligand_atoms: Vec<&PdbAtom> = self
            .atoms
            .iter()
            .filter(|a| a.is_hetero && a.res_name.eq_ignore_ascii_case(ligand_name))
            .collect();
        let ref_atoms: Vec<&PdbAtom> = if !ligand_atoms.is_empty() {
            ligand_atoms
        } else {
            self.atoms.iter().filter(|a| a.is_hetero).collect()
        };
        if ref_atoms.is_empty() {
            return Vec::new();
        }
        let mut res_map: HashMap<(String, i32), (String, f64)> = HashMap::new();
        for a in &self.atoms {
            if !a.is_hetero {
                let mut min_dist = f64::MAX;
                for la in &ref_atoms {
                    let d = ((a.x - la.x).powi(2) + (a.y - la.y).powi(2) + (a.z - la.z).powi(2)).sqrt();
                    if d < min_dist {
                        min_dist = d;
                    }
                }
                if min_dist <= radius {
                    let key = (a.chain_id.clone(), a.res_seq);
                    let entry = res_map.entry(key).or_insert((a.res_name.clone(), min_dist));
                    if min_dist < entry.1 {
                        entry.1 = min_dist;
                    }
                }
            }
        }
        let mut results = Vec::new();
        for ((chain, res_seq), (res_name, min_dist)) in res_map {
            let mut r = HashMap::new();
            r.insert("chain".to_string(), Value::string(chain));
            r.insert("res_seq".to_string(), Value::Int(res_seq as i64));
            r.insert("res_name".to_string(), Value::string(res_name));
            r.insert("distance".to_string(), Value::Float((min_dist * 100.0).round() / 100.0));
            results.push(Value::map(r));
        }
        results.sort_by(|a, b| {
            let d_a = match a {
                Value::Map(m) => match m.lock().get("distance") {
                    Some(Value::Float(f)) => *f,
                    _ => 0.0,
                },
                _ => 0.0,
            };
            let d_b = match b {
                Value::Map(m) => match m.lock().get("distance") {
                    Some(Value::Float(f)) => *f,
                    _ => 0.0,
                },
                _ => 0.0,
            };
            d_a.partial_cmp(&d_b).unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }
}

pub fn parse_pdb(content_or_path: &str) -> Result<Macromolecule, String> {
    let text = if (content_or_path.ends_with(".pdb") || content_or_path.ends_with(".cif"))
        && std::path::Path::new(content_or_path).exists()
    {
        std::fs::read_to_string(content_or_path)
            .map_err(|e| format!("BioError: Failed to read PDB file '{}': {}", content_or_path, e))?
    } else {
        content_or_path.to_string()
    };

    let mut atoms = Vec::new();
    let mut title = "Macromolecule".to_string();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("TITLE") {
            title = trimmed.trim_start_matches("TITLE").trim().to_string();
        } else if trimmed.starts_with("ATOM") || trimmed.starts_with("HETATM") {
            let is_hetero = trimmed.starts_with("HETATM");
            if line.len() >= 54 {
                let serial = line.get(6..11).unwrap_or("0").trim().parse::<usize>().unwrap_or(0);
                let name = line.get(12..16).unwrap_or("").trim().to_string();
                let res_name = line.get(17..20).unwrap_or("").trim().to_string();
                let chain_id = line.get(21..22).unwrap_or("A").trim().to_string();
                let res_seq = line.get(22..26).unwrap_or("0").trim().parse::<i32>().unwrap_or(0);
                let x = line.get(30..38).unwrap_or("0.0").trim().parse::<f64>().unwrap_or(0.0);
                let y = line.get(38..46).unwrap_or("0.0").trim().parse::<f64>().unwrap_or(0.0);
                let z = line.get(46..54).unwrap_or("0.0").trim().parse::<f64>().unwrap_or(0.0);
                let occupancy = if line.len() >= 60 {
                    line.get(54..60).unwrap_or("1.0").trim().parse::<f64>().unwrap_or(1.0)
                } else {
                    1.0
                };
                let b_factor = if line.len() >= 66 {
                    line.get(60..66).unwrap_or("0.0").trim().parse::<f64>().unwrap_or(0.0)
                } else {
                    0.0
                };
                let element = if line.len() >= 78 {
                    line.get(76..78).unwrap_or("").trim().to_string()
                } else if !name.is_empty() {
                    name.chars().next().unwrap_or('C').to_string()
                } else {
                    "C".to_string()
                };

                atoms.push(PdbAtom {
                    serial,
                    name,
                    res_name,
                    chain_id: if chain_id.is_empty() { "A".to_string() } else { chain_id },
                    res_seq,
                    x,
                    y,
                    z,
                    occupancy,
                    b_factor,
                    element,
                    is_hetero,
                });
            } else {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 8 {
                    let serial = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                    let name = parts.get(2).unwrap_or(&"CA").to_string();
                    let res_name = parts.get(3).unwrap_or(&"ALA").to_string();
                    let (chain_id, res_seq, x_idx) = if parts.len() >= 10 && parts[4].len() == 1 && parts[4].chars().all(|c| c.is_alphabetic()) {
                        (parts[4].to_string(), parts[5].parse::<i32>().unwrap_or(0), 6)
                    } else {
                        ("A".to_string(), parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0), 5)
                    };
                    let x = parts.get(x_idx).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    let y = parts.get(x_idx + 1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    let z = parts.get(x_idx + 2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    let b_factor = parts.get(x_idx + 4).and_then(|s| s.parse().ok()).unwrap_or(0.0);

                    atoms.push(PdbAtom {
                        serial,
                        name: name.clone(),
                        res_name,
                        chain_id,
                        res_seq,
                        x,
                        y,
                        z,
                        occupancy: 1.0,
                        b_factor,
                        element: name.chars().next().unwrap_or('C').to_string(),
                        is_hetero,
                    });
                }
            }
        }
    }

    if atoms.is_empty() {
        return Err("BioError: No ATOM or HETATM records found in PDB input".to_string());
    }

    Ok(Macromolecule { title, atoms })
}

pub fn wrap_structure_handle(macromol: Macromolecule) -> Value {
    let id = NEXT_STRUCT_ID.fetch_add(1, Ordering::SeqCst);
    let title = macromol.title.clone();
    let struct_arc = Arc::new(macromol);
    get_struct_registry().lock().insert(id, struct_arc);

    let mut map = HashMap::new();
    map.insert("_bio_struct".to_string(), Value::Int(id as i64));
    map.insert("_type".to_string(), Value::string("Bio.Structure"));
    map.insert("title".to_string(), Value::string(title));
    Value::map(map)
}

pub fn extract_structure(val: &Value) -> Result<Arc<Macromolecule>, String> {
    match val {
        Value::Map(m) => {
            let map = m.lock();
            if let Some(Value::Int(id)) = map.get("_bio_struct") {
                let reg = get_struct_registry().lock();
                reg.get(&(*id as u64))
                    .cloned()
                    .ok_or_else(|| format!("BioError: Structure handle #{} not found or deallocated", id))
            } else {
                Err("BioError: Expected a Bio.Structure object".into())
            }
        }
        Value::String(s) => {
            let macromol = parse_pdb(s)?;
            Ok(Arc::new(macromol))
        }
        other => Err(format!(
            "BioError: Expected Bio.Structure object or PDB string, got {}",
            other.type_name()
        )),
    }
}

// CRISPR Algorithms
pub fn iupac_match(pat_char: char, seq_char: char) -> bool {
    let p = pat_char.to_ascii_uppercase();
    let s = seq_char.to_ascii_uppercase();
    match p {
        'A' => s == 'A',
        'C' => s == 'C',
        'G' => s == 'G',
        'T' | 'U' => s == 'T' || s == 'U',
        'N' => s == 'A' || s == 'C' || s == 'G' || s == 'T' || s == 'U',
        'R' => s == 'A' || s == 'G',
        'Y' => s == 'C' || s == 'T' || s == 'U',
        'S' => s == 'G' || s == 'C',
        'W' => s == 'A' || s == 'T' || s == 'U',
        'K' => s == 'G' || s == 'T' || s == 'U',
        'M' => s == 'A' || s == 'C',
        'B' => s == 'C' || s == 'G' || s == 'T' || s == 'U',
        'D' => s == 'A' || s == 'G' || s == 'T' || s == 'U',
        'H' => s == 'A' || s == 'C' || s == 'T' || s == 'U',
        'V' => s == 'A' || s == 'C' || s == 'G',
        _ => false,
    }
}

pub fn calculate_gc_content(s: &str) -> f64 {
    if s.is_empty() { return 0.0; }
    let gc = s.chars().filter(|c| *c == 'G' || *c == 'C').count();
    (gc as f64 / s.len() as f64) * 100.0
}

pub fn calculate_on_target_score(spacer: &str) -> (f64, bool) {
    let mut score = 0.50f64;
    let has_poly_t = spacer.contains("TTTT");
    if has_poly_t {
        score -= 0.35;
    }
    let gc = calculate_gc_content(spacer);
    if (40.0..=60.0).contains(&gc) {
        score += 0.25;
    } else if (30.0..=75.0).contains(&gc) {
        score += 0.10;
    } else {
        score -= 0.20;
    }

    let chars: Vec<char> = spacer.chars().collect();
    if chars.len() >= 20 {
        if chars[19] == 'G' {
            score += 0.15;
        } else if chars[19] == 'T' {
            score -= 0.10;
        }
        if chars[15] == 'C' || chars[15] == 'G' {
            score += 0.05;
        }
    }

    let clamped = score.clamp(0.05, 0.99);
    ((clamped * 100.0).round() / 100.0, has_poly_t)
}

pub fn find_crispr_targets(dna: &str, pam_pattern: &str, target_len: usize) -> Vec<HashMap<String, Value>> {
    let clean = dna.trim().to_ascii_uppercase().replace('U', "T");
    let pam = pam_pattern.trim().to_ascii_uppercase().replace('U', "T");
    let pam_len = pam.len();
    let chars: Vec<char> = clean.chars().collect();
    let mut results = Vec::new();

    // 1. Forward strand: 5' - Spacer - PAM - 3'
    if chars.len() >= target_len + pam_len {
        for i in target_len..=(chars.len() - pam_len) {
            let mut matches_pam = true;
            for (p_idx, p_ch) in pam.chars().enumerate() {
                if !iupac_match(p_ch, chars[i + p_idx]) {
                    matches_pam = false;
                    break;
                }
            }
            if matches_pam {
                let spacer: String = chars[i - target_len..i].iter().collect();
                let pam_matched: String = chars[i..i + pam_len].iter().collect();
                let gc = calculate_gc_content(&spacer);
                let (score, has_poly_t) = calculate_on_target_score(&spacer);

                let mut target = HashMap::new();
                target.insert("spacer".to_string(), Value::string(spacer));
                target.insert("pam".to_string(), Value::string(pam_matched));
                target.insert("strand".to_string(), Value::string("+"));
                target.insert("position".to_string(), Value::Int((i - target_len) as i64));
                target.insert("gc_content".to_string(), Value::Float(gc));
                target.insert("on_target_score".to_string(), Value::Float(score));
                target.insert("has_poly_t_terminator".to_string(), Value::Bool(has_poly_t));
                results.push(target);
            }
        }
    }

    // 2. Reverse strand: reverse complement
    let rev_comp: String = clean
        .chars()
        .rev()
        .map(|c| match c {
            'A' => 'T',
            'T' => 'A',
            'C' => 'G',
            'G' => 'C',
            other => other,
        })
        .collect();
    let rev_chars: Vec<char> = rev_comp.chars().collect();

    if rev_chars.len() >= target_len + pam_len {
        for i in target_len..=(rev_chars.len() - pam_len) {
            let mut matches_pam = true;
            for (p_idx, p_ch) in pam.chars().enumerate() {
                if !iupac_match(p_ch, rev_chars[i + p_idx]) {
                    matches_pam = false;
                    break;
                }
            }
            if matches_pam {
                let spacer: String = rev_chars[i - target_len..i].iter().collect();
                let pam_matched: String = rev_chars[i..i + pam_len].iter().collect();
                let gc = calculate_gc_content(&spacer);
                let (score, has_poly_t) = calculate_on_target_score(&spacer);
                let orig_pos = chars.len() - (i + pam_len);

                let mut target = HashMap::new();
                target.insert("spacer".to_string(), Value::string(spacer));
                target.insert("pam".to_string(), Value::string(pam_matched));
                target.insert("strand".to_string(), Value::string("-"));
                target.insert("position".to_string(), Value::Int(orig_pos as i64));
                target.insert("gc_content".to_string(), Value::Float(gc));
                target.insert("on_target_score".to_string(), Value::Float(score));
                target.insert("has_poly_t_terminator".to_string(), Value::Bool(has_poly_t));
                results.push(target);
            }
        }
    }

    results
}

pub fn calculate_off_target_score(spacer: &str, genome: &str, pam_pattern: &str) -> f64 {
    let clean_genome = genome.to_ascii_uppercase().replace('U', "T");
    let sp = spacer.to_ascii_uppercase().replace('U', "T");
    let pam = pam_pattern.to_ascii_uppercase().replace('U', "T");
    let sp_len = sp.len();
    let pam_len = pam.len();
    let g_chars: Vec<char> = clean_genome.chars().collect();
    let sp_chars: Vec<char> = sp.chars().collect();

    if g_chars.len() < sp_len + pam_len {
        return 1.0;
    }

    let mut total_penalty = 0.0f64;

    for i in sp_len..=(g_chars.len() - pam_len) {
        let mut matches_pam = true;
        for (p_idx, p_ch) in pam.chars().enumerate() {
            if !iupac_match(p_ch, g_chars[i + p_idx]) {
                matches_pam = false;
                break;
            }
        }
        if !matches_pam {
            continue;
        }

        let cand_spacer = &g_chars[i - sp_len..i];
        let mut mismatches = 0;
        let mut mismatch_penalty = 1.0f64;

        for pos in 0..sp_len {
            if cand_spacer[pos] != sp_chars[pos] {
                mismatches += 1;
                if pos >= 12 {
                    mismatch_penalty *= 0.15;
                } else {
                    mismatch_penalty *= 0.75;
                }
            }
        }

        if mismatches > 0 && mismatches <= 4 {
            total_penalty += mismatch_penalty;
        }
    }

    let specificity = 1.0 / (1.0 + total_penalty);
    ((specificity * 100.0).round() / 100.0).clamp(0.01, 1.0)
}

/// 2-Bit Packed DNA sequence representation.
/// A = 0b00, C = 0b01, G = 0b10, T/U = 0b11.
/// Each byte holds 4 nucleotides: bits 6..7, 4..5, 2..3, 0..1.
#[derive(Clone, Debug, PartialEq)]
pub struct PackedDna {
    pub len: usize,
    pub bytes: Vec<u8>,
}

impl PackedDna {
    pub fn from_str(seq: &str) -> Result<Self, String> {
        let clean = seq.trim().to_ascii_uppercase();
        let len = clean.len();
        let num_bytes = (len + 3) / 4;
        let mut bytes = vec![0u8; num_bytes];

        for (i, ch) in clean.chars().enumerate() {
            let code = match ch {
                'A' => 0b00u8,
                'C' => 0b01u8,
                'G' => 0b10u8,
                'T' | 'U' => 0b11u8,
                other => return Err(format!("BioError: Invalid nucleotide '{}' at index {}. Allowed: A, C, G, T, U", other, i)),
            };
            let byte_idx = i / 4;
            let bit_shift = (3 - (i % 4)) * 2;
            bytes[byte_idx] |= code << bit_shift;
        }

        Ok(Self { len, bytes })
    }

    #[inline]
    pub fn get_code(&self, idx: usize) -> u8 {
        if idx >= self.len {
            return 0;
        }
        let byte_idx = idx / 4;
        let bit_shift = (3 - (idx % 4)) * 2;
        (self.bytes[byte_idx] >> bit_shift) & 0b11
    }

    #[inline]
    pub fn get_base(&self, idx: usize) -> char {
        match self.get_code(idx) {
            0b00 => 'A',
            0b01 => 'C',
            0b10 => 'G',
            _ => 'T',
        }
    }

    pub fn to_string(&self) -> String {
        let mut s = String::with_capacity(self.len);
        for i in 0..self.len {
            s.push(self.get_base(i));
        }
        s
    }

    pub fn gc_content(&self) -> f64 {
        if self.len == 0 {
            return 0.0;
        }
        let mut gc = 0usize;
        for i in 0..self.len {
            let code = self.get_code(i);
            if code == 0b01 || code == 0b10 {
                gc += 1;
            }
        }
        (gc as f64 / self.len as f64) * 100.0
    }

    /// Fast 2-bit reverse complement:
    /// In 2-bit encoding, complement is bitwise NOT:
    /// A (00) -> T (11), C (01) -> G (10), G (10) -> C (01), T (11) -> A (00).
    pub fn reverse_complement(&self) -> Self {
        let num_bytes = (self.len + 3) / 4;
        let mut bytes = vec![0u8; num_bytes];

        for i in 0..self.len {
            let orig_idx = self.len - 1 - i;
            let orig_code = self.get_code(orig_idx);
            let comp_code = (!orig_code) & 0b11;

            let byte_idx = i / 4;
            let bit_shift = (3 - (i % 4)) * 2;
            bytes[byte_idx] |= comp_code << bit_shift;
        }

        Self {
            len: self.len,
            bytes,
        }
    }

    pub fn transcribe(&self) -> String {
        let mut s = String::with_capacity(self.len);
        for i in 0..self.len {
            let b = self.get_base(i);
            if b == 'T' {
                s.push('U');
            } else {
                s.push(b);
            }
        }
        s
    }

    pub fn translate(&self, frame: usize) -> Result<String, String> {
        if frame > 2 {
            return Err("BioError: Frame must be 0, 1, or 2".into());
        }
        let mut protein = String::new();
        let mut i = frame;
        while i + 2 < self.len {
            let c0 = self.get_code(i);
            let c1 = self.get_code(i + 1);
            let c2 = self.get_code(i + 2);
            let codon_index = ((c0 as usize) << 4) | ((c1 as usize) << 2) | (c2 as usize);
            let aa = CODON_TABLE[codon_index];
            protein.push(aa);
            i += 3;
        }
        Ok(protein)
    }

    pub fn kmers(&self, k: usize) -> Result<Vec<String>, String> {
        if k == 0 || k > self.len {
            return Err(format!("BioError: Invalid k-mer size {} for sequence length {}", k, self.len));
        }
        let count = self.len - k + 1;
        let mut res = Vec::with_capacity(count);
        let s = self.to_string();
        for i in 0..count {
            res.push(s[i..i + k].to_string());
        }
        Ok(res)
    }

    pub fn count_kmers(&self, k: usize) -> Result<HashMap<String, usize>, String> {
        let kmers = self.kmers(k)?;
        let mut map = HashMap::new();
        for kmer in kmers {
            *map.entry(kmer).or_insert(0) += 1;
        }
        Ok(map)
    }

    pub fn hamming_distance(&self, other: &Self) -> Result<usize, String> {
        if self.len != other.len {
            return Err(format!(
                "BioError: Sequences must be equal length for Hamming distance (got {} and {})",
                self.len, other.len
            ));
        }
        let mut dist = 0;
        for i in 0..self.len {
            if self.get_code(i) != other.get_code(i) {
                dist += 1;
            }
        }
        Ok(dist)
    }

    pub fn find_orfs(&self, min_codons: usize) -> Vec<HashMap<String, Value>> {
        let mut orfs = Vec::new();
        for frame in 0..3 {
            let mut i = frame;
            let mut in_orf = false;
            let mut start_pos = 0;
            let mut current_protein = String::new();

            while i + 2 < self.len {
                let c0 = self.get_code(i);
                let c1 = self.get_code(i + 1);
                let c2 = self.get_code(i + 2);
                let codon_index = ((c0 as usize) << 4) | ((c1 as usize) << 2) | (c2 as usize);
                let aa = CODON_TABLE[codon_index];

                if !in_orf {
                    // Start codon ATG is: A=00, T=11, G=10 -> (0<<4) | (3<<2) | 2 = 14
                    if codon_index == 14 {
                        in_orf = true;
                        start_pos = i;
                        current_protein.clear();
                        current_protein.push('M');
                    }
                } else {
                    if aa == '*' {
                        // Stop codon
                        if current_protein.len() >= min_codons {
                            let mut record = HashMap::new();
                            record.insert("frame".to_string(), Value::Int(frame as i64));
                            record.insert("start".to_string(), Value::Int(start_pos as i64));
                            record.insert("end".to_string(), Value::Int((i + 3) as i64));
                            record.insert("codons".to_string(), Value::Int(current_protein.len() as i64));
                            record.insert("protein".to_string(), Value::string(current_protein.clone()));
                            orfs.push(record);
                        }
                        in_orf = false;
                    } else {
                        current_protein.push(aa);
                    }
                }
                i += 3;
            }
        }
        orfs
    }
}

// 64-element genetic code table: index = (c0 << 4) | (c1 << 2) | c2
// where A=0, C=1, G=2, T=3
const CODON_TABLE: [char; 64] = [
    // c0 = 0 (A)
    // c1=0 (A): AAA(K), AAC(N), AAG(K), AAT(N)
    'K', 'N', 'K', 'N',
    // c1=1 (C): ACA(T), ACC(T), ACG(T), ACT(T)
    'T', 'T', 'T', 'T',
    // c1=2 (G): AGA(R), AGC(S), AGG(R), AGT(S)
    'R', 'S', 'R', 'S',
    // c1=3 (T): ATA(I), ATC(I), ATG(M), ATT(I)
    'I', 'I', 'M', 'I',

    // c0 = 1 (C)
    // c1=0 (A): CAA(Q), CAC(H), CAG(Q), CAT(H)
    'Q', 'H', 'Q', 'H',
    // c1=1 (C): CCA(P), CCC(P), CCG(P), CCT(P)
    'P', 'P', 'P', 'P',
    // c1=2 (G): CGA(R), CGC(R), CGG(R), CGT(R)
    'R', 'R', 'R', 'R',
    // c1=3 (T): CTA(L), CTC(L), CTG(L), CTT(L)
    'L', 'L', 'L', 'L',

    // c0 = 2 (G)
    // c1=0 (A): GAA(E), GAC(D), GAG(E), GAT(D)
    'E', 'D', 'E', 'D',
    // c1=1 (C): GCA(A), GCC(A), GCG(A), GCT(A)
    'A', 'A', 'A', 'A',
    // c1=2 (G): GGA(G), GGC(G), GGG(G), GGT(G)
    'G', 'G', 'G', 'G',
    // c1=3 (T): GTA(V), GTC(V), GTG(V), GTT(V)
    'V', 'V', 'V', 'V',

    // c0 = 3 (T)
    // c1=0 (A): TAA(*), TAC(Y), TAG(*), TAT(Y)
    '*', 'Y', '*', 'Y',
    // c1=1 (C): TCA(S), TCC(S), TCG(S), TCT(S)
    'S', 'S', 'S', 'S',
    // c1=2 (G): TGA(*), TGC(C), TGG(W), TGT(C)
    '*', 'C', 'W', 'C',
    // c1=3 (T): TTA(L), TTC(F), TTG(L), TTT(F)
    'L', 'F', 'L', 'F',
];

/// Needleman-Wunsch Global Sequence Alignment
pub fn needleman_wunsch(
    seq1: &str,
    seq2: &str,
    match_score: i32,
    mismatch_pen: i32,
    gap_pen: i32,
) -> (i32, String, String, f64) {
    let s1: Vec<char> = seq1.to_ascii_uppercase().chars().collect();
    let s2: Vec<char> = seq2.to_ascii_uppercase().chars().collect();
    let n = s1.len();
    let m = s2.len();

    if n == 0 && m == 0 {
        return (0, String::new(), String::new(), 100.0);
    }
    if n == 0 {
        return (m as i32 * gap_pen, "-".repeat(m), seq2.to_string(), 0.0);
    }
    if m == 0 {
        return (n as i32 * gap_pen, seq1.to_string(), "-".repeat(n), 0.0);
    }

    let mut dp = vec![vec![0i32; m + 1]; n + 1];

    for i in 0..=n {
        dp[i][0] = i as i32 * gap_pen;
    }
    for j in 0..=m {
        dp[0][j] = j as i32 * gap_pen;
    }

    for i in 1..=n {
        for j in 1..=m {
            let score_sub = if s1[i - 1] == s2[j - 1] {
                match_score
            } else {
                mismatch_pen
            };
            let match_mismatch = dp[i - 1][j - 1] + score_sub;
            let delete = dp[i - 1][j] + gap_pen;
            let insert = dp[i][j - 1] + gap_pen;
            dp[i][j] = match_mismatch.max(delete).max(insert);
        }
    }

    // Traceback
    let mut aligned1 = Vec::new();
    let mut aligned2 = Vec::new();
    let mut i = n;
    let mut j = m;
    let mut matches = 0;

    while i > 0 || j > 0 {
        if i > 0 && j > 0 {
            let score_sub = if s1[i - 1] == s2[j - 1] {
                match_score
            } else {
                mismatch_pen
            };
            if dp[i][j] == dp[i - 1][j - 1] + score_sub {
                aligned1.push(s1[i - 1]);
                aligned2.push(s2[j - 1]);
                if s1[i - 1] == s2[j - 1] {
                    matches += 1;
                }
                i -= 1;
                j -= 1;
                continue;
            }
        }
        if i > 0 && dp[i][j] == dp[i - 1][j] + gap_pen {
            aligned1.push(s1[i - 1]);
            aligned2.push('-');
            i -= 1;
        } else {
            aligned1.push('-');
            aligned2.push(s2[j - 1]);
            j -= 1;
        }
    }

    aligned1.reverse();
    aligned2.reverse();
    let al1: String = aligned1.into_iter().collect();
    let al2: String = aligned2.into_iter().collect();
    let align_len = al1.len();
    let identity = if align_len > 0 {
        (matches as f64 / align_len as f64) * 100.0
    } else {
        0.0
    };

    (dp[n][m], al1, al2, identity)
}

pub fn wrap_dna_handle(dna: PackedDna) -> Value {
    let id = NEXT_DNA_ID.fetch_add(1, Ordering::SeqCst);
    let len = dna.len;
    let raw_bytes_len = dna.bytes.len();
    let dna_arc = Arc::new(dna);
    get_dna_registry().lock().insert(id, dna_arc);

    let mut map = HashMap::new();
    map.insert("_bio_dna".to_string(), Value::Int(id as i64));
    map.insert("_type".to_string(), Value::string("Bio.DNA"));
    map.insert("len".to_string(), Value::Int(len as i64));
    map.insert("raw_bytes".to_string(), Value::Int(raw_bytes_len as i64));
    map.insert(
        "compression_ratio".to_string(),
        Value::Float(if raw_bytes_len > 0 {
            len as f64 / raw_bytes_len as f64
        } else {
            1.0
        }),
    );
    Value::map(map)
}

pub fn extract_dna(val: &Value) -> Result<Arc<PackedDna>, String> {
    match val {
        Value::Map(m) => {
            let map = m.lock();
            if let Some(Value::Int(id)) = map.get("_bio_dna") {
                let reg = get_dna_registry().lock();
                reg.get(&(*id as u64))
                    .cloned()
                    .ok_or_else(|| format!("BioError: DNA handle #{} not found or deallocated", id))
            } else {
                Err("BioError: Expected a Bio.DNA object".into())
            }
        }
        Value::String(s) => {
            let dna = PackedDna::from_str(s)?;
            Ok(Arc::new(dna))
        }
        other => Err(format!(
            "BioError: Expected DNA object or string, got {}",
            other.type_name()
        )),
    }
}

pub fn register_bio_module(globals: &mut HashMap<String, Value>) {
    let mut bio = HashMap::new();

    // Bio.dna(str)
    bio.insert(
        "dna".to_string(),
        Value::Native("Bio.dna".into(), |args| {
            if args.is_empty() {
                return Err("Bio.dna(sequence_str) requires a sequence string".into());
            }
            let s = format!("{}", args[0]);
            let dna = PackedDna::from_str(&s)?;
            Ok(wrap_dna_handle(dna))
        }),
    );

    // Bio.gc_content(dna_or_str)
    bio.insert(
        "gc_content".to_string(),
        Value::Native("Bio.gc_content".into(), |args| {
            if args.is_empty() {
                return Err("Bio.gc_content(dna) requires 1 argument".into());
            }
            let dna = extract_dna(&args[0])?;
            Ok(Value::Float(dna.gc_content()))
        }),
    );

    // Bio.reverse_complement(dna_or_str)
    bio.insert(
        "reverse_complement".to_string(),
        Value::Native("Bio.reverse_complement".into(), |args| {
            if args.is_empty() {
                return Err("Bio.reverse_complement(dna) requires 1 argument".into());
            }
            let dna = extract_dna(&args[0])?;
            let rev = dna.reverse_complement();
            Ok(wrap_dna_handle(rev))
        }),
    );

    // Bio.transcribe(dna_or_str)
    bio.insert(
        "transcribe".to_string(),
        Value::Native("Bio.transcribe".into(), |args| {
            if args.is_empty() {
                return Err("Bio.transcribe(dna) requires 1 argument".into());
            }
            let dna = extract_dna(&args[0])?;
            Ok(Value::string(dna.transcribe()))
        }),
    );

    // Bio.translate(dna_or_str, [frame=0])
    bio.insert(
        "translate".to_string(),
        Value::Native("Bio.translate".into(), |args| {
            if args.is_empty() {
                return Err("Bio.translate(dna, [frame]) requires at least 1 argument".into());
            }
            let dna = extract_dna(&args[0])?;
            let frame = if args.len() > 1 {
                match &args[1] {
                    Value::Int(f) => *f as usize,
                    _ => 0,
                }
            } else {
                0
            };
            let prot = dna.translate(frame)?;
            Ok(Value::string(prot))
        }),
    );

    // Bio.kmers(dna_or_str, k)
    bio.insert(
        "kmers".to_string(),
        Value::Native("Bio.kmers".into(), |args| {
            if args.len() < 2 {
                return Err("Bio.kmers(dna, k) requires 2 arguments".into());
            }
            let dna = extract_dna(&args[0])?;
            let k = match &args[1] {
                Value::Int(n) => *n as usize,
                _ => return Err("Bio.kmers: k must be an integer".into()),
            };
            let kmers = dna.kmers(k)?;
            let vals: Vec<Value> = kmers.into_iter().map(Value::string).collect();
            Ok(Value::array(vals))
        }),
    );

    // Bio.count_kmers(dna_or_str, k)
    bio.insert(
        "count_kmers".to_string(),
        Value::Native("Bio.count_kmers".into(), |args| {
            if args.len() < 2 {
                return Err("Bio.count_kmers(dna, k) requires 2 arguments".into());
            }
            let dna = extract_dna(&args[0])?;
            let k = match &args[1] {
                Value::Int(n) => *n as usize,
                _ => return Err("Bio.count_kmers: k must be an integer".into()),
            };
            let map = dna.count_kmers(k)?;
            let mut res_map = HashMap::new();
            for (kmer, count) in map {
                res_map.insert(kmer, Value::Int(count as i64));
            }
            Ok(Value::map(res_map))
        }),
    );

    // Bio.hamming_distance(dna1, dna2)
    bio.insert(
        "hamming_distance".to_string(),
        Value::Native("Bio.hamming_distance".into(), |args| {
            if args.len() < 2 {
                return Err("Bio.hamming_distance(dna1, dna2) requires 2 arguments".into());
            }
            let d1 = extract_dna(&args[0])?;
            let d2 = extract_dna(&args[1])?;
            let dist = d1.hamming_distance(&d2)?;
            Ok(Value::Int(dist as i64))
        }),
    );

    // Bio.find_orfs(dna_or_str, [min_codons=30])
    bio.insert(
        "find_orfs".to_string(),
        Value::Native("Bio.find_orfs".into(), |args| {
            if args.is_empty() {
                return Err("Bio.find_orfs(dna, [min_codons]) requires at least 1 argument".into());
            }
            let dna = extract_dna(&args[0])?;
            let min_codons = if args.len() > 1 {
                match &args[1] {
                    Value::Int(m) => *m as usize,
                    _ => 10,
                }
            } else {
                10
            };
            let orfs = dna.find_orfs(min_codons);
            let vals: Vec<Value> = orfs.into_iter().map(Value::map).collect();
            Ok(Value::array(vals))
        }),
    );

    // Bio.align(seq1, seq2, [match=2, mismatch=-1, gap=-2])
    bio.insert(
        "align".to_string(),
        Value::Native("Bio.align".into(), |args| {
            if args.len() < 2 {
                return Err("Bio.align(seq1, seq2, [match, mismatch, gap]) requires at least 2 arguments".into());
            }
            let s1 = format!("{}", args[0]);
            let s2 = format!("{}", args[1]);
            let match_score = if args.len() > 2 {
                match &args[2] { Value::Int(s) => *s as i32, _ => 2 }
            } else { 2 };
            let mismatch_pen = if args.len() > 3 {
                match &args[3] { Value::Int(s) => *s as i32, _ => -1 }
            } else { -1 };
            let gap_pen = if args.len() > 4 {
                match &args[4] { Value::Int(s) => *s as i32, _ => -2 }
            } else { -2 };

            let (score, al1, al2, identity) = needleman_wunsch(&s1, &s2, match_score, mismatch_pen, gap_pen);
            let mut res = HashMap::new();
            res.insert("score".to_string(), Value::Int(score as i64));
            res.insert("aligned1".to_string(), Value::string(al1));
            res.insert("aligned2".to_string(), Value::string(al2));
            res.insert("identity_percent".to_string(), Value::Float(identity));
            Ok(Value::map(res))
        }),
    );

    // Bio.to_string / str
    bio.insert(
        "to_string".to_string(),
        Value::Native("Bio.to_string".into(), |args| {
            if args.is_empty() {
                return Err("Bio.to_string(dna) requires 1 argument".into());
            }
            let dna = extract_dna(&args[0])?;
            Ok(Value::string(dna.to_string()))
        }),
    );
    bio.insert(
        "str".to_string(),
        Value::Native("Bio.str".into(), |args| {
            if args.is_empty() {
                return Err("Bio.str(dna) requires 1 argument".into());
            }
            let dna = extract_dna(&args[0])?;
            Ok(Value::string(dna.to_string()))
        }),
    );

    // Bio.parse_fasta(fasta_text)
    bio.insert(
        "parse_fasta".to_string(),
        Value::Native("Bio.parse_fasta".into(), |args| {
            if args.is_empty() {
                return Err("Bio.parse_fasta(fasta_text) requires 1 argument".into());
            }
            let text = format!("{}", args[0]);
            let mut records = Vec::new();
            let mut current_id = String::new();
            let mut current_desc = String::new();
            let mut current_seq = String::new();

            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed.starts_with('>') {
                    if !current_id.is_empty() && !current_seq.is_empty() {
                        if let Ok(dna) = PackedDna::from_str(&current_seq) {
                            let mut rec = HashMap::new();
                            rec.insert("id".to_string(), Value::string(current_id.clone()));
                            rec.insert("description".to_string(), Value::string(current_desc.clone()));
                            rec.insert("sequence".to_string(), wrap_dna_handle(dna));
                            records.push(Value::map(rec));
                        }
                    }
                    let header = &trimmed[1..];
                    let mut parts = header.splitn(2, ' ');
                    current_id = parts.next().unwrap_or("").to_string();
                    current_desc = parts.next().unwrap_or("").to_string();
                    current_seq.clear();
                } else {
                    current_seq.push_str(trimmed);
                }
            }

            if !current_id.is_empty() && !current_seq.is_empty() {
                if let Ok(dna) = PackedDna::from_str(&current_seq) {
                    let mut rec = HashMap::new();
                    rec.insert("id".to_string(), Value::string(current_id));
                    rec.insert("description".to_string(), Value::string(current_desc));
                    rec.insert("sequence".to_string(), wrap_dna_handle(dna));
                    records.push(Value::map(rec));
                }
            }

            Ok(Value::array(records))
        }),
    );

    // Bio.colorize(dna_or_str)
    bio.insert(
        "colorize".to_string(),
        Value::Native("Bio.colorize".into(), |args| {
            if args.is_empty() {
                return Err("Bio.colorize(dna_or_str) requires 1 argument".into());
            }
            let s = match &args[0] {
                Value::Map(m) if m.lock().contains_key("_bio_dna") => {
                    let dna = extract_dna(&args[0])?;
                    dna.to_string()
                }
                other => format!("{}", other),
            };
            let mut out = String::with_capacity(s.len() * 12);
            for ch in s.chars() {
                match ch.to_ascii_uppercase() {
                    'A' => out.push_str("\x1b[1;32mA\x1b[0m"), // Green
                    'T' | 'U' => out.push_str("\x1b[1;31mT\x1b[0m"), // Red
                    'C' => out.push_str("\x1b[1;34mC\x1b[0m"), // Blue
                    'G' => out.push_str("\x1b[1;33mG\x1b[0m"), // Yellow
                    '-' => out.push_str("\x1b[90m-\x1b[0m"),   // Grey
                    other => out.push(other),
                }
            }
            Ok(Value::string(out))
        }),
    );

    // Bio.render_alignment(seq1, seq2, [match, mismatch, gap])
    bio.insert(
        "render_alignment".to_string(),
        Value::Native("Bio.render_alignment".into(), |args| {
            if args.len() < 2 {
                return Err("Bio.render_alignment(seq1, seq2) requires 2 arguments".into());
            }
            let s1 = format!("{}", args[0]);
            let s2 = format!("{}", args[1]);
            let match_score = if args.len() > 2 { match &args[2] { Value::Int(s) => *s as i32, _ => 2 } } else { 2 };
            let mismatch_pen = if args.len() > 3 { match &args[3] { Value::Int(s) => *s as i32, _ => -1 } } else { -1 };
            let gap_pen = if args.len() > 4 { match &args[4] { Value::Int(s) => *s as i32, _ => -2 } } else { -2 };

            let (score, al1, al2, identity) = needleman_wunsch(&s1, &s2, match_score, mismatch_pen, gap_pen);
            let mut connectors = String::with_capacity(al1.len());
            let c1: Vec<char> = al1.chars().collect();
            let c2: Vec<char> = al2.chars().collect();
            for i in 0..c1.len() {
                if c1[i] == '-' || c2[i] == '-' {
                    connectors.push(' ');
                } else if c1[i] == c2[i] {
                    connectors.push('|');
                } else {
                    connectors.push('*');
                }
            }
            let view = format!(
                "Alignment (Score: {}, Identity: {:.2}%)\nRef:    {}\nMatch:  {}\nQuery:  {}",
                score, identity, al1, connectors, al2
            );
            Ok(Value::string(view))
        }),
    );

    // Bio.Structure(path_or_text)
    bio.insert(
        "Structure".to_string(),
        Value::Native("Bio.Structure".into(), |args| {
            if args.is_empty() {
                return Err("Bio.Structure(path_or_text) requires 1 argument".into());
            }
            let s = format!("{}", args[0]);
            let macromol = parse_pdb(&s)?;
            Ok(wrap_structure_handle(macromol))
        }),
    );

    // Bio.parse_pdb(path_or_text)
    bio.insert(
        "parse_pdb".to_string(),
        Value::Native("Bio.parse_pdb".into(), |args| {
            if args.is_empty() {
                return Err("Bio.parse_pdb(path_or_text) requires 1 argument".into());
            }
            let s = format!("{}", args[0]);
            let macromol = parse_pdb(&s)?;
            Ok(wrap_structure_handle(macromol))
        }),
    );

    // Bio.atom_count(struct)
    bio.insert(
        "atom_count".to_string(),
        Value::Native("Bio.atom_count".into(), |args| {
            if args.is_empty() {
                return Err("atom_count requires a Structure".into());
            }
            let s = extract_structure(&args[0])?;
            Ok(Value::Int(s.atom_count() as i64))
        }),
    );

    // Bio.residue_count(struct)
    bio.insert(
        "residue_count".to_string(),
        Value::Native("Bio.residue_count".into(), |args| {
            if args.is_empty() {
                return Err("residue_count requires a Structure".into());
            }
            let s = extract_structure(&args[0])?;
            Ok(Value::Int(s.residue_count() as i64))
        }),
    );

    // Bio.chains(struct)
    bio.insert(
        "chains".to_string(),
        Value::Native("Bio.chains".into(), |args| {
            if args.is_empty() {
                return Err("chains requires a Structure".into());
            }
            let s = extract_structure(&args[0])?;
            let chains = s.chains();
            Ok(Value::array(chains.into_iter().map(Value::string).collect()))
        }),
    );

    // Bio.centroid(struct)
    bio.insert(
        "centroid".to_string(),
        Value::Native("Bio.centroid".into(), |args| {
            if args.is_empty() {
                return Err("centroid requires a Structure".into());
            }
            let s = extract_structure(&args[0])?;
            let (x, y, z) = s.centroid();
            Ok(Value::array(vec![
                Value::Float(x),
                Value::Float(y),
                Value::Float(z),
            ]))
        }),
    );

    // Bio.radius_of_gyration(struct)
    bio.insert(
        "radius_of_gyration".to_string(),
        Value::Native("Bio.radius_of_gyration".into(), |args| {
            if args.is_empty() {
                return Err("radius_of_gyration requires a Structure".into());
            }
            let s = extract_structure(&args[0])?;
            Ok(Value::Float(s.radius_of_gyration()))
        }),
    );

    // Bio.plddt_summary(struct)
    bio.insert(
        "plddt_summary".to_string(),
        Value::Native("Bio.plddt_summary".into(), |args| {
            if args.is_empty() {
                return Err("plddt_summary requires a Structure".into());
            }
            let s = extract_structure(&args[0])?;
            Ok(Value::map(s.plddt_summary()))
        }),
    );

    // Bio.rmsd(struct1, struct2)
    bio.insert(
        "rmsd".to_string(),
        Value::Native("Bio.rmsd".into(), |args| {
            if args.len() < 2 {
                return Err("rmsd(struct1, struct2) requires 2 Structures".into());
            }
            let s1 = extract_structure(&args[0])?;
            let s2 = extract_structure(&args[1])?;
            let val = s1.rmsd(&s2)?;
            Ok(Value::Float(val))
        }),
    );

    // Bio.binding_pocket(struct, cx, cy, cz, radius)
    bio.insert(
        "binding_pocket".to_string(),
        Value::Native("Bio.binding_pocket".into(), |args| {
            if args.len() < 5 {
                return Err("binding_pocket(struct, cx, cy, cz, radius) requires 5 arguments".into());
            }
            let s = extract_structure(&args[0])?;
            let cx = match &args[1] {
                Value::Float(f) => *f,
                Value::Int(i) => *i as f64,
                _ => 0.0,
            };
            let cy = match &args[2] {
                Value::Float(f) => *f,
                Value::Int(i) => *i as f64,
                _ => 0.0,
            };
            let cz = match &args[3] {
                Value::Float(f) => *f,
                Value::Int(i) => *i as f64,
                _ => 0.0,
            };
            let r = match &args[4] {
                Value::Float(f) => *f,
                Value::Int(i) => *i as f64,
                _ => 5.0,
            };
            let pocket = s.binding_pocket(cx, cy, cz, r);
            Ok(Value::array(pocket))
        }),
    );

    // Bio.ligand_pocket(struct, ligand_name, radius)
    bio.insert(
        "ligand_pocket".to_string(),
        Value::Native("Bio.ligand_pocket".into(), |args| {
            if args.len() < 2 {
                return Err(
                    "ligand_pocket(struct, ligand_name, [radius=5.0]) requires at least 2 arguments".into(),
                );
            }
            let s = extract_structure(&args[0])?;
            let ligand_name = format!("{}", args[1]);
            let r = if args.len() > 2 {
                match &args[2] {
                    Value::Float(f) => *f,
                    Value::Int(i) => *i as f64,
                    _ => 5.0,
                }
            } else {
                5.0
            };
            let pocket = s.ligand_pocket(&ligand_name, r);
            Ok(Value::array(pocket))
        }),
    );

    // Bio.crispr_find_targets(dna, [pam="NGG", target_len=20])
    bio.insert(
        "crispr_find_targets".to_string(),
        Value::Native("Bio.crispr_find_targets".into(), |args| {
            if args.is_empty() {
                return Err(
                    "Bio.crispr_find_targets(dna, [pam, target_len]) requires at least 1 argument".into(),
                );
            }
            let dna_seq = match &args[0] {
                Value::Map(m) if m.lock().contains_key("_bio_dna") => {
                    let d = extract_dna(&args[0])?;
                    d.to_string()
                }
                other => format!("{}", other),
            };
            let pam = if args.len() > 1 {
                format!("{}", args[1])
            } else {
                "NGG".to_string()
            };
            let target_len = if args.len() > 2 {
                match &args[2] {
                    Value::Int(i) => *i as usize,
                    _ => 20,
                }
            } else {
                20
            };
            let targets = find_crispr_targets(&dna_seq, &pam, target_len);
            Ok(Value::array(targets.into_iter().map(Value::map).collect()))
        }),
    );

    // Bio.crispr_targets(dna, [pam="NGG", target_len=20])
    bio.insert(
        "crispr_targets".to_string(),
        Value::Native("Bio.crispr_targets".into(), |args| {
            if args.is_empty() {
                return Err("crispr_targets(dna, [pam, target_len]) requires at least 1 argument".into());
            }
            let dna_seq = match &args[0] {
                Value::Map(m) if m.lock().contains_key("_bio_dna") => {
                    let d = extract_dna(&args[0])?;
                    d.to_string()
                }
                other => format!("{}", other),
            };
            let pam = if args.len() > 1 {
                format!("{}", args[1])
            } else {
                "NGG".to_string()
            };
            let target_len = if args.len() > 2 {
                match &args[2] {
                    Value::Int(i) => *i as usize,
                    _ => 20,
                }
            } else {
                20
            };
            let targets = find_crispr_targets(&dna_seq, &pam, target_len);
            Ok(Value::array(targets.into_iter().map(Value::map).collect()))
        }),
    );

    // Bio.crispr_off_target_score(spacer, genomic_dna, [pam="NGG"])
    bio.insert(
        "crispr_off_target_score".to_string(),
        Value::Native("Bio.crispr_off_target_score".into(), |args| {
            if args.len() < 2 {
                return Err(
                    "Bio.crispr_off_target_score(spacer, genomic_dna, [pam]) requires 2 arguments".into(),
                );
            }
            let spacer = format!("{}", args[0]);
            let genome = match &args[1] {
                Value::Map(m) if m.lock().contains_key("_bio_dna") => {
                    let d = extract_dna(&args[1])?;
                    d.to_string()
                }
                other => format!("{}", other),
            };
            let pam = if args.len() > 2 {
                format!("{}", args[2])
            } else {
                "NGG".to_string()
            };
            let score = calculate_off_target_score(&spacer, &genome, &pam);
            Ok(Value::Float(score))
        }),
    );

    globals.insert("Bio".to_string(), Value::map(bio));
}
