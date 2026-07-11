# Release Notes: AETHER v1.0.0

Welcome to the **AETHER v1.0.0 Stable Production Release**!

This milestone release establishes the core platform and formalizes distribution.

## Key Highlights

1. **Self-Update Architecture**: Built-in update engine checks releases on GitHub, downloads to temporary directories, conducts SHA-256 validation runs, backs up existing builds, and performs atomic rollbacks.
2. **Library Registry & Package Manager**: Versioned module installations targeting user folders (e.g. `~/.aether/libraries/<name>/<version>/`).
3. **Formal EBNF Specification**: High-fidelity compiler semantics matching the JIT/parser behavior.
4. **Standard Library Bundle**: Built-in collections, system I/O, and networking.
5. **Universal Toolchain Installer**: Seamless detection and installation across Windows, Linux, and macOS platforms.
