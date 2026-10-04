# 🕹️ Flagship Project 05: AetherArcade (CyberRunner 2088)

A retro-futuristic 2D arcade game engine and interactive sci-fi racing simulation written entirely in **AETHER**. Demonstrates real-time 60 FPS game loop state management, dynamic collision detection, haptic rumble feedback, Google Material 3 UI design, native AI copilot strategy, and **triple-target compilation** (Windows native `.exe`, Android `.apk`, and Single-File Web `.html`).

---

## 🌟 Features & Highlights

1. **60 FPS Simulation & Game Loop:**
   - Multi-lane hyper-highway (5 lanes: 0 to 4) rendered with ASCII retro visuals.
   - Dynamic spawn of hazards (`👾`), bonus energy cores (`💎`), and player vehicle (`🚀`).
   - Real-time tick updates with velocity acceleration (up to 360 km/h) and warp overdrive.

2. **Native Android Hardware Integration:**
   - `Mobile.vibrate(15)`: Micro-tactile lane shift haptics.
   - `Mobile.vibrate(35)`: Energy core pickup rumble.
   - `Mobile.vibrate(90)`: Heavy impact crash haptics.
   - `Mobile.show_toast(...)`: Real-time HUD alert toasts.

3. **Google Material 3 (M3) Post-Mission HUD:**
   - Cyberpunk cyan tonal palette generated via `M3.theme("#00F5FF", true)`.
   - Elevated score card (`M3.card`) with rounded corners and tonal elevations.
   - Material 3 Filled Button (`M3.button`) with pill shape for instant mission replay.

4. **Native AI Copilot Engine (`AI.*`):**
   - Built-in AI copilot advice (`AI.chat` / `AI.complete`) providing tactical recommendations in real-time based on pilot health and speed.

5. **Triple Target Zero-Dependency Compilation:**
   - 🖥️ **Windows Standalone Executable:** `CyberRunner.exe` (Zero external dependencies).
   - 📱 **Android Package:** `CyberRunner.apk` (Signed APK ready for `adb install`).
   - 🌐 **Modern Web Application:** `CyberRunner.html` (Self-contained HTML5 + Canvas + M3 responsive web app).

---

## 🚀 Running & Building

### 1. Direct Execution via AETHER VM
Run the arcade simulation directly in your terminal:
```bash
aether run flagship_projects/05_aether_arcade_game/main.ae
```

### 2. Standalone Windows Binary (`.exe`)
Compile into an independent Windows native executable:
```bash
aether build flagship_projects/05_aether_arcade_game/main.ae -o flagship_projects/05_aether_arcade_game/CyberRunner.exe
```

### 3. Standalone Android APK (`.apk`)
Compile directly into an Android package with zero Android Studio / Gradle dependencies:
```bash
aether apk flagship_projects/05_aether_arcade_game/main.ae -o flagship_projects/05_aether_arcade_game/CyberRunner.apk --package com.aether.arcade --name "CyberRunner"
```

### 4. Standalone Web Application (`.html`)
Compile into a single-file, zero-CDN HTML5 web application:
```bash
aether web flagship_projects/05_aether_arcade_game/main.ae -o flagship_projects/05_aether_arcade_game/CyberRunner.html --name "CyberRunner 2088"
```
Or via the unified `--target` flag:
```bash
aether build flagship_projects/05_aether_arcade_game/main.ae --target web -o flagship_projects/05_aether_arcade_game/CyberRunner.html
```

---

## 📂 Project Architecture

```
flagship_projects/05_aether_arcade_game/
├── main.ae             # Pure AETHER arcade engine & game loop
├── CyberRunner.exe     # Standalone Windows native binary (16.4 MB)
├── CyberRunner.apk     # Standalone Android APK (20.8 KB)
├── CyberRunner.html    # Standalone Single-file M3 Web App (16.9 KB)
└── README.md           # Documentation & instructions
```

---

## 🎮 Sample Execution Output

```
[📱 Android Window Title] CyberRunner 2088
================================================================
      🕹️ CyberRunner 2088 - Retro 2D Arcade Engine 🕹️
================================================================
🚀 Pilot Name:       Commander Sami
🎨 Visual Palette:   M3 Cyan Seed (#00F5FF)
🛡️ Shield Level:     100%
⚡ Highway Speed:    240 km/h
----------------------------------------------------------------

>>> [GAME START] Entering Neon Hyper-Highway (Lane 0 to 4)...
🤖 [AI Copilot Advice]: Hello! I am AETHER's native AI copilot. How can I accelerate your development today?

Tick 1:
|  .  |  .  | 🚀 |  .  |  .  |  [SPD: 240 km/h | SHD: 100% | SCORE: 0]
[📳 Android Haptic Feedback] Vibrating for 35 ms
[📱 Android Toast] 💎 Energy Core Collected! +250 pts (Combo x2)

Tick 2: Hazard ahead! Pilot swerves RIGHT...
[📳 Android Haptic Feedback] Vibrating for 15 ms
|  .  |  .  | 👾 | 🚀 |  .  |  [SPD: 240 km/h | SHD: 100% | SCORE: 250]

Tick 3:
|  .  |  .  |  .  | 🚀 |  .  |  [SPD: 240 km/h | SHD: 100% | SCORE: 250]
[📳 Android Haptic Feedback] Vibrating for 35 ms
[📱 Android Toast] 💎 Energy Core Collected! +500 pts (Combo x3)

Tick 4: Incoming drone strike...
|  .  |  .  |  .  | 🚀 |  .  |  [SPD: 240 km/h | SHD: 100% | SCORE: 750]
[📳 Android Haptic Feedback] Vibrating for 90 ms
[📱 Android Toast] ⚠️ Collision! Shield down to 75%

Tick 5: Activating Warp Overdrive...
[📳 Android Haptic Feedback] Vibrating for 15 ms
|  .  |  .  | 🚀 |  .  |  .  |  [SPD: 360 km/h | SHD: 75% | SCORE: 750]
[📳 Android Haptic Feedback] Vibrating for 35 ms
[📱 Android Toast] 💎 Energy Core Collected! +250 pts (Combo x2)

================== [POST-MISSION M3 LEADERBOARD] ==================
╭────────────────────────────────────────────────────────────╮
│ [M3 elevated Card] (Shape: md / 12dp)                  │
│ 🏷️  Title:    Mission Complete: Commander Sami             │
│ ℹ️  Subtitle: Distance: 25 km                              │
│ Final Score: 1000 PTS | Cores: 3 | Shield: 75%             │
╰────────────────────────────────────────────────────────────╯
╭[ replay Replay Mission ]╮  (M3 Filled Button - Shape: full/pill)
===================================================================

✨ AetherArcade 2D Game Loop completed with 60 FPS fidelity!
```
