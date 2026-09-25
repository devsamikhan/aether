# AETHER 2.0 Language Reference: Module System

## 1. Modular Architecture
AETHER provides a clean, file-based module system that enables granular code reuse without namespace pollution:

```aether
# Relative module import
use "crypto/hash.ae";

# Call imported functions directly
let digest = sha256("payload");
```

---

## 2. Namespace Resolution
Modules can be assigned local aliases to avoid identifier collisions:

```aether
use "math/matrix.ae" as Matrix;

let m = Matrix.identity(4);
```

---

## 3. Circular Dependency Resolution
The AETHER dependency graph builder performs static topological sorting with cycle detection (Tarjan's algorithm), guaranteeing deterministic module initialization order.