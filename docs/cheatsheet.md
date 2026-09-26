# ⚡ AETHER 2.0 by Example: The Fast Cheat Sheet

A concise, copy-pasteable reference for developers learning AETHER syntax in 10 minutes.

---

## 1. Variables & Types
```aether
let count = 42;                 # 64-bit Int
let price = 19.99;              # 64-bit Float
let is_active = true;           # Bool
let name = "AETHER";            # UTF-8 String
let empty = nil;                # Nil / None
count = count + 1;              # Mutate variable
```

---

## 2. Arithmetic & Logic
```aether
let a = 15;
let b = 4;
println(a + b);                 # 19
println(a / b);                 # 3 (integer division)
println(a % b);                 # 3 (modulo)
let ok = (a > 10) and (b == 4); # Logical AND (or: '&&', 'or', 'not')
```

---

## 3. Strings & Interpolation
```aether
let s1 = "Hello";
let s2 = "World";
let msg = s1 + ", " + s2 + "!"; # String concatenation
println(to_string(42));         # Cast int to string: "42"
println(to_int("100"));         # Parse string to int: 100
```

---

## 4. Lists (Dynamic Arrays)
```aether
let items = [10, 20, 30, 40];
println(items[0]);              # 10 (zero-indexed)
items.push(50);                 # Append element
let last = items.pop();         # Remove last element (50)
println(len(items));            # Length: 4
```

---

## 5. Dictionaries (Key-Value Maps)
```aether
let user = {"id": 101, "role": "admin", "active": true};
println(user["role"]);          # "admin"
user["email"] = "dev@aether.org"; # Add new key
```

---

## 6. List Comprehensions
```aether
let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
let evens_squared = [x * x for x in numbers if x % 2 == 0];
println(evens_squared);         # [4, 16, 36, 64, 100]
```

---

## 7. Control Flow (`if`, `elif`, `else`)
```aether
let temp = 28;
if temp > 35 {
    println("Hot");
} elif temp > 20 {
    println("Pleasant");
} else {
    println("Cold");
}
```

---

## 8. Loops (`while` & `for..in`)
```aether
# While loop
let i = 0;
while i < 3 {
    println("Count: " + to_string(i));
    i = i + 1;
}

# For-in loop
for word in ["Quantum", "Fiber", "SIMD"] {
    println("Feature: " + word);
}
```

---

## 9. Functions & Closures
```aether
fn add(x, y) {
    return x + y;
}
println(add(10, 25));           # 35

# Higher-order function / Lambda
let double = fn(n) { return n * 2; };
println(double(21));            # 42
```

---

## 10. Classes & Object-Oriented Programming
```aether
class User {
    fn init(name, age) {
        this.name = name;
        this.age = age;
    }

    fn greet() {
        return "Hi, I am " + this.name;
    }
}

let u = User("Alice", 25);
println(u.greet());             # "Hi, I am Alice"
```

---

## 11. Exception Handling (`try`, `catch`, `throw`)
```aether
try {
    throw "InvalidParameterError";
} catch (err) {
    println("Handled error: " + err);
}
```

---

## 12. Green Fibers Concurrency (1M Tasks in 825ms)
```aether
let ch = channel();
spawn(fn() {
    ch.send("Task complete!");
});
let message = ch.recv();
println(message);               # "Task complete!"
```

---

## 13. Accelerated SIMD Vector Compute (2.1x Faster than C++ -O3)
```aether
let a = [1.0, 2.0, 3.0, 4.0];
let b = [10.0, 20.0, 30.0, 40.0];
let dot = Compute.dot_product(a, b);
println("Dot: " + to_string(dot)); # 300.0
```

---

## 14. Declarative Intent Verification
```aether
intent AccountBalance {
    schema {
        balance: Float = 0.0;
    }
    require {
        this.balance >= 0.0;
    }
    ensure {
        this.balance >= 0.0;
    }
}
```

---

## 15. Post-Quantum State Vector Simulation
```aether
let qreg = QuantumRegister(2);
qreg.h(0);                      # Superposition on Q0
qreg.cnot(0, 1);                 # Entangle Q0 and Q1
let result = qreg.measure(0);    # Wavefunction collapse
println("Collapsed Q0: " + to_string(result));
```
