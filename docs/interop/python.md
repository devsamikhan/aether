# Python Interoperability Guide

Python Interoperability: AETHER can interface with the Python ecosystem via FFI, allowing access to libraries like NumPy and Pandas. This capability is powered by a GIL-safe dynamic bridge using the `pyo3` library.

---

## Prerequisites

To use Python interoperability features, the following requirements must be met:
1. Python 3.8 or higher must be installed on your system.
2. The Python dynamic library must be accessible in your system's path.
3. Required Python modules (e.g., `numpy`, `pandas`) must be installed in your system's active Python environment.

---

## Installation & Configuration

AETHER automatically attempts to locate and initialize the system Python interpreter at runtime. Under the hood:
- The compiler compiles with PyO3 targeting the stable ABI (using feature `abi3-py38`).
- This guarantees compatibility with Python versions newer than the build-time environment.
- If no Python runtime is installed, calls to Python interop features will fail gracefully and raise a compiler diagnostic error.

---

## Usage Examples

### 1. Function-Based Interop

The AETHER standard library includes three built-in interop functions:
- `py_import(module_name: String)`: Imports a Python module and caches it for future calls.
- `py_eval(expression: String)`: Evaluates a Python expression and returns the resulting value.
- `py_call(function_name: String, arguments: List)`: Invokes a specific Python function with the given arguments.

```aether
intent PythonInteropDemo {
    fn main() {
        // Import module
        py_import("math");
        
        // Evaluate expression
        let val = py_eval("math.sqrt(25)");
        println(val); // Output: 5.0
        
        // Direct function call
        let result = py_call("math.pow", [2, 10]);
        println(result); // Output: 1024.0
    }
}
```

### 2. Inline Block-Based Interop

For more complex scripts, AETHER supports block-based inline Python execution using the `python { ... } => target;` syntax. This compiles the block into a callback execution context and assigns the returned value to the target variable.

```aether
intent NumPyDemo {
    fn main() {
        python {
            import numpy as np
            arr = np.array([10, 20, 30, 40, 50])
            return arr.mean()
        } => result;
        
        println(result); // Output: 30.0
    }
}
```

---

## Type Mapping Table

Data types are marshaled between AETHER and Python according to the following mapping:

| AETHER Type | Python Type | Direction |
|-------------|-------------|-----------|
| `Integer(i64)` | `int` | Bi-directional |
| `Float(f64)` | `float` | Bi-directional |
| `String(String)` | `str` | Bi-directional |
| `Boolean(bool)` | `bool` | Bi-directional |
| `List(Vec<RuntimeValue>)` | `list` | Bi-directional |
| `Null` | `None` | Bi-directional |

*Note: Nested structures (like lists containing lists or basic primitives) are recursively converted.*

---

## Error Handling Guide

AETHER intercepts Python runtime exceptions and redirect streams (stdout and stderr) to prevent silent execution failures. Exceptions are mapped to AETHER compiler diagnostic codes:

- **E0400 (PythonInteropError)**: Raised during execution of Python blocks, evaluations, or function calls.
- **E0401 (PythonImportFailed)**: Raised if a requested module could not be found or loaded.

```
error[E0401]: Failed to import Python module 'non_existent': No module named 'non_existent'
  --> src/main.aether:3:9
   |
 3 |         py_import("non_existent");
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^
```

---

## Performance Considerations

1. **GIL Synchronization**: Every call across the FFI boundary acquires the Global Interpreter Lock (GIL). For performance-critical loops, group Python operations into a single inline block rather than calling individual functions repeatedly.
2. **Module Caching**: Imported modules are cached in a thread-safe global namespace. This avoids duplicate loading times for standard libraries and external packages.
