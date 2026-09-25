# AETHER 2.0 Formal Language Specification

**Version:** 2.0.0-PROD  
**Paradigm:** Intent-Driven, Multi-Paradigm (Procedural, Functional, Declarative Contracts)  
**Host & Compiler Engine:** Rust 2021 / Native Bytecode VM & Cranelift AOT Target  

---

## 1. Vision & Architectural Philosophy

AETHER 2.0 resolves the historic dichotomy in programming language design:
* **The Ergonomic Goal**: Simpler, cleaner, and less verbose than Python. Zero unnecessary syntax, optional parentheses, automatic return values, and fluent data pipelines (`|>`).
* **The Performance Goal**: Execution speed and memory density on par with C, C++, and C# (.NET Core AOT).
* **The Safety & Reliability Goal**: Intent-driven declarative contract blocks (`require`, `ensure`) verifiable at compile-time and asserted at runtime.

---

## 2. Lexical Structure & Grammar

### 2.1 Character Set & Comments
* Source code is encoded in UTF-8.
* Single-line comments start with `#` or `//`:
  ```aether
  # Single line comment (Python-style)
  // Single line comment (C-style)
  /* Multi-line block comment */
  ```

### 2.2 Core Keywords (~20 Orthogonal Keywords)
AETHER deliberately avoids keyword bloat. The language has exactly 22 core keywords:

| Keyword | Category | Semantics |
| :--- | :--- | :--- |
| `fn` | Declarations | Defines named functions or anonymous lambdas (`fn(x): x * 2`). |
| `let` | Declarations | Binds an immutable variable. |
| `mut` | Declarations | Declares a variable mutable (`let mut count = 0`). |
| `intent` | Contracts | Defines a verified intent block with preconditions and postconditions. |
| `require` | Contracts | Specifies an intent precondition that must hold before entry. |
| `ensure` | Contracts | Specifies an intent postcondition that must hold upon exit. |
| `if`, `elif`, `else` | Flow Control | Conditional branching. Can be used as expressions or statements. |
| `while`, `loop` | Flow Control | Iterative loops. |
| `for`, `in` | Flow Control | Iterates over arrays, ranges, maps, or iterators. |
| `return`, `yield` | Flow Control | Returns or yields a value from a function. |
| `break`, `continue` | Flow Control | Loop termination and continuation. |
| `match` | Flow Control | Exhaustive pattern matching. |
| `type`, `struct` | Types | Defines custom data layouts and types. |
| `trait`, `impl` | Polymorphism | Defines abstract interfaces and their concrete implementations. |
| `use` | Modules | Imports external or standard library namespaces. |
| `spawn`, `channel` | Concurrency | Spawns lightweight green fibers and creates message channels. |
| `defer` | Lifecycle | Defers execution of an expression until enclosing scope exit. |

### 2.3 Values & Literals
* `Int`: 64-bit signed integer (`-42`, `100_000`).
* `Float`: 64-bit IEEE 754 floating-point number (`3.14159`, `1e-6`).
* `Bool`: `true` or `false`.
* `Nil`: `nil` (represents absence of value).
* `String`: UTF-8 immutable string with escape sequences (`"hello\n"`).
* `Array`: Heterogeneous or homogeneous dynamic arrays (`[1, 2, 3]`).
* `Map`: Hash-keyed associative tables (`{"key": 42}`).

### 2.4 Indentation and Block Delimiters
AETHER supports dual block notation:
1. **Significant Indentation (Recommended)**: Follows a colon `:` with indented body statements:
   ```aether
   fn calculate(x, y):
       let sum = x + y
       sum * 2
   ```
2. **Explicit Braces (C/Rust Style)**: For inline code or personal preference:
   ```aether
   fn calculate(x, y) { let sum = x + y; sum * 2 }
   ```

---

## 3. The Type System & Static Semantics

### 3.1 Unboxed Value Types vs. Heap Reference Types
1. **Value Types (Stack-Allocated, Zero Allocation Overhead)**:
   - `int` (`i64`), `float` (`f64`), `bool`, `char`, `nil`.
   - Small custom structs (e.g. `Point { x, y }`).
   - Stored directly in CPU registers or stack frames. No garbage collection, no indirection.
2. **Reference Types (Heap-Allocated, Reference Counted)**:
   - `string`, `array`, `map`, `function`, `channel`.
   - Tracked via deterministic ARC (Automatic Reference Counting).

### 3.2 Bidirectional Type Inference
Type annotations are optional. The compiler infers static types using a Hindley-Milner bidirectional algorithm:
```aether
let a = 10          # Inferred as int (i64)
let b = 2.5         # Inferred as float (f64)
let c = a + b       # Inferred as float (f64), automatic widening

# Explicit annotations are allowed for API documentation or enforcement:
let limit: int = 100
fn distance(dx: float, dy: float) -> float:
    Math.sqrt(dx * dx + dy * dy)
```

---

## 4. Memory Model: Deterministic ARC (Zero-GC)

AETHER guarantees predictable latency by discarding tracing garbage collectors (GC):
* **No Stop-The-World Freezes**: Unlike Java, C#, or Go, execution never pauses for GC collection cycles.
* **Compile-Time Lifetime Elision**:
  - The compiler traces variable scopes and inserts destruction calls (`decref`) at the exact point of last use.
  - Temporary values in pipelines and expressions are freed immediately after evaluation.
* **Cycle Collection**: Isolated weak references or cycle-breaking ownership rules prevent cyclic leaks.

---

## 5. Intent-Driven Programming Semantics

An `intent` is a first-class language construct combining operational logic with formal verification contracts:

```aether
intent withdraw(account, amount):
    require: amount > 0
    require: account.balance >= amount
    ensure: account.balance == old(account.balance) - amount
    ensure: result == true
    body:
        account.balance -= amount
        return true
```

### Invariant Execution Semantics:
1. **`require: <condition>`**: Evaluated before entering the intent body. If false, raises an `IntentPreconditionError`.
2. **`old(<expr>)`**: Snapshots state prior to body mutation.
3. **`ensure: <condition>`**: Evaluated immediately before returning. If false, raises an `IntentPostconditionError`.
4. **Compile-Time Prover**: In AOT release builds, the compiler attempts static proof of constraints using constraint propagation. Unproven assertions remain as zero-overhead branch checks.

---

## 6. Concurrency & Asynchronous Runtime

AETHER rejects OS-thread-per-task models and async/await function coloring:
* **Fibers (Green Threads)**: `spawn` allocates a 2KB stack fiber scheduled onto an M:N work-stealing thread pool across available CPU cores.
* **Communicating Sequential Processes (CSP)**: Data travels between fibers via typed channels:
  ```aether
  let ch = channel()
  
  spawn:
      let processed = heavy_computation()
      send(ch, processed)
      
  let result = recv(ch)
  ```
* **Immutability by Default**: Cross-fiber data passing enforces ownership transfer or immutable views, eliminating data races at the architectural level.

---

## 7. Standard Library Architecture

All built-in system capabilities are structured in first-class native namespaces:

| Module | Core Functions |
| :--- | :--- |
| `File` | `read`, `write`, `append`, `exists`, `delete`, `lines` |
| `Json` | `parse`, `stringify`, `pretty` |
| `Math` | `sqrt`, `abs`, `sin`, `cos`, `tan`, `floor`, `ceil`, `round`, `min`, `max`, `random`, `random_int`, `PI`, `E` |
| `Strings` | `split`, `join`, `trim`, `upper`, `lower`, `contains`, `replace`, `starts_with`, `ends_with` |
| `Sys` | `env`, `cwd`, `time_ms`, `sleep`, `args` |

---

## 8. Compiler & Toolchain Interface

```bash
aether run <file.ae>          # Instant execution via JIT Bytecode VM
aether build --release        # AOT native compilation to standalone executable
aether repl                   # Interactive read-eval-print loop
aether test                   # Runs embedded assertions and test blocks
```
