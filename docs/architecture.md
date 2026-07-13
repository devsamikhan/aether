# AETHER Compiler Architecture

This document describes the compiler architecture, data structures, and pipeline phases of the AETHER programming language toolchain.

## 1. Compiler Pipeline Overview

AETHER is compiled and simulated using a JIT-inspired pipeline designed for local execution check cycles. The compiler consists of the following consecutive phases:

```
Source Files (.aether)
        |
        v
      Lexer
        |
        v
   Token Stream
        |
        v
      Parser
        |
        v
Abstract Syntax Tree (AST)
        |
        v
  Semantic Analysis
        |
        v
  Intent Optimizer
        |
        v
Intermediate Representation (IR)
        |
        v
 JIT Execution / Simulation
```

---

## 2. Pipeline Phase Responsibilities

### 2.1 Lexical Analysis (Lexer)
The Lexer processes raw UTF-8 AETHER source text and tokenizes it into a stream of structured `Token` instances. It supports:
- Over 260 discrete token types including flow control, modifiers, operators, and primitive data types.
- Identifier resolution, negative numeric literal parsing, and string interpolation.
- Stripping comments and whitespace.

### 2.2 Syntax Analysis (Parser)
The Parser receives the token stream and constructs the Abstract Syntax Tree (AST). It uses a hybrid approach:
- **Recursive Descent** for structural blocks (namespaces, modules, classes, and intent declarations).
- **Pratt Precedence Parsing** for expressions to resolve complex operator hierarchies (arithmetic, boolean logic, and assignment operations).

### 2.3 Semantic Analysis & AST Verification
In this phase, the compiler performs basic type checks and verifies scopes. It checks:
- Variable declarations and mutation constraints.
- Schema matching for intent instances.
- Primitive state registers (such as local simulated qubit indices).

### 2.4 Intermediate Representation & JIT Simulation
The JIT compiler lowers the verified AST into intermediate execution logs. It simulates CPU register actions (such as `MOV`, `ADD`, `IMUL`) and maps quantum primitives to an in-process state vector simulator using complex arithmetic.

---

## 3. Key Data Structures

- **`Token`**: Enum representing keywords, identifiers, literals, and symbols.
- **`Statement`**: AST node representing structural actions (variable declarations, conditional branches, assignment, and loop structures).
- **`Expression`**: AST node representing logical, mathematical, or terminal values.
- **`QuantumRegister`**: In-process representation of the simulated state vector space ($2^N$ amplitudes for $N$ qubits) using complex numbers.

---

## 4. Design Decisions

- **Monolithic Single-File Implementation (Temporary)**: During early phases, the core JIT compiler and parser reside in a single monolithic file (`src/main.rs`) to maximize search efficiency and simplify unified testing.
- **Speculative Verification**: Computational features that are not physically realizable on classical hardware (such as multiverse timeline branches or brain-computer interface mappings) are parsed and verified as speculative syntax structures.
