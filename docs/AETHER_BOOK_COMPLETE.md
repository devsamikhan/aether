# 📖 The AETHER Programming Language
## *The Definitive Guide to Intent-Driven, Post-Quantum & Cognitive Systems*
### Official Documentation — Version 2.0.0 (Production Release)

---

## 📑 Table of Contents

* [Introduction: The Philosophy of Intent-Driven Computing](#introduction)
* [Chapter 1: Getting Started & Toolchain Setup](#chapter-1-getting-started--toolchain-setup)
* [Chapter 2: Variables, Mutability & Primitive Data Types](#chapter-2-variables-mutability--primitive-data-types)
* [Chapter 3: Collections & Composite Data Structures](#chapter-3-collections--composite-data-structures)
* [Chapter 4: Control Flow, Loops & Exhaustive Pattern Matching](#chapter-4-control-flow-loops--exhaustive-pattern-matching)
* [Chapter 5: Functions, Closures, Scopes & Unpacking](#chapter-5-functions-closures-scopes--unpacking)
* [Chapter 6: Object-Oriented Programming & Zero-Cost Structs](#chapter-6-object-oriented-programming--zero-cost-structs)
* [Chapter 7: Error Handling, Exceptions & Modular Architecture](#chapter-7-error-handling-exceptions--modular-architecture)
* [Chapter 8: The Core Paradigm: Declarative Intent Contracts](#chapter-8-the-core-paradigm-declarative-intent-contracts)
* [Chapter 9: Concurrency with Green Fibers & CSP Channels](#chapter-9-concurrency-with-green-fibers--csp-channels)
* [Chapter 10: Native Deep Learning, Tensors & Reverse-Mode Autograd](#chapter-10-native-deep-learning-tensors--reverse-mode-autograd)
* [Chapter 11: Post-Quantum Computing & State Vector Simulation](#chapter-11-post-quantum-computing--state-vector-simulation)
* [Chapter 12: AetherGraph: In-Memory Property Graphs & Knowledge Traversal](#chapter-12-aethergraph-in-memory-property-graphs--knowledge-traversal)
* [Chapter 13: Distributed Swarms & Conflict-Free Replicated Data Types (CRDTs)](#chapter-13-distributed-swarms--conflict-free-replicated-data-types-crdts)
* [Chapter 14: Erlang-Grade Live Code Swapping & Zero-Downtime Reloading](#chapter-14-erlang-grade-live-code-swapping--zero-downtime-reloading)
* [Chapter 15: Developer Toolchain, Package Management & AI Pair-Programming (MCP)](#chapter-15-developer-toolchain-package-management--ai-pair-programming-mcp)

---

## Introduction: The Philosophy of Intent-Driven Computing

For sixty years, computer programming has been largely **imperative** or **functional**: developers describe *how* a machine should mutate bytes in memory. The compiler is completely blind to what the developer actually intended to achieve. If a developer accidentally writes code that leaks financial balances or violates invariant states, traditional compilers happily compile it.

**AETHER 2.0 changes this paradigm.**

AETHER is a modern systems programming language that unifies:
1. **Declarative Intent Contracts:** Functions and computational blocks are bounded by explicit schemas and invariant proofs (`require` and `ensure`).
2. **Zero-Dependency Native Execution:** Compiles into a single standalone native binary using pure Rust standard library architecture.
3. **Built-in Next-Generation Subsystems:** First-class primitives for Quantum computing simulation, reverse-mode tensor differentiation, knowledge graph traversal, distributed CRDT state consensus, and zero-downtime hot code reloading.

---

## Chapter 1: Getting Started & Toolchain Setup

### 1.1 The One-Line Terminal Install

AETHER requires no installation wizards, no registry modifications, and no administrative privileges.

#### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/devsamikhan/aether/main/install.ps1 | iex
```

#### Linux & macOS (Bash / Zsh)
```bash
curl -fsSL https://raw.githubusercontent.com/devsamikhan/aether/main/install.sh | bash
```

The installer places the single binary into `~/.aether/bin` (or `%USERPROFILE%\.aether\bin` on Windows) and permanently registers it in your user `PATH`.

### 1.2 System Health Inspection: `aether doctor`

Immediately after installation, verify your environment:
```bash
aether doctor
```
Output:
```text
================================================================================
🩺 AETHER TOOLCHAIN DOCTOR (SYSTEM HEALTH DIAGNOSTICS)
================================================================================
Version: v1.1.0 (stable-x86_64)
Binary Location: C:\Users\...\.aether\bin\aether.exe
Diagnostics Checklist:
  [✅ OK] AETHER_HOME Directory
  [✅ OK] Global User PATH Integration
  [✅ OK] Cranelift AOT/JIT Native Compiler
  [✅ OK] Bytecode VM & Memory Engine
  [✅ OK] Python 3 Interop (Zero-Cost FFI)
Status: SYSTEM 100% HEALTHY & READY FOR PRODUCTION 🚀
================================================================================
```

### 1.3 Creating Your First Project: `aether new`

```bash
aether new my_project --template minimal
cd my_project
aether run src/main.ae
```

---

## Chapter 2: Variables, Mutability & Primitive Data Types

AETHER features zero-ceremony dynamic typing with static inference guarantees.

### 2.1 Variables & Mutability
Variables in AETHER are declared with the `let` keyword:

```aether
# Immutable variable binding
let name = "AETHER"
let release_year = 2026

# Mutable variable binding
let mut counter = 0
counter = counter + 1
println("Counter:", counter)  # Outputs: Counter: 1
```

### 2.2 Primitive Types

AETHER provides 5 core primitive scalar types:

| Type | Description | Example |
| :--- | :--- | :--- |
| `Int` | 64-bit signed integer | `let x = 42` |
| `Float` | 64-bit IEEE 754 floating-point | `let pi = 3.1415926535` |
| `String` | UTF-8 encoded string sequence | `let msg = "Hello, World!"` |
| `Bool` | Boolean truth value | `let active = true` |
| `Nil` | Representation of absence of value | `let empty = nil` |

### 2.3 Numeric Arithmetic & Operators

```aether
let a = 20
let b = 6

println("Addition:       ", a + b)   # 26
println("Subtraction:    ", a - b)   # 14
println("Multiplication: ", a * b)   # 120
println("Division:       ", a / b)   # 3.3333333333333335
println("Modulo:         ", a % b)   # 2
println("Exponentiation: ", a ** 2)  # 400
```

### 2.4 String Operations & Formatting

Strings can be concatenated with `+` or formatted cleanly using `println` arguments:

```aether
let user = "Latif"
let role = "Systems Architect"

# String concatenation
let greeting = "Welcome, " + user + " (" + role + ")"
println(greeting)

# Length check
println("Name character count:", len(user)) # 5
```

---

## Chapter 3: Collections & Composite Data Structures

AETHER provides rich, expressive collection data structures right in the language core.

### 3.1 Lists (Dynamic Arrays)

Lists are ordered, mutable sequences of values:

```aether
let fruits = ["Apple", "Banana", "Cherry"]

# Indexing & slicing
println("First fruit:", fruits[0]) # Apple
println("Last fruit: ", fruits[-1]) # Cherry

# Appending & modifying
fruits.push("Date")
fruits[1] = "Blueberry"
println("Updated fruits:", fruits)

# Array length
println("Total items:", len(fruits))
```

### 3.2 Dictionaries (HashMaps)

Dictionaries store associative key-value pairs with $O(1)$ lookups:

```aether
let config = {
    "host": "127.0.0.1",
    "port": 8080,
    "ssl": true,
    "max_connections": 10000
}

println("Connecting to:", config["host"] + ":" + str(config["port"]))

# Adding new keys
config["timeout_ms"] = 5000
```

### 3.3 List & Dictionary Comprehensions

AETHER supports Pythonic list and dictionary comprehensions:

```aether
let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

# List comprehension with filtering
let evens_squared = [x * x for x in numbers if x % 2 == 0]
println("Even squares:", evens_squared) # [4, 16, 36, 64, 100]

# Dictionary comprehension
let squares_map = {x: x * x for x in numbers if x <= 5}
println("Squares map:", squares_map)
```

---

## Chapter 4: Control Flow, Loops & Exhaustive Pattern Matching

### 4.1 Conditionals: `if`, `elif`, `else`

```aether
let score = 88

if score >= 90:
    println("Grade: A+ (Outstanding)")
elif score >= 80:
    println("Grade: A (Excellent)")
elif score >= 70:
    println("Grade: B (Good)")
else:
    println("Grade: C (Needs Improvement)")
```

### 4.2 Loops: `while` & `for..in`

```aether
# While Loop
let mut n = 5
while n > 0:
    println("Countdown:", n)
    n = n - 1

# For-in Loop with Range
for i in range(1, 6):
    if i == 3:
        continue # Skip 3
    println("Step:", i)
```

### 4.3 Exhaustive Pattern Matching (`match`)

AETHER features structural expression pattern matching with jump-table efficiency:

```aether
let status_code = 404

let status_msg = match status_code:
    200 => "OK - Resource retrieved successfully"
    301 => "Moved Permanently"
    400 => "Bad Request"
    404 => "Not Found - The requested endpoint does not exist"
    500 => "Internal Server Error"
    _   => "Unknown HTTP Status"

println("Response:", status_msg)
```

---

## Chapter 5: Functions, Closures, Scopes & Unpacking

### 5.1 Function Declarations (`fn` or `def`)

AETHER accepts both modern curly-brace `fn` and Pythonic indented `def` declarations:

```aether
# Modern C-Style
fn calculate_area(width, height) {
    return width * height
}

# Pythonic Style
def greet(name, title="Engineer"):
    return "Greetings, " + title + " " + name

println(greet("Sami", title="Lead Architect"))
```

### 5.2 Multiple Return Values & Tuple Unpacking

```aether
def get_coordinates():
    return 33.6844, 73.0479

let lat, lon = get_coordinates()
println("Latitude:", lat, "Longitude:", lon)
```

### 5.3 Closures & Higher-Order Functions

```aether
def make_multiplier(factor):
    def multiplier(n):
        return n * factor
    return multiplier

let double = make_multiplier(2)
let triple = make_multiplier(3)

println("Double 15:", double(15)) # 30
println("Triple 15:", triple(15)) # 45
```

---

## Chapter 6: Object-Oriented Programming & Zero-Cost Structs

### 6.1 Classes, Constructors & Encapsulation

```aether
class BankAccount:
    def __init__(self, owner, initial_balance=0.0):
        self.owner = owner
        self.balance = initial_balance

    def deposit(self, amount):
        if amount <= 0:
            println("Error: Deposit amount must be positive.")
            return false
        self.balance = self.balance + amount
        return true

    def withdraw(self, amount):
        if amount > self.balance:
            println("Error: Insufficient funds.")
            return false
        self.balance = self.balance - amount
        return true

let account = BankAccount("Latif", 1000.0)
account.deposit(500.0)
account.withdraw(200.0)
println("Current Balance:", account.balance) # 1300.0
```

### 6.2 Class Inheritance & Polymorphism

```aether
class Animal:
    def __init__(self, name):
        self.name = name

    def speak(self):
        return "..."

class Dog(Animal):
    def speak(self):
        return "Woof! Woof!"

let pet = Dog("Rex")
println(pet.name + " says: " + pet.speak())
```

---

## Chapter 7: Error Handling, Exceptions & Modular Architecture

### 7.1 Try / Except / Finally

```aether
def safe_divide(a, b):
    try:
        if b == 0:
            raise "DivisionByZeroError: Divisor cannot be zero"
        return a / b
    except err:
        println("Caught Exception:", err)
        return nil
    finally:
        println("Cleanup: Execution block terminated.")

let res = safe_divide(100, 0)
```

### 7.2 Modules & Imports

AETHER organizes standard libraries and user code into clean namespaces:

```aether
# Module-level import
import math
println("Square Root of 144:", math.sqrt(144))

# Named symbol import
from aether_graph import Graph
let g = Graph("NetworkTopology")
```

---

## Chapter 8: The Core Paradigm: Declarative Intent Contracts

The defining innovation of AETHER is the **Intent Contract**. An intent bounds execution behavior with formal invariants.

### 8.1 Anatomy of an Intent Contract

```aether
intent transfer_funds(sender_bal, receiver_bal, amount):
    require: sender_bal >= amount
    require: amount > 0
    ensure: result["sender"] == sender_bal - amount
    ensure: result["receiver"] == receiver_bal + amount
    body:
        let new_sender = sender_bal - amount
        let new_receiver = receiver_bal + amount
        return {
            "sender": new_sender,
            "receiver": new_receiver
        }

let settlement = transfer_funds(1000, 200, 350)
println("Settlement Result:", settlement)
# {"sender": 650, "receiver": 550}
```

If any `require` precondition is violated (e.g. `amount <= 0` or insufficient balance), the runtime halts execution before any state mutation can occur.

---

## Chapter 9: Concurrency with Green Fibers & CSP Channels

AETHER executes concurrent routines using **lightweight green fibers** scheduled over an M:N work-stealing thread pool.

### 9.1 Spawning Fibers

```aether
# Spawns a background green fiber
spawn:
    println("Fiber background task starting...")
    sleep(0.05)
    println("Fiber task finished!")

println("Main fiber continuing without blocking...")
sleep(0.1)
```

### 9.2 Communicating Sequential Processes (CSP Channels)

```aether
from concurrency import Channel

let ch = Channel.new()

spawn:
    println("[Worker] Processing telemetry...")
    ch.send({"status": "healthy", "metrics_collected": 1500})

let data = ch.recv()
println("[Coordinator] Received payload from worker:", data)
```

---

## Chapter 10: Native Deep Learning, Tensors & Reverse-Mode Autograd

AETHER incorporates first-class tensor operations and automatic reverse-mode differentiation directly into the runtime without requiring PyTorch, TensorFlow, or Python dependencies.

### 10.1 Tensor Matrix Operations

```aether
from aether_tensor import Tensor

let m1 = Tensor([[1.0, 2.0], [3.0, 4.0]])
let m2 = Tensor([[2.0, 0.0], [1.0, 2.0]])

# High-throughput BLAS-grade matrix multiplication
let prod = m1.matmul(m2)
println("Product Matrix:\n", prod.to_list())
```

### 10.2 Automatic Reverse-Mode Differentiation (`loss.backward()`)

```aether
from aether_autograd import Variable

let x = Variable([[2.0, 3.0]])
let w = Variable([[4.0, 5.0]])

# Dynamic computational graph: f(x, w) = x * w + x
let y = x * w + x
let loss = y.sum()

# Compute exact analytical gradients
loss.backward()

println("Gradient with respect to x (∂Loss/∂x):", x.grad.to_list())
# Expected: [[5.0, 6.0]]
```

---

## Chapter 11: Post-Quantum Computing & State Vector Simulation

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

## Chapter 12: AetherGraph: In-Memory Property Graphs & Knowledge Traversal

AETHER includes a native in-memory graph database engine for artificial intelligence and knowledge graphs.

### 12.1 Building a Knowledge Graph & Dijkstra Pathfinding

```aether
from aether_graph import Graph

let kg = Graph("AI_Reasoning_Engine")

let n_query = kg.add_node("User_Query", {"text": "Compute optimal path"})
let n_rag   = kg.add_node("Knowledge_Base", {"entries": 50000})
let n_model = kg.add_node("Reasoning_LLM", {"params": "70B"})
let n_exec  = kg.add_node("Action_Executor", {"sandbox": true})

# Directed weighted edges
kg.add_edge(n_query, n_rag, "retrieves", 1.2)
kg.add_edge(n_rag, n_model, "contextualizes", 0.8)
kg.add_edge(n_model, n_exec, "dispatches", 1.5)
kg.add_edge(n_query, n_exec, "direct_bypass", 5.0)

# Find optimal shortest path via Dijkstra's algorithm
let route = kg.shortest_path(n_query, n_exec)
println("Optimal Traversal Node Path:", route["path"])
println("Total Cost:", route["cost"]) # 3.5

# Calculate PageRank authority across all nodes
let ranks = kg.pagerank(25, 0.85)
println("PageRank Distribution:", ranks)
```

---

## Chapter 13: Distributed Swarms & Conflict-Free Replicated Data Types (CRDTs)

AETHER solves distributed consensus through mathematical **join-semilattices** (CRDTs), guaranteeing eventual consistency across network partitions without distributed locks.

### 13.1 GCounter (Grow-Only Counter)

```aether
import crdt

let cluster_node_1 = GCounter.new("datacenter_us_east")
let cluster_node_2 = GCounter.new("datacenter_eu_west")

# Asynchronous operations across datacenters
cluster_node_1.increment(25)
cluster_node_2.increment(40)

# Autonomous state merge across network partition
cluster_node_1.merge(cluster_node_2)
println("Converged Cluster State:", cluster_node_1.read()) # 65
```

---

## Chapter 14: Erlang-Grade Live Code Swapping & Zero-Downtime Reloading

AETHER features in-place bytecode swapping. Long-running servers can be updated on-the-fly without dropping active socket connections or erasing in-memory state.

### 14.1 Running in Live-Reload Mode

```bash
aether live server.ae 250
```

When you edit `server.ae` in your editor and save:
1. The compiler re-parses the AST in memory.
2. Invariant safety checks verify the new code.
3. The VM updates function and method pointers atomically.
4. Active heap state, variables, and client sessions are 100% preserved!

---

## Chapter 15: Developer Toolchain, Package Management & AI Pair-Programming (MCP)

### 15.1 Package Management with AetherPM

```bash
# Add a dependency package
aether add neural_vision@1.0.0

# Remove a dependency
aether remove neural_vision

# Package current library for distribution
aether publish
```

### 15.2 In-Browser Web Playground

Launch a local WebAssembly-powered browser sandbox with live code execution and SVG visualizers:
```bash
aether playground 8080
```
Opens `http://localhost:8080` in your default browser.

### 15.3 Model Context Protocol (MCP) Server for AI Assistants

Connect AETHER directly to Cursor, Claude Desktop, Antigravity, or Zed:
```bash
aether mcp
```
Or export the optimized system prompt directly to ChatGPT or Claude:
```bash
aether ai
```

### 15.4 Interactive Tour of AETHER

Step-by-step 7-lesson developer learning course right in your terminal:
```bash
aether tour list
aether tour 1
```

---

## Summary & Next Steps

You now hold the complete mastery of **AETHER 2.0**: from basic variables to quantum simulations, tensor autograd, distributed swarms, and live zero-downtime microservices.

Welcome to the future of declarative intent-driven computing! 🚀
