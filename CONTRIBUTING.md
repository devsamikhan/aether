# Contributing to AETHER

Thank you for your interest in contributing to Project AETHER. We welcome contributions from everyone — from compiler engineers to documentation writers.

## 1. Code of Conduct

All contributors are expected to adhere to the following principles:
- **Respect**: Treat all community members with respect, regardless of experience level or background.
- **Constructive Communication**: Focus criticisms on code and ideas, not people.
- **Honesty**: Be truthful about the capabilities of AETHER, clearly distinguishing implemented features from experimental and research vision components.

---

## 2. Development Setup

### Prerequisites
- Rust 1.70 or later (stable)
- Git

### Building the Compiler
```bash
# Clone the repository
git clone https://github.com/devsamikhan/aether.git
cd aether

# Build in debug mode
cargo build

# Build in release mode
cargo build --release
```

### Running the Test Suite
```bash
# Run compiler unit tests
cargo test

# Run CRDT integration tests
cargo test --test crdt_tests

# Run Clippy static checks
cargo clippy -- -D warnings

# Run formatting checks
cargo fmt -- --check
```

---

## 3. Repository Folder Structure

- `src/`: Compiler source files (`main.rs`, `lib.rs`, `quantum/`, `complexity/`, `crdt/`).
- `tests/`: Integration test suites (`crdt_tests.rs`).
- `benches/`: Performance profiling files (`crdt_bench.rs`, `quantum_bench.rs`).
- `docs/`: Technical manuals and architecture guides.
- `website/`: Public landing page and interactive sandbox.
- `logo/`: Vector graphic branding assets.

---

## 4. Branch Strategy and Pull Request Process

- **Main Branch**: All development target branch.
- **Pull Requests**:
  1. Create a descriptive feature branch from `main`.
  2. Ensure your changes compile without warnings, pass Clippy checks, and format correctly with `cargo fmt`.
  3. Submit a Pull Request.
  4. Ensure all GitHub Actions CI checks complete successfully.
  5. PRs require a review from at least one core maintainer before merging.

---

## 5. RFC Process

For major language syntax or architecture proposals:
1. Open an issue or discussion tagged with `RFC: Proposed Feature`.
2. Provide code examples showing syntax, usage, and expected compiler behavior.
3. Align design decisions with the Intent-Driven Programming philosophy.
4. After consensus is reached, the RFC will be merged into the steering roadmap.

---

## 6. Issue Labels

- `bug`: Confirmed compiler or toolchain issues.
- `enhancement`: New features or improvements to existing implementations.
- `documentation`: Typos, guides, or API documentation tasks.
- `experimental`: Tasks related to JIT simulator improvements.
- `research`: Long-term speculative computing frameworks.
