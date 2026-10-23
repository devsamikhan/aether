use super::value::Value;
use std::collections::HashMap;

// =========================================================================
// Pure Rust Zero-Dependency SHA-256 Implementation (FIPS 180-4)
// =========================================================================

const K256: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

pub fn sha256_bytes(input: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    let bit_len = (input.len() as u64) * 8;
    let mut msg = input.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[i * 4], chunk[i * 4 + 1], chunk[i * 4 + 2], chunk[i * 4 + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_var = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_var
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K256[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_var = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_var);
    }

    let mut out = [0u8; 32];
    for (i, val) in h.iter().enumerate() {
        let bytes = val.to_be_bytes();
        out[i * 4..i * 4 + 4].copy_from_slice(&bytes);
    }
    out
}

pub fn sha256(input: &[u8]) -> String {
    let bytes = sha256_bytes(input);
    let mut s = String::with_capacity(64);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

// =========================================================================
// Pure Rust Zero-Dependency SHA-1 Implementation (FIPS 180-1 / RFC 3174)
// =========================================================================

pub fn sha1_bytes(input: &[u8]) -> [u8; 20] {
    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xefcdab89;
    let mut h2: u32 = 0x98badcfe;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xc3d2e1f0;

    let bit_len = (input.len() as u64) * 8;
    let mut msg = input.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[i * 4], chunk[i * 4 + 1], chunk[i * 4 + 2], chunk[i * 4 + 3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;

        for i in 0..80 {
            let (f, k) = if i < 20 {
                ((b & c) | ((!b) & d), 0x5a827999u32)
            } else if i < 40 {
                (b ^ c ^ d, 0x6ed9eba1u32)
            } else if i < 60 {
                ((b & c) | (b & d) | (c & d), 0x8f1bbcdcu32)
            } else {
                (b ^ c ^ d, 0xca62c1d6u32)
            };

            let temp = a.rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut out = [0u8; 20];
    out[0..4].copy_from_slice(&h0.to_be_bytes());
    out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes());
    out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
    out
}

pub fn sha1(input: &[u8]) -> String {
    let bytes = sha1_bytes(input);
    let mut s = String::with_capacity(40);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// Computes RFC 6455 Sec-WebSocket-Accept token: base64(sha1(key + GUID))
pub fn sec_websocket_accept(key: &str) -> String {
    const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let concatenated = format!("{}{}", key.trim(), WS_GUID);
    let digest = sha1_bytes(concatenated.as_bytes());
    base64_encode(&digest)
}

// =========================================================================
// Pure Rust Zero-Dependency MD5 Implementation (RFC 1321)
// =========================================================================

pub fn md5(input: &[u8]) -> String {
    let mut a: u32 = 0x67452301;
    let mut b: u32 = 0xefcdab89;
    let mut c: u32 = 0x98badcfe;
    let mut d: u32 = 0x10325476;

    let bit_len = (input.len() as u64) * 8;
    let mut msg = input.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_le_bytes());

    let s = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
        5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
        4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
        6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];

    let k: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
        0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
        0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
        0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
        0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
        0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
        0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
    ];

    for chunk in msg.chunks_exact(64) {
        let mut m = [0u32; 16];
        for i in 0..16 {
            m[i] = u32::from_le_bytes([chunk[i * 4], chunk[i * 4 + 1], chunk[i * 4 + 2], chunk[i * 4 + 3]]);
        }

        let (mut aa, mut bb, mut cc, mut dd) = (a, b, c, d);

        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((bb & cc) | ((!bb) & dd), i),
                16..=31 => ((dd & bb) | ((!dd) & cc), (5 * i + 1) % 16),
                32..=47 => (bb ^ cc ^ dd, (3 * i + 5) % 16),
                _ => (cc ^ (bb | (!dd)), (7 * i) % 16),
            };

            let temp = dd;
            dd = cc;
            cc = bb;
            bb = bb.wrapping_add(
                aa.wrapping_add(f)
                    .wrapping_add(k[i])
                    .wrapping_add(m[g])
                    .rotate_left(s[i]),
            );
            aa = temp;
        }

        a = a.wrapping_add(aa);
        b = b.wrapping_add(bb);
        c = c.wrapping_add(cc);
        d = d.wrapping_add(dd);
    }

    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&a.to_le_bytes());
    out[4..8].copy_from_slice(&b.to_le_bytes());
    out[8..12].copy_from_slice(&c.to_le_bytes());
    out[12..16].copy_from_slice(&d.to_le_bytes());

    let mut res = String::with_capacity(32);
    for b in out {
        res.push_str(&format!("{:02x}", b));
    }
    res
}

// =========================================================================
// Pure Rust Base64 Implementation (RFC 4648)
// =========================================================================

const B64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(input: &[u8]) -> String {
    let mut out = String::new();
    let chunks = input.chunks(3);

    for chunk in chunks {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        out.push(B64_CHARS[(b0 >> 2) as usize] as char);
        out.push(B64_CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);

        if chunk.len() > 1 {
            out.push(B64_CHARS[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }

        if chunk.len() > 2 {
            out.push(B64_CHARS[(b2 & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }

    out
}

pub fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let clean = input.trim().replace(['\r', '\n', ' '], "");
    if clean.is_empty() {
        return Ok(Vec::new());
    }

    let decode_table = |c: u8| -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };

    let mut out = Vec::new();
    let bytes = clean.as_bytes();

    for chunk in bytes.chunks(4) {
        if chunk.len() < 4 {
            return Err("Invalid Base64 chunk length".into());
        }

        let c0 = decode_table(chunk[0]).ok_or_else(|| format!("Invalid Base64 char: {}", chunk[0] as char))?;
        let c1 = decode_table(chunk[1]).ok_or_else(|| format!("Invalid Base64 char: {}", chunk[1] as char))?;

        out.push((c0 << 2) | (c1 >> 4));

        if chunk[2] != b'=' {
            let c2 = decode_table(chunk[2]).ok_or_else(|| format!("Invalid Base64 char: {}", chunk[2] as char))?;
            out.push(((c1 & 0x0F) << 4) | (c2 >> 2));

            if chunk[3] != b'=' {
                let c3 = decode_table(chunk[3]).ok_or_else(|| format!("Invalid Base64 char: {}", chunk[3] as char))?;
                out.push(((c2 & 0x03) << 6) | c3);
            }
        }
    }

    Ok(out)
}

// =========================================================================
// HMAC-SHA256 (RFC 2104)
// =========================================================================

pub fn hmac_sha256_bytes(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut k = [0u8; 64];
    if key.len() > 64 {
        let hash = sha256_bytes(key);
        k[..32].copy_from_slice(&hash);
    } else {
        k[..key.len()].copy_from_slice(key);
    }

    let mut k_ipad = [0u8; 64];
    let mut k_opad = [0u8; 64];
    for i in 0..64 {
        k_ipad[i] = k[i] ^ 0x36;
        k_opad[i] = k[i] ^ 0x5c;
    }

    let mut inner = k_ipad.to_vec();
    inner.extend_from_slice(message);
    let inner_hash = sha256_bytes(&inner);

    let mut outer = k_opad.to_vec();
    outer.extend_from_slice(&inner_hash);
    sha256_bytes(&outer)
}

pub fn hmac_sha256(key: &[u8], message: &[u8]) -> String {
    let outer_hash = hmac_sha256_bytes(key, message);
    let mut res = String::with_capacity(64);
    for b in outer_hash {
        res.push_str(&format!("{:02x}", b));
    }
    res
}

// =========================================================================
// Constant-Time Comparison (Timing Side-Channel Resistant)
// =========================================================================

pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// =========================================================================
// PBKDF2-HMAC-SHA256 Key Derivation (RFC 2898 / NIST SP 800-132)
// =========================================================================

pub fn pbkdf2_hmac_sha256(
    password: &[u8],
    salt: &[u8],
    iterations: u32,
    dk_len: usize,
) -> Vec<u8> {
    let iters = iterations.max(1);
    let blocks = (dk_len + 31) / 32;
    let mut derived = Vec::with_capacity(blocks * 32);

    for block_idx in 1..=(blocks as u32) {
        let mut salt_block = Vec::with_capacity(salt.len() + 4);
        salt_block.extend_from_slice(salt);
        salt_block.extend_from_slice(&block_idx.to_be_bytes());

        let mut u = hmac_sha256_bytes(password, &salt_block);
        let mut t = u;

        for _ in 1..iters {
            u = hmac_sha256_bytes(password, &u);
            for i in 0..32 {
                t[i] ^= u[i];
            }
        }
        derived.extend_from_slice(&t);
    }

    derived.truncate(dk_len);
    derived
}

// =========================================================================
// ChaCha20 Stream Cipher (RFC 8439) + Authenticated AEAD Envelope
// =========================================================================

#[inline]
fn quarter_round(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    state[a] = state[a].wrapping_add(state[b]);
    state[d] ^= state[a];
    state[d] = state[d].rotate_left(16);

    state[c] = state[c].wrapping_add(state[d]);
    state[b] ^= state[c];
    state[b] = state[b].rotate_left(12);

    state[a] = state[a].wrapping_add(state[b]);
    state[d] ^= state[a];
    state[d] = state[d].rotate_left(8);

    state[c] = state[c].wrapping_add(state[d]);
    state[b] ^= state[c];
    state[b] = state[b].rotate_left(7);
}

pub fn chacha20_block(key: &[u8; 32], counter: u32, nonce: &[u8; 12]) -> [u8; 64] {
    let mut state = [0u32; 16];
    state[0] = 0x6170_7865; // "expa"
    state[1] = 0x3320_646e; // "nd 3"
    state[2] = 0x7962_2d32; // "2-by"
    state[3] = 0x6b20_6574; // "te k"

    for i in 0..8 {
        state[4 + i] = u32::from_le_bytes([
            key[i * 4],
            key[i * 4 + 1],
            key[i * 4 + 2],
            key[i * 4 + 3],
        ]);
    }
    state[12] = counter;
    for i in 0..3 {
        state[13 + i] = u32::from_le_bytes([
            nonce[i * 4],
            nonce[i * 4 + 1],
            nonce[i * 4 + 2],
            nonce[i * 4 + 3],
        ]);
    }

    let mut working = state;
    for _ in 0..10 {
        // Column rounds
        quarter_round(&mut working, 0, 4, 8, 12);
        quarter_round(&mut working, 1, 5, 9, 13);
        quarter_round(&mut working, 2, 6, 10, 14);
        quarter_round(&mut working, 3, 7, 11, 15);
        // Diagonal rounds
        quarter_round(&mut working, 0, 5, 10, 15);
        quarter_round(&mut working, 1, 6, 11, 12);
        quarter_round(&mut working, 2, 7, 8, 13);
        quarter_round(&mut working, 3, 4, 9, 14);
    }

    let mut out = [0u8; 64];
    for i in 0..16 {
        let word = working[i].wrapping_add(state[i]);
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    out
}

pub fn chacha20_xor(key: &[u8; 32], initial_counter: u32, nonce: &[u8; 12], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for (block_idx, chunk) in data.chunks(64).enumerate() {
        let keystream = chacha20_block(key, initial_counter.wrapping_add(block_idx as u32), nonce);
        for (i, &b) in chunk.iter().enumerate() {
            out.push(b ^ keystream[i]);
        }
    }
    out
}

fn normalize_key_nonce(key_str: &str, nonce_str: &str) -> ([u8; 32], [u8; 12]) {
    let key = sha256_bytes(key_str.as_bytes());
    let nonce_hash = sha256_bytes(nonce_str.as_bytes());
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&nonce_hash[0..12]);
    (key, nonce)
}

fn compute_aead_tag(key: &[u8; 32], nonce: &[u8; 12], aad: &[u8], ciphertext: &[u8]) -> [u8; 32] {
    let subkey_block = chacha20_block(key, 0, nonce);
    let mut mac_input = Vec::with_capacity(aad.len() + ciphertext.len() + 28);
    mac_input.extend_from_slice(nonce);
    mac_input.extend_from_slice(&(aad.len() as u64).to_le_bytes());
    mac_input.extend_from_slice(aad);
    mac_input.extend_from_slice(&(ciphertext.len() as u64).to_le_bytes());
    mac_input.extend_from_slice(ciphertext);
    hmac_sha256_bytes(&subkey_block[0..32], &mac_input)
}

/// Encrypts `plaintext` with RFC 8439 ChaCha20 and appends a 256-bit authenticated tag,
/// returning a Base64-encoded AEAD token.
pub fn encrypt_aead(key_str: &str, nonce_str: &str, plaintext: &[u8], aad: &[u8]) -> String {
    let (key, nonce) = normalize_key_nonce(key_str, nonce_str);
    let ciphertext = chacha20_xor(&key, 1, &nonce, plaintext);
    let tag = compute_aead_tag(&key, &nonce, aad, &ciphertext);

    let mut payload = Vec::with_capacity(ciphertext.len() + 32);
    payload.extend_from_slice(&ciphertext);
    payload.extend_from_slice(&tag);
    base64_encode(&payload)
}

/// Verifies the AEAD authentication tag in constant time and decrypts the ChaCha20 ciphertext.
pub fn decrypt_aead(
    key_str: &str,
    nonce_str: &str,
    encoded_payload: &str,
    aad: &[u8],
) -> Result<Vec<u8>, String> {
    let payload = base64_decode(encoded_payload)?;
    if payload.len() < 32 {
        return Err("CryptoError: Invalid AEAD payload (shorter than authentication tag)".to_string());
    }
    let ct_len = payload.len() - 32;
    let ciphertext = &payload[0..ct_len];
    let provided_tag = &payload[ct_len..];

    let (key, nonce) = normalize_key_nonce(key_str, nonce_str);
    let expected_tag = compute_aead_tag(&key, &nonce, aad, ciphertext);

    if !constant_time_eq(provided_tag, &expected_tag) {
        return Err("CryptoError: AEAD authentication tag mismatch (tampered ciphertext or wrong key/AAD)".to_string());
    }

    Ok(chacha20_xor(&key, 1, &nonce, ciphertext))
}

pub fn random_hex(byte_len: usize) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let mut out = String::with_capacity(byte_len * 2);
    let mut generated = 0usize;
    while generated < byte_len {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let seq = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let seed = format!("aether-csprng-{}-{}-{:?}", nanos, seq, std::thread::current().id());
        let block = sha256_bytes(seed.as_bytes());
        for &b in &block {
            if generated >= byte_len {
                break;
            }
            out.push_str(&format!("{:02x}", b));
            generated += 1;
        }
    }
    out
}

// =========================================================================
// Registration into Aether Runtime
// =========================================================================

pub fn register_crypto_module(globals: &mut HashMap<String, Value>) {
    let mut crypto_module = HashMap::new();

    crypto_module.insert("sha256".to_string(), Value::Native("Crypto.sha256".into(), |args| {
        if args.is_empty() { return Err("Crypto.sha256(data) expects data argument".into()); }
        let s = format!("{}", args[0]);
        Ok(Value::string(sha256(s.as_bytes())))
    }));

    crypto_module.insert("sha1".to_string(), Value::Native("Crypto.sha1".into(), |args| {
        if args.is_empty() { return Err("Crypto.sha1(data) expects data argument".into()); }
        let s = format!("{}", args[0]);
        Ok(Value::string(sha1(s.as_bytes())))
    }));

    crypto_module.insert("sec_websocket_accept".to_string(), Value::Native("Crypto.sec_websocket_accept".into(), |args| {
        if args.is_empty() { return Err("Crypto.sec_websocket_accept(key) expects key argument".into()); }
        let s = format!("{}", args[0]);
        Ok(Value::string(sec_websocket_accept(&s)))
    }));

    crypto_module.insert("md5".to_string(), Value::Native("Crypto.md5".into(), |args| {
        if args.is_empty() { return Err("Crypto.md5(data) expects data argument".into()); }
        let s = format!("{}", args[0]);
        Ok(Value::string(md5(s.as_bytes())))
    }));

    crypto_module.insert("base64_encode".to_string(), Value::Native("Crypto.base64_encode".into(), |args| {
        if args.is_empty() { return Err("Crypto.base64_encode(data) expects data argument".into()); }
        let s = format!("{}", args[0]);
        Ok(Value::string(base64_encode(s.as_bytes())))
    }));

    crypto_module.insert("base64_decode".to_string(), Value::Native("Crypto.base64_decode".into(), |args| {
        if args.is_empty() { return Err("Crypto.base64_decode(data) expects data string".into()); }
        let s = format!("{}", args[0]);
        let decoded = base64_decode(&s)?;
        let text = String::from_utf8(decoded).map_err(|e| format!("UTF-8 decode error: {}", e))?;
        Ok(Value::string(text))
    }));

    crypto_module.insert("hmac_sha256".to_string(), Value::Native("Crypto.hmac_sha256".into(), |args| {
        if args.len() < 2 { return Err("Crypto.hmac_sha256(key, message) expects 2 arguments".into()); }
        let key = format!("{}", args[0]);
        let msg = format!("{}", args[1]);
        Ok(Value::string(hmac_sha256(key.as_bytes(), msg.as_bytes())))
    }));

    crypto_module.insert("constant_time_eq".to_string(), Value::Native("Crypto.constant_time_eq".into(), |args| {
        if args.len() < 2 { return Err("Crypto.constant_time_eq(a, b) expects 2 arguments".into()); }
        let a = format!("{}", args[0]);
        let b = format!("{}", args[1]);
        Ok(Value::Bool(constant_time_eq(a.as_bytes(), b.as_bytes())))
    }));

    crypto_module.insert("pbkdf2".to_string(), Value::Native("Crypto.pbkdf2".into(), |args| {
        if args.len() < 2 { return Err("Crypto.pbkdf2(password, salt, [iterations], [dk_len]) expects at least 2 arguments".into()); }
        let password = format!("{}", args[0]);
        let salt = format!("{}", args[1]);
        let iterations = if args.len() > 2 {
            match args[2] { Value::Int(i) => i.max(1) as u32, _ => 10_000 }
        } else { 10_000 };
        let dk_len = if args.len() > 3 {
            match args[3] { Value::Int(i) => i.max(1) as usize, _ => 32 }
        } else { 32 };
        let dk = pbkdf2_hmac_sha256(password.as_bytes(), salt.as_bytes(), iterations, dk_len);
        let mut hex = String::with_capacity(dk.len() * 2);
        for b in dk {
            hex.push_str(&format!("{:02x}", b));
        }
        Ok(Value::string(hex))
    }));

    crypto_module.insert("encrypt_aead".to_string(), Value::Native("Crypto.encrypt_aead".into(), |args| {
        if args.len() < 3 { return Err("Crypto.encrypt_aead(key, nonce, plaintext, [aad]) expects at least 3 arguments".into()); }
        let key = format!("{}", args[0]);
        let nonce = format!("{}", args[1]);
        let plaintext = format!("{}", args[2]);
        let aad = if args.len() > 3 { format!("{}", args[3]) } else { String::new() };
        Ok(Value::string(encrypt_aead(&key, &nonce, plaintext.as_bytes(), aad.as_bytes())))
    }));

    crypto_module.insert("decrypt_aead".to_string(), Value::Native("Crypto.decrypt_aead".into(), |args| {
        if args.len() < 3 { return Err("Crypto.decrypt_aead(key, nonce, ciphertext, [aad]) expects at least 3 arguments".into()); }
        let key = format!("{}", args[0]);
        let nonce = format!("{}", args[1]);
        let ct = format!("{}", args[2]);
        let aad = if args.len() > 3 { format!("{}", args[3]) } else { String::new() };
        let plain_bytes = decrypt_aead(&key, &nonce, &ct, aad.as_bytes())?;
        let plain_str = String::from_utf8(plain_bytes).map_err(|e| format!("UTF-8 decode error: {}", e))?;
        Ok(Value::string(plain_str))
    }));

    crypto_module.insert("random_hex".to_string(), Value::Native("Crypto.random_hex".into(), |args| {
        let len = if !args.is_empty() {
            match args[0] { Value::Int(i) => i.clamp(1, 4096) as usize, _ => 32 }
        } else { 32 };
        Ok(Value::string(random_hex(len)))
    }));

    globals.insert("Crypto".to_string(), Value::map(crypto_module));
}
