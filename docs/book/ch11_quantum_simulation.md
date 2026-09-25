# Chapter 11: Post-Quantum Computing & State Vector Simulation

AETHER provides native state vector quantum simulation based on pure linear algebra over complex matrices.

### 11.1 Generating an Entangled Bell State ($|\Phi^+\rangle$)

```aether
import quantum

# Initialize 2-qubit register in state |00⟩
let sim = QuantumSimulator.new(2)

# Step 1: Put Qubit 0 into superposition (|0⟩ + |1⟩)/√2
sim.hadamard(0)

# Step 2: Entangle Qubit 0 and Qubit 1
sim.cnot(0, 1)

# Step 3: Collapse the wave function according to Born's probability rule
let collapsed_state = sim.measure_all()
println("Collapsed Quantum State:", collapsed_state)
# Outputs either 0 (|00⟩) or 3 (|11⟩) with exactly 50% probability each!
```

---
