# 📱 AetherPocket: Flagship Android Mobile Application

An idiomatic, high-performance mobile application written entirely in **AETHER** and compiled directly into an installable Android Package (`.apk`) with **zero external toolchain dependencies**.

---

## 🌟 Features & Highlights

- **Pure AETHER Codebase:** Built using Pythonic syntax, F-strings (`f"..."`), and pipeline operators (`|>`).
- **Zero-Dependency APK Generation:** The AETHER compiler synthesizes Android Binary XML (`AndroidManifest.xml`), Dalvik bytecode (`classes.dex`), resource tables (`resources.arsc`), icons, and self-signed PKCS#7 signatures (`META-INF/`) directly in pure Rust.
- **Native Android Hardware Bridge:**
  - `Mobile.show_toast(msg)`: Displays native Android toasts.
  - `Mobile.vibrate(ms)`: Triggers precise haptic feedback.
  - `Mobile.battery_level()`: Reads device battery percentage.
  - `Mobile.device_info()`: Queries hardware model, Android OS version, and CPU architecture (`arm64-v8a`).
  - `Mobile.network_status()`: Detects Wi-Fi, cellular, and offline states.
- **Reactive UI & State System:** Real-time state management for step counts, hydration goals, calorie burn, and dark/light themes.

---

## 🚀 Building the Android APK

### Option 1: Direct Standalone APK (Recommended)
Compile your `.ae` script directly into a signed, installable Android `.apk` in under 1 second:

```bash
# Using the build command with target
aether build main.ae --target apk -o AetherPocket.apk

# Or using the dedicated apk command
aether apk main.ae -o AetherPocket.apk --package com.aether.pocket --name "AetherPocket"
```

Output:
```
[AETHER APK] Parsed 54 statements for Android package 'com.aether.pocket'.
[AETHER APK] Compiling AETHER Universal Bytecode...
[AETHER APK] Synthesizing Android Manifest, DEX bytecode & Resources...
[AETHER APK] Signing APK with self-signed developer certificate (META-INF)...
✨ Android APK successfully generated: AetherPocket.apk
   📦 Package:      com.aether.pocket
   🏷️  App Name:     AetherPocket
   📱 Target SDK:   Android 14 (API 34) | Min: API 21
   ⚖️  Size:         4.28 KB
   🔑 Signature:    V1 Signed (Self-signed debug keystore)
   🚀 To Install:   adb install -r AetherPocket.apk
```

---

### Option 2: Full Android Studio / Gradle Project Export
If you want to open the project in Android Studio or add Kotlin/Java/Compose dependencies:

```bash
aether build main.ae --target android-project -o AetherPocketAndroid/
```

This generates a complete, modern Android Studio project:
```
AetherPocketAndroid/
├── build.gradle.kts
├── settings.gradle.kts
└── app/
    ├── build.gradle.kts
    └── src/main/
        ├── AndroidManifest.xml
        ├── java/com/aether/runtime/MainActivity.kt
        └── assets/
            └── app.ae
```

---

## 📲 Installing on Device or Emulator

With USB debugging enabled or an Android Virtual Device (AVD) running:

```bash
adb install -r AetherPocket.apk
```

To launch directly via ADB:
```bash
adb shell am start -n com.aether.pocket/com.aether.runtime.MainActivity
```

---

## 📂 Source Code Walkthrough (`main.ae`)

```aether
# 1. Calorie computation pipeline using AETHER's |> operator
def clean_sensor_jitter(val):
    if val < 0: return 0
    return val

def apply_step_calibration(steps):
    return steps * 1.05

def steps_to_calories(steps):
    return steps * 0.04

# 2. Recording steps with Android haptics
def record_steps(raw_steps):
    calibrated = raw_steps |> clean_sensor_jitter |> apply_step_calibration
    current_steps = current_steps + int(calibrated)
    
    # Trigger Android haptic vibration
    Mobile.vibrate(25)
    
    cals = current_steps |> steps_to_calories
    print(f"👟 Total Steps: {current_steps} | Burn: {cals:.1f} kcal")
```
