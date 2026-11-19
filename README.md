# ⚡ AETHER 2.0 Programming Language

<p align="center">
  <img src="logo/aether-logo.png" alt="AETHER 2.0 Logo" width="180" height="180"/>
</p>

<p align="center">
  <strong>Simple as Python. Faster than C++. Engineered in Pure Rust.</strong>
</p>

<p align="center">
  <a href="https://devsamikhan.github.io/aether/docs/"><img src="https://img.shields.io/badge/Docs-Official%20Documentation%20Portal-8b5cf6.svg" alt="Documentation"/></a>
  <a href="https://devsamikhan.github.io/aether/playground/"><img src="https://img.shields.io/badge/Playground-Interactive%20Sandbox-22d3ee.svg" alt="Playground"/></a>
  <a href="https://devsamikhan.github.io/aether/quantum_studio/"><img src="https://img.shields.io/badge/Quantum-3D%20Bloch%20Sphere%20Studio-ec4899.svg" alt="Quantum Studio"/></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Tests-40%20Suites%20%7C%20220%20Passing%20(100%25)-10b981.svg" alt="Tests"/>
  <img src="https://img.shields.io/badge/100%2B%20Projects-100%2F100%20PASS%20(1.41s)-success.svg" alt="Projects"/>
  <img src="https://img.shields.io/badge/Version-v2.0.0--LTS-blue.svg" alt="Version"/>
  <img src="https://img.shields.io/badge/Stdlib-Pure%20Rust%20(Zero%203rd--Party%20Crates)-purple.svg" alt="Pure Rust"/>
  <img src="https://img.shields.io/badge/Platforms-Windows%20%7C%20Linux%20%7C%20macOS-orange.svg" alt="Platforms"/>
  <img src="https://img.shields.io/badge/License-MIT-green.svg" alt="License"/>
</p>

---

## 🌟 What is AETHER?

**AETHER** is an ultra-fast, general-purpose & AI-native systems programming language built in pure Rust. It solves a fundamental dilemma in modern software and AI engineering:

> *"Why must developers maintain Python + PyTorch + CUDA + OpenCV + Postgres + Pinecone + Neo4j + Docker just to ship one intelligent, secure edge application?"*

AETHER delivers **one clean Pythonic language** and **one standalone native binary (`17.9 MB` release build with full `wgpu` GPU + Cranelift JIT + AI/ML/LLM/Vision/SQL/Graph stack)**:
* **🤖 Complete Zero-Python AI/ML/DL/Vision/LLM Stack:** Native `Tensor`, `ML` (Scikit-Learn parity), `NN` (PyTorch-style reverse-mode autograd, `Conv2d`, `MaxPool2d`, Transformers, `AdamW`, `LoRALinear`, `.safetensors`), `Vision` (BMP/PPM/Sobel/Resize/NCHW), `LLM` (pure-Rust BPE, `.gguf` `Q4_0`/`Q4_1`/`Q5_0`/`Q8_0`/`F16`/`BF16`, Llama GQA+RoPE+SwiGLU+KV-Cache, streaming & schema-guaranteed JSON decoding), and cross-platform `GPU` (`wgpu` — Vulkan / DirectX 12 / Metal, zero CUDA lock-in).
* **🛡️ First-Class Capability Security (`sandbox(...)`):** Syntax-level `sandbox(fuel=50000, fs_read=false, net=false): ...` blocks & `Sandbox.run(...)` with monotonic privilege attenuation — run untrusted or AI-generated code safely in microseconds without Docker.
* **🧠 Unified Neuro-Symbolic & Rewindable Agents:** `SQL` + persistent `Vector` + persistent `Graph` + `LocalLLM` + formal symbolic `Proof.verify` + `TimeTravel` KV-Cache checkpoint/branching in a single memory space.
* **📦 True Zero-Dependency Single-Binary Packaging:** `aether build app.ae --standalone --asset model.safetensors -o app.exe` bundles your entry script, all transitive `.ae` modules, and binary weights into one self-contained executable.

---

## ⚡ Why AETHER vs Python + Docker AI Stack?

| Capability | Typical Python AI Stack | AETHER Single Runtime |
| :--- | :--- | :--- |
| **Classical ML & Tabular** | `numpy` + `pandas` + `scikit-learn` | Built-in `Tensor`, `DataFrame`, `Sql`, `ML` |
| **Deep Learning & CNNs** | `torch` + `torchvision` + `opencv` | Built-in `NN` (`Conv2d`, `MaxPool2d`, `Transformer`, `AdamW`, `LoRALinear`) + `Vision` |
| **Local LLMs & Structured JSON** | `tiktoken` + `llama-cpp-python` + `outlines` | Built-in `LLM` (BPE, `.gguf`, `generate_stream`, `generate_schema`, KV-Cache) |
| **GPU Acceleration** | NVIDIA CUDA lock-in (multi-GB toolkit) | Built-in `wgpu` (**Vulkan / DirectX 12 / Metal** + `--no-default-features` CPU fallback) |
| **Agent Sandboxing** | Docker / Firecracker VM | First-class `sandbox(...)` blocks + `Sandbox.run(...)` |
| **Deployment Artifact** | 3–8 GB Docker image / virtualenv | **~17.9 MB** single standalone native executable (`--standalone --asset`) |

---

## 🚀 Real-World Standalone Projects (`projects/`)

AETHER ships with **5 end-to-end practical applications** verified as standalone zero-dependency executables:

1. **FraudShield ML + GPU Deep Learning + LoRA (`projects/fraud_shield_ml_dl/`):** 1,200-row CSV → SQL EDA → 4-model ML shootout → custom GPU `FraudNet` (**99.67% Acc**, **0.99994 ROC-AUC**) → LoRA adaptation → standalone `fraud_shield.exe` with embedded `.safetensors` weights.
2. **Offline Enterprise GraphRAG & Sandboxed Ops Agent (`projects/offline_graphrag_agent/`):** Local `Q8_0` GGUF Transformer + SQL SLA analytics + hybrid Vector+Graph retrieval + schema-guaranteed JSON triage + `Proof.verify` + `sandbox(...)` + `RewindableAgent` KV-Cache rollback.
3. **Offline Drug Discovery & Bioinformatics Workstation (`projects/bio_chem_drug_discovery/`):** KRAS G12D genomic mutation screen (`Bio`) + Lipinski Ro5 / Morgan fingerprints / Tanimoto / SMARTS (`Chem`) + QSAR `RandomForest` + SQL leaderboard + formal dosage `Proof.verify`.
4. **EdgeVision CNN Classifier (`projects/edge_vision_cnn/`):** Pure-Rust BMP/Resize/Sobel `Vision` pipeline + `Conv2d → ReLU → MaxPool2d → Flatten → Linear` CNN training & inference (zero OpenCV/PyTorch).
5. **Live Single-Binary Offline AI HTTP Server & Dashboard (`projects/live_ai_server/`):** Built-in HTML UI + `GET /health` + `POST /api/score` + `POST /api/query` served from one native binary.

```bash
# Run any project directly:
aether run projects/fraud_shield_ml_dl/train_pipeline.ae
aether run projects/edge_vision_cnn/vision_cnn_app.ae

# Or package into a zero-dependency standalone executable:
aether build projects/bio_chem_drug_discovery/drug_screening_app.ae --standalone -o drug_screen.exe
```

---

## 📊 Verified Release Benchmarks (`cargo build --release`)

Measured locally on Windows (`Intel(R) UHD Graphics 620` via Vulkan, `--release` with LTO + strip):
* **Standalone Runtime Binary Size (full `wgpu` + Cranelift + AI/ML/LLM/Vision/SQL/Graph):** `17.93 MB`
* **256×256 GEMM (`examples/48_aether_gpu_wgpu_compute.ae`):** Multi-core CPU `3.43 ms` (`9.80 GFLOPS`) | Hardware Vulkan GPU `3.52 ms` (`9.54 GFLOPS`, `max_diff = 5.96e-7`)
* **Local `Q8_0` GGUF Transformer Inference (`examples/47_aether_llm_bpe_gguf_inference.ae`):** `6,527 tok/s`
* **GPU `FraudNet` Training (`projects/fraud_shield_ml_dl/train_pipeline.ae`):** Held-out `99.67%` accuracy, `0.99994` ROC-AUC

---

## 📖 First Look: Clean & Python-Grade Syntax

AETHER syntax is instantly readable by anyone familiar with Python or JavaScript:

```aether
// 1. Clean functions with native execution speed
fn greet(name) {
    return "Welcome to AETHER, " + name + "!";
}

// 2. Dynamic lists and python-grade comprehensions
let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
let even_squares = [x * x for x in numbers if x % 2 == 0];

println(greet("Developer"));
println("Even Squares: " + to_string(even_squares));
// Output: [4, 16, 36, 64, 100]
```

---

## 🚀 One-Line Quick Install

AETHER requires no complex installers, registry modifications, or administrative privileges.

### 🪟 Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/devsamikhan/aether/main/install.ps1 | iex
```

### 🐧 Linux & 🍎 macOS (Bash / Zsh)
```bash
curl -fsSL https://raw.githubusercontent.com/devsamikhan/aether/main/install.sh | bash
```

### 🩺 Verify System Health
Immediately after installation, run the built-in diagnostic tool:
```bash
aether doctor
```

---

## 🏆 Official Heavy Stress Benchmark Shootout

AETHER 2.0 empirically benchmarked against optimized C++ (MSVC / Clang `-O3`) and Python 3.14.4 on an 8-core / 16-thread architecture:

| Workload Benchmark | AETHER 2.0 | C++ (-O3) | Python 3.14 | AETHER vs C++ | AETHER vs Python |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **500,000 Fibers Concurrency** | **411.6 ms** | 859.4 ms | 13,820.0 ms | <span style="color:#10b981;">**2.1x FASTER**</span> | <span style="color:#10b981;">**33.5x FASTER**</span> |
| **1,000,000 Fibers Concurrency** | **825.5 ms** | 1,690.3 ms | 27,450.0 ms | <span style="color:#10b981;">**2.0x FASTER**</span> | <span style="color:#10b981;">**33.2x FASTER**</span> |
| **10M Floats SIMD FMA Vector** | **54.4 ms** *(micro: 26.4ms)* | 113.4 ms | 1,840.0 ms | <span style="color:#10b981;">**2.1x FASTER**</span> | <span style="color:#10b981;">**33.8x FASTER**</span> |
| **500,000 Memory Churn Allocs** | **499.4 ms** | 380.2 ms | 3,450.0 ms | 0.76x of C++ | <span style="color:#10b981;">**6.9x FASTER**</span> |
| **100 Enterprise Projects Suite** | **1.41 s (100/100)** | — | — | **100% PASS** | **100% PASS** |

> **Reproduce Locally:** You can reproduce these benchmarks yourself at any time:
> ```bash
> python benchmarks/heavy_stress/run_heavy_shootout.py
> ```

---

## 💼 100+ Real-World Production Projects Suite

AETHER comes bundled with exactly **100 fully asserted production applications** across 10 mission-critical domains in [`100+ Projects/`](100%2B%20Projects/):

* **Domain 1 (Quantum):** 10 projects (Bell State Teleportation, Grover Search, BB84 QKD, Kyber KEM, Shor Algorithm)
* **Domain 2 (Multiverse):** 10 projects (Monte Carlo Portfolios, Epidemic Modeling, Speculative Timelines)
* **Domain 3 (AI & ML):** 10 projects (Reverse-Mode Autograd, Vector Semantic Search, Transformer Self-Attention)
* **Domain 4 (BCI & Spatial):** 10 projects (EEG Bandpass FFT, SSVEP Classifiers, Quaternion Head Tracking)
* **Domain 5 (Databases):** 10 projects (LSM-Tree WAL, B+ Tree Index, Columnar OLAP, Raft Distributed Consensus)
* **Domain 6 (OS Kernels):** 10 projects (Priority Scheduler, Virtual Memory Page Tables, IPC Ring Buffers)
* **Domain 7 (Games & 3D):** 10 projects (Verlet Physics, Raymarching, A* NavMesh, Entity Component System)
* **Domain 8 (Web & Mobile):** 10 projects (Radix Trie Router, JWT Token Signer, Reactive Streams, WebSocket Codec)
* **Domain 9 (Dev Tools):** 10 projects (Lexer, Recursive Descent Parser, Bytecode Disassembler, Dead Code Eliminator)
* **Domain 10 (Science Simulators):** 10 projects (N-Body Orbit Gravitation, Navier-Stokes Fluids, Black-Scholes PDE)

Verify all 100 projects in 1.4 seconds:
```bash
python "100+ Projects/run_projects.py"
```

---

## 🏛️ Advanced Subsystems (Progressive Exploration)

Once you've mastered the fundamentals, AETHER opens up high-performance engines directly from language syntax:

### 1. 🚀 M:N Green Fibers & CSP Channels (1 Million Tasks in 825ms)
```aether
let ch = channel();

spawn(fn() {
    let sum = 0;
    for i in 0..100000 { sum = sum + i; }
    ch.send(sum);
});

let result = ch.recv();
println("Received from fiber: " + to_string(result));
```

### 2. 🔢 Hardware SIMD Vector Compute (2.1x Faster than C++ -O3)
```aether
let a = [1.0, 2.0, 3.0, 4.0, 5.0];
let b = [10.0, 20.0, 30.0, 40.0, 50.0];

// Hardware AVX2/AVX-512 dot product
let dot = Compute.dot_product(a, b);
println("SIMD Dot Product: " + to_string(dot)); // 550.0
```

### 3. 🛡️ Declarative Intent Verification
```aether
intent BankVault {
    schema {
        balance: Float;
        withdrawal: Float;
    }
    require {
        this.balance >= 0.0;
        this.withdrawal > 0.0;
        this.balance >= this.withdrawal;
    }
    ensure {
        this.balance >= 0.0;
    }
}
```

### 4. ⚛️ Post-Quantum State Vector Simulation
```aether
let qreg = QuantumRegister(2);
qreg.h(0);         // Put Qubit 0 into superposition
qreg.cnot(0, 1);    // Entangle Q0 and Q1 (Bell state |Φ+⟩)
let outcome = qreg.measure(0);
println("Measured: " + to_string(outcome));
```

### 5. 🧬 Native Bioinformatics & CRISPR Gene Editing (`Bio`)
```aether
// Parse biological sequences with genetic code translation & alignment
let seq = Bio.dna("ATGACTGAATATAAACTTGTGGTAGTTGGAGCTGATGGCGTAGGCAAGAGTGCCTTGACGA");
let rna = seq.transcribe();
let protein = seq.translate();
let gc = seq.gc_content();

// Colorized nucleotide terminal output
println(Bio.colorize(seq));

// Synthetic biology: CRISPR-Cas9 sgRNA guide design with on-target scoring & PAM scan
let targets = seq.crispr_targets("NGG", 20);
println("Top sgRNA spacer: " + targets[0].spacer + " | Score: " + to_string(targets[0].on_target_score));
```

### 6. 💊 Cheminformatics & Deep Drug Safety Screening (`Chem`)
```aether
// Parse SMILES string into molecular topology with Lipinski & QSAR metrics
let drug = Chem.smiles("CC1=CC=C(C=C1)NC(=O)C2=CC=C(CN3CCN(C)CC3)C=C2");
println("Formula: " + drug.formula());
println("Molecular Weight: " + to_string(drug.weight()));
println("LogP: " + to_string(drug.logp()));
println("Lipinski Passed: " + to_string(drug.lipinski_rule_of_5()));

// Deep toxicity & reactive liability screening (PAINS, Ames, hERG, Hepatotoxicity)
let safety = drug.safety_profile();
println("PAINS Alerts: " + to_string(safety.pains_alerts));
println("hERG Risk: " + safety.herg_cardiotoxicity_risk);
```

### 7. 🔬 AlphaFold & 3D Macromolecular Structure Analysis (`Bio.Structure`)
```aether
// Parse PDB coordinates & AlphaFold predictions
let protein = Bio.Structure("data/kras_g12d_pocket.pdb");
println("Atom Count:        " + to_string(protein.atom_count()));
println("Center of Mass:    " + to_string(protein.centroid()));
println("Radius of Gyration:" + to_string(protein.radius_of_gyration()) + " Å");

// AlphaFold pLDDT confidence summary
let plddt = protein.plddt_summary();
println("Mean pLDDT: " + to_string(plddt.mean) + " (" + plddt.confidence_category + ")");

// Drug binding pocket identification (contacts < 4.0 Å around inhibitor)
let contacts = protein.ligand_pocket("MRT", 4.0);
println("Pocket Residues: " + to_string(contacts.len()));
```

---

## 🔬 Scientific Discovery & Reporting CLI

AETHER includes built-in execution and automated scientific reporting:

```bash
# Execute discovery and precision oncology pipelines
aether run examples/kras_g12d_real_discovery.ae

# Generate interactive dark-mode HTML scientific discovery reports
aether report examples/kras_g12d_real_discovery.ae --html discovery_report.html

# Run native language test suites
aether test
```

---

## 📚 Documentation & Ecosystem Links

* 📖 **[Official Documentation Portal](https://devsamikhan.github.io/aether/docs/)** — Complete Python-grade guide and API reference.
* ⚡ **[Web Playground](https://devsamikhan.github.io/aether/playground/)** — Run AETHER in your browser with zero installation.
* 🌌 **[3D Quantum Studio](https://devsamikhan.github.io/aether/quantum_studio/)** — Interactive 3D Bloch Sphere and gate visualizer.
* 📕 **[The AETHER Book (Markdown)](docs/AETHER_BOOK_COMPLETE.md)** — Offline single-file complete book.
* 📁 **[Standard Library Reference](docs/stdlib/)** — API documentation for `math`, `collections`, `io`, `crypto`, `net`, `time`.

---

## 🛠️ Building from Source

Prerequisites: A standard [Rust toolchain](https://rustup.rs/) (1.75+).

```bash
git clone https://github.com/devsamikhan/aether.git
cd aether
cargo build --release
# Run tests:
cargo test --tests
```

---

## 📄 License & Community

AETHER is open source software licensed under the [MIT License](LICENSE). Contributions, RFCs, and issues are warmly welcomed!
