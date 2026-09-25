# AETHER 2.0 Language Reference: Statements & Control Flow

## 1. Variable Declarations (`let`)
Variables are introduced via `let`:
```aether
let x = 42;
let name: String = "AETHER";
```

---

## 2. Conditional Statements (`if`, `elif`, `else`)
Branch execution based on boolean truth values:
```aether
if score >= 90 {
    println("Grade: A");
} elif score >= 75 {
    println("Grade: B");
} else {
    println("Grade: C");
}
```

---

## 3. Loop Statements (`while`, `for`)

### While Loop
Repeats as long as condition evaluates to `true`:
```aether
let count = 0;
while count < 10 {
    count = count + 1;
}
```

### For-In Loop
Iterates through collections, ranges, or generator iterators:
```aether
for item in ["alpha", "beta", "gamma"] {
    println(item);
}

for i in 0..5 {
    println(i);
}
```

---

## 4. Exception Handling (`try`, `catch`, `finally`, `throw`)
Safely manage runtime anomalies without crashing the process:
```aether
try {
    if divisor == 0 {
        throw "DivisionByZero";
    }
    let res = numerator / divisor;
} catch (err) {
    println("Handled error: " + err);
} finally {
    println("Cleanup executed");
}
```