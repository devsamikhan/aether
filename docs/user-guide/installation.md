# AETHER 2.0 User Guide: Installation & Setup

## 1. Quick One-Line Automated Install

### Windows (PowerShell)
Run in PowerShell (no Administrator rights needed):
```powershell
irm https://raw.githubusercontent.com/devsamikhan/aether/main/install.ps1 | iex
```

### Linux & macOS (Bash / Zsh)
Run in your terminal:
```bash
curl -fsSL https://raw.githubusercontent.com/devsamikhan/aether/main/install.sh | bash
```

The script downloads the pre-built native binary for your architecture, installs it into `~/.aether/bin`, and adds it to your user `PATH`.

---

## 2. Compile From Source
Prerequisites: A standard Rust toolchain (`cargo`, `rustc`).

```bash
git clone https://github.com/devsamikhan/aether.git
cd aether
cargo build --release
```
The compiled binary will be located at `target/release/aether` (or `target/release/aether.exe` on Windows).

---

## 3. Verify System Health
Run the diagnostic suite:
```bash
aether doctor
```
All green checks indicate complete toolchain readiness.