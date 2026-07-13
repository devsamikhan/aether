# AETHER

<p align="center">
  <img src="logo/aether-logo.svg" alt="AETHER Logo" width="200" height="200"/>
</p>

[STATUS: ACTIVE DEVELOPMENT / EXPERIMENTAL RESEARCH]

[Build: Passing] [License: MIT] [Version: 0.2.0] [Language: Rust]

> A research-oriented programming language exploring intent-driven abstractions 
> and future computing paradigms. Currently in active development.

---

## Project Status

| Component | Status | Notes |
|-----------|--------|-------|
| Compiler Core | Experimental | Lexer, Parser, AST functional |
| CLI Toolchain | Working | init, build, test, benchmark commands |
| CRDT Library | Production-Ready | GCounter, GSet, PNCounter, ORSet |
| Quantum Primitives | Research | Syntax designed, simulation only |
| Multiverse Features | Research | Conceptual, not physically realizable |
| BCI Integration | Research | API designed, no hardware support |
| Swarm Intelligence | Experimental | CRDT-backed distributed state |
| Self-Healing | Research | Conceptual framework |

**Legend:** Implemented: Feature fully complete and functional. | Experimental: Implemented in-memory under local simulation. | Research Vision: Conceptual design, syntax validation, or theoretical roadmap.

---

## Why AETHER?

AETHER is designed to explore the boundaries of declarative systems programming. Traditional languages require programmers to express how to achieve a state, leaving the compiler unaware of the programmer's ultimate goal. AETHER introduces "intent" blocks to bind verification constraints and schemas directly to structural elements.

Compared to existing systems programming languages:
- **Rust**: Focuses on memory safety via borrow checking. AETHER builds on these safety guarantees to model higher-level distributed convergence and speculative state flows.
- **Go / Zig**: Emphasize simplicity and runtime predictability. AETHER trades simplicity for rich declarative abstractions to support simulation of future computing models.

---

## Language Philosophy: Intent-Driven Programming

Intent-Driven Programming is a paradigm where computational blocks are bounded by declarations of target state (schemas) and verification invariants (assertions):
- **Object-Oriented Programming (OOP)**: Focuses on encapsulated, mutable objects sending message calls.
- **Functional Programming (FP)**: Emphasizes pure functions and immutable data flow.
- **Actor Systems**: Focuses on concurrent execution units communicating via mailboxes.
- **Intent-Driven Programming**: Focuses on declarative goal convergence. The compiler parses the schemas and constraints, and verifies state invariants at execution boundaries.

---

## Compiler Architecture

Below is the compilation pipeline structure:

```
AETHER Source (.aether)
        |
        v
      Lexer
        |
        v
      Parser
        |
        v
       AST
        |
        v
 Semantic Analysis
        |
        v
 Intent Optimizer
        |
        v
 Intermediate Representation
        |
        v
     Backend
        |
        v
 Machine Code / Simulation
```

---

## Available Today

### Core Compiler
- Lexer with 260+ keyword recognition.
- Recursive descent parser for declarations.
- Abstract Syntax Tree (AST) construction.
- Basic type checks and scope validation.
- JIT simulation logging.

### CLI Toolchain
```bash
aether init <project>      # Create new project
aether build               # Compile project
aether run                 # Execute project
aether test                # Run test suite
aether benchmark           # Run benchmarks
aether install <library>   # Install library
aether self-update         # Update compiler
```

### CRDT Library (Production-Ready)
Mathematically correct distributed data structures:
- `GCounter` - Grow-only counter.
- `GSet` - Grow-only set.
- `PNCounter` - Positive-negative counter.
- `ORSet` - Observed-remove set.

All structures are verified under integration tests to satisfy join-semilattice properties (commutativity, associativity, and idempotency).

### Standard Library
- `collections` - Vector and Map definitions.
- `io` - Standard stream and file operations.
- `net` - HTTP and socket abstractions.
- `crypto` - Basic cipher algorithms.
- `math` - Basic arithmetic and statistics.
- `time` - Operations on timestamps.
- `system` - OS environment checks.

---

## Experimental Features

### Quantum Primitives (Simulation)
```aether
qubit q1;
qubit q2;
entangle(q1, q2);
measure(q1) => result;
```
**Status**: Syntax is parsed and simulated. Actual quantum execution requires physical QPU hardware (not yet integrated).

### Intent System
```aether
intent UserAuthentication {
    schema {
        userId: String;
        isAuthenticated: Bool;
    }
}
```
**Status**: Parser recognizes intent blocks. Semantic execution is experimental.

### Swarm Runtime (CRDT-backed)
```aether
swarm_spawn(10) {
    crdt_counter.increment();
}
let total = hive_mind.sum();
```
**Status**: Backed by real CRDT library. Distributed execution is simulated locally.

---

## Research Vision

These are long-term research directions, not current capabilities:

### Brain-Computer Interface
- `cortex_bind` and `thought_intent` keywords designed.
- BCI research direction (no hardware integration).
- Research direction for future thought-to-code interfaces.

### Multiverse Execution
- `branch_reality` and `merge_universe` syntax designed.
- Multiverse-inspired speculative computing concepts.
- Conceptual framework for speculative computing research (no physical mechanism to access alternative timelines).

### Self-Healing Sandbox
- Anti-fragile runtime concept.
- Self-healing concepts (research phase) for fault detection.
- Not yet implemented.

---

## Roadmap

### Version 0.1
- [x] Lexer
- [x] Parser
- [x] CLI
- [x] CRDT Library
- [x] Basic test suite

### Version 0.2 (Current)
- [x] Full type checker
- [x] Module system
- [x] Better error messages
- [x] Improved documentation

### Version 0.3
- [ ] Enhanced package manager
- [ ] Code formatter
- [ ] LSP for IDE support

### Version 0.4
- [ ] LLVM backend integration
- [ ] Real execution (not just simulation)
- [ ] Performance optimizations

### Version 1.0
- [ ] Stable language spec
- [ ] Production-ready compiler
- [ ] Comprehensive standard library

### Long-Term Research
- [ ] Quantum hardware integration
- [ ] Real distributed runtime
- [ ] BCI hardware interfaces
- [ ] Advanced self-healing mechanisms

---

## Documentation

- [User Guide](docs/user-guide/getting-started.md)
- [Language Reference](docs/reference/syntax.md)
- [Standard Library Documentation](docs/stdlib/collections.md)
- [Compiler Internals](docs/internals/architecture.md)
- [Language Specification](SPECIFICATION.md)
- [Whitepaper](WHITEPAPER.md)
- [Architecture Overview](docs/architecture.md)
- [Intent Philosophy](docs/intent-philosophy.md)
- [Keyword Reference](docs/keyword-reference.md)

---

## Installation

### Prerequisites
- **Rust 1.70 or later** - [Install from rustup.rs](https://rustup.rs/)
- **Git** - For cloning the repository
- **Supported platforms:** Windows, macOS, Linux

### Quick Install

**macOS/Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/devsamikhan/aether/main/scripts/install.sh | bash
source ~/.bashrc  # or ~/.zshrc
aether --version
```

**Windows (PowerShell):**
```powershell
iwr -useb https://raw.githubusercontent.com/devsamikhan/aether/main/scripts/install-windows.ps1 | iex
# Restart terminal
aether --version
```

### Build from Source

```bash
git clone https://github.com/devsamikhan/aether.git
cd aether
cargo build --release
./target/release/aether --version
```

### Troubleshooting

**Error: "command not found"**
- Ensure AETHER is in your PATH: `export PATH="$HOME/.aether/bin:$PATH"`
- Restart your terminal

**Error: "permission denied"**
- Make binary executable: `chmod +x ~/.aether/bin/aether`

**Error: Build fails**
- Update Rust: `rustup update`
- Clean build: `cargo clean && cargo build --release`

**Error: Tests fail**
- Check dependencies: `cargo update`
- Run specific test: `cargo test --test crdt_tests`

---

## Quick Start

```bash
# Create new project
aether init hello-aether
cd hello-aether

# Build
aether build

# Run tests
aether test

# Run
aether run
```

Example `main.aether`:
```aether
intent HelloWorld {
    fn main() {
        println("Hello, AETHER!");
    }
}
```

---

## Contributing

We welcome contributions. Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details on build setups, tests, code styling, and pull request procedures.

---

## Benchmarks

(Coming soon - currently building benchmark suite)

Planned comparisons:
- Compilation speed vs Rust, Go, Zig.
- Execution performance.
- Memory usage.
- Binary size.

---

## Learning Resources

- [CRDT Tutorial](docs/crdt-usage.md)
- [Quantum Computing Basics](docs/quantum-computing.md)
- [Intent-Driven Programming](docs/intent-philosophy.md)
- [Examples](examples/)

---

## Acknowledgments

Inspired by:
- Rust's safety guarantees
- Haskell's type system
- Erlang's distributed model
- Quantum computing research
- CRDT literature

---

## License

MIT License - see [LICENSE](LICENSE)

---

## Links

- **GitHub**: https://github.com/devsamikhan/aether
- **Website**: https://devsamikhan.github.io/aether
- **Issues**: https://github.com/devsamikhan/aether/issues
- **Discussions**: https://github.com/devsamikhan/aether/discussions
