# Getting Started with AETHER 2.0

## Introduction
Welcome to AETHER 2.0, the post-quantum, high-performance systems language with declarative intent contracts and Python-grade syntax.

---

## Step 1: Install AETHER
Install using the one-line installer or build from source:

```bash
# Windows
irm https://raw.githubusercontent.com/devsamikhan/aether/main/install.ps1 | iex

# Linux/macOS
curl -fsSL https://raw.githubusercontent.com/devsamikhan/aether/main/install.sh | bash
```

Verify your installation:
```bash
aether doctor
```

---

## Step 2: Interactive REPL
Launch the interactive shell:
```bash
aether repl
```
Try evaluating basic expressions:
```aether
>>> let x = 100 * 2.5;
>>> println(x);
250.0
>>> let reg = QuantumRegister(2);
>>> reg.h(0);
>>> reg.measure(0);
```

---

## Step 3: Run Your First Script
Create a file named `app.ae`:
```aether
let items = ["Quantum", "Fibers", "SIMD", "CRDT"];
for (idx, item) in items {
    println("Module " + to_string(idx) + ": " + item);
}
```

Execute:
```bash
aether run app.ae
```