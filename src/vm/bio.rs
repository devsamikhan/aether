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

    globals.insert("Bio".to_string(), Value::map(bio));
}
