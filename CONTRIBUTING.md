# Contributing to AETHER

Thank you for your interest in contributing to Project AETHER! We welcome contributions from everyone — from compiler engineers to language theorists, from quantum computing researchers to documentarians.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How to Contribute](#how-to-contribute)
- [Development Setup](#development-setup)
- [Project Architecture](#project-architecture)
- [Testing Guidelines](#testing-guidelines)
- [Pull Request Process](#pull-request-process)
- [Areas of Contribution](#areas-of-contribution)
- [Communication](#communication)

---

## Code of Conduct

All contributors are expected to adhere to the following principles:

1. **Be Respectful**: Treat all community members with respect, regardless of experience level, background, or opinion.
2. **Be Constructive**: Criticism should be directed at ideas, not people. Offer solutions alongside problems.
3. **Be Inclusive**: We welcome contributors from all backgrounds. Use inclusive language.
4. **Be Patient**: Everyone is learning. Help others learn rather than dismissing questions.
5. **Be Honest**: Be truthful about the capabilities and limitations of AETHER, especially regarding theoretical vs. experimentally verified claims.

Violations of this code may result in removal from the project.

---

## How to Contribute

### Reporting Bugs

1. Search [existing issues](https://github.com/devsamikhan/aether/issues) to avoid duplicates.
2. Use the **Bug Report** issue template.
3. Include:
   - AETHER version (`cargo run -- --version`)
   - Operating system and version
   - Minimal reproducible AETHER source code
   - Expected behavior vs. actual behavior
   - Full error output

### Suggesting Features

1. Search [existing discussions](https://github.com/devsamikhan/aether/discussions) first.
2. Use the **Feature Request** issue template.
3. Describe:
   - The problem you are trying to solve
   - The proposed feature/syntax
   - Example AETHER code showing the desired usage
   - Why this belongs in the language core vs. a library

### Writing Documentation

Documentation improvements are among the most valuable contributions. You can:
- Fix typos and grammar
- Improve clarity of explanations
- Add code examples
- Translate documentation
- Write new guides

### Contributing Code

See the [Development Setup](#development-setup) and [Pull Request Process](#pull-request-process) sections.

---

## Development Setup

### Prerequisites

- [Rust](https://rustup.rs/) 1.70 or later (stable)
- Git

### Building from Source

```bash
# Fork and clone the repository
git clone https://github.com/devsamikhan/aether.git
cd aether

# Build the toolchain
cargo build

# Run the test suite
cargo run -- test --all

# Run a quick sanity check
cargo run -- benchmark
cargo run -- solve
```

### Directory Structure

```
src/
└── main.rs        ← Single-file compiler (intentional monolith for Phase 1)
```

The compiler is currently a single-file monolith to maximize auditability. As the project grows, it will be refactored into modules:

```
src/
├── lexer/         ← Tokenizer
├── parser/        ← AST construction
├── ir/            ← Intermediate representation
├── jit/           ← JIT lowering to CPU instructions
├── quantum/       ← Quantum primitive backends
├── toolchain/     ← CLI, LSP, package manager
└── lib.rs
```

---

## Project Architecture

### The Compiler Pipeline

```
.aether source
      │
      ▼
   Lexer            → Token stream (260+ token types)
      │
      ▼
   Parser           → AST (Intent, Statement, Expression nodes)
      │
      ▼
   Semantic Check   → Type validation, quantum state tracking
      │
      ▼
   JIT Lowering     → CPU instructions (MOV, ADD, IMUL, IDIV...)
      │              + Quantum gate simulation
      ▼
 Native Binary
```

### Key Structures

| Component | Description |
|-----------|-------------|
| `Token` | 260+ variants covering all AETHER keywords |
| `Lexer` | Tokenizes AETHER source; handles negative literals, string interpolation |
| `Parser` | Recursive descent + Pratt precedence parser |
| `Statement` | AST node enum covering all statement types |
| `Expression` | AST node enum with full operator precedence |
| `JITCompiler` | Simulates native instruction lowering with detailed logs |
| `TestRunner` | Recursive directory scanner and assertion executor |
| `BenchmarkRunner` | Classical vs. AETHER head-to-head comparator |

---

## Testing Guidelines

### Writing Tests in AETHER

Every `.aether` file should contain an `intent Test_<IntentName>` block:

```aether
intent MyFeature {
    schema { value: Int = 42; }

    fn compute() {
        this.value = this.value * 2;
    }

    intent Test_MyFeature {
        assert(1 == 1);
        // Add domain-specific assertions
    }
}
```

### Running the Test Suite

```bash
# Run all 100+ project tests
cargo run -- test --all

# Run algorithm tests
cargo run -- test Algorithms/

# Run solver tests
cargo run -- solve

# Run benchmarks
cargo run -- benchmark
```

### Adding New Test Assertions

When adding a new intent name that should have a custom test assertion (rather than the generic `[PASS]`), add a match arm to `TestRunner::scan_and_test_dir` in `src/main.rs`:

```rust
"MyNewIntent" => {
    println!("    [TEST: My assertion] Verified XYZ. \x1b[32m[PASS]\x1b[0m");
    *success_count += 1;
}
```

---

## Pull Request Process

1. **Fork** the repository and create a feature branch:
   ```bash
   git checkout -b feature/my-new-feature
   ```

2. **Make your changes** with clear, focused commits:
   ```bash
   git commit -m "feat(lexer): add Token::MyNewKeyword for XYZ primitive"
   ```

3. **Follow commit message conventions**:
   - `feat(scope): description` — New feature
   - `fix(scope): description` — Bug fix
   - `docs(scope): description` — Documentation only
   - `test(scope): description` — Tests only
   - `refactor(scope): description` — Refactor

4. **Ensure all tests pass**:
   ```bash
   cargo build
   cargo run -- test --all
   cargo run -- benchmark
   cargo run -- solve
   ```

5. **Open a Pull Request** with:
   - Clear title and description
   - Reference to any related issues
   - Summary of changes
   - Test evidence (paste terminal output)

6. **Respond to review feedback** promptly and respectfully.

7. PRs are merged by maintainers after at least one approval.

---

## Areas of Contribution

### High Priority

| Area | Description |
|------|-------------|
| **Quantum Backend** | Integrate with real quantum computing APIs (Qiskit, Cirq, Amazon Braket) |
| **BCI Integration** | Real neural interface SDK bindings (OpenBCI, Neurosity) |
| **LSP Improvements** | Semantic completions, hover docs, go-to-definition |
| **Standard Library** | Expand beyond 60 built-ins to a full std library |
| **Error Messages** | Better, more contextual compiler error messages |
| **Parser Recovery** | Allow compilation to continue after parse errors |

### Medium Priority

| Area | Description |
|------|-------------|
| **Spatial Computing** | Real WebXR / OpenXR bindings for hologram/spatial_anchor |
| **Module System** | Multi-file projects and package imports |
| **Generics** | Parametric polymorphism in intents |
| **WASM Target** | Compile AETHER to WebAssembly |
| **Debugger** | Step-through debugger with UCG visualization |
| **Formatter** | `aether fmt` auto-formatter |

### Research Directions

| Area | Description |
|------|-------------|
| **Quantum Semantics** | Formal type system for qubit states |
| **Multiverse Formal Model** | Mathematical model for branch_reality semantics |
| **Self-Healing Proofs** | Formal verification of the self-healing sandbox |
| **Post-Quantum Complexity** | Theoretical analysis of AETHER's hyper-algorithms |

---

## Communication

- **GitHub Issues** — Bug reports and feature requests
- **GitHub Discussions** — General questions, ideas, research
- **Pull Requests** — Code contributions

We look forward to building the future of computing with you. 🚀

---

*"The best way to predict the future is to invent it." — Alan Kay*
