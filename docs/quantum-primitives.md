# AETHER Quantum Primitives

> **Status:** Phase 1 — Classical Simulation  
> **Roadmap:** Physical QPU routing and hardware execution are Phase 2 items.

---

## Overview

AETHER's quantum primitives are **first-class language keywords** compiled by the Rust-based JIT into quantum gate sequences. During Phase 1, all quantum computation runs on a high-fidelity classical simulator embedded in the JIT runtime. The JIT logs each gate operation, register allocation, and measurement event to `stderr` with the prefix `[AETHER::QSim]`, enabling full observability of the quantum execution trace without requiring physical quantum hardware.

The design goal is **source-code portability**: AETHER programs written today against the classical simulator will compile and execute unchanged on a physical Quantum Processing Unit (QPU) in Phase 2, with the JIT backend swapped transparently.

```
Compilation pipeline (Phase 1):
  AETHER source → Lexer → AST → UCG IR → Rust JIT → Gate Sequence → Classical QSim
  
Compilation pipeline (Phase 2):
  AETHER source → Lexer → AST → UCG IR → Rust JIT → Gate Sequence → QPU Router → Physical QPU
```

All quantum gate sequences are logged in OpenQASM-compatible notation for interoperability with external quantum toolchains (Qiskit, Cirq, etc.).

---

## `qubit`

### Concept

A `qubit` is the fundamental unit of quantum information in AETHER. Unlike a classical bit which is strictly `0` or `1`, a qubit exists in a **superposition** of both basis states simultaneously:

```
|ψ⟩ = α|0⟩ + β|1⟩
```

where `α` and `β` are complex probability amplitudes satisfying the normalization constraint `|α|² + |β|² = 1`. A qubit collapses to a definite classical value only at the moment of measurement (see [`measure`](#measure)).

The AETHER JIT allocates each declared `qubit` as a slot in the simulator's quantum register. Register addresses are monotonically assigned and logged at allocation time.

### Syntax

```aether
qubit name;
qubit name = |0>;     // Explicit ground state initialization (default)
qubit name = |1>;     // Explicit excited state initialization
```

### JIT Output

```
[AETHER::QSim] ALLOC  qubit 'name' → register[0]  state=|0⟩
[AETHER::QSim] ALLOC  qubit 'name' → register[1]  state=|1⟩
```

### Examples

**Example 1 — Basic qubit declaration:**
```aether
qubit q;
// JIT: [AETHER::QSim] ALLOC qubit 'q' → register[0] state=|0⟩
```

**Example 2 — Multiple qubits forming a register:**
```aether
qubit q0;
qubit q1;
qubit q2;
// JIT allocates register[0], register[1], register[2]
// Full system state: |000⟩
```

**Example 3 — Explicit state initialization:**
```aether
qubit ancilla = |1>;
// JIT: [AETHER::QSim] ALLOC qubit 'ancilla' → register[0] state=|1⟩
// Useful as ancilla qubits in phase kickback algorithms
```

**Example 4 — Qubit array:**
```aether
qubit[8] register;
// JIT: [AETHER::QSim] ALLOC qubit[8] 'register' → register[0..7] state=|00000000⟩
// Declares an 8-qubit register for byte-width quantum arithmetic
```

---

## `entangle`

### Concept

`entangle` creates a **Bell State** entanglement between two qubits. Once entangled, the qubits are no longer independent — their quantum states become a single, inseparable joint state. Measuring one qubit instantaneously determines the outcome of measuring the other, regardless of the physical distance between them (non-locality).

The canonical Bell State produced by `entangle` is:

```
|Ψ⁺⟩ = 1/√2 (|00⟩ + |11⟩)
```

Internally, the JIT compiles `entangle(q1, q2)` to a two-gate sequence:
1. **Hadamard (H)** applied to `q1` — puts `q1` into superposition
2. **CNOT** with `q1` as control and `q2` as target — correlates the two states

### Syntax

```aether
entangle(q1, q2);
```

### JIT Output

```
[AETHER::QSim] GATE   H     register[0]           // q1 → superposition
[AETHER::QSim] GATE   CNOT  control=register[0]  target=register[1]
[AETHER::QSim] STATE  Bell  |Ψ⁺⟩ = 1/√2(|00⟩+|11⟩)  register[0,1]
```

### Examples

**Example 1 — Basic Bell pair:**
```aether
qubit q1;
qubit q2;
entangle(q1, q2);
// q1 and q2 are now maximally entangled
```

**Example 2 — Entangle then measure:**
```aether
qubit alice;
qubit bob;
entangle(alice, bob);
measure(alice) => a_result;
measure(bob)   => b_result;
// a_result and b_result are guaranteed to match (both 0 or both 1)
// with 50% probability each
```

**Example 3 — Entangled register pair (quantum teleportation setup):**
```aether
qubit message;
qubit epr_a;
qubit epr_b;

// Prepare the EPR pair
entangle(epr_a, epr_b);

// Apply message qubit through CNOT with epr_a (Bell measurement setup)
cnot(message, epr_a);
hadamard(message);
```

---

## `measure`

### Concept

`measure` collapses the wave function of a qubit to a classical bit value. Prior to measurement, a qubit in superposition `α|0⟩ + β|1⟩` carries no definite value. The act of measurement is fundamentally probabilistic:

- Outcome `0` with probability `|α|²`
- Outcome `1` with probability `|β|²`

After measurement, the qubit collapses to the measured state and all quantum coherence is destroyed. The result is bound to a classical variable of type `bit`.

The JIT logs the pre-measurement probability amplitudes and the post-collapse eigenvalue.

### Syntax

```aether
measure(q) => result;
```

### JIT Output

```
[AETHER::QSim] MEASURE register[0]  α=0.707+0i  β=0.707+0i
[AETHER::QSim] COLLAPSE register[0]  eigenvalue=1  P(0)=0.500  P(1)=0.500
[AETHER::QSim] BIND    'result' ← 1
```

### Examples

**Example 1 — Measure a qubit in superposition:**
```aether
qubit q;
superpose(q);              // H gate: equal superposition
measure(q) => outcome;    // outcome = 0 or 1 with P=0.5 each
```

**Example 2 — Conditional logic on measurement result:**
```aether
qubit q;
superpose(q);
measure(q) => bit result;

if result == 1 {
    emit("Quantum coin: HEADS");
} else {
    emit("Quantum coin: TAILS");
}
```

**Example 3 — Measure all qubits in a register:**
```aether
qubit[4] reg;
// ... apply gates ...
measure(reg[0]) => b0;
measure(reg[1]) => b1;
measure(reg[2]) => b2;
measure(reg[3]) => b3;
int result = (b3 << 3) | (b2 << 2) | (b1 << 1) | b0;
```

**Example 4 — Mid-circuit measurement with reset:**
```aether
qubit ancilla;
hadamard(ancilla);
cnot(ancilla, data_qubit);
measure(ancilla) => syndrome;
ancilla = |0>;             // Reset ancilla for reuse
```

---

## `superpose`

### Concept

`superpose` places a qubit into an **equal superposition** of `|0⟩` and `|1⟩` by applying the **Hadamard gate**. It is equivalent to `hadamard(q)` and is provided as a higher-level alias for readability. The resulting state is:

```
|+⟩ = 1/√2 (|0⟩ + |1⟩)
```

This is the canonical starting point for most quantum algorithms — it is the mechanism by which quantum parallelism is achieved.

### Syntax

```aether
superpose(q);
```

### Examples

**Example 1 — Single qubit superposition:**
```aether
qubit q;
superpose(q);
// q is now in state |+⟩ = 1/√2(|0⟩+|1⟩)
measure(q) => r;   // r = 0 or 1 with equal probability
```

**Example 2 — Superpose all qubits for parallel search:**
```aether
qubit[5] search_space;
for i in 0..5 {
    superpose(search_space[i]);
}
// All 32 computational basis states are now in superposition
// The system represents all 5-bit strings simultaneously
```

**Example 3 — Superpose then interfere:**
```aether
qubit q;
superpose(q);       // |+⟩
phase_gate(q, PI); // Apply phase flip
superpose(q);       // Interference collapses to |1⟩ deterministically
measure(q) => r;   // r = 1 with P=1.0
```

---

## Gate Primitives

### `hadamard`

Applies the Hadamard gate to a qubit. Puts a `|0⟩` qubit into `|+⟩` superposition and a `|1⟩` qubit into `|-⟩` superposition. Self-inverse: `hadamard(hadamard(q)) == q`.

```
H = 1/√2 * [[1,  1],
             [1, -1]]
```

```aether
hadamard(q);
// JIT: [AETHER::QSim] GATE H register[n]
```

---

### `cnot`

The Controlled-NOT gate. Flips the target qubit if and only if the control qubit is `|1⟩`. The fundamental entangling gate in most quantum circuits.

```
CNOT: |control, target⟩ → |control, control XOR target⟩
```

```aether
cnot(control_qubit, target_qubit);
// JIT: [AETHER::QSim] GATE CNOT control=register[m] target=register[n]
```

---

### `phase_gate`

Applies a phase rotation by angle `θ` to the `|1⟩` component of a qubit's state. Does not affect the `|0⟩` component. Useful for phase kickback and quantum phase estimation.

```
P(θ) = [[1,    0  ],
         [0,  e^(iθ)]]
```

```aether
phase_gate(q, PI / 4);     // T-gate (π/4 phase shift)
phase_gate(q, PI / 2);     // S-gate (π/2 phase shift)
phase_gate(q, PI);         // Z-gate (π phase flip)
// JIT: [AETHER::QSim] GATE P(θ) register[n]  θ=0.785398
```

---

### `toffoli`

The Toffoli (CCNOT) gate. A three-qubit gate that flips the target qubit only if **both** control qubits are `|1⟩`. It is a universal reversible classical gate and is used for quantum error correction and arithmetic.

```aether
toffoli(control1, control2, target);
// JIT: [AETHER::QSim] GATE CCNOT control=[register[m], register[n]] target=register[p]
```

---

### `swap_gate`

Swaps the quantum states of two qubits. Equivalent to three CNOT gates in sequence. Used in quantum routing and the construction of multi-qubit algorithms.

```aether
swap_gate(q1, q2);
// JIT: [AETHER::QSim] GATE SWAP register[m] ↔ register[n]
```

---

## `grover_oracle`

### Concept

`grover_oracle` declares an **oracle function** for use in Grover's quantum search algorithm. The oracle is a black-box unitary that marks the target state(s) by applying a phase flip of `-1` to any basis state satisfying the search predicate. The oracle itself does not reveal which states are marked — it only flips their phase.

Grover's algorithm achieves a **quadratic speedup** over classical search: finding one item in an unsorted database of `N` items requires O(√N) oracle queries, compared to O(N) classically.

### Syntax

```aether
grover_oracle(target_register) {
    // Predicate: mark states where condition is true
    if target_register == target_value {
        phase_flip(target_register);
    }
};
```

### Examples

**Example 1 — Search for a specific value:**
```aether
qubit[4] search_reg;
int target = 11;     // Search for the value 11 in a 4-bit space (N=16)
int iterations = floor(PI / 4 * sqrt(16));  // ≈ 3 iterations

// Initialize: superpose all 16 states
for i in 0..4 { superpose(search_reg[i]); }

// Grover iterations
repeat iterations {
    // Oracle: mark |1011⟩
    grover_oracle(search_reg) {
        if search_reg == target { phase_flip(search_reg); }
    };

    // Diffusion operator (inversion about average)
    for i in 0..4 { hadamard(search_reg[i]); }
    phase_flip_zero(search_reg);
    for i in 0..4 { hadamard(search_reg[i]); }
}

measure(search_reg) => found;
// found = 11 with probability ≈ 0.961
```

**Example 2 — Multi-target search:**
```aether
qubit[4] reg;
for i in 0..4 { superpose(reg[i]); }

grover_oracle(reg) {
    // Mark two solutions
    if reg == 3 || reg == 12 { phase_flip(reg); }
};
```

---

## `quantum_fourier`

### Concept

`quantum_fourier` applies the **Quantum Fourier Transform (QFT)** — the quantum analogue of the Discrete Fourier Transform — to a qubit register. The QFT maps a quantum state from the computational basis to the Fourier basis in O(n²) gate operations (compared to O(n·2ⁿ) for a classical DFT on the same data).

The QFT is a subroutine in Shor's factoring algorithm, quantum phase estimation, and quantum signal processing.

### Syntax

```aether
quantum_fourier(register);
quantum_fourier_inverse(register);   // Inverse QFT
```

### Examples

**Example 1 — QFT on a 4-qubit register:**
```aether
qubit[4] qft_reg;
// Prepare an input state
hadamard(qft_reg[0]);
phase_gate(qft_reg[1], PI / 2);

// Apply QFT
quantum_fourier(qft_reg);
// JIT: [AETHER::QSim] QFT  register[0..3]  gates=10  depth=4
```

**Example 2 — Phase estimation subroutine:**
```aether
qubit[8] phase_reg;
qubit    eigenstate;

for i in 0..8 { superpose(phase_reg[i]); }
// ... apply controlled-U operations ...
quantum_fourier_inverse(phase_reg);
measure(phase_reg) => estimated_phase;
```

**Example 3 — Quantum period finding (Shor's subroutine):**
```aether
qubit[12] input_reg;
qubit[6]  output_reg;

for i in 0..12 { superpose(input_reg[i]); }
// Modular exponentiation: output_reg ← a^input_reg mod N
modular_exp(input_reg, output_reg, base: 7, modulus: 15);
quantum_fourier(input_reg);
measure(input_reg) => period_estimate;
```

---

## Use Cases

### CollapseSort

A quantum sorting algorithm that leverages superposition to evaluate all permutations simultaneously. The oracle marks the sorted permutation; Grover amplification is applied to collapse to it.

```
Theoretical complexity: O(√(n!)) oracle queries
Classical comparison sort: O(n log n)
```

```aether
qubit[4] sort_reg;
// Encode unsorted array into qubit amplitudes
encode_array([5, 2, 8, 1], sort_reg);

// Define sort oracle
grover_oracle(sort_reg) {
    if is_sorted(sort_reg) { phase_flip(sort_reg); }
};

// Amplify and collapse
int grover_steps = floor(PI / 4 * sqrt(24));  // √(4!) ≈ 4.9
repeat grover_steps { grover_diffuse(sort_reg); }
measure(sort_reg) => sorted_result;
```

### GroverSwarmSearch

Combines AETHER's `swarm` primitives with Grover's algorithm. Each swarm agent independently maintains a search subspace; entanglement across agents collapses the global solution. See `swarm-primitives.md` for the full implementation.

### ConsensusLedger

A quantum Byzantine fault-tolerant ledger using entangled validator qubits for consensus. Validators that are entangled cannot independently produce conflicting measurements, guaranteeing fork-free finality. See `swarm-primitives.md` for details.

---

## Performance Characteristics

| Operation          | Gate Count | Circuit Depth |
|--------------------|-----------|---------------|
| `superpose`        | 1 (H)     | 1             |
| `entangle`         | 2 (H+CNOT)| 2             |
| `grover_oracle`    | O(n)      | O(n)          |
| `quantum_fourier`  | O(n²)     | O(n²)         |
| `toffoli`          | 6 (CNOT decomp) | 5        |

> **Note:** These gate count and depth benchmarks reflect simulated complexity analysis and theoretical bounds, not runs on physical hardware.

### Simulator Complexity (Phase 1)

The classical simulator maintains a full state vector of `2ⁿ` complex amplitudes for an `n`-qubit system. Memory and compute scale **exponentially** with qubit count:

- **Memory:** `2ⁿ × 16 bytes` (two 64-bit floats per amplitude)
- **Gate application:** O(2ⁿ) per single-qubit gate; O(2ⁿ) per two-qubit gate

| Qubits | State vector size | Simulator RAM |
|--------|------------------|---------------|
| 10     | 1,024 amplitudes | ~16 KB        |
| 20     | 1,048,576        | ~16 MB        |
| 30     | ~1 billion       | ~16 GB        |
| 40     | ~1 trillion      | ~16 TB        |

> **Note:** These figures represent simulated complexity analysis and classical simulation scaling characteristics, not runs on physical hardware. Practical simulation on commodity hardware is limited to approximately 28–32 qubits. For larger circuits, AETHER Phase 1 will use tensor-network simulation which trades fidelity for scalability.

### Physical Hardware Requirements (Phase 2)

QPU execution will require:
- **Qubit coherence time** exceeding circuit depth × gate time
- **Gate fidelity** > 99.9% for error-corrected algorithms
- **Connectivity graph** compatible with CNOT topology (see QPU routing docs, Phase 2)
- **Cryogenic control electronics** at ~15 mK for superconducting qubit targets

> [!NOTE]
> Phase 2 QPU routing will automatically transpile AETHER gate sequences to native gate sets (e.g., `{CZ, Rz, SX}` for IBM, `{CZ, Rz, XY}` for Google) using the Sabre routing algorithm.
