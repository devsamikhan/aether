# Chapter 2: Variables, Mutability & Primitive Data Types

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
