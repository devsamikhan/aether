# AETHER Development Roadmap

This document outlines the version milestones and research directions for the AETHER programming language.

## Milestone 1: Version 0.1 (Current Status)
- Status: Completed
- Focus: Build foundational parsing and JIT simulation frameworks.
- Deliverables:
  - Lexical analyzer and parser for intent declarations and expressions.
  - Command-line toolchain (`init`, `build`, `test`, `benchmark` commands).
  - Production-ready `crdt` library implementing state-based semilattices (GCounter, GSet, PNCounter, ORSet).
  - Standing unit test suite.

## Milestone 2: Version 0.2 (Next Phase)
- Status: Planned
- Focus: Type verification and developer tools.
- Deliverables:
  - Full static type checker for variable mutations and schemas.
  - Multi-file module import system.
  - Detailed compile-time error reporting with line and column spans.

## Milestone 3: Version 0.3
- Status: Planned
- Focus: Developer Experience (DX) tooling.
- Deliverables:
  - Integrated package installer and solver.
  - Automatic code formatter.
  - Language Server Protocol (LSP) implementation for IDE completions.

## Milestone 4: Version 0.4
- Status: Planned
- Focus: Real Execution Backends.
- Deliverables:
  - LLVM compiler backend integration.
  - Native machine code generation (executables for Windows, macOS, Linux).
  - In-memory execution profiling.

## Milestone 5: Version 1.0
- Status: Long-Term Goal
- Focus: Production Release.
- Deliverables:
  - Standardized stable language specification.
  - Complete, optimized standard library coverage.
  - Production-ready compiler toolchain.

---

## Long-Term Research Directions

These areas represent conceptual explorations for future computing architectures:
- **Quantum Hardware Integration**: Translating simulated quantum primitive gate sequences to physical QPU instruction standards (such as OpenQASM).
- **Physical Swarm Runtime**: Running CRDT state-based convergence across real multi-node network clusters.
- **Brain-Computer Interface**: Mapping cognitive signal streams (EEG inputs) to language primitives under a formal hardware abstraction layer.
- **Autonomous Self-Healing**: Real JIT recovery of fault states using dynamic runtime patch generation.
