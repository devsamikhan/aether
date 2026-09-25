# Chapter 1: Getting Started & Toolchain Setup

### 1.1 The One-Line Terminal Install

AETHER requires no installation wizards, no registry modifications, and no administrative privileges.

#### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/devsamikhan/aether/main/install.ps1 | iex
```

#### Linux & macOS (Bash / Zsh)
```bash
curl -fsSL https://raw.githubusercontent.com/devsamikhan/aether/main/install.sh | bash
```

The installer places the single binary into `~/.aether/bin` (or `%USERPROFILE%\.aether\bin` on Windows) and permanently registers it in your user `PATH`.

### 1.2 System Health Inspection: `aether doctor`

Immediately after installation, verify your environment:
```bash
aether doctor
```
Output:
```text
================================================================================
🩺 AETHER TOOLCHAIN DOCTOR (SYSTEM HEALTH DIAGNOSTICS)
================================================================================
Version: v1.1.0 (stable-x86_64)
Binary Location: C:\Users\...\.aether\bin\aether.exe
Diagnostics Checklist:
  [✅ OK] AETHER_HOME Directory
  [✅ OK] Global User PATH Integration
  [✅ OK] Cranelift AOT/JIT Native Compiler
  [✅ OK] Bytecode VM & Memory Engine
  [✅ OK] Python 3 Interop (Zero-Cost FFI)
Status: SYSTEM 100% HEALTHY & READY FOR PRODUCTION 🚀
================================================================================
```

### 1.3 Creating Your First Project: `aether new`

```bash
aether new my_project --template minimal
cd my_project
aether run src/main.ae
```

---
