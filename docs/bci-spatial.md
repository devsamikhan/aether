# AETHER BCI & Spatial Computing Primitives

> **Status:** Phase 1 — Conceptual / Simulated  
> **Roadmap:** Real hardware integration (OpenBCI, Neurosity Crown) and XR runtimes (OpenXR, ARKit, ARCore) are Phase 2 items.  
> **Disclaimer:** Neural interfaces described here are simulated in Phase 1. No actual neural signals are read or written.

---

## Overview

AETHER's BCI (Brain-Computer Interface) and Spatial Computing primitives extend the language into two converging frontiers of human-computer interaction:

**Brain-Computer Interfaces** allow the human nervous system to serve as an input device. Rather than keyboard or mouse, programs respond to **neural signals** — electrical patterns in the brain measurable by EEG (electroencephalography) headsets. AETHER models this with primitives that bind brain region streams to program execution and map detected cognitive intents to actions.

**Spatial Computing** dissolves the boundary between screen and world. Holographic, AR, VR, and XR interfaces place interactive digital objects in physical space. AETHER's spatial primitives provide language-level abstractions for declaring holograms, anchoring content to physical surfaces, processing gaze and hand tracking, and managing immersive sessions.

### Architecture

```
Phase 1 (Simulated):

  Neural input:     Simulated EEG signal generator
  Intent mapping:   Rule-based pattern classifier (threshold on band power)
  Rendering:        Software renderer / browser WebXR preview
  Haptic feedback:  System audio as proxy for haptic events

Phase 2 (Hardware):

  Neural input:     OpenBCI Cyton (8ch) / Neurosity Crown (8ch, ML-processed)
  Intent mapping:   On-device neural net (EEGNet architecture)
  Rendering:        OpenXR runtime (SteamVR / Meta / HoloLens / WebXR)
  Haptic feedback:  Haptic gloves / controller rumble / ultrasonic haptics
```

### Signal Processing Pipeline

```
Raw EEG (μV) → Bandpass Filter → Band Power Extraction → Intent Classifier → AETHER Event
     ↓               ↓                    ↓                      ↓
  250 Hz         [1–50 Hz]           δ,θ,α,β,γ            thought_intent("name")
```

---

## `cortex_bind`

### Concept

`cortex_bind` is the **top-level BCI primitive**. It establishes a binding between a neural signal stream from a specified brain region and a block of AETHER code. The bound block defines a mapping from detected cognitive intents to program actions using `thought_intent` clauses.

While the `cortex_bind` block is active, the AETHER runtime continuously monitors the neural stream. When a recognized intent pattern is detected, its corresponding action fires. `cortex_bind` is **non-blocking**: the program continues executing other code concurrently while the binding listens for intents.

### Syntax

```aether
cortex_bind neural_stream("region") {
    thought_intent("intent_name") => action;
    thought_intent("intent_name") => { multi_line_block; };
};
```

### Examples

**Example 1 — Basic motor intent binding:**
```aether
cortex_bind neural_stream("motor_cortex") {
    thought_intent("left_hand_move")  => robot_arm.move(direction: LEFT);
    thought_intent("right_hand_move") => robot_arm.move(direction: RIGHT);
    thought_intent("grip")            => robot_arm.grip(force: 0.7);
    thought_intent("release")         => robot_arm.release();
};
```

**Example 2 — Prefrontal context switching:**
```aether
cortex_bind neural_stream("prefrontal") {
    thought_intent("focus_deep") => {
        display.set_mode(DISTRACTION_FREE);
        notifications.mute(duration: 30min);
        emit("Deep focus mode activated");
    };
    
    thought_intent("context_switch") => {
        let saved = workspace.snapshot();
        workspace.switch_to(next_context);
    };
    
    thought_intent("recall") => {
        let memory = semantic_memory.retrieve(attention.current_focus);
        display.overlay(memory, position: gaze_position());
    };
};
```

**Example 3 — Multisensory game controller:**
```aether
cortex_bind neural_stream("motor_cortex") {
    thought_intent("jump")       => character.jump();
    thought_intent("run")        => character.sprint(speed: 1.5);
    thought_intent("attack")     => character.attack(weapon: equipped);
    thought_intent("defend")     => character.block();
};

cortex_bind neural_stream("occipital") {
    // Occipital signals correlate with visual attention focus
    thought_intent("target_locked") => targeting_system.lock(gaze_target());
};
```

**Example 4 — Neural accessibility interface:**
```aether
// Enable full computer control via thought alone for motor-impaired users
cortex_bind neural_stream("motor_cortex") {
    thought_intent("select")    => cursor.click();
    thought_intent("back")      => os.navigate_back();
    thought_intent("scroll_up") => display.scroll(direction: UP, speed: 2.0);
    thought_intent("scroll_down") => display.scroll(direction: DOWN, speed: 2.0);
};

cortex_bind neural_stream("prefrontal") {
    thought_intent("type_word") => {
        let word = semantic_decoder.predict_word(neural_stream.current_pattern);
        keyboard.type(word);
    };
};
```

---

## `neural_stream`

### Concept

A `neural_stream` is a **real-time stream of neural signals** from a specified brain region, sampled by a BCI headset at the configured sample rate (typically 250 Hz or 512 Hz). Each sample contains the raw EEG voltage values for all active electrodes in that region, plus processed band-power features extracted by the AETHER runtime.

In Phase 1, `neural_stream` is backed by a **signal simulator** that generates synthetic EEG data matching statistical properties of real recordings (realistic band power ratios, 1/f noise spectrum, artifact injection).

### Brain Regions

| Region            | Electrodes (10-20 system) | Associated Signals                      |
|-------------------|--------------------------|-----------------------------------------|
| `motor_cortex`    | C3, Cz, C4              | Motor imagery, movement intention       |
| `sensory_cortex`  | CP3, CPz, CP4           | Tactile sensation, proprioception       |
| `prefrontal`      | Fp1, Fpz, Fp2, AF3, AF4 | Working memory, attention, executive fn |
| `temporal`        | T7, T8, TP9, TP10       | Auditory processing, language           |
| `occipital`       | O1, Oz, O2, PO7, PO8   | Visual processing, attention targets    |
| `parietal`        | P3, Pz, P4              | Spatial reasoning, attention            |

### Syntax

```aether
neural_stream("region")
neural_stream("region", sample_rate: 512, channels: ["C3", "Cz", "C4"])
```

### Examples

**Example 1 — Read raw band power:**
```aether
let stream = neural_stream("motor_cortex");
let alpha_power = stream.band_power(eeg_band.alpha);
let beta_power  = stream.band_power(eeg_band.beta);

// Event-related desynchronization (ERD) signals motor intent
float erd = (baseline_alpha - alpha_power) / baseline_alpha;
if erd > 0.3 { emit("Motor intent detected (ERD = ${erd})"); }
```

**Example 2 — Stream snapshot for ML inference:**
```aether
let stream  = neural_stream("prefrontal", sample_rate: 256);
let window  = stream.sliding_window(duration: 1s);   // 256 samples × N channels
let features = feature_extractor.psd(window);        // Power spectral density
let intent   = intent_model.predict(features);       // EEGNet inference
emit("Detected intent: ${intent.label}  confidence: ${intent.confidence}");
```

**Example 3 — Continuous stream monitoring:**
```aether
neural_stream("motor_cortex") on_sample { sample =>
    // Called at 250 Hz
    if sample.beta_power > MOVEMENT_THRESHOLD {
        pheromone("movement_intent", intensity: sample.beta_power);
    }
};
```

---

## `thought_intent`

### Concept

`thought_intent` **maps a named cognitive intent** to a program action. The intent name corresponds to a pattern stored in the runtime's intent classifier. When the classifier's confidence for a named intent exceeds the detection threshold, the mapped action fires.

Intent names are strings that reference pre-trained classification templates. AETHER ships with a set of **common intents** (motor imagery patterns, attention state changes) and supports user-defined custom intent training (Phase 2 feature).

### Common Built-in Intents

| Intent Name           | EEG Correlate                              | Reliability (simulated) |
|-----------------------|--------------------------------------------|------------------------|
| `left_hand_move`      | Contralateral C3 desynchronization (8-13Hz)| High                   |
| `right_hand_move`     | Contralateral C4 desynchronization (8-13Hz)| High                   |
| `both_hands`          | Bilateral C3+C4 desynchronization          | High                   |
| `feet_move`           | Cz desynchronization                       | Medium                 |
| `focus_deep`          | Frontal theta ↑, posterior alpha ↓         | Medium                 |
| `relax`               | Posterior alpha ↑                          | High                   |
| `target_locked`       | Occipital P300 component                   | Medium                 |
| `context_switch`      | Prefrontal beta ↑ with N200 component      | Low                    |
| `type_word`           | Language cortex Broca activation (temporal)| Low (Phase 2)          |

> **Note:** Reliability is based on simulated complexity analysis and cognitive signal model characteristics under simulation, not runs on physical BCI hardware.

### Syntax

```aether
thought_intent("intent_name") => single_action;
thought_intent("intent_name", threshold: 0.8) => action;   // custom confidence
thought_intent("intent_name") => {
    // multi-line action block
};
```

### Examples

**Example 1 — Single line action:**
```aether
thought_intent("relax") => music.play(playlist: AMBIENT);
```

**Example 2 — Custom confidence threshold:**
```aether
// Higher threshold = fewer false positives, more false negatives
thought_intent("left_hand_move", threshold: 0.90) => robot.move_left();
```

**Example 3 — Multi-action sequence:**
```aether
thought_intent("focus_deep") => {
    notifications.mute_all();
    display.set_brightness(0.4);
    ambient_sound.play("brown_noise");
    timer.start(duration: 25min, on_end: { notifications.unmute(); });
};
```

---

## EEG Band & Cortex Primitives

### `eeg_band`

An enum type representing the five canonical EEG frequency bands. Used to extract band-specific power from a neural stream. Each band correlates with distinct cognitive states.

```aether
eeg_band.delta   // 0.5–4 Hz   — deep sleep, unconscious processing
eeg_band.theta   // 4–8 Hz     — drowsiness, meditation, memory encoding
eeg_band.alpha   // 8–13 Hz    — relaxed alertness, idle state
eeg_band.beta    // 13–30 Hz   — active thought, motor readiness, anxiety
eeg_band.gamma   // 30–100 Hz  — high-level cognition, binding, consciousness

let power = neural_stream("motor_cortex").band_power(eeg_band.beta);
```

---

### `motor_cortex`

A shorthand stream alias for the motor cortex region. Equivalent to `neural_stream("motor_cortex")`. Designed for programs where motor intent is the primary BCI modality.

```aether
cortex_bind motor_cortex {
    thought_intent("left_hand_move")  => action_left();
    thought_intent("right_hand_move") => action_right();
};
```

---

### `sensory_cortex`

A shorthand stream alias for the sensory cortex. Used for programs that synthesize tactile or proprioceptive feedback, or read sensory evoked potentials (SEPs).

```aether
cortex_bind sensory_cortex {
    thought_intent("touch_detected") => haptic.pulse(finger: index, intensity: 0.5);
};
```

---

## `hologram`

### Concept

`hologram` declares a **spatial holographic rendering target** — a volumetric or planar digital object placed at a defined position in the user's physical or virtual space. In Phase 1, holograms render to a WebXR or software compositor preview window. In Phase 2, they render to HoloLens, Magic Leap, or compatible AR displays.

Each hologram has a **world-space position**, a **content descriptor** (what to render), and optional **interaction handlers** (what to do when the user gazes at, points at, or taps the hologram).

### Syntax

```aether
hologram name {
    position: vec3(x, y, z);         // World position in meters
    rotation: quaternion(x, y, z, w); // Orientation (optional, default: identity)
    scale:    vec3(sx, sy, sz);       // Scale (optional, default: 1,1,1)
    content:  ContentType { ... };    // What to render
    on_gaze:  { action };            // Triggered when user looks at it
    on_select: { action };           // Triggered when user taps/clicks it
};
```

### Examples

**Example 1 — Floating data dashboard:**
```aether
hologram dashboard {
    position: vec3(0.0, 1.5, -1.0);    // 1.5m up, 1m in front
    content:  panel {
        title: "System Status";
        metrics: [cpu_usage, mem_usage, net_throughput];
        refresh: 1Hz;
    };
    on_gaze: { dashboard.highlight(); }
    on_select: { dashboard.expand(); }
};
```

**Example 2 — 3D molecular structure visualization:**
```aether
hologram molecule_viewer {
    position:  vec3(-0.5, 1.2, -0.8);
    scale:     vec3(0.3, 0.3, 0.3);
    content:   model_3d {
        source: "atp_molecule.glb";
        animation: rotate(axis: Y, speed: 30deg_per_sec);
    };
    on_select: { molecule_viewer.toggle_bond_labels(); }
};
```

**Example 3 — Spatial code editor:**
```aether
hologram code_panel {
    position: vec3(0.0, 1.6, -0.9);
    scale:    vec3(1.2, 0.8, 0.01);
    content:  code_editor {
        language: "aether";
        theme:    "aether_dark";
        font_size: 14pt;
    };
    on_gaze:   { cursor.set_target(code_panel); }
    on_select: { code_panel.activate_cursor(); }
};

// BCI-driven code editing
cortex_bind neural_stream("prefrontal") {
    thought_intent("type_word") => {
        let word = semantic_decoder.predict_word(neural_stream.current_pattern);
        code_panel.insert(word);
    };
};
```

**Example 4 — AR navigation overlay:**
```aether
hologram nav_arrow {
    position:  gps_to_world(next_waypoint);
    content:   arrow_3d {
        direction: heading_to(next_waypoint);
        color:     rgba(0, 220, 255, 0.85);
        pulse:     true;
    };
};
```

---

## Spatial Anchor & Depth Primitives

### `spatial_anchor`

Pins a hologram or virtual object to a **specific physical location** in the real world. When the user leaves and returns to that location, the anchored object reappears at the same world-space position. Uses SLAM (Simultaneous Localization and Mapping) in Phase 2.

```aether
spatial_anchor "desk_anchor" {
    world_position: vec3(1.2, 0.0, 3.4);   // Physical world coordinates
    attach: [dashboard, code_panel];         // Holograms locked to this anchor
};

// Persist anchor across sessions
spatial_anchor.save("desk_anchor", to: cloud_anchor_store);
spatial_anchor.load("desk_anchor", from: cloud_anchor_store);
```

---

### `depth_mesh`

Requests a **real-time 3D mesh** of the physical environment from the device's depth sensor. Used for **occlusion** (holograms hidden behind real objects), **collision** (virtual objects resting on real surfaces), and **scene understanding**.

```aether
depth_mesh scene {
    resolution: medium;           // low | medium | high
    update_rate: 5Hz;
    on_update: { occlusion.rebuild(scene.mesh); }
};

// Place a virtual object on the real floor
let floor_plane = scene.detect_plane(type: horizontal, min_area: 0.5m²);
hologram virtual_table {
    position: floor_plane.center + vec3(0, 0.75, 0);   // Table height
    content:  model_3d { source: "table.glb"; };
};
```

---

### `haptic`

Triggers a **haptic feedback event** — a vibrotactile signal sent to a haptic device (controller, haptic glove, ultrasonic mid-air haptics). In Phase 1, haptic events trigger a system audio pulse as a proxy.

```aether
haptic.pulse(intensity: 0.8, duration: 50ms);
haptic.pattern(type: "double_click");
haptic.texture(surface: gaze_target(), roughness: 0.6);
haptic.rumble(frequency: 120Hz, duration: 200ms);
```

---

## Input Tracking Primitives

### `gaze_track`

Provides **real-time eye tracking data**: gaze direction, fixation point in world space, saccade detection, and pupil dilation. Used for foveated rendering, intent prediction, and gaze-based interaction.

```aether
gaze_track enabled;

gaze_track on_fixation { fixation =>
    if fixation.duration > 800ms {
        // Dwell-based selection — user looked at something for 0.8s
        fixation.target.select();
    }
};

let gaze_pos = gaze_track.world_intersection(depth_mesh.scene);
```

---

### `hand_track`

Provides **real-time hand and finger pose tracking** — 21 keypoints per hand (MediaPipe-style skeleton). Used for pinch, grab, point, swipe, and custom gesture recognition.

```aether
hand_track enabled;

hand_track on_gesture { gesture =>
    match gesture.type {
        "pinch"      => selected_object.select(),
        "grab"       => selected_object.grab(hand: gesture.hand),
        "open_palm"  => selected_object.release(),
        "point"      => cursor.move_to(gesture.ray_intersection),
        "swipe_left" => workspace.navigate(LEFT),
        "swipe_right"=> workspace.navigate(RIGHT),
    }
};

let right_index_tip = hand_track.right.finger(index).tip.world_position;
```

---

### `voice_cmd`

Registers a **spoken voice command** that triggers an action when the keyword or phrase is recognized. Uses on-device speech recognition in Phase 2; uses OS speech API in Phase 1.

```aether
voice_cmd "hey aether, open terminal"  => terminal.open();
voice_cmd "hey aether, close"          => active_window.close();
voice_cmd "hey aether, run"            => compiler.run(active_file);
voice_cmd "hey aether, zoom in"        => display.zoom(factor: 1.5);

// Wildcard argument capture
voice_cmd "hey aether, open {filename}" => {
    let file = workspace.find(filename);
    editor.open(file);
};
```

---

### `ar_overlay`

Renders a 2D or 3D element as an **augmented reality overlay** composited over the live camera feed. Simpler than `hologram` — does not require world-space anchoring or depth.

```aether
ar_overlay "fps_counter" {
    anchor:  screen_corner(TOP_RIGHT);
    content: text { value: "${fps}fps"; color: rgba(0,255,128,0.9); font_size: 12pt; };
    update:  every(100ms);
};

ar_overlay "object_label" {
    anchor:  world_object(detected_object);   // Follows a tracked real object
    content: text { value: detected_object.label; };
};
```

---

### `vr_world`

Declares a **fully immersive virtual reality world** context. Inside a `vr_world` block, the physical environment is replaced by a virtual scene. All spatial primitives (`hologram`, `spatial_anchor`, `depth_mesh`) operate within the virtual coordinate system.

```aether
vr_world "dev_environment" {
    skybox:    "aether_nebula.hdr";
    gravity:   vec3(0, -9.8, 0);
    ambient:   rgba(20, 10, 40, 1.0);
    
    hologram workstation {
        position: vec3(0, 0.9, -0.6);
        content:  multi_monitor { screens: 3; resolution: 2K; };
    };
    
    hologram reference_docs {
        position: vec3(1.2, 1.3, -0.8);
        content:  browser { url: "https://aether-lang.dev/docs"; };
    };
};
```

---

### `xr_session`

Manages the **lifecycle of an XR (Extended Reality) session** — the top-level context that activates the device's XR runtime. Handles session start, render loop, frame submission, and session end.

```aether
xr_session "main" {
    mode:       AR | VR | MR;           // Augmented, Virtual, or Mixed reality
    refresh:    90Hz;                   // Display refresh rate
    fov:        110deg;                 // Field of view (device-limited)
    resolution: vec2(2064, 2096);       // Per-eye resolution
    
    on_start:  { emit("XR session started"); }
    on_frame:  { render_frame(xr_session.current_frame); }
    on_end:    { emit("XR session ended"); cleanup(); }
    on_error:  { err => log.error("XR error: ${err}"); }
};
```

---

## Use Cases

### Neural Game Controller

A hands-free game controller that maps motor cortex signals to in-game actions. Players imagine physical movements (left hand grip, right hand lift) to control characters without pressing buttons.

```aether
vr_world "game_arena" {
    // ... world setup ...
};

cortex_bind motor_cortex {
    thought_intent("left_hand_move")  => player.strafe(LEFT);
    thought_intent("right_hand_move") => player.strafe(RIGHT);
    thought_intent("both_hands")      => player.jump();
    thought_intent("feet_move")       => player.crouch();
    thought_intent("grip")            => player.attack();
};

cortex_bind neural_stream("occipital") {
    thought_intent("target_locked") => targeting.lock(gaze_track.fixation_target);
};
```

---

### Thought-Driven Code Synthesis — Theoretical Proposal

A theoretical system where a developer thinks about a function's purpose and the system generates code. The neural decoder translates prefrontal activation patterns into semantic intent vectors, which a code-generation model maps to AETHER source code.

> [!NOTE]
> Full thought-to-code synthesis requires advances in neural decoding beyond Phase 2. This is a long-term research proposal (Phase 3+). Phase 2 targets word-by-word neural typing.

```aether
// Conceptual Phase 3+ code
cortex_bind neural_stream("prefrontal") {
    thought_intent("synthesize_function") => {
        let intent_vector = neural_stream.semantic_embedding();
        let synthesized   = code_synthesizer.generate(intent_vector, lang: "aether");
        code_panel.insert(synthesized);
        emit("Synthesized: ${synthesized.signature}");
    };
};
```

---

### AR Heads-Up Display

An augmented reality HUD overlaid on the real world, displaying live system metrics, navigation, and contextual information without requiring the user to look at a screen.

```aether
ar_overlay "system_hud" {
    anchor: screen_bottom_left;
    content: hud_panel {
        items: [
            { label: "CPU",  value: sys.cpu_pct,  unit: "%" },
            { label: "MEM",  value: sys.mem_gb,   unit: "GB" },
            { label: "NET",  value: sys.net_mbps, unit: "Mbps" },
            { label: "TIME", value: clock.local(), format: "HH:MM" },
        ];
        theme:  "minimal_dark";
        opacity: 0.75;
    };
    update: every(500ms);
};

// Gaze-activated detail expansion
gaze_track on_fixation { fixation =>
    if fixation.target == "system_hud" && fixation.duration > 1s {
        system_hud.expand(show_graphs: true);
    }
};
```

---

### VR Development Environment

A fully immersive development environment where code panels, terminal windows, documentation, and 3D visualizations float in virtual space around the developer. Finger tracking handles text selection; voice handles commands.

```aether
vr_world "aether_ide" {
    skybox: "deep_space.hdr";
    
    hologram main_editor {
        position:  vec3(0.0, 1.5, -0.8);
        scale:     vec3(1.6, 1.0, 0.01);
        content:   code_editor { language: "aether"; theme: "aether_dark"; };
    };
    
    hologram terminal {
        position: vec3(-1.0, 0.9, -0.7);
        scale:    vec3(0.8, 0.5, 0.01);
        content:  terminal { shell: "pwsh"; font: "JetBrains Mono"; };
    };
    
    hologram docs {
        position: vec3(1.0, 1.5, -0.7);
        scale:    vec3(0.7, 0.9, 0.01);
        content:  browser { url: "https://aether-lang.dev/docs"; };
    };
    
    hologram ast_viewer {
        position: vec3(0.0, 0.5, -1.2);
        content:  graph_3d {
            data:    compiler.live_ast(main_editor.content);
            layout:  radial_tree;
            animate: true;
        };
    };
};

voice_cmd "compile"         => compiler.run(main_editor.active_file);
voice_cmd "run tests"       => test_runner.run_all();
voice_cmd "git commit {msg}"=> git.commit(message: msg);

hand_track on_gesture { g =>
    if g.type == "pinch" { main_editor.cursor_at(g.ray_intersection); }
};
```

> [!TIP]
> Pair the VR development environment with `cortex_bind motor_cortex` for hands-free navigation when your hands are occupied with holographic interaction.
