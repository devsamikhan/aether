# ⚡ AETHER Language Support for Visual Studio Code

Official VS Code extension providing comprehensive syntax highlighting, smart snippets, bracket matching, and toolchain integration for the **AETHER** programming language.

---

## 🌟 Features

- **Rich Syntax Highlighting:**
  - Keywords (`let`, `fn`, `intent`, `require`, `ensure`, `class`, `spawn`, `await`).
  - Python-style F-string interpolation (`f"Hello {name}!"`).
  - Native standard modules (`DB`, `Database`, `AI`, `Mobile`, `M3`, `Quantum`, `Fiber`, `Simd`).
- **Smart Snippets:**
  - `fn`: Function declarations.
  - `class`: Object-oriented class scaffolding.
  - `intent`: Declarative intent contract definition.
  - `db_open`: Relational database connection and schema setup.
  - `ai_agent`: Autonomous AI agent initialization and task dispatch.
  - `m3_card`: Material 3 card factory.
  - `toast`: Android mobile toast bridge.
- **Language Configuration:**
  - Single-line comments (`#`) and multi-line comments (`/* */`).
  - Auto-closing pairs for parentheses, braces, brackets, and quotes.

---

## 🚀 Installation

### Option 1: Direct Link into VS Code Extensions Directory
Copy or symlink this folder into your local VS Code extensions directory:

```bash
# Windows (PowerShell)
Copy-Item -Recurse editors/vscode "$HOME/.vscode/extensions/aether-language-support"

# Linux / macOS
cp -r editors/vscode ~/.vscode/extensions/aether-language-support
```

Restart VS Code or press `Ctrl+Shift+P` -> **Developer: Reload Window**.

---

## 🛠️ CLI Toolchain Integration

You can also use AETHER's built-in language server and hot reload features directly:

```bash
# Live Dev Server with Hot-Reloading
aether dev src/main.ae

# Interactive Web Studio & Playground
aether studio

# Language Server diagnostics
aether lsp --check src/main.ae
```
