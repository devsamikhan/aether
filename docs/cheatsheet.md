# ⚡ AETHER 2.0 by Example: The Fast Cheat Sheet

A concise, copy-pasteable reference for developers learning AETHER syntax in 10 minutes.

---

## 1. Variables & Types
```aether
let count = 42;                 # 64-bit Int
let price = 19.99;              # 64-bit Float
let is_active = true;           # Bool
let name = "AETHER";            # UTF-8 String
let empty = nil;                # Nil / None
count = count + 1;              # Mutate variable
```

---

## 2. Arithmetic & Logic
```aether
let a = 15;
let b = 4;
println(a + b);                 # 19
println(a / b);                 # 3 (integer division)
println(a % b);                 # 3 (modulo)
let ok = (a > 10) and (b == 4); # Logical AND (or: '&&', 'or', 'not')
```

---

## 3. Strings & Python-Style F-Strings
```aether
let s1 = "Hello";
let s2 = "World";
let name = "AETHER";
let version = 2;

# Python-style formatted string interpolation
let greeting = f"Welcome to {name} v{version}!"; 
println(greeting);              # "Welcome to AETHER v2!"

# Expression evaluation inside f-strings:
println(f"Calculation: 2 + 2 = {2 + 2}, escaped: {{x}}");

let msg = s1 + ", " + s2 + "!"; # String concatenation
println(to_string(42));         # Cast int to string: "42"
println(to_int("100"));         # Parse string to int: 100
```

---

## 4. Lists (Dynamic Arrays)
```aether
let items = [10, 20, 30, 40];
println(items[0]);              # 10 (zero-indexed)
items.push(50);                 # Append element
let last = items.pop();         # Remove last element (50)
println(len(items));            # Length: 4
```

---

## 5. Dictionaries (Key-Value Maps)
```aether
let user = {"id": 101, "role": "admin", "active": true};
println(user["role"]);          # "admin"
user["email"] = "dev@aether.org"; # Add new key
```

---

## 6. List Comprehensions
```aether
let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
let evens_squared = [x * x for x in numbers if x % 2 == 0];
println(evens_squared);         # [4, 16, 36, 64, 100]
```

---

## 7. Control Flow (`if`, `elif`, `else`)
```aether
let temp = 28;
if temp > 35 {
    println("Hot");
} elif temp > 20 {
    println("Pleasant");
} else {
    println("Cold");
}
```

---

## 8. Loops (`while` & `for..in`)
```aether
# While loop
let i = 0;
while i < 3 {
    println("Count: " + to_string(i));
    i = i + 1;
}

# For-in loop
for word in ["Quantum", "Fiber", "SIMD"] {
    println("Feature: " + word);
}
```

---

## 9. Functions & Closures
```aether
fn add(x, y) {
    return x + y;
}
println(add(10, 25));           # 35

# Higher-order function / Lambda
let double = fn(n) { return n * 2; };
println(double(21));            # 42
```

---

## 10. Classes & Object-Oriented Programming
```aether
class User {
    fn init(name, age) {
        this.name = name;
        this.age = age;
    }

    fn greet() {
        return "Hi, I am " + this.name;
    }
}

let u = User("Alice", 25);
println(u.greet());             # "Hi, I am Alice"
```

---

## 11. Exception Handling (`try`, `catch`, `throw`)
```aether
try {
    throw "InvalidParameterError";
} catch (err) {
    println("Handled error: " + err);
}
```

---

## 12. Green Fibers Concurrency (1M Tasks in 825ms)
```aether
let ch = channel();
spawn(fn() {
    ch.send("Task complete!");
});
let message = ch.recv();
println(message);               # "Task complete!"
```

---

## 13. Accelerated SIMD Vector Compute (2.1x Faster than C++ -O3)
```aether
let a = [1.0, 2.0, 3.0, 4.0];
let b = [10.0, 20.0, 30.0, 40.0];
let dot = Compute.dot_product(a, b);
println("Dot: " + to_string(dot)); # 300.0
```

---

## 14. Declarative Intent Verification
```aether
intent AccountBalance {
    schema {
        balance: Float = 0.0;
    }
    require {
        this.balance >= 0.0;
    }
    ensure {
        this.balance >= 0.0;
    }
}
```

---

## 15. Post-Quantum State Vector Simulation
```aether
let qreg = QuantumRegister(2);
qreg.h(0);                      # Superposition on Q0
qreg.cnot(0, 1);                 # Entangle Q0 and Q1
let result = qreg.measure(0);    # Wavefunction collapse
println("Collapsed Q0: " + to_string(result));
```

---

## 16. Pipeline Chaining Operator (`|>`)
```aether
fn double(x):
    return x * 2

fn add_five(x):
    return x + 5

fn square(x):
    return x * x

# Pipeline data flow: 3 -> double(6) -> add_five(11) -> square(121)
let result = 3 |> double |> add_five |> square;
println(f"Pipeline Result: {result}"); # 121

# Multi-argument pipeline forwarding:
fn multiply(val, factor):
    return val * factor

let total = 10 |> multiply(5); # calls multiply(10, 5) => 50
```

---

## 17. Standalone Native Binary Compilation (`aether build`)
```bash
# Compile any AETHER script into a zero-dependency standalone native Windows executable (.exe):
aether build my_script.ae -o my_app.exe

# Execute standalone binary anywhere with no dependencies:
./my_app.exe
```

---

## 18. Android Mobile Compilation & APK Packaging (`.apk`)

AETHER can compile applications directly into signed, standalone Android Packages (`.apk`) with zero external SDKs or Android Studio dependencies:

```bash
# Option 1: Direct APK compilation with target flag
aether build app.ae --target apk -o app.apk

# Option 2: Dedicated apk command with custom metadata
aether apk app.ae -o app.apk --package com.myorg.app --name "MyApp"

# Option 3: Export full Gradle / Android Studio project
aether build app.ae --target android-project -o MyAndroidProject/
```

### Mobile Hardware & Runtime APIs (`Mobile`)
```aether
# Native toast message
Mobile.show_toast("Welcome to AETHER on Android!")

# Precise haptic vibration (milliseconds)
Mobile.vibrate(50)

# Device battery level percentage (0-100)
let battery = Mobile.battery_level()

# Hardware information
let info = Mobile.device_info()
print(f"Device: {info['brand']} {info['model']} | OS: {info['os']}")

# Network connectivity status
let net = Mobile.network_status() # 'wifi_connected', 'cellular', 'offline'
```

---

## 19. Google Material 3 (M3 / Material You) Design System

AETHER provides full native support for Google's official Material 3 design system, including algorithmic tonal palettes, component factories, typography, and shape tokens:

```aether
# 1. Dynamic Tonal Palette Theming (from seed color)
let theme = M3.theme("#00F5FF", dark_mode=True)
print(f"Primary: {theme['primary']} | Surface: {theme['surface']}")

# 2. Material 3 UI Components
let top_bar = M3.top_app_bar("AetherPocket", "Fitness & Wellness")
let step_card = M3.card("elevated", "Daily Steps", "Sensor Goal", "8,641 / 10,000 steps")
let hydration_card = M3.card("filled", "Hydration", "Optimal Intake", "1,250 / 2,500 ml")

let btn = M3.filled_button("Log Activity", icon="add")
let fab = M3.fab("add", label="New Task", extended=True)

let nav = M3.navigation_bar([
    {"label": "Home", "icon": "home", "active": True},
    {"label": "Activity", "icon": "fitness", "active": False},
    {"label": "Profile", "icon": "person", "active": False}
])


# 3. Interactive Visual Preview
M3.render_preview(top_bar)
M3.render_preview(step_card)
M3.render_preview(btn)
M3.render_preview(nav)
```

---

## 20. Single-File Web Target Builder (`.html`)

AETHER compiles pure scripts into self-contained HTML5 single-file applications with zero CDN or external server dependencies:

```bash
# Option 1: Dedicated web command
aether web app.ae -o app.html --name "My Aether Web App"

# Option 2: Unified target compilation
aether build app.ae --target web -o app.html
```

- Embeds full Material 3 styling, responsive canvas, and reactive UI state directly into one portable `.html` file.
- Works offline in any browser across desktop, mobile, and tablets.

---

## 21. Native AI & LLM Engine (`AI.*`)

AETHER features a built-in AI module for generative completions, intelligent chat sessions, autonomous task agents, sentiment classification, and vector embeddings:

```aether
# 1. Text Completion
let poem = AI.complete("Explain quantum computing in one sentence.")
println(poem)

# 2. Multi-turn Conversational Chat
let reply = AI.chat([
    {"role": "user", "content": "Hello, how can you help me build my app?"}
])
println(reply)

# 3. Autonomous Task Agents
let agent = AI.agent("AetherCoder", "You are an expert compiler and games architect.")
let advice = agent.ask("What is the optimal frame budget for 60 FPS?")
println(advice)

# 4. Sentiment Analysis
let sent = AI.sentiment("AETHER makes development delightful and super fast!")
println(sent["label"]) # "positive"

# 5. Vector Embeddings
let vec = AI.embeddings("Neon hyper-highway arcade runner")
println(len(vec))      # 16-dimensional embedding vector
```

---

## 22. Aether Package Manager (`AetherPM`)

Install, resolve, and manage project dependencies declared in `aether.toml` / `Aether.toml`:

```toml
# aether.toml
[package]
name = "my_arcade_game"
version = "1.0.0"

[dependencies]
std = "1.0.0"
aether_m3 = "1.0.0"
arcade_engine = "0.5.0"
```

```bash
# Install all project dependencies declared in aether.toml
aether install

# Install or update a specific library
aether install aether_m3

# Search package registry
aether search game
```

---

## 23. Flagship Projects

1. **`01_hyperpulse_web`**: High-concurrency async web server with streaming pipelines and zero-copy JSON responses.
2. **`02_aetherframe_analytics`**: Vectorized in-memory DataFrame engine with SQL-like query builder.
3. **`03_aetherstore_database`**: Embedded ACID key-value and document store with write-ahead logging (WAL).
4. **`04_android_mobile_app`**: Standalone Material 3 mobile application (`.apk`) with native hardware sensors & haptics.
5. **`05_aether_arcade_game`**: Retro 2D arcade game engine (`CyberRunner 2088`) compiling to `.exe`, `.apk`, and single-file `.html`.
6. **`06_aetherbrain_ai_saas`**: Enterprise AI SaaS knowledge platform integrating Relational SQL, AI Agents, and M3 Executive Dashboard.

---

## 24. Live Dev Server with Hot-Reloading (`aether dev`)

AETHER includes an Erlang BEAM-grade live code swapping and hot-reloading dev server with sub-millisecond incremental state swapping:

```bash
# Watch file for changes and instantly hot-swap bytecode without dropping state
aether dev app.ae

# Custom polling interval
aether dev app.ae --interval 50
```

---

## 25. Interactive Web Studio & Playground (`aether studio`)

Launch a local, zero-dependency browser-based development studio and live REPL:

```bash
# Start local Web Studio on default port 8080 (auto-opens browser)
aether studio

# Start on custom port
aether studio 3000
```

- Live in-browser AETHER bytecode VM runner (`/api/run`).
- Built-in templates for CyberRunner Arcade, AI SaaS, Relational SQL, Material 3 Mobile, and Quantum Teleportation.

---

## 26. Relational Database & SQL Engine (`DB.*`)

AETHER features a built-in, zero-dependency embedded relational SQL database supporting ACID operations, schema creation, and parameterized queries:

```aether
# 1. Open Database (in-memory or persistent file)
let db = DB.open("data/production.db")

# 2. Schema DDL
db.execute("CREATE TABLE users (id INT, name TEXT, role TEXT, score INT);")

# 3. Insert Records
db.execute("INSERT INTO users (id, name, role, score) VALUES (1, 'Sami', 'admin', 950);")
db.insert("users", {"id": 2, "name": "Elena", "role": "researcher", "score": 880})

# 4. Parameterized Querying
let high_scorers = db.query("SELECT name, score FROM users WHERE score >= ? ORDER BY score DESC;", [800])
println(high_scorers)

# 5. Table Metadata
println(db.tables()) # ["users"]
println(db.count("users")) # 2

# 6. Mutations
db.execute("UPDATE users SET score = 999 WHERE name = 'Sami';")
db.execute("DELETE FROM users WHERE role = 'researcher';")
```

---

## 27. Official Visual Studio Code Extension

Install official syntax highlighting, IntelliSense, and snippets for VS Code:

```bash
# Install extension locally
cp -r editors/vscode ~/.vscode/extensions/aether-language-support
```

---

## 28. AetherShell Universal Bundler & AetherBridge (`aether bundle`)

AETHER gives web frontend developers (React 18, Vue 3, Svelte 5, Tailwind CSS) a unified, zero-SDK pipeline to bundle into native Android APKs, Windows executables, and offline single-file HTML containers **without Android Studio, Gradle, NDK, or Tauri**:

### 1. Scaffold UI Project
```bash
# Available UI templates: react, vue, svelte, tailwind
aether new MyOmniApp --template react
```

### 2. Universal CLI Bundler
```bash
# Bundle web frontend into signed Android APK (<1 second, zero Android Studio!)
aether bundle frontend/ --target apk -o MyOmniApp.apk --name "My Omni App" --package com.example.omni

# Bundle web frontend into standalone Windows Desktop EXE (zero Electron/Tauri!)
aether bundle frontend/ --target desktop -o MyOmniApp.exe --name "My Omni App"

# Bundle web frontend into single-file portable offline HTML
aether bundle frontend/ --target web -o MyOmniApp.html --name "My Omni App"
```

### 3. Universal JavaScript Bridge (`window.Aether`)
Inside any React, Vue, or Tailwind app, communicate with native device hardware and AETHER engines:
```javascript
// Hardware Haptics & Vibration
window.Aether.vibrate(50);
window.Aether.hapticFeedback('medium'); // 'light', 'medium', 'heavy'

// Native Android Toast Notifications
window.Aether.showToast("Saved to AETHER DB!");

// Built-in Relational SQL (DB.*)
const rows = await window.Aether.dbQuery("SELECT * FROM users WHERE active = 1;");

// Autonomous Neural AI Substrate
const aiResp = await window.Aether.ai("Analyze user telemetry");

// Hardware & Battery Telemetry
const info = await window.Aether.deviceInfo();
```


