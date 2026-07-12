# AETHER Quantum Gate Reference

This reference details the mathematical definitions of quantum gates supported in AETHER.

---

## 1. Single-Qubit Gates

### Hadamard (H)
Transforms standard basis states to superpositions:
$$H = \frac{1}{\sqrt{2}} \begin{pmatrix} 1 & 1 \\ 1 & -1 \end{pmatrix}$$

### Pauli-X (NOT)
Flips the state:
$$X = \begin{pmatrix} 0 & 1 \\ 1 & 0 \end{pmatrix}$$

### Pauli-Y
Pauli-Y phase shift:
$$Y = \begin{pmatrix} 0 & -i \\ i & 0 \end{pmatrix}$$

### Pauli-Z
Flips phase:
$$Z = \begin{pmatrix} 1 & 0 \\ 0 & -1 \end{pmatrix}$$

---

## 2. Multi-Qubit Gates

### Controlled-NOT (CNOT)
Flips target qubit if control qubit is 1.
