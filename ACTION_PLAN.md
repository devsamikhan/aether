# AETHER 7-Day Action Plan

This document outlines the immediate daily tasks for the AETHER team.

---

## 📅 Daily Execution Schedule

### Day 1: Diagnostic Span Mapping
- **Tasks**: Update the Lexer in `src/main.rs` to record character position offsets (line, column) inside tokens.
- **Expected Outcome**: Tokens contain span bounds.
- **Verification**: Run `cargo check` and compile a test file with parsing syntax errors to print target line/column details.

### Day 2: Diagnostic Error Reporting
- **Tasks**: Modify AST and Parser nodes to display source location spans on syntax errors.
- **Expected Outcome**: Clean, visual terminal error spans similar to Rust compiler outputs.
- **Verification**: Verify error printing formatting on mock code files containing mismatched braces.

### Day 3: Multi-Platform Installer Edge-Cases
- **Tasks**: Update installer scripts to check system architectures for arm64/x64 edge cases.
- **Expected Outcome**: Universal script supports macOS, Linux, and Windows.
- **Verification**: Run local tests on different virtual machine instances.

### Day 4: Standard Library Testing Integration
- **Tasks**: Add unit tests for `libraries/std/collections.aether` operations (push, pop, insert, get).
- **Expected Outcome**: Automated verification of basic standard library syntax.
- **Verification**: Run `aether test` on collection samples.

### Day 5: Speculative Timeline Threading
- **Tasks**: Implement thread pool structures to execute parallel `branch_reality` simulations.
- **Expected Outcome**: Non-blockingSpeculative evaluations.
- **Verification**: Measure performance improvements on pathfinder queries.

### Day 6: OpenQASM Parser Integration
- **Tasks**: Write early exporters for quantum gates to support OpenQASM specifications.
- **Expected Outcome**: Qubit operations can export to standard OpenQASM files.
- **Verification**: Inspect generated `.qasm` outputs.

### Day 7: Documentation Audit
- **Tasks**: Audit all hyperlinks across root files and docs, updating outdated paths.
- **Expected Outcome**: Zero broken references or incorrect links.
- **Verification**: Run a markdown link checker.
