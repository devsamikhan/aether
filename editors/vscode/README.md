# ⚡ AETHER Language Support for Visual Studio Code

Official VS Code extension providing rich syntax highlighting, code snippets, and language configuration for the **AETHER 2.0 Programming Language** (`.ae`, `.aether`).

---

## ✨ Features

* **🎨 Syntax Highlighting:** Keywords (`let`, `fn`, `intent`, `class`), types (`Int`, `Float`, `String`, `Qubit`), strings, numbers, operators, and comments.
* **⚡ Smart Code Snippets:**
  * `fn` ➔ Function definition
  * `class` ➔ Class with `init` constructor
  * `intent` ➔ Declarative intent contract with `schema`, `require`, `ensure`
  * `spawn` ➔ Fiber concurrency block
  * `comp` ➔ List comprehension
  * `try` ➔ Structured exception handling
  * `pl` ➔ `println(...)`
* **🔄 Auto-closing Pairs & Brackets:** Auto-completes `{}` , `[]` , `()` , `""`.
* **💬 Comment Toggling:** `#` or `//` with `Ctrl+/` (`Cmd+/` on macOS).

---

## 🚀 Easy Installation (Local)

To install this extension into your local VS Code:

### Windows (PowerShell)
```powershell
Copy-Item -Path "editors\vscode" -Destination "$HOME\.vscode\extensions\aether-language" -Recurse -Force
```

### Linux & macOS (Bash)
```bash
cp -r editors/vscode ~/.vscode/extensions/aether-language
```

Restart or reload your VS Code window (`Ctrl+Shift+P` ➔ `Developer: Reload Window`). All `.ae` files will now be recognized and highlighted automatically!
