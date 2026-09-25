# Standard Library: `std::time`

## Overview
High-resolution timers, duration measurement, and fiber sleep.

---

## Functions

### `clock_ms() -> Float`
Returns high-resolution monotonic millisecond timestamp since process start.
```aether
let start = clock_ms();
# execute task
let elapsed = clock_ms() - start;
println("Duration: " + to_string(elapsed) + " ms");
```

### `sleep(milliseconds: Int) -> Nil`
Suspends execution of the current fiber for the specified duration without blocking underlying OS threads.
```aether
sleep(500); # 0.5s pause
```