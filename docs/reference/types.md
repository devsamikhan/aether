# AETHER 2.0 Language Reference: Types & Type System

## Overview
AETHER features a gradual, intent-verified type system combining dynamic ergonomics with static verification guarantees. All types have well-defined runtime semantics and zero-cost representations in memory.

---

## 1. Primitive Types

### 1.1 `Int` (64-bit Signed Integer)
- Memory representation: `i64` (two's complement).
- Value range: \(-2^{63}\) to \(2^{63} - 1\).
- Literal forms: `42`, `-10`, `0`, `0xFF` (hexadecimal), `0b1010` (binary).

```aether
let count: Int = 1000;
let hex_mask = 0xFF;
```

### 1.2 `Float` (64-bit Double Precision IEEE 754)
- Memory representation: `f64`.
- Precision: 53 bits significand (~15-17 decimal digits).
- Special values: `+inf`, `-inf`, `nan`.

```aether
let pi: Float = 3.141592653589793;
let scientific = 1.5e-4;
```

### 1.3 `Bool` (Boolean)
- Values: `true`, `false`.
- Memory representation: `u8` (1 byte).
- Supports logical operators: `and`, `or`, `not`.

```aether
let is_valid: Bool = true;
let flag = (count > 0) and not is_valid;
```

### 1.4 `String` (UTF-8 Immutable Byte Sequence)
- Encoding: Guaranteed valid UTF-8.
- Memory: Contiguous heap buffer with length and capacity (Rust `String`).
- Indexing: Byte-level and codepoint-level indexing.
- Escape sequences: `\n`, `\t`, `\r`, `\\`, `\"`.

```aether
let greeting: String = "Hello, AETHER 2.0!";
let path = "C:\\Projects\\aether";
```

### 1.5 `Nil` (Unit / Null Type)
- Represents the intentional absence of value.
- Singleton value: `nil`.
- Equivalent to Python's `None` or Rust's `None`.

```aether
let unassigned = nil;
```

---

## 2. Compound & Composite Types

### 2.1 `List` / `Array` (Dynamic Sequence)
- Memory layout: Contiguous memory chunk of 64-bit pointers/tagged values.
- Operations: `push`, `pop`, `len`, indexing `[i]`, slicing `[a..b]`.
- Time complexity: \(O(1)\) amortized append, \(O(1)\) random access.

```aether
let numbers = [10, 20, 30, 40];
numbers.push(50);
let first = numbers[0]; # 10
```

### 2.2 `Map` / `Dictionary` (Associative Hash Table)
- High-performance Robin Hood or Quadratic Probing hash table.
- Keys must be hashable (`Int`, `String`, `Bool`).
- Amortized \(O(1)\) insert, lookup, and deletion.

```aether
let table = {
    "version": "2.0.0",
    "threads": 8,
    "fast_mode": true
};
let v = table["version"];
```

### 2.3 `Tuple` (Fixed-Length Heterogeneous Container)
- Fixed length, immutable product type.
- Unpacking support: `let (x, y, z) = point;`

```aether
let coord = (10, 20, "label");
let (x, y, tag) = coord;
```

---

## 3. Advanced Engine Types

### 3.1 `Qubit` & `QuantumRegister`
- Represents quantum states on the complex Hilbert space \(\mathbb{C}^{2^n}\).
- Managed by the in-memory double-precision state vector simulator.

```aether
let qreg = QuantumRegister(2);
qreg.h(0);
qreg.cnot(0, 1);
let result = qreg.measure(0);
```

### 3.2 `Channel` (CSP Inter-Fiber Pipe)
- Unbuffered or buffered channels for non-blocking message passing between fibers.

```aether
let ch = channel();
ch.send("payload");
let val = ch.recv();
```