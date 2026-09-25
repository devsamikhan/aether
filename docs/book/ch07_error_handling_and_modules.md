# Chapter 7: Error Handling, Exceptions & Modular Architecture

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
