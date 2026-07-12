# AETHER Quantum Enhancement Plan

This plan details the implementation strategy to elevate AETHER's quantum simulation engine to mathematically rigorous execution.

---

## 1. Mathematical Simulation Design

We will implement an active state vector simulator inside `src/main.rs` that maintains a complex state vector of size $2^N$ where $N$ is the number of active qubits in the context.

### 1.1 State Representation
- **State Vector**: `Vec<Complex>` of length $2^N$.
- **Complex Number**: Real and imaginary components (`f64`).

### 1.2 Implemented Quantum Gates
- **Single-Qubit Gates**: Hadamard (H), Pauli-X (NOT), Pauli-Y, Pauli-Z, Phase (S), Pi/8 (T).
- **Multi-Qubit Gates**: Controlled-NOT (CNOT), Controlled-Z (CZ), SWAP.

---

## 2. Roadmap Priorities

- **Phase 25.1**: Build `Complex` number arithmetic and state vector operations in `QuantumCompiler`.
- **Phase 25.2**: Add multi-qubit tensor product math for gates.
- **Phase 25.3**: Add probabilistic wave-function collapse (measurement).
- **Phase 25.4**: Add Bloch sphere coordinate projection.
