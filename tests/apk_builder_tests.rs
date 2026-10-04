use aether::codegen::apk_builder::{
    base64_encode, calculate_adler32, calculate_crc32, calculate_sha1, generate_binary_manifest,
    generate_classes_dex, generate_launcher_icon, ApkBuilder, ApkConfig, ZipWriter,
};
use std::fs;

#[test]
fn test_crc32_and_adler32_hashing() {
    let data = b"123456789";
    // Standard test vector for CRC-32 (IEEE 802.3)
    let crc = calculate_crc32(data);
    assert_eq!(crc, 0xCBF43926);

    // Standard test vector for Adler-32
    let adler = calculate_adler32(data);
    assert_eq!(adler, 0x091E01DE);
}

#[test]
fn test_sha1_and_base64() {
    let data = b"The quick brown fox jumps over the lazy dog";
    let sha1 = calculate_sha1(data);
    // Known SHA-1: 2fd4e1c67a2d28fced849ee1bb76e7391b93eb12
    let expected_hex = "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12";
    let actual_hex = sha1.iter().map(|b| format!("{:02x}", b)).collect::<String>();
    assert_eq!(actual_hex, expected_hex);

    let b64 = base64_encode(&sha1);
    assert_eq!(b64, "L9ThxnotKPzthJ7hu3bnORuT6xI=");
}

#[test]
fn test_zip_writer_standalone() {
    let mut zip = ZipWriter::new();
    zip.add_file("hello.txt", b"Hello, AETHER Android!");
    zip.add_file("assets/test.json", b"{\"app\": \"AetherMobile\"}");
    let bytes = zip.finish();

    // Verify ZIP magic header PK\x03\x04
    assert_eq!(&bytes[0..4], &[0x50, 0x4B, 0x03, 0x04]);
    assert!(bytes.len() > 100);

    // Verify End of Central Directory signature PK\x05\x06
    let eocd_sig = [0x50, 0x4B, 0x05, 0x06];
    assert!(bytes.windows(4).any(|w| w == eocd_sig));
}

#[test]
fn test_launcher_icon_png_format() {
    let png = generate_launcher_icon();
    // Verify PNG magic [0x89, 'P', 'N', 'G', '\r', '\n', 0x1A, '\n']
    assert_eq!(&png[0..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    // Must contain IHDR, IDAT, IEND
    assert!(png.windows(4).any(|w| w == b"IHDR"));
    assert!(png.windows(4).any(|w| w == b"IDAT"));
    assert!(png.windows(4).any(|w| w == b"IEND"));
}

#[test]
fn test_dalvik_classes_dex_format() {
    let config = ApkConfig {
        app_name: "Test App".to_string(),
        package_name: "com.aether.test".to_string(),
        version_code: 1,
        version_name: "1.0.0".to_string(),
        min_sdk: 21,
        target_sdk: 34,
        permissions: vec!["android.permission.INTERNET".to_string()],
        orientation: "portrait".to_string(),
        main_activity: "com.aether.runtime.MainActivity".to_string(),
    };
    let dex = generate_classes_dex(&config);
    // Verify DEX magic: dex\n035\0
    assert_eq!(&dex[0..8], b"dex\n035\0");
    // Verify endian constant at offset 40
    let endian = u32::from_le_bytes([dex[40], dex[41], dex[42], dex[43]]);
    assert_eq!(endian, 0x12345678);
}

#[test]
fn test_binary_axml_manifest_format() {
    let config = ApkConfig {
        app_name: "Aether Calc".to_string(),
        package_name: "com.aether.calc".to_string(),
        version_code: 1,
        version_name: "1.0.0".to_string(),
        min_sdk: 21,
        target_sdk: 34,
        permissions: vec![],
        orientation: "portrait".to_string(),
        main_activity: "com.aether.runtime.MainActivity".to_string(),
    };
    let axml = generate_binary_manifest(&config);
    // Verify RES_XML_TYPE = 0x0003
    let chunk_type = u16::from_le_bytes([axml[0], axml[1]]);
    assert_eq!(chunk_type, 0x0003);
    // Header size = 8
    let header_size = u16::from_le_bytes([axml[2], axml[3]]);
    assert_eq!(header_size, 8);
    // Total size matches axml len
    let total_size = u32::from_le_bytes([axml[4], axml[5], axml[6], axml[7]]) as usize;
    assert_eq!(total_size, axml.len());
}

#[test]
fn test_full_apk_compilation_and_packaging() {
    let temp_dir = tempfile::tempdir().unwrap();
    let app_ae = temp_dir.path().join("mobile_app.ae");
    let out_apk = temp_dir.path().join("mobile_app.apk");

    let source = r#"
# AETHER Mobile Counter Application
let count = 0

fn increment() {
    count = count + 1
    Mobile.vibrate(30)
    print(f"Counter updated to: {count}")
}

print(f"Mobile app initialized with count = {count}")
increment()
"#;
    fs::write(&app_ae, source).unwrap();

    let config = ApkConfig::from_source_file(&app_ae);
    assert_eq!(config.app_name, "Mobile App");
    assert_eq!(config.package_name, "com.aether.mobile_app");

    let builder = ApkBuilder::new(config);
    let result = builder.build_apk(&app_ae, &out_apk);
    assert!(result.is_ok(), "APK build failed: {:?}", result.err());

    assert!(out_apk.exists());
    let apk_bytes = fs::read(&out_apk).unwrap();
    assert!(apk_bytes.len() > 1000);

    // Verify ZIP signature
    assert_eq!(&apk_bytes[0..4], &[0x50, 0x4B, 0x03, 0x04]);

    // Verify key files are present inside the APK archive
    let entries = [
        "AndroidManifest.xml",
        "classes.dex",
        "resources.arsc",
        "res/drawable/ic_launcher.png",
        "assets/app.ae",
        "assets/app.aeb",
        "assets/aether_config.json",
        "META-INF/MANIFEST.MF",
        "META-INF/CERT.SF",
        "META-INF/CERT.RSA",
    ];

    for entry in entries {
        assert!(
            apk_bytes.windows(entry.len()).any(|w| w == entry.as_bytes()),
            "Entry '{}' not found in generated APK",
            entry
        );
    }
}

#[test]
fn test_export_android_studio_project() {
    let temp_dir = tempfile::tempdir().unwrap();
    let app_ae = temp_dir.path().join("calc.ae");
    let out_project = temp_dir.path().join("calc_android_project");

    fs::write(&app_ae, "print('Android Studio Scaffolding Test')").unwrap();

    let config = ApkConfig::from_source_file(&app_ae);
    let builder = ApkBuilder::new(config);
    let result = builder.export_android_project(&app_ae, &out_project);
    assert!(result.is_ok());

    // Verify project structure
    assert!(out_project.join("build.gradle.kts").exists());
    assert!(out_project.join("settings.gradle.kts").exists());
    assert!(out_project.join("app/build.gradle.kts").exists());
    assert!(out_project.join("app/src/main/AndroidManifest.xml").exists());
    assert!(out_project.join("app/src/main/java/com/aether/runtime/MainActivity.kt").exists());
    assert!(out_project.join("app/src/main/assets/app.ae").exists());
}
