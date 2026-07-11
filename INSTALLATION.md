# Installing AETHER

AETHER is a post-quantum, intent-driven programming language. Follow these instructions to set it up on your system.

## Quick Install (Recommended)

### macOS/Linux
```bash
curl -fsSL https://raw.githubusercontent.com/devsamikhan/aether/main/scripts/install.sh | bash
```

### Windows (PowerShell)
```powershell
iwr -useb https://raw.githubusercontent.com/devsamikhan/aether/main/scripts/install-windows.ps1 | iex
```

---

## Manual Installation

### 1. Download Binary
- **Windows**: [aether-windows-x64.exe](releases/v1.0.0/aether-windows-x64.exe)
- **macOS (Universal)**: [aether-macos-universal](releases/v1.0.0/aether-macos-universal)
- **Linux**: [aether-linux-x64](releases/v1.0.0/aether-linux-x64)

### 2. Verify with Checksums
Verify file integrity using SHA-256:
- [checksums.txt](releases/v1.0.0/checksums.txt)

```bash
# macOS/Linux
shasum -a 256 -c aether-linux-x64.sha256

# Windows
Get-FileHash aether-windows-x64.exe -Algorithm SHA256
```

---

## Verify Installation

```bash
aether --version
```

---

## Self-Update
Keep AETHER updated automatically:
```bash
aether self-update
```
