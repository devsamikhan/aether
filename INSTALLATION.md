# Installing & Distributing AETHER 2.0

AETHER 2.0 is a modern, high-performance, intent-driven programming language featuring native tensor autograd, quantum state simulation, CRDT swarms, graph traversals, and Erlang-grade live code swapping.

AETHER ships as a **standalone, zero-dependency native binary**. It requires no separate runtime, no external DLLs, no Python/Node.js dependencies, and no administrator privileges.

---

## 🚀 Quick Install (Recommended)

### 🪟 Windows (PowerShell)
Open PowerShell and run the one-line bootstrap installer:
```powershell
irm https://raw.githubusercontent.com/devsamikhan/aether/main/install.ps1 | iex
```
*What this does:*
1. Installs `aether.exe` into `%USERPROFILE%\.aether\bin`.
2. Copies standard libraries (`aether_tensor.ae`, `aether_graph.ae`, `aether_live.ae`, etc.) to `%USERPROFILE%\.aether\libraries`.
3. Permanently registers `%USERPROFILE%\.aether\bin` in your User `PATH`.

---

### 🐧 Linux & 🍎 macOS (Terminal)
Open your terminal and run:
```bash
curl -fsSL https://raw.githubusercontent.com/devsamikhan/aether/main/install.sh | bash
```
*What this does:*
1. Auto-detects OS (`Linux` / `macOS`) and architecture (`x86_64` / `arm64`).
2. Installs `aether` binary into `~/.aether/bin/`.
3. Copies standard libraries to `~/.aether/libraries/`.
4. Adds `~/.aether/bin` to your `PATH` across `.bashrc`, `.zshrc`, or `.profile`.

---

## 📦 Standalone Portable Binaries (Zero Installation)

For offline environments, containers, or direct downloads without scripts:

| Platform | Architecture | Package Archive | Direct Binary |
| :--- | :--- | :--- | :--- |
| **Windows** | x86_64 | [aether-windows-x86_64.zip](https://github.com/devsamikhan/aether/releases/latest/download/aether-windows-x86_64.zip) | [aether-windows-x64.exe](https://github.com/devsamikhan/aether/releases/latest/download/aether-windows-x64.exe) |
| **Linux** | x86_64 | [aether-linux-x86_64.tar.gz](https://github.com/devsamikhan/aether/releases/latest/download/aether-linux-x86_64.tar.gz) | [aether-linux-x64](https://github.com/devsamikhan/aether/releases/latest/download/aether-linux-x64) |
| **macOS** | Universal (Intel & Apple Silicon) | [aether-macos-universal.tar.gz](https://github.com/devsamikhan/aether/releases/latest/download/aether-macos-universal.tar.gz) | [aether-macos-universal](https://github.com/devsamikhan/aether/releases/latest/download/aether-macos-universal) |

### Checksum Verification (SHA-256)
All release artifacts include cryptographic SHA-256 checksum files (`.sha256`).
```bash
# Linux/macOS
shasum -a 256 -c aether-linux-x86_64.tar.gz.sha256

# Windows PowerShell
(Get-FileHash aether-windows-x86_64.zip -Algorithm SHA256).Hash
```

---

## 🌐 Community Package Managers

### Windows Package Manager (Winget)
```powershell
winget install aether-lang
```

### Scoop (Windows)
```powershell
scoop install aether
```

### macOS / Linux (Homebrew)
```bash
brew install aether-lang
```

### Rust Cargo Toolchain
```bash
cargo install --git https://github.com/devsamikhan/aether
```

---

## 🩺 System Verification & Diagnostics

Verify your toolchain installation with the built-in diagnostic doctor:
```bash
aether doctor
```
Example Output:
```text
================================================================================
AETHER SYSTEM DIAGNOSTIC REPORT (Doctor)
================================================================================
  [✓] AETHER Core Engine:      v1.1.0 (target: x86_64-pc-windows-msvc)
  [✓] Binary Location:         C:\Users\...\.aether\bin\aether.exe
  [✓] Standard Libraries:      C:\Users\...\.aether\libraries (11 libraries)
  [✓] User PATH Config:        Configured permanently
  [✓] JIT / Execution Target:  Cranelift & Native Bytecode Ready
  [✓] Quantum Engine:          Matrix State Simulator Active
  [✓] Graph Engine:            Property Graph & Traversal Engine Ready
  [✓] Live Code Swapper:       Zero-Downtime Socket Ready
================================================================================
STATUS: Toolchain is 100% healthy and ready for production!
```

---

## ⚡ Production Benchmarks

Run hardware-calibrated microbenchmarks to verify system throughput:
```bash
aether bench
```
Benchmarks include:
- `tensor_matrix_multiplication` (Float arithmetic & SIMD throughput)
- `crdt_state_convergence` (Distributed semilattice operations)
- `quantum_state_entanglement` (Unitary matrix simulations)
- `graph_dijkstra_traversal` (Memory locality & pathfinding)
- `bytecode_vm_dispatch` (Opcode instruction loop overhead)

---

## 📁 Creating Your First Project

Generate project scaffolds with preconfigured templates:
```bash
# Minimal starter project
aether new my_app

# Machine learning & tensor pipeline
aether new my_ai --template ai

# Distributed backend & WebSocket service
aether new my_api --template web

# High-frequency trading & graph analytics
aether new my_fintech --template fintech
```

Run your code immediately:
```bash
cd my_app
aether run src/main.ae
```

---

## 🔄 Self-Update Engine

To update your AETHER installation in-place without touching installers or registry:
```bash
aether update
```

---

## 🛠️ Building from Source

AETHER relies strictly on the **Rust Standard Library** and current workspace crates. Zero obscure C/C++ dependencies:
```bash
git clone https://github.com/devsamikhan/aether.git
cd aether
cargo build --release
```
To register your newly built binary globally:
- **Windows:** Run `.\install.ps1`
- **Linux/macOS:** Run `./install.sh`
