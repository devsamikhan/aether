# AETHER 2.0 Language Reference: Functions & Closures

## 1. Function Declarations
Functions in AETHER are defined using the `fn` keyword followed by the function identifier, parameter list, and execution body block:

```aether
fn add(a, b) {
    return a + b;
}

let result = add(10, 25); # 35
```

---

## 2. Return Values & Early Exit
Functions return `nil` by default if execution reaches the end without an explicit `return` statement.

```aether
fn early_exit(val) {
    if val < 0 {
        return "negative";
    }
    return "positive";
}
```

---

## 3. First-Class Functions & Anonymous Lambdas
Functions in AETHER are first-class values. They can be stored in variables, passed to other functions, or returned from higher-order functions:

```aether
# Anonymous closure
let square = fn(x) { return x * x; };
println(square(6)); # 36

# Higher-order function
fn apply_twice(f, x) {
    return f(f(x));
}

let res = apply_twice(fn(n) { return n + 5; }, 10); # 20
```

---

## 4. Lexical Scoping & Closures
A function captures references from its enclosing lexical environment:

```aether
fn create_counter(start) {
    let count = start;
    return fn() {
        count = count + 1;
        return count;
    };
}

let counter = create_counter(100);
println(counter()); # 101
println(counter()); # 102
```

---

## 5. Recursion & Tail Optimization
AETHER supports direct and indirect recursion. Recursion depth is bounded by fiber stack limits:

```aether
fn fib(n) {
    if n <= 1 { return n; }
    return fib(n - 1) + fib(n - 2);
}
```