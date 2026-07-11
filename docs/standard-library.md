# AETHER Standard Library Reference
A documentation of built-in standard modules in AETHER.

---

## 1. `std::core`
Standard primitives, conversions, and assertions.

- `println(s: String) -> Void`
  Prints `s` to stdout.
- `readln() -> String`
  Reads a line of text from stdin.
- `to_string(v: Any) -> String`
  Converts a value to its string representation.
- `to_int(s: String) -> Int`
  Parses a string into a 64-bit integer.
- `assert(condition: Bool) -> Void`
  Fails the running program/test suite if the condition is false.

---

## 2. `std::math`
Mathematical operations and floating-point logic.

- `sqrt(x: Float) -> Float`
  Calculates the square root of `x`.
- `pow(base: Float, exp: Float) -> Float`
  Calculates `base` raised to `exp`.
- `abs(x: Float) -> Float`
  Absolute value of `x`.
- `floor(x: Float) -> Int`
  Largest integer less than or equal to `x`.
- `ceil(x: Float) -> Int`
  Smallest integer greater than or equal to `x`.
- `sin(x: Float) -> Float` / `cos(x: Float) -> Float` / `tan(x: Float) -> Float`
  Trigonometric ratios (inputs in radians).

---

## 3. `std::io`
Filesystem read and write functions.

- `file_read(path: String) -> String`
  Reads the full text content of a file.
- `file_write(path: String, data: String) -> Void`
  Writes text content to a target file.
- `file_append(path: String, data: String) -> Void`
  Appends text content to the end of a file.
- `file_exists(path: String) -> Bool`
  Checks if a file exists at the path location.

---

## 4. `std::collections`
Standard collection operations.

- `len(a: Array<T>) -> Int`
  Returns the number of elements in the array.
- `push(a: Array<T>, v: T) -> Void`
  Appends an element to the array.
- `pop(a: Array<T>) -> T`
  Removes and returns the last element of the array.

---

## 5. `std::quantum`
Simulated quantum gates.

- `hadamard(q: qubit) -> Void`
  Applies a Hadamard gate to qubit `q`.
- `cnot(control: qubit, target: qubit) -> Void`
  Applies a controlled-NOT gate.
- `phase_gate(q: qubit, theta: Float) -> Void`
  Applies a phase shift by `theta`.

---

## 6. `std::swarm`
Collective swarm intelligence helpers.

- `broadcast(msg: String) -> Void`
  Sends a message to all active agents in the spawned swarm.
- `consensus_status() -> String`
  Returns the current global CRDT consensus state identifier.

---

## 7. `std::ai`
Deep learning and tensor utility routines.

- `infer(model_ref: model, input: tensor) -> tensor`
  Runs forward propagation inference through a neural network model.
- `train(model_ref: model, dataset: Array<tensor>) -> Void`
  Performs gradient optimization and updates neural weights.
