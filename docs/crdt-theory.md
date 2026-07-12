# CRDT Mathematical Foundations

Conflict-free Replicated Data Types (CRDTs) are distributed data structures that guarantee eventual convergence without coordination.

## 1. Semilattice Definition
A state-based CRDT is modeled as a bounded join-semilattice:
- **Set $S$**: All possible states.
- **Partial Order $\le$**: Defines state progression (a state is "greater than" or equal to a previous state).
- **Least Upper Bound (LUB) operator $\sqcup$**: Merges states.

For a merge operator to be mathematically sound, it must satisfy three properties:

1. **Commutativity**:
   $$A \sqcup B = B \sqcup A$$
2. **Associativity**:
   $$(A \sqcup B) \sqcup C = A \sqcup (B \sqcup C)$$
3. **Idempotency**:
   $$A \sqcup A = A$$

These properties guarantee that regardless of network latency, out-of-order delivery, or duplication, nodes that receive the same set of updates will merge to the identical state.
