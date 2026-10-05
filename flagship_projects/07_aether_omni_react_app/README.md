# Flagship Project 07: AETHER Omni (Universal React + Tailwind Native App)

> **Zero-SDK Multi-Platform Compilation**: Bundle React 18, Vue 3, Svelte, or Tailwind CSS into native, signed Android APKs and Windows executables with **ZERO Android Studio**, **ZERO Gradle**, and **ZERO external dependencies**.

---

## ⚡ Overview

While frameworks like **Tauri** and **React Native** enable web frontend developers to target desktop and mobile, they impose massive friction on mobile workflows:
* **Tauri Mobile & React Native:** Require downloading and configuring **10–15 GB of external tools** (Android Studio, Android Command Line Tools, Android SDK platforms, NDK, Java JDK, Gradle). Building a simple test APK often takes **5 to 10 minutes**.
* **AETHER 2.0 (`AetherShell`):** Ships with a **built-in, pure-Rust Dalvik DEX bytecode compiler, Android Binary XML (AXML) synthesizer, and APK signer**. You can compile any React, Vue, Svelte, or Tailwind project into a signed, installable Android APK (`.apk`) and a standalone Windows executable (`.exe`) in **less than 1 second** with **0 MB external downloads**.

---

## 🏗️ Architecture

```
┌────────────────────────────────────────────────────────┐
│   React 18 / Tailwind / Vue / Svelte Frontend UI       │
│        (frontend/index.html + Modern Components)       │
└───────────────────────────┬────────────────────────────┘
                            │
               window.Aether Bridge (aether_bridge.js)
                            │
       ┌────────────────────┴────────────────────┐
       ▼                                         ▼
┌───────────────────────────────┐ ┌───────────────────────────────┐
│       Android APK Target      │ │    Desktop Windows Target     │
│  - Pure-Rust Dalvik (035 DEX) │ │  - Native Self-Contained EXE  │
│  - Binary AXML Manifest       │ │  - Embedded In-Memory DB      │
│  - Android V1 RSA Signature   │ │  - Instant Cold Start (<10ms) │
│  - Native Hardware Haptics    │ │  - Cranelift Native Engine    │
└───────────────────────────────┘ └───────────────────────────────┘
```

---

## 🚀 How to Run & Bundle

### 1. Run Backend Logic Directly
```bash
aether run flagship_projects/07_aether_omni_react_app/main.ae
```

### 2. Bundle to Android APK (Zero Android Studio!)
```bash
aether bundle flagship_projects/07_aether_omni_react_app/frontend \
  --target apk \
  -o flagship_projects/07_aether_omni_react_app/AetherOmni.apk \
  --name "AETHER Omni" \
  --package com.aether.omni
```
* **Build Time:** ~0.05 seconds
* **Install:** `adb install -r AetherOmni.apk`

### 3. Bundle to Native Windows Executable
```bash
aether bundle flagship_projects/07_aether_omni_react_app/frontend \
  --target desktop \
  -o flagship_projects/07_aether_omni_react_app/AetherOmni.exe \
  --name "AETHER Omni"
```

### 4. Bundle to Single-File Web Application
```bash
aether bundle flagship_projects/07_aether_omni_react_app/frontend \
  --target web \
  -o flagship_projects/07_aether_omni_react_app/AetherOmni.html \
  --name "AETHER Omni"
```

---

## 📳 `AetherBridge` JavaScript API

Any modern web UI framework (React, Vue, Svelte, Angular, Solid) can communicate with native hardware and backend systems via `window.Aether`:

```javascript
// 1. Hardware Vibration & Haptics
window.Aether.vibrate(50); // Vibrates 50ms
window.Aether.hapticFeedback('medium'); // Presets: 'light', 'medium', 'heavy'

// 2. Native System Toast Notifications
window.Aether.showToast("Record successfully updated!");

// 3. Hardware & Telemetry Metrics
const info = await window.Aether.deviceInfo();
console.log(info.platform, info.cores, info.memory);

// 4. Built-in Relational SQL (DB.*)
const result = await window.Aether.dbQuery("SELECT * FROM tasks WHERE status = 'Active'");
console.log(result.results);

// 5. Autonomous Local AI Engine
const aiResp = await window.Aether.ai("Analyze user sentiment");
console.log(aiResp.response);

// 6. Generic RPC Invocation
const rpc = await window.Aether.invoke("custom_command", { key: "value" });
```

---

## ⚖️ Feature Comparison Matrix

| Feature | AETHER 2.0 | Tauri Mobile | React Native |
|---|---|---|---|
| **Android Studio Required?** | ❌ **NO (0 MB)** | ⚠️ YES (15+ GB) | ⚠️ YES (15+ GB) |
| **NDK & Gradle Required?** | ❌ **NO** | ⚠️ YES | ⚠️ YES |
| **APK Build Time** | ⚡ **< 1 Second** | ⏱️ 4–8 Minutes | ⏱️ 5–12 Minutes |
| **Bytecode Generation** | Pure Rust Dalvik Synthesizer | Java / Gradle Pipeline | Metro + Gradle Pipeline |
| **Built-in Relational SQL** | ✅ **Yes (`DB.*`)** | ❌ Plugin Needed | ❌ NPM Package Needed |
| **Desktop Executable Bundling** | ✅ **Native Single-File EXE** | ✅ Supported | ❌ Complex / Electron |
| **Single-File Web Distribution** | ✅ **Standalone Inlined HTML** | ❌ Not Supported | ❌ Not Supported |
