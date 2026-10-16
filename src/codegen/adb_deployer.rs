// ==============================================================================
// AETHER 2.0 ADB Mobile Device Deployer & Wireless Hot-Reload Bridge
// 1-Click APK Installation & Activity Launch via ADB with Zero External Dependencies
// ==============================================================================

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct AdbDevice {
    pub id: String,
    pub model: String,
    pub product: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct DeployReport {
    pub device: AdbDevice,
    pub apk_path: PathBuf,
    pub installed: bool,
    pub launched: bool,
    pub reverse_port_forwarded: bool,
    pub message: String,
}

pub struct AdbDeployer;

impl AdbDeployer {
    /// Detect all currently connected Android devices via ADB
    pub fn list_devices() -> Result<Vec<AdbDevice>, String> {
        let output = Command::new("adb")
            .arg("devices")
            .arg("-l")
            .output()
            .map_err(|e| format!("Failed to execute 'adb devices': {}. Please ensure Android SDK Platform-Tools (adb) is in your PATH.", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("ADB returned error: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut devices = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("List of devices") || line.starts_with('*') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let id = parts[0].to_string();
                let status = parts[1].to_string();

                let mut model = "Android Device".to_string();
                let mut product = String::new();

                for part in &parts[2..] {
                    if let Some(m) = part.strip_prefix("model:") {
                        model = m.replace('_', " ");
                    } else if let Some(p) = part.strip_prefix("product:") {
                        product = p.to_string();
                    }
                }

                devices.push(AdbDevice {
                    id,
                    model,
                    product,
                    status,
                });
            }
        }

        Ok(devices)
    }

    /// Select an active device or automatically choose the default if only one device is connected
    pub fn select_device(preferred_id: Option<&str>) -> Result<AdbDevice, String> {
        let devices = Self::list_devices()?;

        if devices.is_empty() {
            return Err("No Android devices found. Please connect your phone via USB with USB Debugging enabled, or start an emulator.".to_string());
        }

        if let Some(id) = preferred_id {
            devices
                .into_iter()
                .find(|d| d.id.eq_ignore_ascii_case(id) || d.model.to_lowercase().contains(&id.to_lowercase()))
                .ok_or_else(|| format!("Specified device '{}' not found in connected devices.", id))
        } else if devices.len() == 1 {
            Ok(devices[0].clone())
        } else {
            let list_str = devices
                .iter()
                .map(|d| format!("  • {} ({}) - {}", d.id, d.model, d.status))
                .collect::<Vec<_>>()
                .join("\n");
            Err(format!(
                "Multiple devices detected. Please specify using `--device <id>`:\n{}",
                list_str
            ))
        }
    }

    /// Deploy and install an APK onto the target Android device
    pub fn deploy(
        device_id: Option<&str>,
        apk_path: &Path,
        package_name: &str,
        activity: Option<&str>,
        launch: bool,
        setup_live_reload_port: Option<u16>,
    ) -> Result<DeployReport, String> {
        if !apk_path.exists() {
            return Err(format!("Target APK file '{}' does not exist.", apk_path.display()));
        }

        let device = Self::select_device(device_id)?;

        println!("================================================================================");
        println!("📱 AETHER 1-CLICK MOBILE DEPLOYER");
        println!("Target Device: {} ({}) [{}]", device.model, device.product, device.id);
        println!("Target APK:    {} ({} KB)", apk_path.display(), apk_path.metadata().map(|m| m.len() / 1024).unwrap_or(0));
        println!("================================================================================");

        // 1. Setup reverse port forwarding for Live Hot-Reload if requested
        let mut reverse_ok = false;
        if let Some(port) = setup_live_reload_port {
            let port_str = format!("tcp:{}", port);
            print!("  [1/3] Reverse port-forwarding {} -> mobile localhost... ", port);
            let rev_res = Command::new("adb")
                .args(["-s", &device.id, "reverse", &port_str, &port_str])
                .output();
            if rev_res.map(|o| o.status.success()).unwrap_or(false) {
                println!("OK ✅");
                reverse_ok = true;
            } else {
                println!("SKIPPED ⚠️");
            }
        }

        // 2. Install APK with replace (-r) and downgrade permissions (-d)
        print!("  [2/3] Installing APK onto {}... ", device.model);
        let install_res = Command::new("adb")
            .args(["-s", &device.id, "install", "-r", "-d", &apk_path.to_string_lossy()])
            .output()
            .map_err(|e| format!("Failed to invoke 'adb install': {}", e))?;

        let install_out = String::from_utf8_lossy(&install_res.stdout);
        let install_err = String::from_utf8_lossy(&install_res.stderr);

        if !install_res.status.success() || (!install_out.contains("Success") && !install_err.contains("Success")) {
            println!("FAILED ❌");
            return Err(format!("ADB installation failed:\n{}{}", install_out, install_err));
        }
        println!("SUCCESS ✅");

        // 3. Launch App Activity
        let mut launched = false;
        if launch {
            let target_activity = activity.unwrap_or(".MainActivity");
            let component = format!("{}/{}", package_name, target_activity);
            print!("  [3/3] Launching {}... ", component);
            let launch_res = Command::new("adb")
                .args(["-s", &device.id, "shell", "am", "start", "-n", &component])
                .output();

            if launch_res.map(|o| o.status.success()).unwrap_or(false) {
                println!("LAUNCHED 🚀");
                launched = true;
            } else {
                println!("FAILED ⚠️");
            }
        }

        println!("================================================================================");
        println!("✨ App is now LIVE on your {}!", device.model);
        println!("================================================================================\n");

        Ok(DeployReport {
            device,
            apk_path: apk_path.to_path_buf(),
            installed: true,
            launched,
            reverse_port_forwarded: reverse_ok,
            message: "Mobile deployment completed successfully.".into(),
        })
    }
}
