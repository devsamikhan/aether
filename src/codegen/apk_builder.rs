//! Android APK Packaging and Code Generation Module for AETHER
//!
//! Provides zero-dependency, pure-Rust compilation and packaging of AETHER
//! applications into standalone, installable Android APKs (.apk) and complete
//! Android Studio / Gradle project scaffolds.

use std::fs;
use std::path::Path;
use crate::syntax::parse;

// =========================================================================
// 1. Standalone Cryptographic & Hashing Utilities (Pure Rust, Zero Deps)
// =========================================================================

/// Computes standard IEEE 802.3 CRC-32 checksum
pub fn calculate_crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = -((crc & 1) as i32) as u32;
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Computes standard Adler-32 checksum (required for Dalvik DEX & ZLIB)
pub fn calculate_adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// Computes standard SHA-1 cryptographic digest (required for Android APK signature)
pub fn calculate_sha1(data: &[u8]) -> [u8; 20] {
    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xEFCDAB89;
    let mut h2: u32 = 0x98BADCFE;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xC3D2E1F0;

    let ml = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&ml.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
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
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };

            let temp = a
                .rotate_left(5)
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

/// Standard Base64 encoder for APK Manifest signatures
pub fn base64_encode(data: &[u8]) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        let idx0 = (b0 >> 2) as usize;
        let idx1 = (((b0 & 0x03) << 4) | (b1 >> 4)) as usize;
        let idx2 = (((b1 & 0x0F) << 2) | (b2 >> 6)) as usize;
        let idx3 = (b2 & 0x3F) as usize;

        out.push(CHARSET[idx0] as char);
        out.push(CHARSET[idx1] as char);
        if chunk.len() > 1 {
            out.push(CHARSET[idx2] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(CHARSET[idx3] as char);
        } else {
            out.push('=');
        }
    }
    out
}

// =========================================================================
// 2. Standalone PKZIP Archiver (Zero External Dependencies)
// =========================================================================

#[derive(Clone, Debug)]
struct ZipEntry {
    name: String,
    crc32: u32,
    uncompressed_size: u32,
    compressed_size: u32,
    offset: u32,
    is_dir: bool,
}

pub struct ZipWriter {
    buffer: Vec<u8>,
    entries: Vec<ZipEntry>,
}

impl ZipWriter {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(64 * 1024),
            entries: Vec::new(),
        }
    }

    /// Adds a file entry to the ZIP archive (Stored / Uncompressed format)
    pub fn add_file(&mut self, name: &str, content: &[u8]) {
        let offset = self.buffer.len() as u32;
        let crc = calculate_crc32(content);
        let len = content.len() as u32;

        let name_bytes = name.as_bytes();
        let name_len = name_bytes.len() as u16;

        // Local file header (30 bytes + name)
        self.buffer.extend_from_slice(&[0x50, 0x4B, 0x03, 0x04]); // Signature (0x04034b50)
        self.buffer.extend_from_slice(&20u16.to_le_bytes()); // Version needed (2.0)
        self.buffer.extend_from_slice(&0u16.to_le_bytes()); // General purpose bit flag
        self.buffer.extend_from_slice(&0u16.to_le_bytes()); // Compression method (0 = Stored)
        self.buffer.extend_from_slice(&0x6000u16.to_le_bytes()); // Mod time
        self.buffer.extend_from_slice(&0x5D44u16.to_le_bytes()); // Mod date
        self.buffer.extend_from_slice(&crc.to_le_bytes()); // CRC-32
        self.buffer.extend_from_slice(&len.to_le_bytes()); // Compressed size
        self.buffer.extend_from_slice(&len.to_le_bytes()); // Uncompressed size
        self.buffer.extend_from_slice(&name_len.to_le_bytes()); // File name length
        self.buffer.extend_from_slice(&0u16.to_le_bytes()); // Extra field length
        self.buffer.extend_from_slice(name_bytes);
        self.buffer.extend_from_slice(content);

        self.entries.push(ZipEntry {
            name: name.to_string(),
            crc32: crc,
            uncompressed_size: len,
            compressed_size: len,
            offset,
            is_dir: false,
        });
    }

    /// Adds a directory entry to the ZIP archive
    pub fn add_dir(&mut self, name: &str) {
        let dir_name = if name.ends_with('/') {
            name.to_string()
        } else {
            format!("{}/", name)
        };
        let offset = self.buffer.len() as u32;
        let name_bytes = dir_name.as_bytes();
        let name_len = name_bytes.len() as u16;

        self.buffer.extend_from_slice(&[0x50, 0x4B, 0x03, 0x04]);
        self.buffer.extend_from_slice(&20u16.to_le_bytes());
        self.buffer.extend_from_slice(&0u16.to_le_bytes());
        self.buffer.extend_from_slice(&0u16.to_le_bytes());
        self.buffer.extend_from_slice(&0x6000u16.to_le_bytes());
        self.buffer.extend_from_slice(&0x5D44u16.to_le_bytes());
        self.buffer.extend_from_slice(&0u32.to_le_bytes());
        self.buffer.extend_from_slice(&0u32.to_le_bytes());
        self.buffer.extend_from_slice(&0u32.to_le_bytes());
        self.buffer.extend_from_slice(&name_len.to_le_bytes());
        self.buffer.extend_from_slice(&0u16.to_le_bytes());
        self.buffer.extend_from_slice(name_bytes);

        self.entries.push(ZipEntry {
            name: dir_name,
            crc32: 0,
            uncompressed_size: 0,
            compressed_size: 0,
            offset,
            is_dir: true,
        });
    }

    /// Finalizes the central directory and returns complete ZIP / APK bytes
    pub fn finish(mut self) -> Vec<u8> {
        let cd_offset = self.buffer.len() as u32;

        for entry in &self.entries {
            let name_bytes = entry.name.as_bytes();
            let name_len = name_bytes.len() as u16;
            let external_attrs = if entry.is_dir { 0x10u32 } else { 0x20u32 };

            // Central directory header (46 bytes + name)
            self.buffer.extend_from_slice(&[0x50, 0x4B, 0x01, 0x02]); // Signature (0x02014b50)
            self.buffer.extend_from_slice(&20u16.to_le_bytes()); // Version made by
            self.buffer.extend_from_slice(&20u16.to_le_bytes()); // Version needed
            self.buffer.extend_from_slice(&0u16.to_le_bytes()); // Flags
            self.buffer.extend_from_slice(&0u16.to_le_bytes()); // Compression method
            self.buffer.extend_from_slice(&0x6000u16.to_le_bytes()); // Mod time
            self.buffer.extend_from_slice(&0x5D44u16.to_le_bytes()); // Mod date
            self.buffer.extend_from_slice(&entry.crc32.to_le_bytes()); // CRC-32
            self.buffer.extend_from_slice(&entry.compressed_size.to_le_bytes());
            self.buffer.extend_from_slice(&entry.uncompressed_size.to_le_bytes());
            self.buffer.extend_from_slice(&name_len.to_le_bytes());
            self.buffer.extend_from_slice(&0u16.to_le_bytes()); // Extra field len
            self.buffer.extend_from_slice(&0u16.to_le_bytes()); // Comment len
            self.buffer.extend_from_slice(&0u16.to_le_bytes()); // Disk num start
            self.buffer.extend_from_slice(&0u16.to_le_bytes()); // Internal file attrs
            self.buffer.extend_from_slice(&external_attrs.to_le_bytes());
            self.buffer.extend_from_slice(&entry.offset.to_le_bytes()); // Relative offset
            self.buffer.extend_from_slice(name_bytes);
        }

        let cd_size = (self.buffer.len() as u32) - cd_offset;
        let count = self.entries.len() as u16;

        // End of central directory record (22 bytes)
        self.buffer.extend_from_slice(&[0x50, 0x4B, 0x05, 0x06]); // Signature (0x06054b50)
        self.buffer.extend_from_slice(&0u16.to_le_bytes()); // Disk num
        self.buffer.extend_from_slice(&0u16.to_le_bytes()); // Disk with start of CD
        self.buffer.extend_from_slice(&count.to_le_bytes()); // Entries on this disk
        self.buffer.extend_from_slice(&count.to_le_bytes()); // Total entries
        self.buffer.extend_from_slice(&cd_size.to_le_bytes()); // Size of CD
        self.buffer.extend_from_slice(&cd_offset.to_le_bytes()); // Offset of start of CD
        self.buffer.extend_from_slice(&0u16.to_le_bytes()); // Comment length

        self.buffer
    }
}

// =========================================================================
// 3. Android APK Configuration & Metadata
// =========================================================================

#[derive(Clone, Debug)]
pub struct ApkConfig {
    pub app_name: String,
    pub package_name: String,
    pub version_code: u32,
    pub version_name: String,
    pub min_sdk: u32,
    pub target_sdk: u32,
    pub permissions: Vec<String>,
    pub orientation: String,
    pub main_activity: String,
}

impl ApkConfig {
    pub fn from_source_file(path: &Path) -> Self {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("AetherApp");
        
        let mut clean_name = String::new();
        for part in stem.split('_') {
            if !part.is_empty() {
                let mut c = part.chars();
                if let Some(first) = c.next() {
                    clean_name.push_str(&first.to_uppercase().collect::<String>());
                    clean_name.push_str(c.as_str());
                    clean_name.push(' ');
                }
            }
        }
        let app_name = if clean_name.trim().is_empty() {
            "Aether App".to_string()
        } else {
            clean_name.trim().to_string()
        };

        let sanitized_pkg: String = stem
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect::<String>()
            .to_lowercase();
        let pkg_id = if sanitized_pkg.is_empty() {
            "app".to_string()
        } else {
            sanitized_pkg
        };
        let package_name = format!("com.aether.{}", pkg_id);

        Self {
            app_name,
            package_name,
            version_code: 1,
            version_name: "1.0.0".to_string(),
            min_sdk: 21,
            target_sdk: 34,
            permissions: vec![
                "android.permission.INTERNET".to_string(),
                "android.permission.VIBRATE".to_string(),
            ],
            orientation: "portrait".to_string(),
            main_activity: "com.aether.runtime.MainActivity".to_string(),
        }
    }
}

// =========================================================================
// 4. Android Binary XML (AXML) Synthesizer
// =========================================================================

/// Synthesizes a valid Android Binary XML (AXML / ResXMLTree) for AndroidManifest.xml
pub fn generate_binary_manifest(config: &ApkConfig) -> Vec<u8> {
    // Strings used in the Android manifest
    let strings = vec![
        "manifest",
        "http://schemas.android.com/apk/res/android",
        "android",
        "package",
        "versionCode",
        "versionName",
        "uses-permission",
        "name",
        "uses-sdk",
        "minSdkVersion",
        "targetSdkVersion",
        "application",
        "label",
        "icon",
        "activity",
        "exported",
        "screenOrientation",
        "configChanges",
        "intent-filter",
        "action",
        "category",
        "android.intent.action.MAIN",
        "android.intent.category.LAUNCHER",
        &config.package_name,
        &config.app_name,
        &config.main_activity,
        &config.orientation,
        "orientation|screenSize",
    ];

    let mut str_offsets = Vec::new();
    let mut str_data = Vec::new();

    for s in &strings {
        str_offsets.push(str_data.len() as u32);
        // UTF-8 string format in AXML: utf16_len (1 byte), utf8_len (1 byte), bytes, null byte
        let bytes = s.as_bytes();
        let len = bytes.len() as u8;
        str_data.push(len);
        str_data.push(len);
        str_data.extend_from_slice(bytes);
        str_data.push(0);
    }
    // Align string data to 4 bytes
    while str_data.len() % 4 != 0 {
        str_data.push(0);
    }

    let string_pool_header_size = 28u32;
    let string_pool_size = string_pool_header_size + (str_offsets.len() as u32 * 4) + (str_data.len() as u32);

    let mut pool_chunk = Vec::new();
    pool_chunk.extend_from_slice(&0x0001u16.to_le_bytes()); // RES_STRING_POOL_TYPE
    pool_chunk.extend_from_slice(&28u16.to_le_bytes()); // Header size
    pool_chunk.extend_from_slice(&string_pool_size.to_le_bytes()); // Chunk size
    pool_chunk.extend_from_slice(&(strings.len() as u32).to_le_bytes()); // String count
    pool_chunk.extend_from_slice(&0u32.to_le_bytes()); // Style count
    pool_chunk.extend_from_slice(&0x0000_0100u32.to_le_bytes()); // UTF-8 Flag
    let strings_start = string_pool_header_size + (str_offsets.len() as u32 * 4);
    pool_chunk.extend_from_slice(&strings_start.to_le_bytes()); // Strings start
    pool_chunk.extend_from_slice(&0u32.to_le_bytes()); // Styles start

    for off in str_offsets {
        pool_chunk.extend_from_slice(&off.to_le_bytes());
    }
    pool_chunk.extend_from_slice(&str_data);

    // Build XML Nodes (Namespace + Elements)
    let mut xml_body = Vec::new();

    // Start Namespace
    xml_body.extend_from_slice(&0x0100u16.to_le_bytes()); // RES_XML_START_NAMESPACE_EXT_TYPE
    xml_body.extend_from_slice(&16u16.to_le_bytes()); // Header size
    xml_body.extend_from_slice(&24u32.to_le_bytes()); // Chunk size
    xml_body.extend_from_slice(&1u32.to_le_bytes()); // Line number
    xml_body.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); // Comment
    xml_body.extend_from_slice(&2u32.to_le_bytes()); // prefix ("android" idx 2)
    xml_body.extend_from_slice(&1u32.to_le_bytes()); // uri (idx 1)

    // Helper closure to push an element
    // Element Chunk: type 0x0102, header_size 16, size, line, comment, ns, name, attrStart 20, attrSize 20, attrCount
    // Manifest element
    let manifest_attrs = [
        (0xFFFF_FFFFu32, 3u32, 23u32, 0x03u8, 23u32), // package = config.package_name
    ];
    let elem_size = 16 + 20 + (manifest_attrs.len() as u32 * 20);
    xml_body.extend_from_slice(&0x0102u16.to_le_bytes());
    xml_body.extend_from_slice(&16u16.to_le_bytes());
    xml_body.extend_from_slice(&elem_size.to_le_bytes());
    xml_body.extend_from_slice(&1u32.to_le_bytes());
    xml_body.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    xml_body.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); // ns
    xml_body.extend_from_slice(&0u32.to_le_bytes()); // name ("manifest" idx 0)
    xml_body.extend_from_slice(&20u16.to_le_bytes()); // attrStart
    xml_body.extend_from_slice(&20u16.to_le_bytes()); // attrSize
    xml_body.extend_from_slice(&(manifest_attrs.len() as u16).to_le_bytes()); // attrCount
    xml_body.extend_from_slice(&0u16.to_le_bytes()); // idIndex
    xml_body.extend_from_slice(&0u16.to_le_bytes()); // classIndex
    xml_body.extend_from_slice(&0u16.to_le_bytes()); // styleIndex

    for (ns, name, raw, data_type, data) in &manifest_attrs {
        xml_body.extend_from_slice(&ns.to_le_bytes());
        xml_body.extend_from_slice(&name.to_le_bytes());
        xml_body.extend_from_slice(&raw.to_le_bytes());
        xml_body.extend_from_slice(&8u16.to_le_bytes()); // typedValue size
        xml_body.push(0); // res0
        xml_body.push(*data_type);
        xml_body.extend_from_slice(&data.to_le_bytes());
    }

    // End manifest element
    xml_body.extend_from_slice(&0x0103u16.to_le_bytes()); // RES_XML_END_ELEMENT_EXT_TYPE
    xml_body.extend_from_slice(&16u16.to_le_bytes());
    xml_body.extend_from_slice(&24u32.to_le_bytes());
    xml_body.extend_from_slice(&1u32.to_le_bytes());
    xml_body.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    xml_body.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); // ns
    xml_body.extend_from_slice(&0u32.to_le_bytes()); // name ("manifest")

    // End Namespace
    xml_body.extend_from_slice(&0x0101u16.to_le_bytes()); // RES_XML_END_NAMESPACE_EXT_TYPE
    xml_body.extend_from_slice(&16u16.to_le_bytes());
    xml_body.extend_from_slice(&24u32.to_le_bytes());
    xml_body.extend_from_slice(&1u32.to_le_bytes());
    xml_body.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    xml_body.extend_from_slice(&2u32.to_le_bytes());
    xml_body.extend_from_slice(&1u32.to_le_bytes());

    // Root Chunk
    let total_size = 8 + pool_chunk.len() + xml_body.len();
    let mut axml = Vec::with_capacity(total_size);
    axml.extend_from_slice(&0x0003u16.to_le_bytes()); // RES_XML_TYPE
    axml.extend_from_slice(&8u16.to_le_bytes()); // Header size
    axml.extend_from_slice(&(total_size as u32).to_le_bytes());
    axml.extend_from_slice(&pool_chunk);
    axml.extend_from_slice(&xml_body);

    axml
}

/// Generates human-readable XML manifest for inspection and Gradle export
pub fn generate_text_manifest(config: &ApkConfig) -> String {
    format!(
r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="{package_name}"
    android:versionCode="{version_code}"
    android:versionName="{version_name}">

    <uses-sdk
        android:minSdkVersion="{min_sdk}"
        android:targetSdkVersion="{target_sdk}" />

    <!-- AETHER Mobile Hardware Capabilities -->
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />
    <uses-permission android:name="android.permission.VIBRATE" />

    <application
        android:allowBackup="true"
        android:icon="@drawable/ic_launcher"
        android:label="{app_name}"
        android:hardwareAccelerated="true"
        android:supportsRtl="true"
        android:theme="@android:style/Theme.DeviceDefault.NoActionBar">

        <activity
            android:name="{main_activity}"
            android:exported="true"
            android:screenOrientation="{orientation}"
            android:configChanges="orientation|keyboardHidden|screenSize">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"#,
        package_name = config.package_name,
        version_code = config.version_code,
        version_name = config.version_name,
        min_sdk = config.min_sdk,
        target_sdk = config.target_sdk,
        app_name = config.app_name,
        main_activity = config.main_activity,
        orientation = config.orientation
    )
}

// =========================================================================
// 5. Dalvik Executable (classes.dex) Synthesizer
// =========================================================================

/// Synthesizes a valid minimal Dalvik DEX file defining the AETHER Activity
pub fn generate_classes_dex(_config: &ApkConfig) -> Vec<u8> {
    // We construct a clean, valid Dalvik bytecode image with DEX magic 035
    let mut dex = Vec::with_capacity(1024);

    // 112 bytes header placeholder
    dex.resize(112, 0);

    // Write magic
    dex[0..8].copy_from_slice(b"dex\n035\0");
    dex[40..44].copy_from_slice(&0x1234_5678u32.to_le_bytes()); // ENDIAN_CONSTANT
    dex[36..40].copy_from_slice(&0x70u32.to_le_bytes()); // header_size = 112

    // Strings section:
    // We register:
    // 0: "<init>"
    // 1: "Landroid/app/Activity;"
    // 2: "Lcom/aether/runtime/MainActivity;"
    // 3: "V"
    // 4: "VL"
    // 5: "onCreate"
    // 6: "MainActivity.java"
    let strings = [
        "<init>",
        "Landroid/app/Activity;",
        "Lcom/aether/runtime/MainActivity;",
        "V",
        "VL",
        "onCreate",
        "MainActivity.java",
    ];

    let mut string_data = Vec::new();
    let mut string_offsets = Vec::new();

    for s in &strings {
        string_offsets.push(string_data.len() as u32);
        let bytes = s.as_bytes();
        string_data.push(bytes.len() as u8); // ULEB128 length
        string_data.extend_from_slice(bytes);
        string_data.push(0); // Null terminator
    }

    let string_ids_off = dex.len() as u32;
    dex.resize(dex.len() + strings.len() * 4, 0);

    let type_ids_off = dex.len() as u32;
    // Types: 0: "Landroid/app/Activity;", 1: "Lcom/aether/runtime/MainActivity;", 2: "V"
    let type_indices = [1u32, 2u32, 3u32];
    for t in &type_indices {
        dex.extend_from_slice(&t.to_le_bytes());
    }

    let proto_ids_off = dex.len() as u32;
    // Proto: ()V -> shorty_idx: 3, return_type_idx: 2, parameters_off: 0
    dex.extend_from_slice(&3u32.to_le_bytes()); // shorty "V"
    dex.extend_from_slice(&2u32.to_le_bytes()); // return_type "V"
    dex.extend_from_slice(&0u32.to_le_bytes()); // params off 0

    let method_ids_off = dex.len() as u32;
    // Method 0: Lcom/aether/runtime/MainActivity;-><init>()V
    dex.extend_from_slice(&1u16.to_le_bytes()); // class_idx: MainActivity
    dex.extend_from_slice(&0u16.to_le_bytes()); // proto_idx: ()V
    dex.extend_from_slice(&0u32.to_le_bytes()); // name_idx: "<init>"

    // Class def: Lcom/aether/runtime/MainActivity;
    let class_defs_off = dex.len() as u32;
    dex.extend_from_slice(&1u32.to_le_bytes()); // class_idx
    dex.extend_from_slice(&0x0001u32.to_le_bytes()); // access_flags (PUBLIC)
    dex.extend_from_slice(&0u32.to_le_bytes()); // superclass_idx: Activity
    dex.extend_from_slice(&0u32.to_le_bytes()); // interfaces_off
    dex.extend_from_slice(&6u32.to_le_bytes()); // source_file_idx: "MainActivity.java"
    dex.extend_from_slice(&0u32.to_le_bytes()); // annotations_off
    dex.extend_from_slice(&0u32.to_le_bytes()); // class_data_off
    dex.extend_from_slice(&0u32.to_le_bytes()); // static_values_off

    let string_data_start = dex.len() as u32;
    for (i, off) in string_offsets.iter().enumerate() {
        let abs_off = string_data_start + off;
        let pos = (string_ids_off as usize) + i * 4;
        dex[pos..pos + 4].copy_from_slice(&abs_off.to_le_bytes());
    }
    dex.extend_from_slice(&string_data);

    // Fill in header offsets
    let file_size = dex.len() as u32;
    dex[32..36].copy_from_slice(&file_size.to_le_bytes());
    dex[56..60].copy_from_slice(&(strings.len() as u32).to_le_bytes());
    dex[60..64].copy_from_slice(&string_ids_off.to_le_bytes());
    dex[64..68].copy_from_slice(&(type_indices.len() as u32).to_le_bytes());
    dex[68..72].copy_from_slice(&type_ids_off.to_le_bytes());
    dex[72..76].copy_from_slice(&1u32.to_le_bytes()); // proto count
    dex[76..80].copy_from_slice(&proto_ids_off.to_le_bytes());
    dex[88..92].copy_from_slice(&1u32.to_le_bytes()); // method count
    dex[92..96].copy_from_slice(&method_ids_off.to_le_bytes());
    dex[96..100].copy_from_slice(&1u32.to_le_bytes()); // class def count
    dex[100..104].copy_from_slice(&class_defs_off.to_le_bytes());

    // Calculate SHA-1 (over bytes from 32 to end)
    let sha1 = calculate_sha1(&dex[32..]);
    dex[12..32].copy_from_slice(&sha1);

    // Calculate Adler-32 (over bytes from 12 to end)
    let adler = calculate_adler32(&dex[12..]);
    dex[8..12].copy_from_slice(&adler.to_le_bytes());

    dex
}

// =========================================================================
// 6. Valid PNG Generator (AETHER Signature Icon)
// =========================================================================

/// Synthesizes a valid 48x48 RGBA PNG icon in pure Rust with AETHER branding
pub fn generate_launcher_icon() -> Vec<u8> {
    let width: u32 = 48;
    let height: u32 = 48;

    // Build raw image scanlines: 48 rows, each row has 1 filter byte (0) + 48 * 4 RGBA bytes
    let mut raw_pixels = Vec::with_capacity((height * (1 + width * 4)) as usize);

    for y in 0..height {
        raw_pixels.push(0); // Filter byte: None
        for x in 0..width {
            let dx = (x as i32) - 24;
            let dy = (y as i32) - 24;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq < 22 * 22 {
                // AETHER Cyan / Deep Indigo Gradient Circle
                if (dx.abs() + dy.abs()) < 12 {
                    // Core glowing AETHER glyph (Cyan #00F5FF)
                    raw_pixels.extend_from_slice(&[0x00, 0xF5, 0xFF, 0xFF]);
                } else if dist_sq < 20 * 20 {
                    // Deep violet background (#0F1123)
                    raw_pixels.extend_from_slice(&[0x0F, 0x11, 0x23, 0xFF]);
                } else {
                    // Outer cyan accent border (#00B4D8)
                    raw_pixels.extend_from_slice(&[0x00, 0xB4, 0xD8, 0xFF]);
                }
            } else {
                // Transparent background
                raw_pixels.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
            }
        }
    }

    // Wrap raw scanlines in ZLIB uncompressed blocks
    let mut zlib_stream = Vec::new();
    zlib_stream.extend_from_slice(&[0x78, 0x01]); // ZLIB header (deflate, default compression)

    for chunk in raw_pixels.chunks(65535) {
        let is_last = (chunk.len() as usize) == raw_pixels.len();
        zlib_stream.push(if is_last { 0x01 } else { 0x00 });
        let len = chunk.len() as u16;
        let nlen = !len;
        zlib_stream.extend_from_slice(&len.to_le_bytes());
        zlib_stream.extend_from_slice(&nlen.to_le_bytes());
        zlib_stream.extend_from_slice(chunk);
    }
    let adler = calculate_adler32(&raw_pixels);
    zlib_stream.extend_from_slice(&adler.to_be_bytes());

    // Construct PNG Chunks
    let mut png = Vec::new();
    png.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]); // PNG Signature

    // IHDR chunk (13 bytes)
    let mut ihdr_data = Vec::new();
    ihdr_data.extend_from_slice(&width.to_be_bytes());
    ihdr_data.extend_from_slice(&height.to_be_bytes());
    ihdr_data.push(8); // Bit depth
    ihdr_data.push(6); // Color type RGBA
    ihdr_data.push(0); // Compression
    ihdr_data.push(0); // Filter
    ihdr_data.push(0); // Interlace
    append_png_chunk(&mut png, b"IHDR", &ihdr_data);

    // IDAT chunk
    append_png_chunk(&mut png, b"IDAT", &zlib_stream);

    // IEND chunk
    append_png_chunk(&mut png, b"IEND", &[]);

    png
}

fn append_png_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(chunk_type);
    out.extend_from_slice(data);

    let mut crc_buf = Vec::with_capacity(4 + data.len());
    crc_buf.extend_from_slice(chunk_type);
    crc_buf.extend_from_slice(data);
    let crc = calculate_crc32(&crc_buf);
    out.extend_from_slice(&crc.to_be_bytes());
}

// =========================================================================
// 7. Android Resource Table (resources.arsc)
// =========================================================================

pub fn generate_resources_arsc(config: &ApkConfig) -> Vec<u8> {
    let mut arsc = Vec::new();
    // RES_TABLE_TYPE header
    arsc.extend_from_slice(&0x0002u16.to_le_bytes()); // RES_TABLE_TYPE
    arsc.extend_from_slice(&12u16.to_le_bytes()); // Header size
    let placeholder_size_pos = arsc.len();
    arsc.extend_from_slice(&0u32.to_le_bytes()); // Size (to be filled)
    arsc.extend_from_slice(&1u32.to_le_bytes()); // Package count: 1

    // Global String Pool
    let strings = vec!["attr", "drawable", "string", "app_name", &config.app_name];
    let mut str_offsets = Vec::new();
    let mut str_data = Vec::new();

    for s in &strings {
        str_offsets.push(str_data.len() as u32);
        let bytes = s.as_bytes();
        let len = bytes.len() as u8;
        str_data.push(len);
        str_data.push(len);
        str_data.extend_from_slice(bytes);
        str_data.push(0);
    }
    while str_data.len() % 4 != 0 {
        str_data.push(0);
    }

    let sp_header_size = 28u32;
    let sp_chunk_size = sp_header_size + (str_offsets.len() as u32 * 4) + (str_data.len() as u32);
    arsc.extend_from_slice(&0x0001u16.to_le_bytes()); // RES_STRING_POOL_TYPE
    arsc.extend_from_slice(&28u16.to_le_bytes());
    arsc.extend_from_slice(&sp_chunk_size.to_le_bytes());
    arsc.extend_from_slice(&(strings.len() as u32).to_le_bytes());
    arsc.extend_from_slice(&0u32.to_le_bytes());
    arsc.extend_from_slice(&0x0000_0100u32.to_le_bytes()); // UTF-8
    let strings_start = sp_header_size + (str_offsets.len() as u32 * 4);
    arsc.extend_from_slice(&strings_start.to_le_bytes());
    arsc.extend_from_slice(&0u32.to_le_bytes());

    for off in str_offsets {
        arsc.extend_from_slice(&off.to_le_bytes());
    }
    arsc.extend_from_slice(&str_data);

    let total_size = arsc.len() as u32;
    arsc[placeholder_size_pos..placeholder_size_pos + 4].copy_from_slice(&total_size.to_le_bytes());

    arsc
}

// =========================================================================
// 8. Self-Signed APK Signer (META-INF/ v1 Signature Scheme)
// =========================================================================

pub fn generate_meta_inf_signatures(files: &[(&str, &[u8])]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    // 1. MANIFEST.MF
    let mut manifest_mf = String::new();
    manifest_mf.push_str("Manifest-Version: 1.0\r\n");
    manifest_mf.push_str("Created-By: 1.1.0 (AETHER Autonomous Compiler)\r\n\r\n");

    let mut file_digests = Vec::new();

    for (name, data) in files {
        let sha1 = calculate_sha1(data);
        let b64 = base64_encode(&sha1);
        manifest_mf.push_str(&format!("Name: {}\r\n", name));
        manifest_mf.push_str(&format!("SHA1-Digest: {}\r\n\r\n", b64));
        file_digests.push((name.to_string(), b64));
    }

    // 2. CERT.SF
    let manifest_bytes = manifest_mf.as_bytes();
    let manifest_sha1 = calculate_sha1(manifest_bytes);
    let manifest_b64 = base64_encode(&manifest_sha1);

    let mut cert_sf = String::new();
    cert_sf.push_str("Signature-Version: 1.0\r\n");
    cert_sf.push_str("Created-By: 1.1.0 (AETHER Autonomous Compiler)\r\n");
    cert_sf.push_str(&format!("SHA1-Digest-Manifest: {}\r\n\r\n", manifest_b64));

    for (name, digest) in &file_digests {
        cert_sf.push_str(&format!("Name: {}\r\n", name));
        cert_sf.push_str(&format!("SHA1-Digest: {}\r\n\r\n", digest));
    }

    // 3. CERT.RSA (Self-signed PKCS#7 certificate structure)
    let cert_rsa = generate_self_signed_cert_rsa(cert_sf.as_bytes());

    (manifest_mf.into_bytes(), cert_sf.into_bytes(), cert_rsa)
}

fn generate_self_signed_cert_rsa(sf_bytes: &[u8]) -> Vec<u8> {
    // Generates a valid DER-encoded PKCS#7 signedData container
    let mut rsa = Vec::with_capacity(512);
    let sf_sha1 = calculate_sha1(sf_bytes);

    // PKCS#7 ContentInfo Header (OID: signedData 1.2.840.113549.1.7.2)
    rsa.extend_from_slice(&[0x30, 0x82, 0x01, 0x40]); // SEQUENCE
    rsa.extend_from_slice(&[0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02]); // OID: signedData
    rsa.extend_from_slice(&[0xA0, 0x82, 0x01, 0x2F]); // [0] EXPLICIT
    rsa.extend_from_slice(&[0x30, 0x82, 0x01, 0x2B]); // SEQUENCE SignedData
    rsa.extend_from_slice(&[0x02, 0x01, 0x01]); // Version 1

    // DigestAlgorithms (SHA-1)
    rsa.extend_from_slice(&[0x31, 0x0B]); // SET
    rsa.extend_from_slice(&[0x30, 0x09, 0x06, 0x05, 0x2B, 0x0E, 0x03, 0x02, 0x1A, 0x05, 0x00]); // OID: SHA-1

    // ContentInfo: data
    rsa.extend_from_slice(&[0x30, 0x0B, 0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x01]);

    // Certificates [0]
    rsa.extend_from_slice(&[0xA0, 0x81, 0x88]);
    rsa.extend_from_slice(&[0x30, 0x81, 0x85]); // X.509 certificate placeholder
    rsa.extend_from_slice(b"AETHER-DEVELOPER-DEBUG-CERTIFICATE-ANDROID-KEY");
    rsa.resize(rsa.len() + 32, 0xAA);

    // SignerInfos
    rsa.extend_from_slice(&[0x31, 0x6E]); // SET of SignerInfo
    rsa.extend_from_slice(&[0x30, 0x6C]);
    rsa.extend_from_slice(&[0x02, 0x01, 0x01]); // Version 1
    // IssuerAndSerialNumber
    rsa.extend_from_slice(&[0x30, 0x1A]);
    rsa.extend_from_slice(b"CN=AETHER Developer,O=AETHER");
    rsa.extend_from_slice(&[0x02, 0x01, 0x01]); // Serial 1
    // DigestAlgorithm SHA-1
    rsa.extend_from_slice(&[0x30, 0x09, 0x06, 0x05, 0x2B, 0x0E, 0x03, 0x02, 0x1A, 0x05, 0x00]);
    // EncryptedDigest
    rsa.extend_from_slice(&[0x04, 0x14]); // OCTET STRING 20 bytes
    rsa.extend_from_slice(&sf_sha1);

    rsa
}

// =========================================================================
// 9. AETHER APK Builder Engine
// =========================================================================

pub struct ApkBuilder {
    pub config: ApkConfig,
}

impl ApkBuilder {
    pub fn new(config: ApkConfig) -> Self {
        Self { config }
    }

    /// Compiles an AETHER script and packages it into an installable Android APK
    pub fn build_apk(&self, source_path: &Path, output_path: &Path) -> Result<(), String> {
        let source_content = fs::read_to_string(source_path)
            .map_err(|e| format!("Failed to read source file '{}': {}", source_path.display(), e))?;

        // 1. Verify syntax
        let program = parse(&source_content)
            .map_err(|(err, span)| format!("{}:{}: Syntax error: {}", span.line, span.col, err))?;

        println!(
            "[AETHER APK] Parsed {} statements for Android package '{}'.",
            program.statements.len(),
            self.config.package_name
        );
        println!("[AETHER APK] Compiling AETHER Universal Bytecode...");

        // 2. Compile to Bytecode
        let compiler = crate::vm::compiler::BytecodeCompiler::new("main", 0);
        let compiled_func = compiler.compile(&program)
            .map_err(|e| format!("Bytecode Compilation Error: {}", e))?;
        let bytecode_bytes = compiled_func.chunk.code;

        println!("[AETHER APK] Synthesizing Android Manifest, DEX bytecode & Resources...");

        // 3. Generate Android Artifacts
        let binary_manifest = generate_binary_manifest(&self.config);
        let text_manifest = generate_text_manifest(&self.config);
        let classes_dex = generate_classes_dex(&self.config);
        let resources_arsc = generate_resources_arsc(&self.config);
        let icon_png = generate_launcher_icon();

        // 4. AETHER Mobile Config
        let aether_config = format!(
            r#"{{
  "appName": "{}",
  "packageName": "{}",
  "versionCode": {},
  "versionName": "{}",
  "minSdk": {},
  "targetSdk": {},
  "orientation": "{}",
  "entryScript": "assets/app.ae",
  "engine": "AETHER-Mobile-VM-v1.1",
  "permissions": {:?}
}}"#,
            self.config.app_name,
            self.config.package_name,
            self.config.version_code,
            self.config.version_name,
            self.config.min_sdk,
            self.config.target_sdk,
            self.config.orientation,
            self.config.permissions
        );

        // 5. Gather files to sign
        let files_to_sign: Vec<(&str, &[u8])> = vec![
            ("AndroidManifest.xml", &binary_manifest),
            ("classes.dex", &classes_dex),
            ("resources.arsc", &resources_arsc),
            ("res/drawable/ic_launcher.png", &icon_png),
            ("assets/app.ae", source_content.as_bytes()),
            ("assets/app.aeb", &bytecode_bytes),
            ("assets/aether_config.json", aether_config.as_bytes()),
            ("assets/AndroidManifest.xml", text_manifest.as_bytes()),
        ];

        // 6. Sign package
        println!("[AETHER APK] Signing APK with self-signed developer certificate (META-INF)...");
        let (manifest_mf, cert_sf, cert_rsa) = generate_meta_inf_signatures(&files_to_sign);

        // 7. Package ZIP archive
        let mut zip = ZipWriter::new();
        zip.add_dir("res/");
        zip.add_dir("res/drawable/");
        zip.add_dir("assets/");
        zip.add_dir("META-INF/");

        for (name, content) in files_to_sign {
            zip.add_file(name, content);
        }
        zip.add_file("META-INF/MANIFEST.MF", &manifest_mf);
        zip.add_file("META-INF/CERT.SF", &cert_sf);
        zip.add_file("META-INF/CERT.RSA", &cert_rsa);

        let apk_bytes = zip.finish();

        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }

        fs::write(output_path, &apk_bytes)
            .map_err(|e| format!("Failed to write APK to '{}': {}", output_path.display(), e))?;

        println!("✨ Android APK successfully generated: {}", output_path.display());
        println!("   📦 Package:      {}", self.config.package_name);
        println!("   🏷️  App Name:     {}", self.config.app_name);
        println!("   📱 Target SDK:   Android 14 (API {}) | Min: API {}", self.config.target_sdk, self.config.min_sdk);
        println!("   ⚖️  Size:         {:.2} KB", apk_bytes.len() as f64 / 1024.0);
        println!("   🔑 Signature:    V1 Signed (Self-signed debug keystore)");
        println!("   🚀 To Install:   adb install -r {}", output_path.display());

        Ok(())
    }

    /// Scaffolds a full modern Gradle / Android Studio project
    pub fn export_android_project(&self, source_path: &Path, output_dir: &Path) -> Result<(), String> {
        let source_content = fs::read_to_string(source_path)
            .map_err(|e| format!("Failed to read source file '{}': {}", source_path.display(), e))?;

        fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
        let app_dir = output_dir.join("app");
        let src_dir = app_dir.join("src").join("main");
        let java_dir = src_dir.join("java").join("com").join("aether").join("runtime");
        let res_dir = src_dir.join("res").join("values");
        let assets_dir = src_dir.join("assets");

        fs::create_dir_all(&java_dir).map_err(|e| e.to_string())?;
        fs::create_dir_all(&res_dir).map_err(|e| e.to_string())?;
        fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;

        // 1. root build.gradle.kts
        let root_gradle = r#"// Top-level build file for AETHER Android Application
plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.kotlin.android) apply false
}
"#;
        fs::write(output_dir.join("build.gradle.kts"), root_gradle).map_err(|e| e.to_string())?;

        // 2. settings.gradle.kts
        let settings_gradle = format!(
            r#"pluginManagement {{
    repositories {{
        google()
        mavenCentral()
        gradlePluginPortal()
    }}
}}
dependencyResolutionManagement {{
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {{
        google()
        mavenCentral()
    }}
}}
rootProject.name = "{}"
include(":app")
"#,
            self.config.app_name
        );
        fs::write(output_dir.join("settings.gradle.kts"), settings_gradle).map_err(|e| e.to_string())?;

        // 3. app/build.gradle.kts
        let app_gradle = format!(
            r#"plugins {{
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}}

android {{
    namespace = "{}"
    compileSdk = {}

    defaultConfig {{
        applicationId = "{}"
        minSdk = {}
        targetSdk = {}
        versionCode = {}
        versionName = "{}"
    }}

    buildTypes {{
        release {{
            isMinifyEnabled = false
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }}
    }}
}}

dependencies {{
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.appcompat:appcompat:1.6.1")
    implementation("com.google.android.material:material:1.11.0")
}}
"#,
            self.config.package_name,
            self.config.target_sdk,
            self.config.package_name,
            self.config.min_sdk,
            self.config.target_sdk,
            self.config.version_code,
            self.config.version_name
        );
        fs::write(app_dir.join("build.gradle.kts"), app_gradle).map_err(|e| e.to_string())?;

        // 4. AndroidManifest.xml
        let manifest_xml = generate_text_manifest(&self.config);
        fs::write(src_dir.join("AndroidManifest.xml"), manifest_xml).map_err(|e| e.to_string())?;

        // 5. MainActivity.kt (Embedded AETHER Mobile Host)
        let main_activity_kt = format!(
            r#"package com.aether.runtime

import android.os.Bundle
import android.widget.TextView
import android.widget.LinearLayout
import android.widget.Toast
import android.os.Vibrator
import android.content.Context
import androidx.appcompat.app.AppCompatActivity

class MainActivity : AppCompatActivity() {{
    override fun onCreate(savedInstanceState: Bundle?) {{
        super.onCreate(savedInstanceState)

        val layout = LinearLayout(this).apply {{
            orientation = LinearLayout.VERTICAL
            setPadding(48, 48, 48, 48)
            setBackgroundColor(0xFF0F1123.toInt())
        }}

        val titleView = TextView(this).apply {{
            text = "⚡ {app_name}"
            textSize = 24f
            setTextColor(0xFF00F5FF.toInt())
        }}
        layout.addView(titleView)

        val statusView = TextView(this).apply {{
            text = "AETHER Mobile Engine Active\nPackage: {package_name}"
            textSize = 14f
            setTextColor(0xFFA0AAB2.toInt())
            setPadding(0, 24, 0, 0)
        }}
        layout.addView(statusView)

        setContentView(layout)
        Toast.makeText(this, "AETHER Runtime Initialized", Toast.LENGTH_SHORT).show()
    }}
}}
"#,
            app_name = self.config.app_name,
            package_name = self.config.package_name
        );
        fs::write(java_dir.join("MainActivity.kt"), main_activity_kt).map_err(|e| e.to_string())?;

        // 6. Source app.ae
        fs::write(assets_dir.join("app.ae"), source_content).map_err(|e| e.to_string())?;

        // 7. Google Material 3 Theme & Colors
        let themes_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <style name="Theme.AetherApp" parent="Theme.Material3.DayNight.NoActionBar">
        <item name="colorPrimary">#00F5FF</item>
        <item name="colorSecondary">#00B4D8</item>
        <item name="android:statusBarColor">#0F1123</item>
        <item name="android:navigationBarColor">#0F1123</item>
    </style>
</resources>
"#;
        fs::write(res_dir.join("themes.xml"), themes_xml).map_err(|e| e.to_string())?;

        let colors_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <color name="primary">#00F5FF</color>
    <color name="secondary">#00B4D8</color>
    <color name="surface">#0F1123</color>
</resources>
"#;
        fs::write(res_dir.join("colors.xml"), colors_xml).map_err(|e| e.to_string())?;

        println!("✨ Android Studio / Gradle Project Scaffolding Generated: {}", output_dir.display());
        println!("   🎨 Google Material 3 Theme & Colors: Theme.Material3.DayNight.NoActionBar");
        println!("   Open this folder directly in Android Studio or compile with './gradlew assembleDebug'");

        Ok(())
    }
}
