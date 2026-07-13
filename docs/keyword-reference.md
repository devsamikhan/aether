# AETHER Keyword Reference

This reference documents the primary keywords supported by the AETHER lexical analyzer and compiler parser, detailing their syntax, compiler behavior, and implementation status.

## 1. Core Flow and Declarations

### `intent`
- **Purpose**: Declares an intent unit containing schemas, execution methods, and assertions.
- **Syntax**: `intent Name { ... }`
- **Compiler Behavior**: Constructs an AST `Statement::Intent` block node.
- **Status**: Implemented (parser and syntax checker active; runtime execution under simulation).

### `fn`
- **Purpose**: Declares an execution method or function.
- **Syntax**: `fn compute() { ... }`
- **Compiler Behavior**: Defines entry points in JIT lowering.
- **Status**: Implemented.

---

## 2. Distributed and Swarm Operations

### `swarm_spawn`
- **Purpose**: Spawns multiple local simulated nodes to run a block using CRDT replication.
- **Syntax**: `swarm_spawn(10) { ... }`
- **Compiler Behavior**: Parses execution blocks and registers CRDT variables to local cluster contexts.
- **Status**: Experimental (simulation backed by production-ready Rust CRDT library).

### `hive_mind`
- **Purpose**: Provides global read operations on converged swarm metrics.
- **Syntax**: `let total = hive_mind.sum();`
- **Compiler Behavior**: Merges state-based counters and returns the aggregate value.
- **Status**: Experimental.

---

## 3. Quantum Primitives (Simulated)

### `qubit`
- **Purpose**: Allocates a qubit index in the simulated quantum register.
- **Syntax**: `qubit q;`
- **Compiler Behavior**: Expands the active state vector space dynamically by tensor-multiplying the state vector by a new $|0\rangle$ state.
- **Status**: Experimental (classical simulation only).

### `entangle`
- **Purpose**: Applies a Hadamard gate and CNOT gate sequence to generate a Bell state superposition across two qubits.
- **Syntax**: `entangle(q1, q2);`
- **Compiler Behavior**: Applies unitary matrices to the state vector.
- **Status**: Experimental (classical simulation only).

### `measure`
- **Purpose**: Collapses a simulated qubit's wave-function using norm-squared Born rule probabilities.
- **Syntax**: `measure(q) => outcome;`
- **Compiler Behavior**: Computes probabilities, performs a simulated probabilistic collapse, and normalizes the resulting state vector.
- **Status**: Experimental (classical simulation only).

---

## 4. Speculative Features (Research Vision)

### `branch_reality`
- **Purpose**: Forks the current execution context into separate speculative state timelines.
- **Syntax**: `branch_reality { ... }`
- **Compiler Behavior**: Parsed and validated as structural blocks.
- **Status**: Research Vision (conceptual framework for multiverse speculative computing; not physically realizable).

### `merge_universe`
- **Purpose**: Selects and merges the optimal timeline based on cost metrics.
- **Syntax**: `merge_universe(outcome);`
- **Compiler Behavior**: Parsed and verified.
- **Status**: Research Vision.

### `cortex_bind`
- **Purpose**: Direct mapping of brain-computer interface signal streams to variables.
- **Syntax**: `cortex_bind(stream_source) => var;`
- **Compiler Behavior**: Parsed as experimental keyword token.
- **Status**: Research Vision (no hardware integration).
