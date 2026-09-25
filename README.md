# ⚡ AETHER 2.0 Programming Language

<p align="center">
  <img src="logo/aether-logo.svg" alt="AETHER Logo" width="180" height="180"/>
</p>

<p align="center">
  <strong>The Unified Intent-Driven, Post-Quantum, Tensor & Live-Swapping Systems Language</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Build-Passing%20(100%25)-brightgreen.svg" alt="Build Status"/>
  <img src="https://img.shields.io/badge/Tests-38%20Suites%20%7C%20213%20Passing-success.svg" alt="Tests"/>
  <img src="https://img.shields.io/badge/Version-2.0.0--stable-blue.svg" alt="Version"/>
  <img src="https://img.shields.io/badge/Architecture-Zero%20Third--Party%20Crates-purple.svg" alt="Zero Crates"/>
  <img src="https://img.shields.io/badge/Platforms-Windows%20%7C%20Linux%20%7C%20macOS-orange.svg" alt="Platforms"/>
  <img src="https://img.shields.io/badge/License-MIT-green.svg" alt="License"/>
</p>

---

## 🌌 What is AETHER?

**AETHER** is a next-generation systems programming language designed for the convergence of **declarative intent verification, quantum computing simulation, native deep learning tensors, distributed CRDT swarms, knowledge graph traversal, and zero-downtime hot reloading**.

Unlike traditional compiled languages that compile code into opaque, immutable binary state, AETHER unifies execution predictability with dynamic runtime evolution:
* **Zero Runtime Dependencies:** Compiles down to a single standalone native binary. No Python, Node.js, or external runtime required.
* **Pure Standard Library:** 100% crafted in pure Rust standard library with zero external third-party crates.
* **Deterministic Convergence:** Distributed CRDTs guarantee mathematical join-semilattice consensus across asynchronous network partitions.
* **BEAM-Grade Reliability:** Live code hot-reloading allows state migration and method patching with zero process downtime.

---

## 🚀 One-Line Quick Install

### 🪟 Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/devsamikhan/aether/main/install.ps1 | iex
```

### 🐧 Linux & 🍎 macOS (Bash / Zsh)
```bash
curl -fsSL https://raw.githubusercontent.com/devsamikhan/aether/main/install.sh | bash
```

Once installed, verify your system immediately:
```bash
aether doctor
```

---

## 🏛️ Architectural Pillars

```
+-----------------------------------------------------------------------------------+
|                              AETHER 2.0 UNIFIED RUNTIME                            |
+---------------------+---------------------+-------------------+-------------------+
|  ⚡ AetherGraph      |  🧠 AetherTensor    |  ⚛️ Post-Quantum   |  🔄 AetherLive    |
|  Native Graph DB    |  Autograd & Backprop|  Matrix Simulator |  Zero-Downtime    |
|  Dijkstra/PageRank  |  Conv2D / Adam / SGD|  QFT & Grover     |  Hot Code Swap    |
+---------------------+---------------------+-------------------+-------------------+
|                      🐝 Swarm Intelligence & CRDT State Engine                    |
|                      GCounter | PNCounter | GSet | ORSet | LWWRegister             |
+-----------------------------------------------------------------------------------+
|                       ⚙️ AetherVM Instruction Dispatch & JIT                       |
|                       Stack Machine + Cranelift JIT + WebAssembly                 |
+-----------------------------------------------------------------------------------+
```

### 1. 🧠 Native Deep Learning & Autograd (`AetherTensor`)
First-class tensor primitives with automatic reverse-mode differentiation:
```aether
import tensor;

// Define learnable tensors with gradient tracking
let w = Tensor.randn([2, 2], requires_grad=True);
let x = Tensor.from_vec([[1.0, 2.0], [3.0, 4.0]]);
let b = Tensor.zeros([2, 2]);

// Forward pass
let y = w.matmul(x) + b;
let loss = y.sum();

// Reverse-mode automatic differentiation
loss.backward();
print("Weight Gradients: " + str(w.grad()));
```

### 2. ⚛️ Post-Quantum Algorithmic Simulation
State vector matrix transformations with full Bloch sphere and phase support:
```aether
import quantum;

let sim = QuantumSimulator.new(2);
sim.hadamard(0);      // Create superposition on Qubit 0
sim.cnot(0, 1);        // Entangle Qubit 0 and Qubit 1 (Bell State |Φ+⟩)

let measurement = sim.measure_all();
print("Collapsed State: " + str(measurement));
```

### 3. 🕸️ AetherGraph: Native Knowledge Traversal Engine
In-memory property graph with native shortest-path and centrality metrics:
```aether
import aether_graph;

let g = Graph.new();
g.add_node("Agent_Alpha", {"role": "Coordinator"});
g.add_node("Agent_Beta", {"role": "Worker"});
g.add_edge("Agent_Alpha", "Agent_Beta", 1.5, "manages");

let path = g.dijkstra("Agent_Alpha", "Agent_Beta");
print("Optimal Traversal: " + str(path));
```

### 4. 🔄 AetherLive: Zero-Downtime Hot Code Reloading
Update running production services on-the-fly without dropping active connections:
```bash
# Launch a long-running server in live-reload mode
aether live server.ae --port 9090

# Edit server.ae in your editor and save -> AETHER patches the memory state instantly!
```

### 5. 🐝 Distributed Swarm Intelligence & CRDTs
Conflict-Free Replicated Data Types ensuring eventual consistency across distributed nodes:
* `GCounter` / `PNCounter`: Commutative and associative counters.
* `GSet` / `ORSet`: Observed-Remove sets with tombstone resolution.
* `LWWRegister`: Last-Write-Wins timestamps with deterministic tie-breaking.

---

## 🛠️ CLI Toolchain & Developer Experience

AETHER provides an integrated developer toolchain right out of the box:

| Command | Description |
| :--- | :--- |
| `aether doctor` | Diagnostic scorecard checking compiler, PATH, JIT, and sub-engines. |
| `aether bench` | Executes 5 production hardware benchmarks (Tensor, CRDT, Quantum, Graph, VM). |
| `aether new <name> [--template <t>]` | Scaffolds a new project (`ai`, `web`, `fintech`, `minimal`). |
| `aether live <file.ae>` | Runs program in zero-downtime hot-reloading daemon mode. |
| `aether run <file.ae>` | Executes source file via AetherVM bytecode interpreter. |
| `aether test` | Runs all integrated unit and functional tests. |
| `aether update` | Performs in-place self-update to the latest stable release. |
| `aether repl` | Launches interactive Read-Eval-Print Loop. |

---

## 🧪 Comprehensive Verification & Test Suite

AETHER is thoroughly verified with **38 test suites** containing **213 test cases** passing with **100% accuracy**:

* ✅ `aether_tensor_tests` — Tensor math, matmul, autograd backward pass
* ✅ `aether_graph_tests` — Property graph, Dijkstra, PageRank, cycle detection
* ✅ `aether_hotreload_tests` — Live code swap, state migration, AST diffing
* ✅ `aether_quantum_tests` — Matrix simulator, Hadamard, CNOT, Shor's, Grover's
* ✅ `aether_production_stress_tests` — Memory safety, concurrency, allocations
* ✅ `aether_websocket_tests` — RFC 6455 full-duplex socket handshakes
* ✅ `crdt_tests` — Join-semilattice commutativity, associativity, and idempotency
* ✅ `fiber_concurrency_tests` — Work-stealing M:N green fibers and channels
* ✅ `cranelift_tests` — Native JIT machine code generation

Run all tests yourself:
```bash
cargo test
```

---

## 📂 Project Structure

```text
├── src/
│   ├── ast.rs               # Abstract Syntax Tree definitions
│   ├── lexer.rs             # 260+ token lexical analyzer
│   ├── parser.rs            # Recursive-descent Pratt parser
│   ├── compiler.rs          # AST-to-Bytecode compiler
│   ├── toolchain.rs         # Doctor, update, scaffold & system installer
│   ├── vm/
│   │   ├── mod.rs           # Core Virtual Machine execution loop
│   │   ├── tensor.rs        # Native Tensor & Autograd engine
│   │   ├── quantum.rs       # Unitary Matrix Quantum simulator
│   │   ├── graph.rs         # AetherGraph property traversal engine
│   │   ├── hotreload.rs     # AetherLive zero-downtime code swapper
│   │   ├── crdt.rs          # Join-semilattice distributed structures
│   │   └── websocket.rs     # RFC 6455 full-duplex socket protocol
├── libraries/               # Standard library modules (.ae)
├── examples/                # 43+ Production runnable examples
├── tests/                   # 38 Integration and stress test suites
├── install.ps1              # Windows one-line installer
├── install.sh               # Linux & macOS one-line installer
└── .github/workflows/       # Multi-platform CI/CD release automation
```

---

## 📄 License

AETHER is licensed under the [MIT License](LICENSE).
Open source, post-quantum ready, and built for the future of computation.
