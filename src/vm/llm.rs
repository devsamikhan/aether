// ==============================================================================
// AetherLLM — Pure-Rust BPE Tokenizer, GGUF v2/v3 Quantized Model Engine &
// Local Transformer / Llama-Architecture Inference + Dense Embeddings
// Zero Python / Zero llama-cpp / Zero HuggingFace Runtime Dependencies
// ==============================================================================

use super::nn::load_safetensors_file;
use super::tensor::{get_tensor_data, wrap_tensor_handle, TensorData};
use super::value::Value;
use parking_lot::Mutex;
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

// ==============================================================================
// 1. Half-Precision (IEEE 754 F16) & BF16 Conversions
// ==============================================================================

#[inline]
pub(crate) fn f16_to_f32(bits: u16) -> f32 {
    let sign = ((bits >> 15) & 1) as u32;
    let exp = ((bits >> 10) & 0x1F) as u32;
    let frac = (bits & 0x03FF) as u32;

    if exp == 0 {
        if frac == 0 {
            f32::from_bits(sign << 31)
        } else {
            // Subnormal
            let mut f = frac;
            let mut e: i32 = -14;
            while (f & 0x0400) == 0 {
                f <<= 1;
                e -= 1;
            }
            f &= 0x03FF;
            let f32_exp = (e + 127) as u32;
            f32::from_bits((sign << 31) | (f32_exp << 23) | (f << 13))
        }
    } else if exp == 31 {
        if frac == 0 {
            f32::from_bits((sign << 31) | (0xFF << 23))
        } else {
            f32::NAN
        }
    } else {
        let f32_exp = exp + 112;
        f32::from_bits((sign << 31) | (f32_exp << 23) | (frac << 13))
    }
}

#[inline]
pub(crate) fn f32_to_f16(val: f32) -> u16 {
    let bits = val.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exp = ((bits >> 23) & 0xFF) as i32;
    let frac = bits & 0x007F_FFFF;

    if exp == 255 {
        if frac == 0 {
            return sign | 0x7C00; // Inf
        }
        return sign | 0x7E00; // NaN
    }

    let new_exp = exp - 127 + 15;
    if new_exp >= 31 {
        return sign | 0x7C00; // Overflow -> Inf
    }
    if new_exp <= 0 {
        if new_exp < -10 {
            return sign; // Underflow -> 0
        }
        let mant = frac | 0x0080_0000;
        let shift = (14 - new_exp) as u32;
        let half_frac = (mant >> shift) as u16;
        return sign | half_frac;
    }

    let rounded = (frac + 0x0000_1000) >> 13;
    if rounded >= 0x0400 {
        let next_exp = (new_exp + 1) as u16;
        if next_exp >= 31 {
            sign | 0x7C00
        } else {
            sign | (next_exp << 10)
        }
    } else {
        sign | ((new_exp as u16) << 10) | (rounded as u16)
    }
}

#[inline]
pub(crate) fn bf16_to_f32(bits: u16) -> f32 {
    f32::from_bits((bits as u32) << 16)
}

#[inline]
pub(crate) fn f32_to_bf16(val: f32) -> u16 {
    let bits = val.to_bits();
    let round = bits.wrapping_add(0x7FFF + ((bits >> 16) & 1));
    (round >> 16) as u16
}

// ==============================================================================
// 2. GPT-2 Reversible Byte-to-Unicode Mapping & Pure-Rust BPE Tokenizer
// ==============================================================================

fn build_byte_unicode_tables() -> ([char; 256], HashMap<char, u8>) {
    let mut b2u = ['\0'; 256];
    let mut u2b = HashMap::with_capacity(256);
    let mut present = [false; 256];

    for b in 33u8..=126 {
        present[b as usize] = true;
        let c = b as char;
        b2u[b as usize] = c;
        u2b.insert(c, b);
    }
    for b in 161u8..=172 {
        present[b as usize] = true;
        let c = char::from(b);
        b2u[b as usize] = c;
        u2b.insert(c, b);
    }
    for b in 174u8..=255 {
        present[b as usize] = true;
        let c = char::from(b);
        b2u[b as usize] = c;
        u2b.insert(c, b);
    }

    let mut n = 0u32;
    for b in 0usize..256 {
        if !present[b] {
            let c = char::from_u32(256 + n).unwrap_or('?');
            b2u[b] = c;
            u2b.insert(c, b as u8);
            n += 1;
        }
    }
    (b2u, u2b)
}

fn byte_tables() -> &'static ([char; 256], HashMap<char, u8>) {
    static TABLES: OnceLock<([char; 256], HashMap<char, u8>)> = OnceLock::new();
    TABLES.get_or_init(build_byte_unicode_tables)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenizerKind {
    ByteLevel,
    SentencePiece,
}

#[derive(Clone, Debug)]
pub struct BpeTokenizer {
    pub kind: TokenizerKind,
    pub id_to_token: Vec<String>,
    pub token_to_id: HashMap<String, u32>,
    pub merge_ranks: HashMap<(String, String), usize>,
    pub merges: Vec<(String, String)>,
    pub scores: Vec<f32>,
    pub special_tokens: HashMap<String, u32>,
    pub bos_id: Option<u32>,
    pub eos_id: Option<u32>,
    pub unk_id: Option<u32>,
}

impl BpeTokenizer {
    /// Trains a byte-level BPE tokenizer directly from raw text samples.
    pub fn train(texts: &[String], target_vocab_size: usize, special: &[String]) -> Self {
        let (b2u, _) = byte_tables();
        let mut id_to_token = Vec::with_capacity(target_vocab_size.max(256 + special.len()));
        let mut token_to_id = HashMap::new();

        // 1. Base 256 byte alphabet
        for &c in b2u.iter() {
            let tok = c.to_string();
            let id = id_to_token.len() as u32;
            token_to_id.insert(tok.clone(), id);
            id_to_token.push(tok);
        }

        // 2. Pre-tokenize corpus into whitespace-preserving chunks
        let mut word_counts: HashMap<Vec<u32>, usize> = HashMap::new();
        for text in texts {
            for chunk in pretokenize_chunks(text) {
                let ids: Vec<u32> = chunk
                    .as_bytes()
                    .iter()
                    .map(|&b| {
                        let s = b2u[b as usize].to_string();
                        *token_to_id.get(&s).unwrap_or(&0)
                    })
                    .collect();
                if !ids.is_empty() {
                    *word_counts.entry(ids).or_insert(0) += 1;
                }
            }
        }

        let max_merges = target_vocab_size.saturating_sub(256 + special.len());
        let mut merges = Vec::new();
        let mut merge_ranks = HashMap::new();

        for _ in 0..max_merges {
            let mut pair_counts: HashMap<(u32, u32), usize> = HashMap::new();
            for (seq, &freq) in &word_counts {
                if seq.len() < 2 {
                    continue;
                }
                for w in seq.windows(2) {
                    *pair_counts.entry((w[0], w[1])).or_insert(0) += freq;
                }
            }

            let best = pair_counts
                .into_iter()
                .filter(|&(_, c)| c >= 2)
                .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)));

            let Some(((left, right), _)) = best else {
                break;
            };

            let left_str = id_to_token[left as usize].clone();
            let right_str = id_to_token[right as usize].clone();
            let merged_str = format!("{}{}", left_str, right_str);
            let new_id = id_to_token.len() as u32;

            let rank = merges.len();
            merges.push((left_str.clone(), right_str.clone()));
            merge_ranks.insert((left_str, right_str), rank);
            token_to_id.insert(merged_str.clone(), new_id);
            id_to_token.push(merged_str);

            // Apply merge across corpus word counts
            let mut next_counts: HashMap<Vec<u32>, usize> =
                HashMap::with_capacity(word_counts.len());
            for (seq, freq) in word_counts {
                let mut merged = Vec::with_capacity(seq.len());
                let mut i = 0;
                while i < seq.len() {
                    if i + 1 < seq.len() && seq[i] == left && seq[i + 1] == right {
                        merged.push(new_id);
                        i += 2;
                    } else {
                        merged.push(seq[i]);
                        i += 1;
                    }
                }
                *next_counts.entry(merged).or_insert(0) += freq;
            }
            word_counts = next_counts;
        }

        // 3. Register special tokens
        let mut special_tokens = HashMap::new();
        let mut bos_id = None;
        let mut eos_id = None;
        let mut unk_id = None;
        for sp in special {
            let id = if let Some(&existing) = token_to_id.get(sp) {
                existing
            } else {
                let id = id_to_token.len() as u32;
                token_to_id.insert(sp.clone(), id);
                id_to_token.push(sp.clone());
                id
            };
            special_tokens.insert(sp.clone(), id);
            let lower = sp.to_lowercase();
            if lower.contains("bos") || sp == "<s>" {
                bos_id = Some(id);
            }
            if lower.contains("eos") || sp == "</s>" {
                eos_id = Some(id);
            }
            if lower.contains("unk") {
                unk_id = Some(id);
            }
        }

        Self {
            kind: TokenizerKind::ByteLevel,
            id_to_token,
            token_to_id,
            merge_ranks,
            merges,
            scores: Vec::new(),
            special_tokens,
            bos_id,
            eos_id,
            unk_id,
        }
    }

    pub fn from_json_str(raw: &str) -> Result<Self, String> {
        let root: serde_json::Value =
            serde_json::from_str(raw).map_err(|e| format!("Invalid tokenizer JSON: {}", e))?;

        let mut token_to_id = HashMap::new();
        let mut max_id = 0usize;

        // Supports HuggingFace tokenizer.json (`model.vocab`) and flat Aether JSON (`vocab`)
        let vocab_obj = root
            .get("model")
            .and_then(|m| m.get("vocab"))
            .or_else(|| root.get("vocab"))
            .and_then(|v| v.as_object())
            .ok_or_else(|| "Tokenizer JSON missing 'vocab' object".to_string())?;

        for (tok, id_val) in vocab_obj {
            if let Some(id) = id_val.as_u64() {
                token_to_id.insert(tok.clone(), id as u32);
                max_id = max_id.max(id as usize);
            }
        }

        let mut special_tokens = HashMap::new();
        if let Some(added) = root.get("added_tokens").and_then(|v| v.as_array()) {
            for entry in added {
                if let (Some(content), Some(id)) = (
                    entry.get("content").and_then(|v| v.as_str()),
                    entry.get("id").and_then(|v| v.as_u64()),
                ) {
                    let is_special = entry
                        .get("special")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    token_to_id.insert(content.to_string(), id as u32);
                    max_id = max_id.max(id as usize);
                    if is_special {
                        special_tokens.insert(content.to_string(), id as u32);
                    }
                }
            }
        }
        if let Some(sp_obj) = root.get("special_tokens").and_then(|v| v.as_object()) {
            for (tok, id_val) in sp_obj {
                if let Some(id) = id_val.as_u64() {
                    special_tokens.insert(tok.clone(), id as u32);
                    token_to_id.insert(tok.clone(), id as u32);
                    max_id = max_id.max(id as usize);
                }
            }
        }

        let mut id_to_token = vec![String::new(); max_id + 1];
        for (tok, &id) in &token_to_id {
            if (id as usize) < id_to_token.len() {
                id_to_token[id as usize] = tok.clone();
            }
        }

        let mut merges = Vec::new();
        let mut merge_ranks = HashMap::new();
        let merges_arr = root
            .get("model")
            .and_then(|m| m.get("merges"))
            .or_else(|| root.get("merges"))
            .and_then(|v| v.as_array());

        if let Some(arr) = merges_arr {
            for item in arr {
                if let Some(line) = item.as_str() {
                    if let Some((a, b)) = line.split_once(' ') {
                        let rank = merges.len();
                        merges.push((a.to_string(), b.to_string()));
                        merge_ranks.insert((a.to_string(), b.to_string()), rank);
                    }
                } else if let Some(pair) = item.as_array() {
                    if pair.len() == 2 {
                        if let (Some(a), Some(b)) = (pair[0].as_str(), pair[1].as_str()) {
                            let rank = merges.len();
                            merges.push((a.to_string(), b.to_string()));
                            merge_ranks.insert((a.to_string(), b.to_string()), rank);
                        }
                    }
                }
            }
        }

        let kind_str = root
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("byte_level");
        let kind = if kind_str.eq_ignore_ascii_case("sentencepiece")
            || token_to_id.keys().any(|k| k.starts_with('\u{2581}'))
        {
            TokenizerKind::SentencePiece
        } else {
            TokenizerKind::ByteLevel
        };

        let bos_id = root
            .get("bos_id")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .or_else(|| token_to_id.get("<|bos|>").copied())
            .or_else(|| token_to_id.get("<s>").copied());
        let eos_id = root
            .get("eos_id")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .or_else(|| token_to_id.get("<|eos|>").copied())
            .or_else(|| token_to_id.get("</s>").copied());
        let unk_id = root
            .get("unk_id")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .or_else(|| token_to_id.get("<|unk|>").copied())
            .or_else(|| token_to_id.get("<unk>").copied());

        Ok(Self {
            kind,
            id_to_token,
            token_to_id,
            merge_ranks,
            merges,
            scores: Vec::new(),
            special_tokens,
            bos_id,
            eos_id,
            unk_id,
        })
    }

    pub fn to_json_string(&self) -> Result<String, String> {
        let mut vocab_map = serde_json::Map::new();
        for (id, tok) in self.id_to_token.iter().enumerate() {
            vocab_map.insert(tok.clone(), serde_json::Value::from(id as u64));
        }
        let merges_val: Vec<serde_json::Value> = self
            .merges
            .iter()
            .map(|(a, b)| serde_json::Value::String(format!("{} {}", a, b)))
            .collect();
        let mut sp_map = serde_json::Map::new();
        for (k, &v) in &self.special_tokens {
            sp_map.insert(k.clone(), serde_json::Value::from(v as u64));
        }

        let obj = serde_json::json!({
            "version": "1.0",
            "kind": match self.kind {
                TokenizerKind::ByteLevel => "byte_level",
                TokenizerKind::SentencePiece => "sentencepiece",
            },
            "bos_id": self.bos_id,
            "eos_id": self.eos_id,
            "unk_id": self.unk_id,
            "special_tokens": sp_map,
            "model": {
                "type": "BPE",
                "vocab": vocab_map,
                "merges": merges_val,
            }
        });
        serde_json::to_string_pretty(&obj)
            .map_err(|e| format!("Failed to serialize tokenizer JSON: {}", e))
    }

    pub fn from_gguf(gguf: &GgufFile) -> Result<Self, String> {
        let tokens_meta = gguf
            .metadata
            .get("tokenizer.ggml.tokens")
            .ok_or_else(|| "GGUF missing 'tokenizer.ggml.tokens' metadata".to_string())?;
        let id_to_token: Vec<String> = match tokens_meta {
            GgufMetaValue::Array(items) => items
                .iter()
                .map(|v| match v {
                    GgufMetaValue::String(s) => s.clone(),
                    other => format!("{:?}", other),
                })
                .collect(),
            _ => return Err("GGUF 'tokenizer.ggml.tokens' must be a string array".into()),
        };

        let mut token_to_id = HashMap::with_capacity(id_to_token.len());
        for (idx, tok) in id_to_token.iter().enumerate() {
            token_to_id.insert(tok.clone(), idx as u32);
        }

        let mut merges = Vec::new();
        let mut merge_ranks = HashMap::new();
        if let Some(GgufMetaValue::Array(m_items)) = gguf.metadata.get("tokenizer.ggml.merges") {
            for item in m_items {
                if let GgufMetaValue::String(line) = item {
                    if let Some((a, b)) = line.split_once(' ') {
                        let rank = merges.len();
                        merges.push((a.to_string(), b.to_string()));
                        merge_ranks.insert((a.to_string(), b.to_string()), rank);
                    }
                }
            }
        }

        let scores: Vec<f32> = match gguf.metadata.get("tokenizer.ggml.scores") {
            Some(GgufMetaValue::Array(s_items)) => s_items
                .iter()
                .map(|v| match v {
                    GgufMetaValue::Float32(f) => *f,
                    GgufMetaValue::Float64(f) => *f as f32,
                    _ => 0.0,
                })
                .collect(),
            _ => Vec::new(),
        };

        let model_type = gguf
            .meta_string("tokenizer.ggml.model")
            .unwrap_or_else(|| "gpt2".to_string());
        let kind = if model_type.eq_ignore_ascii_case("llama")
            || (merges.is_empty() && token_to_id.keys().any(|k| k.starts_with('\u{2581}')))
        {
            TokenizerKind::SentencePiece
        } else {
            TokenizerKind::ByteLevel
        };

        let bos_id = gguf
            .meta_u64("tokenizer.ggml.bos_token_id")
            .map(|v| v as u32);
        let eos_id = gguf
            .meta_u64("tokenizer.ggml.eos_token_id")
            .map(|v| v as u32);
        let unk_id = gguf
            .meta_u64("tokenizer.ggml.unknown_token_id")
            .map(|v| v as u32);

        let mut special_tokens = HashMap::new();
        for sid in [bos_id, eos_id, unk_id].into_iter().flatten() {
            if let Some(tok) = id_to_token.get(sid as usize) {
                special_tokens.insert(tok.clone(), sid);
            }
        }

        Ok(Self {
            kind,
            id_to_token,
            token_to_id,
            merge_ranks,
            merges,
            scores,
            special_tokens,
            bos_id,
            eos_id,
            unk_id,
        })
    }

    pub fn encode(&self, text: &str, add_special: bool) -> Vec<u32> {
        let mut out = Vec::new();
        if add_special {
            if let Some(bos) = self.bos_id {
                out.push(bos);
            }
        }

        if text.is_empty() {
            if add_special {
                if let Some(eos) = self.eos_id {
                    out.push(eos);
                }
            }
            return out;
        }

        // Split around special tokens (longest first)
        let mut specials: Vec<(&String, u32)> =
            self.special_tokens.iter().map(|(k, &v)| (k, v)).collect();
        specials.sort_by_key(|b| std::cmp::Reverse(b.0.len()));

        let mut cursor = 0usize;
        while cursor < text.len() {
            let mut matched_special = None;
            for &(sp, sid) in &specials {
                if !sp.is_empty() && text[cursor..].starts_with(sp.as_str()) {
                    matched_special = Some((sp.len(), sid));
                    break;
                }
            }
            if let Some((len, sid)) = matched_special {
                out.push(sid);
                cursor += len;
                continue;
            }

            // Find next special token boundary
            let mut next_pos = text.len();
            for &(sp, _) in &specials {
                if !sp.is_empty() {
                    if let Some(rel) = text[cursor..].find(sp.as_str()) {
                        next_pos = next_pos.min(cursor + rel);
                    }
                }
            }

            let segment = &text[cursor..next_pos];
            match self.kind {
                TokenizerKind::ByteLevel => self.encode_byte_level(segment, &mut out),
                TokenizerKind::SentencePiece => self.encode_sentencepiece(segment, &mut out),
            }
            cursor = next_pos;
        }

        if add_special {
            if let Some(eos) = self.eos_id {
                out.push(eos);
            }
        }
        out
    }

    fn encode_byte_level(&self, segment: &str, out: &mut Vec<u32>) {
        let (b2u, _) = byte_tables();
        for chunk in pretokenize_chunks(segment) {
            let mut symbols: Vec<String> = chunk
                .as_bytes()
                .iter()
                .map(|&b| b2u[b as usize].to_string())
                .collect();

            if !self.merge_ranks.is_empty() {
                loop {
                    if symbols.len() < 2 {
                        break;
                    }
                    let mut best_rank = usize::MAX;
                    let mut best_idx = None;
                    for i in 0..(symbols.len() - 1) {
                        if let Some(&rank) = self
                            .merge_ranks
                            .get(&(symbols[i].clone(), symbols[i + 1].clone()))
                        {
                            if rank < best_rank {
                                best_rank = rank;
                                best_idx = Some(i);
                            }
                        }
                    }
                    let Some(idx) = best_idx else {
                        break;
                    };
                    let merged = format!("{}{}", symbols[idx], symbols[idx + 1]);
                    symbols[idx] = merged;
                    symbols.remove(idx + 1);
                }
            }

            for sym in symbols {
                if let Some(&id) = self.token_to_id.get(&sym) {
                    out.push(id);
                } else if let Some(unk) = self.unk_id {
                    out.push(unk);
                }
            }
        }
    }

    fn encode_sentencepiece(&self, segment: &str, out: &mut Vec<u32>) {
        let normalized = format!("\u{2581}{}", segment.replace(' ', "\u{2581}"));
        let mut symbols: Vec<String> = Vec::new();
        for ch in normalized.chars() {
            let s = ch.to_string();
            if self.token_to_id.contains_key(&s) {
                symbols.push(s);
            } else {
                // Byte fallback `<0xXX>`
                let mut buf = [0u8; 4];
                for &b in ch.encode_utf8(&mut buf).as_bytes() {
                    let hex = format!("<0x{:02X}>", b);
                    if self.token_to_id.contains_key(&hex) {
                        symbols.push(hex);
                    }
                }
            }
        }

        loop {
            if symbols.len() < 2 {
                break;
            }
            let mut best_idx = None;
            if !self.merge_ranks.is_empty() {
                let mut best_rank = usize::MAX;
                for i in 0..(symbols.len() - 1) {
                    if let Some(&rank) = self
                        .merge_ranks
                        .get(&(symbols[i].clone(), symbols[i + 1].clone()))
                    {
                        if rank < best_rank {
                            best_rank = rank;
                            best_idx = Some(i);
                        }
                    }
                }
            } else {
                let mut best_score = f32::NEG_INFINITY;
                for i in 0..(symbols.len() - 1) {
                    let cand = format!("{}{}", symbols[i], symbols[i + 1]);
                    if let Some(&id) = self.token_to_id.get(&cand) {
                        let score = self.scores.get(id as usize).copied().unwrap_or(0.0);
                        if score > best_score {
                            best_score = score;
                            best_idx = Some(i);
                        }
                    }
                }
            }

            let Some(idx) = best_idx else {
                break;
            };
            let merged = format!("{}{}", symbols[idx], symbols[idx + 1]);
            symbols[idx] = merged;
            symbols.remove(idx + 1);
        }

        for sym in symbols {
            if let Some(&id) = self.token_to_id.get(&sym) {
                out.push(id);
            } else if let Some(unk) = self.unk_id {
                out.push(unk);
            }
        }
    }

    pub fn decode(&self, ids: &[u32], skip_special: bool) -> String {
        let (_, u2b) = byte_tables();
        let mut raw_bytes: Vec<u8> = Vec::new();
        let mut spm_text = String::new();

        for &id in ids {
            let Some(tok) = self.id_to_token.get(id as usize) else {
                continue;
            };
            if skip_special && self.special_tokens.contains_key(tok) {
                continue;
            }

            match self.kind {
                TokenizerKind::ByteLevel => {
                    if self.special_tokens.contains_key(tok) {
                        raw_bytes.extend_from_slice(tok.as_bytes());
                    } else {
                        for ch in tok.chars() {
                            if let Some(&b) = u2b.get(&ch) {
                                raw_bytes.push(b);
                            } else {
                                let mut buf = [0u8; 4];
                                raw_bytes.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                            }
                        }
                    }
                }
                TokenizerKind::SentencePiece => {
                    if tok.starts_with("<0x") && tok.ends_with('>') && tok.len() == 6 {
                        if let Ok(b) = u8::from_str_radix(&tok[3..5], 16) {
                            raw_bytes.push(b);
                            continue;
                        }
                    }
                    if !raw_bytes.is_empty() {
                        spm_text.push_str(&String::from_utf8_lossy(&raw_bytes));
                        raw_bytes.clear();
                    }
                    spm_text.push_str(&tok.replace('\u{2581}', " "));
                }
            }
        }

        match self.kind {
            TokenizerKind::ByteLevel => String::from_utf8_lossy(&raw_bytes).into_owned(),
            TokenizerKind::SentencePiece => {
                if !raw_bytes.is_empty() {
                    spm_text.push_str(&String::from_utf8_lossy(&raw_bytes));
                }
                spm_text
                    .strip_prefix(' ')
                    .map(|s| s.to_string())
                    .unwrap_or(spm_text)
            }
        }
    }
}

fn pretokenize_chunks(text: &str) -> Vec<&str> {
    let mut chunks = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let ch = text[i..].chars().next().unwrap();
        let clen = ch.len_utf8();
        if i > start && (ch == ' ' || ch == '\n' || ch == '\t') {
            chunks.push(&text[start..i]);
            start = i;
        }
        i += clen;
    }
    if start < text.len() {
        chunks.push(&text[start..]);
    }
    chunks
}

// ==============================================================================
// 3. Pure-Rust GGUF v2 / v3 Binary Reader, Quantizer & Dequantizer
// ==============================================================================

pub const GGUF_TYPE_F32: u32 = 0;
pub const GGUF_TYPE_F16: u32 = 1;
pub const GGUF_TYPE_Q4_0: u32 = 2;
pub const GGUF_TYPE_Q4_1: u32 = 3;
pub const GGUF_TYPE_Q5_0: u32 = 6;
pub const GGUF_TYPE_Q8_0: u32 = 8;
pub const GGUF_TYPE_BF16: u32 = 30;

#[derive(Clone, Debug)]
pub enum GgufMetaValue {
    Uint8(u8),
    Int8(i8),
    Uint16(u16),
    Int16(i16),
    Uint32(u32),
    Int32(i32),
    Float32(f32),
    Bool(bool),
    String(String),
    Array(Vec<GgufMetaValue>),
    Uint64(u64),
    Int64(i64),
    Float64(f64),
}

impl GgufMetaValue {
    pub fn to_aether_value(&self) -> Value {
        match self {
            GgufMetaValue::Uint8(v) => Value::Int(*v as i64),
            GgufMetaValue::Int8(v) => Value::Int(*v as i64),
            GgufMetaValue::Uint16(v) => Value::Int(*v as i64),
            GgufMetaValue::Int16(v) => Value::Int(*v as i64),
            GgufMetaValue::Uint32(v) => Value::Int(*v as i64),
            GgufMetaValue::Int32(v) => Value::Int(*v as i64),
            GgufMetaValue::Uint64(v) => Value::Int(*v as i64),
            GgufMetaValue::Int64(v) => Value::Int(*v),
            GgufMetaValue::Float32(v) => Value::Float(*v as f64),
            GgufMetaValue::Float64(v) => Value::Float(*v),
            GgufMetaValue::Bool(v) => Value::Bool(*v),
            GgufMetaValue::String(s) => Value::string(s.clone()),
            GgufMetaValue::Array(items) => {
                Value::array(items.iter().map(|i| i.to_aether_value()).collect())
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct GgufTensorInfo {
    pub name: String,
    pub gguf_dims: Vec<u64>,
    pub row_major_shape: Vec<usize>,
    pub ggml_type: u32,
    pub offset: u64,
}

#[derive(Clone, Debug)]
pub struct GgufFile {
    pub path: String,
    pub version: u32,
    pub alignment: u64,
    pub metadata: BTreeMap<String, GgufMetaValue>,
    pub tensors: BTreeMap<String, GgufTensorInfo>,
    pub tensor_data_offset: u64,
}

impl GgufFile {
    pub fn open(path: &str) -> Result<Self, String> {
        let file =
            File::open(path).map_err(|e| format!("Failed to open GGUF '{}': {}", path, e))?;
        let mut r = BufReader::new(file);

        let mut magic = [0u8; 4];
        r.read_exact(&mut magic)
            .map_err(|e| format!("GGUF header read failed: {}", e))?;
        if &magic != b"GGUF" {
            return Err(format!(
                "Invalid GGUF magic bytes {:?} in '{}'",
                magic, path
            ));
        }

        let version = read_u32(&mut r)?;
        if version != 2 && version != 3 {
            return Err(format!(
                "Unsupported GGUF version {} (supported: v2, v3)",
                version
            ));
        }

        let tensor_count = read_u64(&mut r)?;
        let kv_count = read_u64(&mut r)?;

        let mut metadata = BTreeMap::new();
        for _ in 0..kv_count {
            let key = read_gguf_string(&mut r)?;
            let val_type = read_u32(&mut r)?;
            let val = read_gguf_meta_value(&mut r, val_type)?;
            metadata.insert(key, val);
        }

        let mut tensors = BTreeMap::new();
        for _ in 0..tensor_count {
            let name = read_gguf_string(&mut r)?;
            let n_dims = read_u32(&mut r)? as usize;
            let mut gguf_dims = Vec::with_capacity(n_dims);
            for _ in 0..n_dims {
                gguf_dims.push(read_u64(&mut r)?);
            }
            let ggml_type = read_u32(&mut r)?;
            let offset = read_u64(&mut r)?;
            // GGUF stores dims innermost-first [ne0, ne1, ...]; reverse to row-major [rows, cols]
            let row_major_shape: Vec<usize> = gguf_dims.iter().rev().map(|&d| d as usize).collect();
            tensors.insert(
                name.clone(),
                GgufTensorInfo {
                    name,
                    gguf_dims,
                    row_major_shape,
                    ggml_type,
                    offset,
                },
            );
        }

        let alignment = match metadata.get("general.alignment") {
            Some(GgufMetaValue::Uint32(a)) if *a > 0 => *a as u64,
            Some(GgufMetaValue::Uint64(a)) if *a > 0 => *a,
            _ => 32,
        };

        let pos = r
            .stream_position()
            .map_err(|e| format!("Failed to read stream pos: {}", e))?;
        let tensor_data_offset = align_up(pos, alignment);

        Ok(Self {
            path: path.to_string(),
            version,
            alignment,
            metadata,
            tensors,
            tensor_data_offset,
        })
    }

    pub fn meta_u64(&self, key: &str) -> Option<u64> {
        match self.metadata.get(key)? {
            GgufMetaValue::Uint8(v) => Some(*v as u64),
            GgufMetaValue::Uint16(v) => Some(*v as u64),
            GgufMetaValue::Uint32(v) => Some(*v as u64),
            GgufMetaValue::Uint64(v) => Some(*v),
            GgufMetaValue::Int32(v) if *v >= 0 => Some(*v as u64),
            GgufMetaValue::Int64(v) if *v >= 0 => Some(*v as u64),
            _ => None,
        }
    }

    pub fn meta_f64(&self, key: &str) -> Option<f64> {
        match self.metadata.get(key)? {
            GgufMetaValue::Float32(v) => Some(*v as f64),
            GgufMetaValue::Float64(v) => Some(*v),
            GgufMetaValue::Uint32(v) => Some(*v as f64),
            GgufMetaValue::Int32(v) => Some(*v as f64),
            _ => None,
        }
    }

    pub fn meta_string(&self, key: &str) -> Option<String> {
        match self.metadata.get(key)? {
            GgufMetaValue::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    pub fn read_tensor_f32(&self, name: &str) -> Result<(Vec<usize>, Vec<f32>), String> {
        let info = self
            .tensors
            .get(name)
            .ok_or_else(|| format!("Tensor '{}' not found in GGUF '{}'", name, self.path))?;
        let n_elems: usize = info.row_major_shape.iter().product();
        let byte_len = ggml_tensor_byte_size(info.ggml_type, n_elems)?;

        let mut file = File::open(&self.path)
            .map_err(|e| format!("Failed to reopen GGUF '{}': {}", self.path, e))?;
        file.seek(SeekFrom::Start(self.tensor_data_offset + info.offset))
            .map_err(|e| format!("Failed to seek to tensor '{}': {}", name, e))?;

        let mut raw = vec![0u8; byte_len];
        file.read_exact(&mut raw)
            .map_err(|e| format!("Failed to read tensor '{}' bytes: {}", name, e))?;

        let floats = dequantize_ggml(info.ggml_type, &raw, n_elems)?;
        Ok((info.row_major_shape.clone(), floats))
    }
}

pub fn ggml_type_name(t: u32) -> &'static str {
    match t {
        GGUF_TYPE_F32 => "F32",
        GGUF_TYPE_F16 => "F16",
        GGUF_TYPE_Q4_0 => "Q4_0",
        GGUF_TYPE_Q4_1 => "Q4_1",
        GGUF_TYPE_Q5_0 => "Q5_0",
        GGUF_TYPE_Q8_0 => "Q8_0",
        GGUF_TYPE_BF16 => "BF16",
        _ => "UNKNOWN",
    }
}

fn ggml_tensor_byte_size(ggml_type: u32, n_elems: usize) -> Result<usize, String> {
    match ggml_type {
        GGUF_TYPE_F32 => Ok(n_elems * 4),
        GGUF_TYPE_F16 | GGUF_TYPE_BF16 => Ok(n_elems * 2),
        GGUF_TYPE_Q4_0 => {
            if !n_elems.is_multiple_of(32) {
                return Err(format!(
                    "Q4_0 element count {} not divisible by 32",
                    n_elems
                ));
            }
            Ok((n_elems / 32) * 18)
        }
        GGUF_TYPE_Q4_1 => {
            if !n_elems.is_multiple_of(32) {
                return Err(format!(
                    "Q4_1 element count {} not divisible by 32",
                    n_elems
                ));
            }
            Ok((n_elems / 32) * 20)
        }
        GGUF_TYPE_Q5_0 => {
            if !n_elems.is_multiple_of(32) {
                return Err(format!(
                    "Q5_0 element count {} not divisible by 32",
                    n_elems
                ));
            }
            Ok((n_elems / 32) * 22)
        }
        GGUF_TYPE_Q8_0 => {
            if !n_elems.is_multiple_of(32) {
                return Err(format!(
                    "Q8_0 element count {} not divisible by 32",
                    n_elems
                ));
            }
            Ok((n_elems / 32) * 34)
        }
        other => Err(format!("Unsupported GGUF quantization type ID {}", other)),
    }
}

pub fn dequantize_ggml(ggml_type: u32, raw: &[u8], n_elems: usize) -> Result<Vec<f32>, String> {
    let mut out = vec![0.0f32; n_elems];
    match ggml_type {
        GGUF_TYPE_F32 => {
            for (i, chunk) in raw.chunks_exact(4).take(n_elems).enumerate() {
                out[i] = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            }
        }
        GGUF_TYPE_F16 => {
            for (i, chunk) in raw.chunks_exact(2).take(n_elems).enumerate() {
                out[i] = f16_to_f32(u16::from_le_bytes([chunk[0], chunk[1]]));
            }
        }
        GGUF_TYPE_BF16 => {
            for (i, chunk) in raw.chunks_exact(2).take(n_elems).enumerate() {
                out[i] = bf16_to_f32(u16::from_le_bytes([chunk[0], chunk[1]]));
            }
        }
        GGUF_TYPE_Q4_0 => {
            let n_blocks = n_elems / 32;
            for b in 0..n_blocks {
                let blk = &raw[b * 18..(b + 1) * 18];
                let d = f16_to_f32(u16::from_le_bytes([blk[0], blk[1]]));
                let qs = &blk[2..18];
                let base = b * 32;
                for j in 0..16 {
                    let q0 = (qs[j] & 0x0F) as i32 - 8;
                    let q1 = (qs[j] >> 4) as i32 - 8;
                    out[base + j] = (q0 as f32) * d;
                    out[base + j + 16] = (q1 as f32) * d;
                }
            }
        }
        GGUF_TYPE_Q4_1 => {
            let n_blocks = n_elems / 32;
            for b in 0..n_blocks {
                let blk = &raw[b * 20..(b + 1) * 20];
                let d = f16_to_f32(u16::from_le_bytes([blk[0], blk[1]]));
                let m = f16_to_f32(u16::from_le_bytes([blk[2], blk[3]]));
                let qs = &blk[4..20];
                let base = b * 32;
                for j in 0..16 {
                    let q0 = (qs[j] & 0x0F) as f32;
                    let q1 = (qs[j] >> 4) as f32;
                    out[base + j] = q0 * d + m;
                    out[base + j + 16] = q1 * d + m;
                }
            }
        }
        GGUF_TYPE_Q5_0 => {
            let n_blocks = n_elems / 32;
            for b in 0..n_blocks {
                let blk = &raw[b * 22..(b + 1) * 22];
                let d = f16_to_f32(u16::from_le_bytes([blk[0], blk[1]]));
                let qh = u32::from_le_bytes([blk[2], blk[3], blk[4], blk[5]]);
                let qs = &blk[6..22];
                let base = b * 32;
                for j in 0..16 {
                    let xh0 = (((qh >> j) & 1) << 4) as i32;
                    let xh1 = (((qh >> (j + 16)) & 1) << 4) as i32;
                    let q0 = ((qs[j] & 0x0F) as i32 | xh0) - 16;
                    let q1 = ((qs[j] >> 4) as i32 | xh1) - 16;
                    out[base + j] = (q0 as f32) * d;
                    out[base + j + 16] = (q1 as f32) * d;
                }
            }
        }
        GGUF_TYPE_Q8_0 => {
            let n_blocks = n_elems / 32;
            for b in 0..n_blocks {
                let blk = &raw[b * 34..(b + 1) * 34];
                let d = f16_to_f32(u16::from_le_bytes([blk[0], blk[1]]));
                let qs = &blk[2..34];
                let base = b * 32;
                for j in 0..32 {
                    out[base + j] = (qs[j] as i8 as f32) * d;
                }
            }
        }
        other => return Err(format!("Unsupported GGML dequantization type {}", other)),
    }
    Ok(out)
}

pub fn quantize_ggml(ggml_type: u32, data: &[f32]) -> Result<Vec<u8>, String> {
    match ggml_type {
        GGUF_TYPE_F32 => {
            let mut buf = Vec::with_capacity(data.len() * 4);
            for &v in data {
                buf.extend_from_slice(&v.to_le_bytes());
            }
            Ok(buf)
        }
        GGUF_TYPE_F16 => {
            let mut buf = Vec::with_capacity(data.len() * 2);
            for &v in data {
                buf.extend_from_slice(&f32_to_f16(v).to_le_bytes());
            }
            Ok(buf)
        }
        GGUF_TYPE_BF16 => {
            let mut buf = Vec::with_capacity(data.len() * 2);
            for &v in data {
                buf.extend_from_slice(&f32_to_bf16(v).to_le_bytes());
            }
            Ok(buf)
        }
        GGUF_TYPE_Q8_0 => {
            if !data.len().is_multiple_of(32) {
                return Err(format!(
                    "Q8_0 requires length divisible by 32, got {}",
                    data.len()
                ));
            }
            let n_blocks = data.len() / 32;
            let mut buf = Vec::with_capacity(n_blocks * 34);
            for blk in data.chunks_exact(32) {
                let amax = blk.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
                let d = amax / 127.0;
                let id = if d > 0.0 { 1.0 / d } else { 0.0 };
                buf.extend_from_slice(&f32_to_f16(d).to_le_bytes());
                for &v in blk {
                    let q = (v * id).round().clamp(-127.0, 127.0) as i8;
                    buf.push(q as u8);
                }
            }
            Ok(buf)
        }
        GGUF_TYPE_Q4_0 => {
            if !data.len().is_multiple_of(32) {
                return Err(format!(
                    "Q4_0 requires length divisible by 32, got {}",
                    data.len()
                ));
            }
            let n_blocks = data.len() / 32;
            let mut buf = Vec::with_capacity(n_blocks * 18);
            for blk in data.chunks_exact(32) {
                let amax = blk.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
                let d = amax / 7.0;
                let id = if d > 0.0 { 1.0 / d } else { 0.0 };
                buf.extend_from_slice(&f32_to_f16(d).to_le_bytes());
                for j in 0..16 {
                    let q0 = ((blk[j] * id).round().clamp(-8.0, 7.0) as i32 + 8) as u8;
                    let q1 = ((blk[j + 16] * id).round().clamp(-8.0, 7.0) as i32 + 8) as u8;
                    buf.push((q0 & 0x0F) | ((q1 & 0x0F) << 4));
                }
            }
            Ok(buf)
        }
        GGUF_TYPE_Q4_1 => {
            if !data.len().is_multiple_of(32) {
                return Err(format!(
                    "Q4_1 requires length divisible by 32, got {}",
                    data.len()
                ));
            }
            let n_blocks = data.len() / 32;
            let mut buf = Vec::with_capacity(n_blocks * 20);
            for blk in data.chunks_exact(32) {
                let min_v = blk.iter().copied().fold(f32::INFINITY, f32::min);
                let max_v = blk.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                let d = (max_v - min_v) / 15.0;
                let id = if d > 0.0 { 1.0 / d } else { 0.0 };
                buf.extend_from_slice(&f32_to_f16(d).to_le_bytes());
                buf.extend_from_slice(&f32_to_f16(min_v).to_le_bytes());
                for j in 0..16 {
                    let q0 = ((blk[j] - min_v) * id).round().clamp(0.0, 15.0) as u8;
                    let q1 = ((blk[j + 16] - min_v) * id).round().clamp(0.0, 15.0) as u8;
                    buf.push((q0 & 0x0F) | ((q1 & 0x0F) << 4));
                }
            }
            Ok(buf)
        }
        GGUF_TYPE_Q5_0 => {
            if !data.len().is_multiple_of(32) {
                return Err(format!(
                    "Q5_0 requires length divisible by 32, got {}",
                    data.len()
                ));
            }
            let n_blocks = data.len() / 32;
            let mut buf = Vec::with_capacity(n_blocks * 22);
            for blk in data.chunks_exact(32) {
                let amax = blk.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
                let d = amax / 15.0;
                let id = if d > 0.0 { 1.0 / d } else { 0.0 };
                buf.extend_from_slice(&f32_to_f16(d).to_le_bytes());
                let mut qh: u32 = 0;
                let mut qs = [0u8; 16];
                for j in 0..16 {
                    let q0 = ((blk[j] * id).round().clamp(-16.0, 15.0) as i32 + 16) as u32;
                    let q1 = ((blk[j + 16] * id).round().clamp(-16.0, 15.0) as i32 + 16) as u32;
                    qs[j] = ((q0 & 0x0F) | ((q1 & 0x0F) << 4)) as u8;
                    if (q0 & 0x10) != 0 {
                        qh |= 1 << j;
                    }
                    if (q1 & 0x10) != 0 {
                        qh |= 1 << (j + 16);
                    }
                }
                buf.extend_from_slice(&qh.to_le_bytes());
                buf.extend_from_slice(&qs);
            }
            Ok(buf)
        }
        other => Err(format!("Unsupported GGML quantization type {}", other)),
    }
}

pub fn save_gguf_file(
    path: &str,
    metadata: &BTreeMap<String, GgufMetaValue>,
    tensors: &BTreeMap<String, TensorData>,
    quant_mode: &str,
) -> Result<(), String> {
    let target_qtype = match quant_mode.to_uppercase().as_str() {
        "F32" => GGUF_TYPE_F32,
        "F16" => GGUF_TYPE_F16,
        "BF16" => GGUF_TYPE_BF16,
        "Q8_0" => GGUF_TYPE_Q8_0,
        "Q4_0" => GGUF_TYPE_Q4_0,
        "Q4_1" => GGUF_TYPE_Q4_1,
        "Q5_0" => GGUF_TYPE_Q5_0,
        other => {
            return Err(format!(
                "Unsupported GGUF quant_mode '{}' (use F32, F16, BF16, Q8_0, Q4_0, Q4_1, Q5_0)",
                other
            ))
        }
    };

    struct PreparedGgufTensor {
        name: String,
        gguf_dims: Vec<u64>,
        qtype: u32,
        offset: u64,
        raw_bytes: Vec<u8>,
    }

    let alignment: u64 = 32;
    let mut prepared: Vec<PreparedGgufTensor> = Vec::with_capacity(tensors.len());
    let mut cur_offset: u64 = 0;

    for (name, td) in tensors {
        let f32_data: Vec<f32> = td.data.iter().map(|&v| v as f32).collect();
        // Keep 1D norm tensors or non-multiple-of-32 tensors in F32 (llama.cpp standard)
        let actual_type = if matches!(
            target_qtype,
            GGUF_TYPE_Q8_0 | GGUF_TYPE_Q4_0 | GGUF_TYPE_Q4_1 | GGUF_TYPE_Q5_0
        ) && (td.shape.len() < 2
            || !f32_data.len().is_multiple_of(32)
            || name.contains("norm"))
        {
            GGUF_TYPE_F32
        } else {
            target_qtype
        };

        let raw_bytes = quantize_ggml(actual_type, &f32_data)?;
        let gguf_dims: Vec<u64> = td.shape.iter().rev().map(|&d| d as u64).collect();
        let aligned_offset = align_up(cur_offset, alignment);
        cur_offset = aligned_offset + raw_bytes.len() as u64;
        prepared.push(PreparedGgufTensor {
            name: name.clone(),
            gguf_dims,
            qtype: actual_type,
            offset: aligned_offset,
            raw_bytes,
        });
    }

    let file =
        File::create(path).map_err(|e| format!("Failed to create GGUF file '{}': {}", path, e))?;
    let mut w = BufWriter::new(file);

    // Header
    w.write_all(b"GGUF").map_err(|e| e.to_string())?;
    w.write_all(&3u32.to_le_bytes())
        .map_err(|e| e.to_string())?;
    w.write_all(&(prepared.len() as u64).to_le_bytes())
        .map_err(|e| e.to_string())?;
    w.write_all(&(metadata.len() as u64).to_le_bytes())
        .map_err(|e| e.to_string())?;

    // Metadata KVs
    for (k, v) in metadata {
        write_gguf_string(&mut w, k)?;
        write_gguf_meta_value(&mut w, v)?;
    }

    // Tensor directory
    for entry in &prepared {
        write_gguf_string(&mut w, &entry.name)?;
        w.write_all(&(entry.gguf_dims.len() as u32).to_le_bytes())
            .map_err(|e| e.to_string())?;
        for &d in &entry.gguf_dims {
            w.write_all(&d.to_le_bytes()).map_err(|e| e.to_string())?;
        }
        w.write_all(&entry.qtype.to_le_bytes())
            .map_err(|e| e.to_string())?;
        w.write_all(&entry.offset.to_le_bytes())
            .map_err(|e| e.to_string())?;
    }

    w.flush().map_err(|e| e.to_string())?;
    let header_end = w.stream_position().map_err(|e| e.to_string())?;
    let data_start = align_up(header_end, alignment);
    let pad = (data_start - header_end) as usize;
    if pad > 0 {
        w.write_all(&vec![0u8; pad]).map_err(|e| e.to_string())?;
    }

    let mut written_data = 0u64;
    for entry in &prepared {
        if entry.offset > written_data {
            let gap = (entry.offset - written_data) as usize;
            w.write_all(&vec![0u8; gap]).map_err(|e| e.to_string())?;
            written_data = entry.offset;
        }
        w.write_all(&entry.raw_bytes).map_err(|e| e.to_string())?;
        written_data += entry.raw_bytes.len() as u64;
    }

    w.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn align_up(val: u64, align: u64) -> u64 {
    if align <= 1 {
        return val;
    }
    val.div_ceil(align) * align
}

fn read_u8<R: Read>(r: &mut R) -> Result<u8, String> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b).map_err(|e| e.to_string())?;
    Ok(b[0])
}
fn read_u16<R: Read>(r: &mut R) -> Result<u16, String> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b).map_err(|e| e.to_string())?;
    Ok(u16::from_le_bytes(b))
}
fn read_u32<R: Read>(r: &mut R) -> Result<u32, String> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b).map_err(|e| e.to_string())?;
    Ok(u32::from_le_bytes(b))
}
fn read_u64<R: Read>(r: &mut R) -> Result<u64, String> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b).map_err(|e| e.to_string())?;
    Ok(u64::from_le_bytes(b))
}

fn read_gguf_string<R: Read>(r: &mut R) -> Result<String, String> {
    let len = read_u64(r)? as usize;
    if len > 64 * 1024 * 1024 {
        return Err(format!("GGUF string length {} exceeds safety bound", len));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn read_gguf_meta_value<R: Read>(r: &mut R, vtype: u32) -> Result<GgufMetaValue, String> {
    match vtype {
        0 => Ok(GgufMetaValue::Uint8(read_u8(r)?)),
        1 => Ok(GgufMetaValue::Int8(read_u8(r)? as i8)),
        2 => Ok(GgufMetaValue::Uint16(read_u16(r)?)),
        3 => Ok(GgufMetaValue::Int16(read_u16(r)? as i16)),
        4 => Ok(GgufMetaValue::Uint32(read_u32(r)?)),
        5 => Ok(GgufMetaValue::Int32(read_u32(r)? as i32)),
        6 => Ok(GgufMetaValue::Float32(f32::from_bits(read_u32(r)?))),
        7 => Ok(GgufMetaValue::Bool(read_u8(r)? != 0)),
        8 => Ok(GgufMetaValue::String(read_gguf_string(r)?)),
        9 => {
            let elem_type = read_u32(r)?;
            let count = read_u64(r)? as usize;
            let mut items = Vec::with_capacity(count.min(1_000_000));
            for _ in 0..count {
                items.push(read_gguf_meta_value(r, elem_type)?);
            }
            Ok(GgufMetaValue::Array(items))
        }
        10 => Ok(GgufMetaValue::Uint64(read_u64(r)?)),
        11 => Ok(GgufMetaValue::Int64(read_u64(r)? as i64)),
        12 => Ok(GgufMetaValue::Float64(f64::from_bits(read_u64(r)?))),
        other => Err(format!("Unknown GGUF metadata type {}", other)),
    }
}

fn write_gguf_string<W: Write>(w: &mut W, s: &str) -> Result<(), String> {
    let bytes = s.as_bytes();
    w.write_all(&(bytes.len() as u64).to_le_bytes())
        .map_err(|e| e.to_string())?;
    w.write_all(bytes).map_err(|e| e.to_string())?;
    Ok(())
}

fn gguf_meta_type_id(val: &GgufMetaValue) -> u32 {
    match val {
        GgufMetaValue::Uint8(_) => 0,
        GgufMetaValue::Int8(_) => 1,
        GgufMetaValue::Uint16(_) => 2,
        GgufMetaValue::Int16(_) => 3,
        GgufMetaValue::Uint32(_) => 4,
        GgufMetaValue::Int32(_) => 5,
        GgufMetaValue::Float32(_) => 6,
        GgufMetaValue::Bool(_) => 7,
        GgufMetaValue::String(_) => 8,
        GgufMetaValue::Array(_) => 9,
        GgufMetaValue::Uint64(_) => 10,
        GgufMetaValue::Int64(_) => 11,
        GgufMetaValue::Float64(_) => 12,
    }
}

fn write_gguf_meta_payload<W: Write>(w: &mut W, val: &GgufMetaValue) -> Result<(), String> {
    match val {
        GgufMetaValue::Uint8(v) => w.write_all(&[*v]).map_err(|e| e.to_string()),
        GgufMetaValue::Int8(v) => w.write_all(&[*v as u8]).map_err(|e| e.to_string()),
        GgufMetaValue::Uint16(v) => w.write_all(&v.to_le_bytes()).map_err(|e| e.to_string()),
        GgufMetaValue::Int16(v) => w.write_all(&v.to_le_bytes()).map_err(|e| e.to_string()),
        GgufMetaValue::Uint32(v) => w.write_all(&v.to_le_bytes()).map_err(|e| e.to_string()),
        GgufMetaValue::Int32(v) => w.write_all(&v.to_le_bytes()).map_err(|e| e.to_string()),
        GgufMetaValue::Float32(v) => w.write_all(&v.to_le_bytes()).map_err(|e| e.to_string()),
        GgufMetaValue::Bool(v) => w
            .write_all(&[if *v { 1 } else { 0 }])
            .map_err(|e| e.to_string()),
        GgufMetaValue::String(s) => write_gguf_string(w, s),
        GgufMetaValue::Array(items) => {
            let elem_type = items.first().map(gguf_meta_type_id).unwrap_or(8);
            w.write_all(&elem_type.to_le_bytes())
                .map_err(|e| e.to_string())?;
            w.write_all(&(items.len() as u64).to_le_bytes())
                .map_err(|e| e.to_string())?;
            for item in items {
                write_gguf_meta_payload(w, item)?;
            }
            Ok(())
        }
        GgufMetaValue::Uint64(v) => w.write_all(&v.to_le_bytes()).map_err(|e| e.to_string()),
        GgufMetaValue::Int64(v) => w.write_all(&v.to_le_bytes()).map_err(|e| e.to_string()),
        GgufMetaValue::Float64(v) => w.write_all(&v.to_le_bytes()).map_err(|e| e.to_string()),
    }
}

fn write_gguf_meta_value<W: Write>(w: &mut W, val: &GgufMetaValue) -> Result<(), String> {
    let tid = gguf_meta_type_id(val);
    w.write_all(&tid.to_le_bytes()).map_err(|e| e.to_string())?;
    write_gguf_meta_payload(w, val)
}

fn aether_value_to_gguf_meta(val: &Value) -> Result<GgufMetaValue, String> {
    match val {
        Value::Int(i) => {
            if *i >= 0 && *i <= u32::MAX as i64 {
                Ok(GgufMetaValue::Uint32(*i as u32))
            } else {
                Ok(GgufMetaValue::Int64(*i))
            }
        }
        Value::Float(f) => Ok(GgufMetaValue::Float32(*f as f32)),
        Value::Bool(b) => Ok(GgufMetaValue::Bool(*b)),
        Value::String(s) => Ok(GgufMetaValue::String(s.to_string())),
        Value::Array(arr) => {
            let items: Vec<Value> = arr.lock().clone();
            let mut out = Vec::with_capacity(items.len());
            for it in &items {
                out.push(aether_value_to_gguf_meta(it)?);
            }
            Ok(GgufMetaValue::Array(out))
        }
        other => Ok(GgufMetaValue::String(format!("{}", other))),
    }
}

// ==============================================================================
// 4. Local Transformer / Llama-Architecture Inference Engine (RoPE + GQA + SwiGLU + KV-Cache)
// ==============================================================================

#[derive(Clone, Debug)]
pub struct LlmConfig {
    pub architecture: String,
    pub vocab_size: usize,
    pub hidden_size: usize,
    pub intermediate_size: usize,
    pub num_layers: usize,
    pub num_heads: usize,
    pub num_kv_heads: usize,
    pub head_dim: usize,
    pub max_seq_len: usize,
    pub rms_norm_eps: f32,
    pub rope_theta: f32,
    pub bos_token_id: Option<u32>,
    pub eos_token_id: Option<u32>,
}

#[derive(Clone)]
pub struct LlamaLayerWeights {
    pub attn_norm: Vec<f32>,
    pub wq: Vec<f32>, // [num_heads * head_dim, hidden_size]
    pub wk: Vec<f32>, // [num_kv_heads * head_dim, hidden_size]
    pub wv: Vec<f32>, // [num_kv_heads * head_dim, hidden_size]
    pub wo: Vec<f32>, // [hidden_size, num_heads * head_dim]
    pub ffn_norm: Vec<f32>,
    pub w_gate: Vec<f32>, // [intermediate_size, hidden_size]
    pub w_up: Vec<f32>,   // [intermediate_size, hidden_size]
    pub w_down: Vec<f32>, // [hidden_size, intermediate_size]
}

#[derive(Clone)]
pub struct KvCache {
    pub k_layers: Vec<Vec<f32>>, // per layer: [seq_len * kv_dim]
    pub v_layers: Vec<Vec<f32>>,
    pub seq_len: usize,
}

impl KvCache {
    pub fn new(num_layers: usize, kv_dim: usize, max_seq_len: usize) -> Self {
        let cap = max_seq_len.min(4096) * kv_dim;
        Self {
            k_layers: (0..num_layers).map(|_| Vec::with_capacity(cap)).collect(),
            v_layers: (0..num_layers).map(|_| Vec::with_capacity(cap)).collect(),
            seq_len: 0,
        }
    }

    pub fn clear(&mut self) {
        for k in &mut self.k_layers {
            k.clear();
        }
        for v in &mut self.v_layers {
            v.clear();
        }
        self.seq_len = 0;
    }
}

#[derive(Clone, Debug)]
pub struct GenerateOptions {
    pub max_new_tokens: usize,
    pub temperature: f32,
    pub top_k: usize,
    pub top_p: f32,
    pub rep_penalty: f32,
    pub seed: u64,
    pub reset_cache: bool,
}

pub struct LocalLlm {
    pub config: LlmConfig,
    pub token_embd: Vec<f32>,
    pub layers: Vec<LlamaLayerWeights>,
    pub output_norm: Vec<f32>,
    pub lm_head: Vec<f32>,
    pub cache: KvCache,
    pub tokenizer: Option<BpeTokenizer>,
}

impl LocalLlm {
    pub fn from_gguf(gguf: &GgufFile) -> Result<Self, String> {
        let arch = gguf
            .meta_string("general.architecture")
            .unwrap_or_else(|| "llama".to_string());

        let (emb_shape, token_embd) = gguf.read_tensor_f32("token_embd.weight")?;
        if emb_shape.len() != 2 {
            return Err(format!(
                "token_embd.weight expected 2D shape, got {:?}",
                emb_shape
            ));
        }
        let vocab_size = emb_shape[0];
        let hidden_size = gguf
            .meta_u64(&format!("{}.embedding_length", arch))
            .map(|v| v as usize)
            .unwrap_or(emb_shape[1]);
        let num_layers = gguf
            .meta_u64(&format!("{}.block_count", arch))
            .map(|v| v as usize)
            .ok_or_else(|| format!("GGUF missing '{}.block_count'", arch))?;
        let num_heads = gguf
            .meta_u64(&format!("{}.attention.head_count", arch))
            .map(|v| v as usize)
            .ok_or_else(|| format!("GGUF missing '{}.attention.head_count'", arch))?;
        let num_kv_heads = gguf
            .meta_u64(&format!("{}.attention.head_count_kv", arch))
            .map(|v| v as usize)
            .unwrap_or(num_heads);
        let max_seq_len = gguf
            .meta_u64(&format!("{}.context_length", arch))
            .map(|v| v as usize)
            .unwrap_or(2048);
        let rms_norm_eps = gguf
            .meta_f64(&format!("{}.attention.layer_norm_rms_epsilon", arch))
            .map(|v| v as f32)
            .unwrap_or(1e-5);
        let rope_theta = gguf
            .meta_f64(&format!("{}.rope.freq_base", arch))
            .map(|v| v as f32)
            .unwrap_or(10000.0);

        let head_dim = hidden_size / num_heads.max(1);
        let mut intermediate_size = gguf
            .meta_u64(&format!("{}.feed_forward_length", arch))
            .map(|v| v as usize)
            .unwrap_or(0);

        let mut layers = Vec::with_capacity(num_layers);
        for i in 0..num_layers {
            let (_, attn_norm) = gguf.read_tensor_f32(&format!("blk.{}.attn_norm.weight", i))?;
            let (_, wq) = gguf.read_tensor_f32(&format!("blk.{}.attn_q.weight", i))?;
            let (_, wk) = gguf.read_tensor_f32(&format!("blk.{}.attn_k.weight", i))?;
            let (_, wv) = gguf.read_tensor_f32(&format!("blk.{}.attn_v.weight", i))?;
            let (_, wo) = gguf.read_tensor_f32(&format!("blk.{}.attn_output.weight", i))?;
            let (_, ffn_norm) = gguf.read_tensor_f32(&format!("blk.{}.ffn_norm.weight", i))?;
            let (gate_shape, w_gate) =
                gguf.read_tensor_f32(&format!("blk.{}.ffn_gate.weight", i))?;
            let (_, w_up) = gguf.read_tensor_f32(&format!("blk.{}.ffn_up.weight", i))?;
            let (_, w_down) = gguf.read_tensor_f32(&format!("blk.{}.ffn_down.weight", i))?;

            if intermediate_size == 0 && !gate_shape.is_empty() {
                intermediate_size = gate_shape[0];
            }

            layers.push(LlamaLayerWeights {
                attn_norm,
                wq,
                wk,
                wv,
                wo,
                ffn_norm,
                w_gate,
                w_up,
                w_down,
            });
        }

        let (_, output_norm) = gguf.read_tensor_f32("output_norm.weight")?;
        let lm_head = if gguf.tensors.contains_key("output.weight") {
            gguf.read_tensor_f32("output.weight")?.1
        } else {
            token_embd.clone()
        };

        let tokenizer = BpeTokenizer::from_gguf(gguf).ok();
        let bos_token_id = gguf
            .meta_u64("tokenizer.ggml.bos_token_id")
            .map(|v| v as u32)
            .or_else(|| tokenizer.as_ref().and_then(|t| t.bos_id));
        let eos_token_id = gguf
            .meta_u64("tokenizer.ggml.eos_token_id")
            .map(|v| v as u32)
            .or_else(|| tokenizer.as_ref().and_then(|t| t.eos_id));

        let kv_dim = num_kv_heads * head_dim;
        let config = LlmConfig {
            architecture: arch,
            vocab_size,
            hidden_size,
            intermediate_size,
            num_layers,
            num_heads,
            num_kv_heads,
            head_dim,
            max_seq_len,
            rms_norm_eps,
            rope_theta,
            bos_token_id,
            eos_token_id,
        };

        Ok(Self {
            cache: KvCache::new(num_layers, kv_dim, max_seq_len),
            config,
            token_embd,
            layers,
            output_norm,
            lm_head,
            tokenizer,
        })
    }

    pub fn from_safetensors(
        weights_path: &str,
        cfg_map: &HashMap<String, Value>,
        tokenizer: Option<BpeTokenizer>,
    ) -> Result<Self, String> {
        let loaded = load_safetensors_file(weights_path)?;

        let get_f32 = |names: &[&str]| -> Result<(Vec<usize>, Vec<f32>), String> {
            for &name in names {
                if let Some(td) = loaded.get(name) {
                    return Ok((
                        td.shape.clone(),
                        td.data.iter().map(|&v| v as f32).collect(),
                    ));
                }
            }
            Err(format!(
                "Safetensors missing expected tensor (tried {:?})",
                names
            ))
        };

        let (emb_shape, token_embd) = get_f32(&["model.embed_tokens.weight", "token_embd.weight"])?;
        if emb_shape.len() != 2 {
            return Err("embed_tokens.weight must be 2D".into());
        }
        let vocab_size = emb_shape[0];
        let hidden_size = emb_shape[1];

        let cfg_usize = |k: &str, default: usize| -> usize {
            match cfg_map.get(k) {
                Some(Value::Int(i)) if *i > 0 => *i as usize,
                _ => default,
            }
        };
        let cfg_f32 = |k: &str, default: f32| -> f32 {
            match cfg_map.get(k) {
                Some(Value::Float(f)) => *f as f32,
                Some(Value::Int(i)) => *i as f32,
                _ => default,
            }
        };

        let num_layers = cfg_usize("num_layers", {
            let mut n = 0;
            while loaded.contains_key(&format!("model.layers.{}.input_layernorm.weight", n))
                || loaded.contains_key(&format!("blk.{}.attn_norm.weight", n))
            {
                n += 1;
            }
            n
        });
        let num_heads = cfg_usize("num_heads", 4);
        let num_kv_heads = cfg_usize("num_kv_heads", num_heads);
        let head_dim = hidden_size / num_heads.max(1);
        let max_seq_len = cfg_usize("max_seq_len", 2048);
        let rms_norm_eps = cfg_f32("rms_norm_eps", 1e-5);
        let rope_theta = cfg_f32("rope_theta", 10000.0);

        let mut intermediate_size = 0usize;
        let mut layers = Vec::with_capacity(num_layers);
        for i in 0..num_layers {
            let (_, attn_norm) = get_f32(&[
                &format!("model.layers.{}.input_layernorm.weight", i),
                &format!("blk.{}.attn_norm.weight", i),
            ])?;
            let (_, wq) = get_f32(&[
                &format!("model.layers.{}.self_attn.q_proj.weight", i),
                &format!("blk.{}.attn_q.weight", i),
            ])?;
            let (_, wk) = get_f32(&[
                &format!("model.layers.{}.self_attn.k_proj.weight", i),
                &format!("blk.{}.attn_k.weight", i),
            ])?;
            let (_, wv) = get_f32(&[
                &format!("model.layers.{}.self_attn.v_proj.weight", i),
                &format!("blk.{}.attn_v.weight", i),
            ])?;
            let (_, wo) = get_f32(&[
                &format!("model.layers.{}.self_attn.o_proj.weight", i),
                &format!("blk.{}.attn_output.weight", i),
            ])?;
            let (_, ffn_norm) = get_f32(&[
                &format!("model.layers.{}.post_attention_layernorm.weight", i),
                &format!("blk.{}.ffn_norm.weight", i),
            ])?;
            let (gate_shape, w_gate) = get_f32(&[
                &format!("model.layers.{}.mlp.gate_proj.weight", i),
                &format!("blk.{}.ffn_gate.weight", i),
            ])?;
            let (_, w_up) = get_f32(&[
                &format!("model.layers.{}.mlp.up_proj.weight", i),
                &format!("blk.{}.ffn_up.weight", i),
            ])?;
            let (_, w_down) = get_f32(&[
                &format!("model.layers.{}.mlp.down_proj.weight", i),
                &format!("blk.{}.ffn_down.weight", i),
            ])?;
            if intermediate_size == 0 && !gate_shape.is_empty() {
                intermediate_size = gate_shape[0];
            }
            layers.push(LlamaLayerWeights {
                attn_norm,
                wq,
                wk,
                wv,
                wo,
                ffn_norm,
                w_gate,
                w_up,
                w_down,
            });
        }

        let (_, output_norm) = get_f32(&["model.norm.weight", "output_norm.weight"])?;
        let lm_head = get_f32(&["lm_head.weight", "output.weight"])
            .map(|(_, d)| d)
            .unwrap_or_else(|_| token_embd.clone());

        let bos_token_id = cfg_map
            .get("bos_token_id")
            .and_then(|v| match v {
                Value::Int(i) if *i >= 0 => Some(*i as u32),
                _ => None,
            })
            .or_else(|| tokenizer.as_ref().and_then(|t| t.bos_id));
        let eos_token_id = cfg_map
            .get("eos_token_id")
            .and_then(|v| match v {
                Value::Int(i) if *i >= 0 => Some(*i as u32),
                _ => None,
            })
            .or_else(|| tokenizer.as_ref().and_then(|t| t.eos_id));

        let kv_dim = num_kv_heads * head_dim;
        let config = LlmConfig {
            architecture: "llama".to_string(),
            vocab_size,
            hidden_size,
            intermediate_size,
            num_layers,
            num_heads,
            num_kv_heads,
            head_dim,
            max_seq_len,
            rms_norm_eps,
            rope_theta,
            bos_token_id,
            eos_token_id,
        };

        Ok(Self {
            cache: KvCache::new(num_layers, kv_dim, max_seq_len),
            config,
            token_embd,
            layers,
            output_norm,
            lm_head,
            tokenizer,
        })
    }

    /// Runs a single token forward pass at `pos`, updating the provided `KvCache`.
    /// Returns `(final_hidden_state, logits)`.
    pub fn forward_token_with_cache(
        &self,
        token_id: u32,
        pos: usize,
        cache: &mut KvCache,
    ) -> Result<(Vec<f32>, Vec<f32>), String> {
        let cfg = &self.config;
        let tid = token_id as usize;
        if tid >= cfg.vocab_size {
            return Err(format!(
                "Token ID {} out of range for vocab_size {}",
                tid, cfg.vocab_size
            ));
        }
        if pos >= cfg.max_seq_len {
            return Err(format!(
                "Sequence position {} exceeds max_seq_len {}",
                pos, cfg.max_seq_len
            ));
        }

        let dim = cfg.hidden_size;
        let kv_dim = cfg.num_kv_heads * cfg.head_dim;
        let kv_mul = (cfg.num_heads / cfg.num_kv_heads.max(1)).max(1);
        let inv_sqrt_hd = 1.0f32 / (cfg.head_dim as f32).sqrt();

        let mut x = self.token_embd[tid * dim..(tid + 1) * dim].to_vec();
        let mut xb = vec![0.0f32; dim];

        for (l_idx, layer) in self.layers.iter().enumerate() {
            // 1. Attention RMSNorm
            rms_norm_into(&x, &layer.attn_norm, cfg.rms_norm_eps, &mut xb);

            // 2. Q, K, V projections
            let mut q = matvec(&layer.wq, &xb, cfg.num_heads * cfg.head_dim, dim);
            let mut k = matvec(&layer.wk, &xb, kv_dim, dim);
            let v = matvec(&layer.wv, &xb, kv_dim, dim);

            // 3. RoPE (Rotary Positional Embedding)
            apply_rope(&mut q, cfg.num_heads, cfg.head_dim, pos, cfg.rope_theta);
            apply_rope(&mut k, cfg.num_kv_heads, cfg.head_dim, pos, cfg.rope_theta);

            // 4. Append K, V into layer KV-Cache
            cache.k_layers[l_idx].extend_from_slice(&k);
            cache.v_layers[l_idx].extend_from_slice(&v);
            let total_steps = pos + 1;
            let k_hist = &cache.k_layers[l_idx];
            let v_hist = &cache.v_layers[l_idx];

            // 5. Grouped-Query Attention (GQA)
            let mut attn_concat = vec![0.0f32; cfg.num_heads * cfg.head_dim];
            let mut scores = vec![0.0f32; total_steps];

            for h in 0..cfg.num_heads {
                let q_head = &q[h * cfg.head_dim..(h + 1) * cfg.head_dim];
                let kv_h = h / kv_mul;
                let mut max_score = f32::NEG_INFINITY;

                for (t, score_slot) in scores.iter_mut().enumerate().take(total_steps) {
                    let k_off = t * kv_dim + kv_h * cfg.head_dim;
                    let k_head = &k_hist[k_off..k_off + cfg.head_dim];
                    let mut dot = 0.0f32;
                    for d in 0..cfg.head_dim {
                        dot += q_head[d] * k_head[d];
                    }
                    let s = dot * inv_sqrt_hd;
                    *score_slot = s;
                    if s > max_score {
                        max_score = s;
                    }
                }

                // Softmax
                let mut sum_exp = 0.0f32;
                for s in &mut scores {
                    *s = (*s - max_score).exp();
                    sum_exp += *s;
                }
                let inv_sum = 1.0f32 / sum_exp.max(1e-12);
                for s in &mut scores {
                    *s *= inv_sum;
                }

                // Weighted V sum
                let out_head = &mut attn_concat[h * cfg.head_dim..(h + 1) * cfg.head_dim];
                for (t, &w) in scores.iter().enumerate().take(total_steps) {
                    let v_off = t * kv_dim + kv_h * cfg.head_dim;
                    let v_head = &v_hist[v_off..v_off + cfg.head_dim];
                    for d in 0..cfg.head_dim {
                        out_head[d] += w * v_head[d];
                    }
                }
            }

            // 6. Output projection + residual
            let attn_out = matvec(&layer.wo, &attn_concat, dim, cfg.num_heads * cfg.head_dim);
            for i in 0..dim {
                x[i] += attn_out[i];
            }

            // 7. FFN RMSNorm + SwiGLU
            rms_norm_into(&x, &layer.ffn_norm, cfg.rms_norm_eps, &mut xb);
            let mut gate = matvec(&layer.w_gate, &xb, cfg.intermediate_size, dim);
            let up = matvec(&layer.w_up, &xb, cfg.intermediate_size, dim);
            for i in 0..cfg.intermediate_size {
                let g = gate[i];
                let silu = g / (1.0 + (-g).exp());
                gate[i] = silu * up[i];
            }
            let ffn_out = matvec(&layer.w_down, &gate, dim, cfg.intermediate_size);
            for i in 0..dim {
                x[i] += ffn_out[i];
            }
        }

        cache.seq_len = pos + 1;

        // Final norm + LM head
        let mut hidden = vec![0.0f32; dim];
        rms_norm_into(&x, &self.output_norm, cfg.rms_norm_eps, &mut hidden);
        let logits = matvec(&self.lm_head, &hidden, cfg.vocab_size, dim);
        Ok((hidden, logits))
    }

    pub fn forward_sequence_logits(&mut self, token_ids: &[u32]) -> Result<Vec<f32>, String> {
        if token_ids.is_empty() {
            return Err("forward_logits requires at least 1 token ID".into());
        }
        let mut cache = self.cache.clone();
        let mut last_logits = Vec::new();
        for &tok in token_ids {
            let pos = cache.seq_len;
            let (_, logits) = self.forward_token_with_cache(tok, pos, &mut cache)?;
            last_logits = logits;
        }
        self.cache = cache;
        Ok(last_logits)
    }

    /// Computes a dense L2-normalized Transformer embedding vector from token IDs.
    pub fn embed_tokens(&self, token_ids: &[u32], pooling: &str) -> Result<Vec<f64>, String> {
        if token_ids.is_empty() {
            return Err("embed requires at least 1 token".into());
        }
        let kv_dim = self.config.num_kv_heads * self.config.head_dim;
        let mut tmp_cache = KvCache::new(
            self.config.num_layers,
            kv_dim,
            token_ids.len().max(self.config.max_seq_len),
        );
        let mut states = Vec::with_capacity(token_ids.len());
        for (pos, &tok) in token_ids.iter().enumerate() {
            let (hidden, _) = self.forward_token_with_cache(tok, pos, &mut tmp_cache)?;
            states.push(hidden);
        }

        let dim = self.config.hidden_size;
        let mut pooled = vec![0.0f64; dim];
        match pooling.to_lowercase().as_str() {
            "last" => {
                if let Some(last) = states.last() {
                    for (dst, &src) in pooled.iter_mut().zip(last.iter()) {
                        *dst = src as f64;
                    }
                }
            }
            "cls" | "first" => {
                if let Some(first) = states.first() {
                    for (dst, &src) in pooled.iter_mut().zip(first.iter()) {
                        *dst = src as f64;
                    }
                }
            }
            _ => {
                // Mean pooling
                let inv_n = 1.0 / (states.len() as f64);
                for st in &states {
                    for (dst, &src) in pooled.iter_mut().zip(st.iter()) {
                        *dst += (src as f64) * inv_n;
                    }
                }
            }
        }

        let norm = pooled.iter().map(|&v| v * v).sum::<f64>().sqrt();
        if norm > 1e-12 {
            for v in &mut pooled {
                *v /= norm;
            }
        }
        Ok(pooled)
    }

    pub fn generate_tokens(
        &mut self,
        prompt_tokens: &[u32],
        opts: &GenerateOptions,
    ) -> Result<(Vec<u32>, f64), String> {
        if prompt_tokens.is_empty() {
            return Err("generate requires non-empty prompt tokens".into());
        }
        if opts.reset_cache {
            self.cache.clear();
        }

        let start = Instant::now();
        let mut cache = self.cache.clone();
        let mut logits = Vec::new();
        for &tok in prompt_tokens {
            let pos = cache.seq_len;
            let (_, lg) = self.forward_token_with_cache(tok, pos, &mut cache)?;
            logits = lg;
        }

        let mut rng_state = if opts.seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            opts.seed
        };
        let mut history: Vec<u32> = prompt_tokens.to_vec();
        let mut generated: Vec<u32> = Vec::with_capacity(opts.max_new_tokens);

        for _ in 0..opts.max_new_tokens {
            let next_tok = sample_logits(
                &mut logits,
                &history,
                opts.temperature,
                opts.top_k,
                opts.top_p,
                opts.rep_penalty,
                &mut rng_state,
            );
            generated.push(next_tok);
            history.push(next_tok);

            if Some(next_tok) == self.config.eos_token_id {
                break;
            }
            if cache.seq_len >= self.config.max_seq_len {
                break;
            }
            let pos = cache.seq_len;
            let (_, lg) = self.forward_token_with_cache(next_tok, pos, &mut cache)?;
            logits = lg;
        }

        self.cache = cache;
        let elapsed_secs = start.elapsed().as_secs_f64().max(1e-9);
        let tps = (generated.len() as f64) / elapsed_secs;
        Ok((generated, tps))
    }
}

#[inline]
fn rms_norm_into(x: &[f32], weight: &[f32], eps: f32, out: &mut [f32]) {
    let n = x.len();
    let ss: f32 = x.iter().map(|&v| v * v).sum::<f32>() / (n as f32);
    let inv = 1.0f32 / (ss + eps).sqrt();
    for i in 0..n {
        let w = weight.get(i).copied().unwrap_or(1.0);
        out[i] = x[i] * inv * w;
    }
}

#[inline]
fn apply_rope(vec: &mut [f32], n_heads: usize, head_dim: usize, pos: usize, theta: f32) {
    let half = head_dim / 2;
    let pos_f = pos as f32;
    for h in 0..n_heads {
        let base = h * head_dim;
        for i in 0..half {
            let freq = 1.0f32 / theta.powf((2 * i) as f32 / (head_dim as f32));
            let angle = pos_f * freq;
            let cos = angle.cos();
            let sin = angle.sin();
            let x0 = vec[base + 2 * i];
            let x1 = vec[base + 2 * i + 1];
            vec[base + 2 * i] = x0 * cos - x1 * sin;
            vec[base + 2 * i + 1] = x0 * sin + x1 * cos;
        }
    }
}

fn matvec(w: &[f32], x: &[f32], rows: usize, cols: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; rows];
    let work = rows.saturating_mul(cols);
    if work >= 131_072 && rows >= 32 {
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .min(rows / 8)
            .max(1);
        let chunk_rows = rows.div_ceil(threads);
        std::thread::scope(|s| {
            for (ti, out_chunk) in out.chunks_mut(chunk_rows).enumerate() {
                let row_start = ti * chunk_rows;
                s.spawn(move || {
                    for (local_r, dst) in out_chunk.iter_mut().enumerate() {
                        let r = row_start + local_r;
                        let row = &w[r * cols..(r + 1) * cols];
                        let mut acc = 0.0f32;
                        for c in 0..cols {
                            acc += row[c] * x[c];
                        }
                        *dst = acc;
                    }
                });
            }
        });
    } else {
        for (r, dst) in out.iter_mut().enumerate().take(rows) {
            let row = &w[r * cols..(r + 1) * cols];
            let mut acc = 0.0f32;
            for c in 0..cols {
                acc += row[c] * x[c];
            }
            *dst = acc;
        }
    }
    out
}

fn xorshift64(state: &mut u64) -> f32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    ((x >> 40) as f32) / ((1u64 << 24) as f32)
}

fn sample_logits(
    logits: &mut [f32],
    history: &[u32],
    temperature: f32,
    top_k: usize,
    top_p: f32,
    rep_penalty: f32,
    rng: &mut u64,
) -> u32 {
    if rep_penalty > 1.0 {
        for &tok in history {
            let idx = tok as usize;
            if idx < logits.len() {
                if logits[idx] > 0.0 {
                    logits[idx] /= rep_penalty;
                } else {
                    logits[idx] *= rep_penalty;
                }
            }
        }
    }

    if temperature <= 0.0 {
        let mut best_i = 0usize;
        let mut best_v = f32::NEG_INFINITY;
        for (i, &v) in logits.iter().enumerate() {
            if v > best_v {
                best_v = v;
                best_i = i;
            }
        }
        return best_i as u32;
    }

    let inv_t = 1.0 / temperature;
    let mut indexed: Vec<(usize, f32)> = logits
        .iter()
        .enumerate()
        .map(|(i, &v)| (i, v * inv_t))
        .collect();
    indexed.sort_by(|a, b| b.1.total_cmp(&a.1));

    if top_k > 0 && top_k < indexed.len() {
        indexed.truncate(top_k);
    }

    let max_v = indexed.first().map(|x| x.1).unwrap_or(0.0);
    let mut probs: Vec<(usize, f32)> = indexed
        .iter()
        .map(|&(i, v)| (i, (v - max_v).exp()))
        .collect();
    let total: f32 = probs.iter().map(|x| x.1).sum::<f32>().max(1e-12);
    for p in &mut probs {
        p.1 /= total;
    }

    if top_p > 0.0 && top_p < 1.0 {
        let mut cum = 0.0f32;
        let mut keep = 0usize;
        for &(_, p) in &probs {
            cum += p;
            keep += 1;
            if cum >= top_p {
                break;
            }
        }
        probs.truncate(keep.max(1));
        let renorm: f32 = probs.iter().map(|x| x.1).sum::<f32>().max(1e-12);
        for p in &mut probs {
            p.1 /= renorm;
        }
    }

    let u = xorshift64(rng);
    let mut acc = 0.0f32;
    for &(idx, p) in &probs {
        acc += p;
        if u <= acc {
            return idx as u32;
        }
    }
    probs.last().map(|x| x.0 as u32).unwrap_or(0)
}

// ==============================================================================
// 5. Global Registries (Tokenizers, GGUF Files, Local LLM Models)
// ==============================================================================

static NEXT_LLM_HANDLE: AtomicU64 = AtomicU64::new(1);
static TOKENIZERS: OnceLock<Mutex<HashMap<u64, Arc<BpeTokenizer>>>> = OnceLock::new();
static GGUF_FILES: OnceLock<Mutex<HashMap<u64, Arc<GgufFile>>>> = OnceLock::new();
static LLM_MODELS: OnceLock<Mutex<HashMap<u64, Arc<Mutex<LocalLlm>>>>> = OnceLock::new();

fn tokenizers() -> &'static Mutex<HashMap<u64, Arc<BpeTokenizer>>> {
    TOKENIZERS.get_or_init(|| Mutex::new(HashMap::new()))
}
fn gguf_files() -> &'static Mutex<HashMap<u64, Arc<GgufFile>>> {
    GGUF_FILES.get_or_init(|| Mutex::new(HashMap::new()))
}
fn llm_models() -> &'static Mutex<HashMap<u64, Arc<Mutex<LocalLlm>>>> {
    LLM_MODELS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_id() -> u64 {
    NEXT_LLM_HANDLE.fetch_add(1, Ordering::SeqCst)
}

fn get_tokenizer(id: u64) -> Result<Arc<BpeTokenizer>, String> {
    tokenizers()
        .lock()
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("Tokenizer handle {} not found", id))
}

fn get_gguf(id: u64) -> Result<Arc<GgufFile>, String> {
    gguf_files()
        .lock()
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("GGUF handle {} not found", id))
}

fn get_llm(id: u64) -> Result<Arc<Mutex<LocalLlm>>, String> {
    llm_models()
        .lock()
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("LocalLLM handle {} not found", id))
}

fn extract_u64_id(val: &Value, label: &str) -> Result<u64, String> {
    match val {
        Value::Int(i) if *i > 0 => Ok(*i as u64),
        _ => Err(format!("Expected positive integer handle for {}", label)),
    }
}

fn extract_u32_vec(val: &Value) -> Result<Vec<u32>, String> {
    match val {
        Value::Array(arr) => {
            let items = arr.lock();
            let mut out = Vec::with_capacity(items.len());
            for v in items.iter() {
                match v {
                    Value::Int(i) if *i >= 0 => out.push(*i as u32),
                    _ => return Err("Token ID array must contain non-negative integers".into()),
                }
            }
            Ok(out)
        }
        _ => Err("Expected array of integer token IDs".into()),
    }
}

// ==============================================================================
// 6. Native Module Registration (`LLM` & `__native_llm`)
// ==============================================================================

pub fn register_llm_module(globals: &mut HashMap<String, Value>) {
    let mut llm_mod = HashMap::new();

    // --- BPE Tokenizer APIs ---
    llm_mod.insert(
        "bpe_train".to_string(),
        Value::Native("LLM.bpe_train".into(), |args| {
            if args.len() < 2 {
                return Err(
                    "LLM.bpe_train(texts_array, vocab_size, special_tokens?) requires at least 2 args"
                        .into(),
                );
            }
            let texts: Vec<String> = match &args[0] {
                Value::Array(arr) => arr.lock().iter().map(|v| format!("{}", v)).collect(),
                Value::String(s) => vec![s.to_string()],
                _ => return Err("texts must be an Array of strings or a String".into()),
            };
            let vocab_size = match args[1] {
                Value::Int(i) if i >= 256 => i as usize,
                _ => return Err("vocab_size must be an integer >= 256".into()),
            };
            let special: Vec<String> = if args.len() > 2 && args[2] != Value::Nil {
                match &args[2] {
                    Value::Array(arr) => arr.lock().iter().map(|v| format!("{}", v)).collect(),
                    _ => return Err("special_tokens must be an Array of strings".into()),
                }
            } else {
                vec!["<|bos|>".into(), "<|eos|>".into(), "<|unk|>".into()]
            };

            let tok = BpeTokenizer::train(&texts, vocab_size, &special);
            let id = next_id();
            tokenizers().lock().insert(id, Arc::new(tok));
            Ok(Value::Int(id as i64))
        }),
    );

    llm_mod.insert(
        "bpe_load".to_string(),
        Value::Native("LLM.bpe_load".into(), |args| {
            if args.is_empty() {
                return Err("LLM.bpe_load(path) requires a file path".into());
            }
            let path = format!("{}", args[0]);
            crate::vm::interpreter::with_current_vm(|vm| vm.security_policy.check_fs_read(&path))?;
            let raw = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read tokenizer '{}': {}", path, e))?;
            let tok = BpeTokenizer::from_json_str(&raw)?;
            let id = next_id();
            tokenizers().lock().insert(id, Arc::new(tok));
            Ok(Value::Int(id as i64))
        }),
    );

    llm_mod.insert(
        "bpe_save".to_string(),
        Value::Native("LLM.bpe_save".into(), |args| {
            if args.len() < 2 {
                return Err("LLM.bpe_save(tok_id, path) requires 2 arguments".into());
            }
            let tok_id = extract_u64_id(&args[0], "tok_id")?;
            let path = format!("{}", args[1]);
            crate::vm::interpreter::with_current_vm(|vm| vm.security_policy.check_fs_write(&path))?;
            let tok = get_tokenizer(tok_id)?;
            let json = tok.to_json_string()?;
            std::fs::write(&path, json)
                .map_err(|e| format!("Failed to save tokenizer '{}': {}", path, e))?;
            Ok(Value::Bool(true))
        }),
    );

    llm_mod.insert(
        "bpe_from_gguf".to_string(),
        Value::Native("LLM.bpe_from_gguf".into(), |args| {
            if args.is_empty() {
                return Err("LLM.bpe_from_gguf(gguf_id) requires gguf_id".into());
            }
            let gid = extract_u64_id(&args[0], "gguf_id")?;
            let gguf = get_gguf(gid)?;
            let tok = BpeTokenizer::from_gguf(&gguf)?;
            let id = next_id();
            tokenizers().lock().insert(id, Arc::new(tok));
            Ok(Value::Int(id as i64))
        }),
    );

    llm_mod.insert(
        "bpe_encode".to_string(),
        Value::Native("LLM.bpe_encode".into(), |args| {
            if args.len() < 2 {
                return Err("LLM.bpe_encode(tok_id, text, add_special?) requires 2 args".into());
            }
            let tid = extract_u64_id(&args[0], "tok_id")?;
            let text = format!("{}", args[1]);
            let add_special = args
                .get(2)
                .and_then(|v| match v {
                    Value::Bool(b) => Some(*b),
                    _ => None,
                })
                .unwrap_or(false);
            let tok = get_tokenizer(tid)?;
            let ids = tok.encode(&text, add_special);
            Ok(Value::array(
                ids.into_iter().map(|i| Value::Int(i as i64)).collect(),
            ))
        }),
    );

    llm_mod.insert(
        "bpe_decode".to_string(),
        Value::Native("LLM.bpe_decode".into(), |args| {
            if args.len() < 2 {
                return Err("LLM.bpe_decode(tok_id, ids, skip_special?) requires 2 args".into());
            }
            let tid = extract_u64_id(&args[0], "tok_id")?;
            let ids = extract_u32_vec(&args[1])?;
            let skip_special = args
                .get(2)
                .and_then(|v| match v {
                    Value::Bool(b) => Some(*b),
                    _ => None,
                })
                .unwrap_or(true);
            let tok = get_tokenizer(tid)?;
            Ok(Value::string(tok.decode(&ids, skip_special)))
        }),
    );

    llm_mod.insert(
        "bpe_vocab_size".to_string(),
        Value::Native("LLM.bpe_vocab_size".into(), |args| {
            if args.is_empty() {
                return Err("LLM.bpe_vocab_size(tok_id) requires tok_id".into());
            }
            let tid = extract_u64_id(&args[0], "tok_id")?;
            let tok = get_tokenizer(tid)?;
            Ok(Value::Int(tok.id_to_token.len() as i64))
        }),
    );

    // --- GGUF Binary Reader / Quantized Writer APIs ---
    llm_mod.insert(
        "load_gguf".to_string(),
        Value::Native("LLM.load_gguf".into(), |args| {
            if args.is_empty() {
                return Err("LLM.load_gguf(path) requires path".into());
            }
            let path = format!("{}", args[0]);
            crate::vm::interpreter::with_current_vm(|vm| vm.security_policy.check_fs_read(&path))?;
            let gguf = GgufFile::open(&path)?;
            let id = next_id();
            gguf_files().lock().insert(id, Arc::new(gguf));
            Ok(Value::Int(id as i64))
        }),
    );

    llm_mod.insert(
        "save_gguf".to_string(),
        Value::Native("LLM.save_gguf".into(), |args| {
            if args.len() < 3 {
                return Err(
                    "LLM.save_gguf(path, metadata_map, tensors_map, quant_mode?) requires 3 args"
                        .into(),
                );
            }
            let path = format!("{}", args[0]);
            crate::vm::interpreter::with_current_vm(|vm| vm.security_policy.check_fs_write(&path))?;

            let mut meta = BTreeMap::new();
            match &args[1] {
                Value::Map(m) => {
                    let entries: Vec<(String, Value)> = m
                        .lock()
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    for (k, v) in entries {
                        meta.insert(k, aether_value_to_gguf_meta(&v)?);
                    }
                }
                _ => return Err("metadata_map must be a Map".into()),
            }

            let mut tensors = BTreeMap::new();
            match &args[2] {
                Value::Map(m) => {
                    let entries: Vec<(String, Value)> = m
                        .lock()
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    for (k, v) in entries {
                        let td = get_tensor_data(&v)?;
                        tensors.insert(k, td);
                    }
                }
                _ => return Err("tensors_map must be a Map of name -> Tensor".into()),
            }

            let quant_mode = args
                .get(3)
                .filter(|v| !matches!(v, Value::Nil))
                .map(|v| format!("{}", v))
                .unwrap_or_else(|| "F32".to_string());

            save_gguf_file(&path, &meta, &tensors, &quant_mode)?;
            Ok(Value::Bool(true))
        }),
    );

    llm_mod.insert(
        "gguf_metadata".to_string(),
        Value::Native("LLM.gguf_metadata".into(), |args| {
            if args.is_empty() {
                return Err("LLM.gguf_metadata(gguf_id) requires gguf_id".into());
            }
            let gid = extract_u64_id(&args[0], "gguf_id")?;
            let gguf = get_gguf(gid)?;
            let mut out = HashMap::new();
            for (k, v) in &gguf.metadata {
                out.insert(k.clone(), v.to_aether_value());
            }
            Ok(Value::map(out))
        }),
    );

    llm_mod.insert(
        "gguf_tensor_names".to_string(),
        Value::Native("LLM.gguf_tensor_names".into(), |args| {
            if args.is_empty() {
                return Err("LLM.gguf_tensor_names(gguf_id) requires gguf_id".into());
            }
            let gid = extract_u64_id(&args[0], "gguf_id")?;
            let gguf = get_gguf(gid)?;
            Ok(Value::array(
                gguf.tensors
                    .keys()
                    .map(|k| Value::string(k.clone()))
                    .collect(),
            ))
        }),
    );

    llm_mod.insert(
        "gguf_tensor_info".to_string(),
        Value::Native("LLM.gguf_tensor_info".into(), |args| {
            if args.len() < 2 {
                return Err("LLM.gguf_tensor_info(gguf_id, name) requires 2 arguments".into());
            }
            let gid = extract_u64_id(&args[0], "gguf_id")?;
            let name = format!("{}", args[1]);
            let gguf = get_gguf(gid)?;
            let info = gguf
                .tensors
                .get(&name)
                .ok_or_else(|| format!("Tensor '{}' not found in GGUF", name))?;
            let mut m = HashMap::new();
            m.insert("name".to_string(), Value::string(info.name.clone()));
            m.insert(
                "shape".to_string(),
                Value::array(
                    info.row_major_shape
                        .iter()
                        .map(|&s| Value::Int(s as i64))
                        .collect(),
                ),
            );
            m.insert(
                "dtype".to_string(),
                Value::string(ggml_type_name(info.ggml_type).to_string()),
            );
            m.insert("type_id".to_string(), Value::Int(info.ggml_type as i64));
            m.insert("offset".to_string(), Value::Int(info.offset as i64));
            Ok(Value::map(m))
        }),
    );

    llm_mod.insert(
        "gguf_read_tensor".to_string(),
        Value::Native("LLM.gguf_read_tensor".into(), |args| {
            if args.len() < 2 {
                return Err("LLM.gguf_read_tensor(gguf_id, name) requires 2 arguments".into());
            }
            let gid = extract_u64_id(&args[0], "gguf_id")?;
            let name = format!("{}", args[1]);
            let gguf = get_gguf(gid)?;
            let (shape, floats) = gguf.read_tensor_f32(&name)?;
            let f64_data: Vec<f64> = floats.into_iter().map(|v| v as f64).collect();
            let td = TensorData::new(shape, f64_data)?;
            Ok(wrap_tensor_handle(td))
        }),
    );

    // --- Local Transformer / Llama LLM Inference & Embeddings ---
    llm_mod.insert(
        "load_model_gguf".to_string(),
        Value::Native("LLM.load_model_gguf".into(), |args| {
            if args.is_empty() {
                return Err("LLM.load_model_gguf(path_or_gguf_id, tok_id?) requires 1 arg".into());
            }
            let gguf = match &args[0] {
                Value::Int(i) if *i > 0 => get_gguf(*i as u64)?,
                other => {
                    let path = format!("{}", other);
                    crate::vm::interpreter::with_current_vm(|vm| {
                        vm.security_policy.check_fs_read(&path)
                    })?;
                    Arc::new(GgufFile::open(&path)?)
                }
            };
            let mut model = LocalLlm::from_gguf(&gguf)?;
            if let Some(Value::Int(tid)) = args.get(1) {
                if *tid > 0 {
                    model.tokenizer = Some((*get_tokenizer(*tid as u64)?).clone());
                }
            }
            let id = next_id();
            llm_models().lock().insert(id, Arc::new(Mutex::new(model)));
            Ok(Value::Int(id as i64))
        }),
    );

    llm_mod.insert(
        "load_model_safetensors".to_string(),
        Value::Native("LLM.load_model_safetensors".into(), |args| {
            if args.len() < 2 {
                return Err(
                    "LLM.load_model_safetensors(path, config_map, tok_id?) requires 2 args".into(),
                );
            }
            let path = format!("{}", args[0]);
            crate::vm::interpreter::with_current_vm(|vm| vm.security_policy.check_fs_read(&path))?;
            let cfg_map = match &args[1] {
                Value::Map(m) => m.lock().clone(),
                _ => return Err("config_map must be a Map".into()),
            };
            let tok = match args.get(2) {
                Some(Value::Int(tid)) if *tid > 0 => Some((*get_tokenizer(*tid as u64)?).clone()),
                _ => None,
            };
            let model = LocalLlm::from_safetensors(&path, &cfg_map, tok)?;
            let id = next_id();
            llm_models().lock().insert(id, Arc::new(Mutex::new(model)));
            Ok(Value::Int(id as i64))
        }),
    );

    llm_mod.insert(
        "model_info".to_string(),
        Value::Native("LLM.model_info".into(), |args| {
            if args.is_empty() {
                return Err("LLM.model_info(model_id) requires model_id".into());
            }
            let mid = extract_u64_id(&args[0], "model_id")?;
            let arc = get_llm(mid)?;
            let model = arc.lock();
            let c = &model.config;
            let mut m = HashMap::new();
            m.insert(
                "architecture".to_string(),
                Value::string(c.architecture.clone()),
            );
            m.insert("vocab_size".to_string(), Value::Int(c.vocab_size as i64));
            m.insert("hidden_size".to_string(), Value::Int(c.hidden_size as i64));
            m.insert(
                "intermediate_size".to_string(),
                Value::Int(c.intermediate_size as i64),
            );
            m.insert("num_layers".to_string(), Value::Int(c.num_layers as i64));
            m.insert("num_heads".to_string(), Value::Int(c.num_heads as i64));
            m.insert(
                "num_kv_heads".to_string(),
                Value::Int(c.num_kv_heads as i64),
            );
            m.insert("head_dim".to_string(), Value::Int(c.head_dim as i64));
            m.insert("max_seq_len".to_string(), Value::Int(c.max_seq_len as i64));
            m.insert("rope_theta".to_string(), Value::Float(c.rope_theta as f64));
            m.insert(
                "cached_tokens".to_string(),
                Value::Int(model.cache.seq_len as i64),
            );
            m.insert(
                "has_tokenizer".to_string(),
                Value::Bool(model.tokenizer.is_some()),
            );
            Ok(Value::map(m))
        }),
    );

    llm_mod.insert(
        "reset_cache".to_string(),
        Value::Native("LLM.reset_cache".into(), |args| {
            if args.is_empty() {
                return Err("LLM.reset_cache(model_id) requires model_id".into());
            }
            let mid = extract_u64_id(&args[0], "model_id")?;
            let arc = get_llm(mid)?;
            arc.lock().cache.clear();
            Ok(Value::Bool(true))
        }),
    );

    llm_mod.insert(
        "forward_logits".to_string(),
        Value::Native("LLM.forward_logits".into(), |args| {
            if args.len() < 2 {
                return Err("LLM.forward_logits(model_id, token_ids) requires 2 arguments".into());
            }
            let mid = extract_u64_id(&args[0], "model_id")?;
            let tokens = extract_u32_vec(&args[1])?;
            let arc = get_llm(mid)?;
            let logits = arc.lock().forward_sequence_logits(&tokens)?;
            Ok(Value::array(
                logits.into_iter().map(|v| Value::Float(v as f64)).collect(),
            ))
        }),
    );

    llm_mod.insert(
        "generate".to_string(),
        Value::Native("LLM.generate".into(), |args| {
            if args.len() < 2 {
                return Err(
                    "LLM.generate(model_id, prompt_or_tokens, opts_map?) requires 2 args".into(),
                );
            }
            let mid = extract_u64_id(&args[0], "model_id")?;
            let opts = match args.get(2) {
                Some(Value::Map(m)) => m.lock().clone(),
                _ => HashMap::new(),
            };
            let max_tokens = match opts.get("max_tokens") {
                Some(Value::Int(i)) if *i > 0 => *i as usize,
                _ => 32,
            };
            let temperature = match opts.get("temperature") {
                Some(Value::Float(f)) => *f as f32,
                Some(Value::Int(i)) => *i as f32,
                _ => 0.0,
            };
            let top_k = match opts.get("top_k") {
                Some(Value::Int(i)) if *i > 0 => *i as usize,
                _ => 40,
            };
            let top_p = match opts.get("top_p") {
                Some(Value::Float(f)) => *f as f32,
                _ => 0.95,
            };
            let rep_penalty = match opts.get("repetition_penalty") {
                Some(Value::Float(f)) => *f as f32,
                _ => 1.0,
            };
            let seed = match opts.get("seed") {
                Some(Value::Int(i)) => *i as u64,
                _ => 42,
            };
            let reset_cache = match opts.get("reset_cache") {
                Some(Value::Bool(b)) => *b,
                _ => true,
            };

            let arc = get_llm(mid)?;
            let mut model = arc.lock();

            let prompt_tokens = match &args[1] {
                Value::Array(_) => extract_u32_vec(&args[1])?,
                Value::String(s) => {
                    let tok = model.tokenizer.as_ref().ok_or_else(|| {
                        "Model has no attached tokenizer; pass token IDs or attach a tokenizer"
                            .to_string()
                    })?;
                    let mut ids = tok.encode(s, false);
                    if ids.is_empty() {
                        if let Some(bos) = model.config.bos_token_id {
                            ids.push(bos);
                        } else {
                            ids.push(0);
                        }
                    }
                    ids
                }
                _ => return Err("prompt must be String or Array of token IDs".into()),
            };

            let prompt_len = prompt_tokens.len();
            let gen_opts = GenerateOptions {
                max_new_tokens: max_tokens,
                temperature,
                top_k,
                top_p,
                rep_penalty,
                seed,
                reset_cache,
            };
            let (gen_tokens, tps) = model.generate_tokens(&prompt_tokens, &gen_opts)?;

            let decoded = model
                .tokenizer
                .as_ref()
                .map(|t| t.decode(&gen_tokens, true))
                .unwrap_or_default();

            let mut res = HashMap::new();
            res.insert("text".to_string(), Value::string(decoded));
            res.insert(
                "tokens".to_string(),
                Value::array(gen_tokens.iter().map(|&t| Value::Int(t as i64)).collect()),
            );
            res.insert("prompt_tokens".to_string(), Value::Int(prompt_len as i64));
            res.insert(
                "generated_tokens".to_string(),
                Value::Int(gen_tokens.len() as i64),
            );
            res.insert("tokens_per_sec".to_string(), Value::Float(tps));
            Ok(Value::map(res))
        }),
    );

    llm_mod.insert(
        "embed".to_string(),
        Value::Native("LLM.embed".into(), |args| {
            if args.len() < 2 {
                return Err(
                    "LLM.embed(model_id, text_or_tokens, pooling?) requires at least 2 args".into(),
                );
            }
            let mid = extract_u64_id(&args[0], "model_id")?;
            let pooling = args
                .get(2)
                .filter(|v| !matches!(v, Value::Nil))
                .map(|v| format!("{}", v))
                .unwrap_or_else(|| "mean".to_string());

            let arc = get_llm(mid)?;
            let model = arc.lock();
            let tokens = match &args[1] {
                Value::Array(_) => extract_u32_vec(&args[1])?,
                Value::String(s) => {
                    let tok = model.tokenizer.as_ref().ok_or_else(|| {
                        "Model has no attached tokenizer; pass token IDs or attach a tokenizer"
                            .to_string()
                    })?;
                    let mut ids = tok.encode(s, false);
                    if ids.is_empty() {
                        ids.push(0);
                    }
                    ids
                }
                _ => return Err("embed input must be String or Array of token IDs".into()),
            };

            let vec = model.embed_tokens(&tokens, &pooling)?;
            Ok(Value::array(vec.into_iter().map(Value::Float).collect()))
        }),
    );

    llm_mod.insert(
        "cosine_similarity".to_string(),
        Value::Native("LLM.cosine_similarity".into(), |args| {
            if args.len() < 2 {
                return Err("LLM.cosine_similarity(a, b) requires 2 vectors".into());
            }
            let extract_f64 = |v: &Value| -> Result<Vec<f64>, String> {
                match v {
                    Value::Array(arr) => Ok(arr
                        .lock()
                        .iter()
                        .map(|x| match x {
                            Value::Float(f) => *f,
                            Value::Int(i) => *i as f64,
                            _ => 0.0,
                        })
                        .collect()),
                    _ => Err("cosine_similarity arguments must be numeric arrays".into()),
                }
            };
            let a = extract_f64(&args[0])?;
            let b = extract_f64(&args[1])?;
            if a.len() != b.len() || a.is_empty() {
                return Err("Vectors must be non-empty and of equal length".into());
            }
            let mut dot = 0.0f64;
            let mut na = 0.0f64;
            let mut nb = 0.0f64;
            for i in 0..a.len() {
                dot += a[i] * b[i];
                na += a[i] * a[i];
                nb += b[i] * b[i];
            }
            let denom = (na.sqrt() * nb.sqrt()).max(1e-12);
            Ok(Value::Float(dot / denom))
        }),
    );

    globals.insert("LLM".to_string(), Value::map(llm_mod.clone()));
    globals.insert("__native_llm".to_string(), Value::map(llm_mod));
}
