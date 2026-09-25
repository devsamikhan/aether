# Standard Library: `std::math`

## Overview
The `std::math` module provides hardware-accelerated mathematical primitives, trigonometry, logarithms, and floating-point conversions.

---

## Functions

### `sqrt(x: Float) -> Float`
Calculates the principal square root of `x`.
```aether
let s = sqrt(144.0); # 12.0
```

### `pow(base: Float, exp: Float) -> Float`
Calculates `base` raised to the power `exp`.
```aether
let p = pow(2.0, 10.0); # 1024.0
```

### `abs(x: Float | Int) -> Float | Int`
Returns the absolute value of `x`.
```aether
let a = abs(-42); # 42
```

### `floor(x: Float) -> Int`
Returns the largest integer less than or equal to `x`.
```aether
let f = floor(3.9); # 3
```

### `ceil(x: Float) -> Int`
Returns the smallest integer greater than or equal to `x`.
```aether
let c = ceil(3.1); # 4
```

### `round(x: Float) -> Int`
Rounds float to the nearest integer.
```aether
let r = round(3.5); # 4
```

### `sin(x: Float) -> Float` / `cos(x: Float) -> Float` / `tan(x: Float) -> Float`
Trigonometric operations (angle in radians).
```aether
let s = sin(1.57079632679); # ~1.0
```

### `min(a, b)` / `max(a, b)`
Returns the minimum or maximum of two values.
```aether
let m = max(10, 25); # 25
```