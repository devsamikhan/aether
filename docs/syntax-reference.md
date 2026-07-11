# AETHER Syntax Reference
A complete guide to AETHER's keywords, operators, and built-ins.

---

## 1. Core Flow Control
These keywords govern execution path branching, loops, and statement evaluation.

- **`if` / `else if` / `else`**: Scoped condition testing.
  ```aether
  if (value > 100) {
      this.status = "high";
  } else if (value > 50) {
      this.status = "medium";
  } else {
      this.status = "low";
  }
  ```
- **`match`**: Standard pattern matcher block.
  ```aether
  match code {
      1 => { this.msg = "Ok"; }
      2 => { this.msg = "Error"; }
      _ => { this.msg = "Unknown"; }
  }
  ```
- **`while`**: Traditional conditional loop.
  ```aether
  let i = 0;
  while (i < 10) {
      i = i + 1;
  }
  ```
- **`for` / `in`**: Container element iterator.
  ```aether
  for (item in this.items) {
      println(item);
  }
  ```
- **`return`**: Exit current function execution and return a value.
- **`break` / `continue`**: Loop iteration controls.

---

## 2. Declarations
These keywords define logic containers, parameters, types, and variables.

- **`intent`**: Primary program schema and logical component definition block.
  ```aether
  intent DataProcessor {
      schema { records: Int = 0; }
  }
  ```
- **`schema`**: Defines mutable internal state for the enclosing intent.
- **`fn`**: Declares a callable method inside an intent.
- **`let` / `const`**: Local variable declarations (mutable / immutable).
- **`type` / `enum` / `struct`**: Domain types, enumerations, and structures.

---

## 3. Modifiers
These keywords modify scope, state mutability, and lifetime.

- **`pub` / `priv`**: Access control modifiers.
- **`async` / `await`**: Asynchronous function declaration and scheduling.
- **`mut`**: Explicit mutability marker.
- **`ref` / `move`**: Variable binding pass-by-reference / transfer-of-ownership.

---

## 4. Memory & Safety
Special keywords for memory layout and safety bounds constraints.

- **`alloc` / `dealloc`**: Fluid Allocator heap controls.
- **`epoch` / `hazard`**: Zero-STW garbage collector helpers.
- **`null_check` / `bounds_check`**: Sandbox verification checks.

---

## 5. Concurrency & Async
Keywords controlling parallel threads of execution.

- **`spawn` / `join`**: Thread spawning and synchronization.
- **`channel` / `send` / `recv`**: Message-passing actors.
- **`lock_free` / `rcu`**: RCU-based transactional state modifications.

---

## 6. Data Structures
Standard language collections.

- **`array` / `map` / `set`**: Continuous array, key-value map, and uniqueness set.
- **`queue` / `stack`**: Sequential collections.
- **`tensor` / `matrix`**: AI/linear algebra tensors.

---

## 7. Math & Logic Operators
Standard operators and functions.

- **`+`, `-`, `*`, `/`, `%`**: Arithmetic operators.
- **`==`, `!=`, `<`, `>`, `<=`, `>=`**: Comparison operators.
- **`&&`, `||`, `!`**: Logical operators.

---

## 8. Quantum Primitives
Core primitives for post-quantum computing.

- **`qubit`**: Allocates a qubit register reference.
  ```aether
  qubit q;
  ```
- **`entangle`**: Creates a Bell state link between two qubits.
  ```aether
  entangle(q1, q2);
  ```
- **`measure`**: Collapses state vector probability amplitude to a classical value.
  ```aether
  measure(q) => result;
  ```
- **`superpose`**: Applies a Hadamard gate, placing a qubit into equal superposition.
  ```aether
  superpose(q);
  ```

---

## 9. Multiverse Primitives
Speculative multi-timeline execution operations.

- **`branch_reality`**: Forks the current program environment into a speculative context.
  ```aether
  branch_reality {
      // timeline logic
      observe_timeline(res);
  };
  ```
- **`observe_timeline`**: Queries a parallel speculative timeline's state.
- **`merge_universe`**: Integrates a chosen speculative timeline back into main.

---

## 10. Swarm Primitives
Distributed autonomous routines.

- **`swarm_spawn`**: Spawns N parallel agents.
  ```aether
  swarm_spawn(100);
  ```
- **`hive_mind`**: Scopes collective consensus updates across swarm agents.
- **`von_neumann_replicate`**: Duplicates active agent states across the swarm.

---

## 11. BCI & Spatial Primitives
Thought intention binding and holographic representations.

- **`cortex_bind` / `neural_stream`**: Links program variables to real-time cognitive activity.
  ```aether
  cortex_bind neural_stream("motor_cortex") {
      thought_intent("synthesize") => this.synthesis_flow()
  };
  ```
- **`hologram`**: Defines rendering layouts in spatial coordinate spaces.
- **`spatial_anchor` / `depth_mesh`**: Real-world spatial tracking elements.

---

## 12. Database & AI Primitives
Native database execution and neural model layers.

- **`db` / `query`**: In-place database transaction query block.
  ```aether
  db { query: "SELECT * FROM users"; }
  ```
- **`model`**: Deep neural network architecture declaration block.
  ```aether
  model Net {
      Dense(units: 128),
      Dense(units: 10)
  }
  ```

---

## 13. Plain-English Built-ins
A selection of the 60+ standard built-in functions:
- `println(String)`: Prints string.
- `readln() -> String`: Reads line.
- `sqrt(Float) -> Float`: Square root.
- `pow(Float, Float) -> Float`: Exponentiation.
- `abs(Float) -> Float`: Absolute value.
- `floor(Float) -> Int`: Nearest lower integer.
- `ceil(Float) -> Int`: Nearest higher integer.
- `to_string(Any) -> String`: String conversion.
- `to_int(String) -> Int`: Integer parsing.
- `assert(Bool)`: Testing assert.
