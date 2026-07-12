# AETHER Quantum Computing Guide

AETHER natively integrates quantum execution statements within its classical system architecture using the Unified Context Graph (UCG).

---

## ⚛️ Quantum Primitives

AETHER provides three primary keywords for interacting with simulated or physical qubits:

1. **`qubit`**: Allocates a qubit in the quantum register.
2. **`entangle`**: Applies a Hadamard gate and controlled-NOT gate to generate a Bell state superposition.
3. **`measure`**: Collapses the wave-function probabilistically to a classical bit.

### Example Code
```aether
intent QuantumRNG {
    fn generate() {
        qubit q;
        superpose(q);
        measure(q) => outcome;
        return outcome;
    }
}
```

---

## 🎛️ Simulation Engine Mathematics
State vectors maintain $2^N$ complex amplitudes where $N$ is the number of active qubits in the register. Measurements are computed via sum-of-squares probability coordinates.
