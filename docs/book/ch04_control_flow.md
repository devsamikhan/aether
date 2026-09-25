# Chapter 4: Control Flow, Loops & Exhaustive Pattern Matching

### 4.1 Conditionals: `if`, `elif`, `else`

```aether
let score = 88

if score >= 90:
    println("Grade: A+ (Outstanding)")
elif score >= 80:
    println("Grade: A (Excellent)")
elif score >= 70:
    println("Grade: B (Good)")
else:
    println("Grade: C (Needs Improvement)")
```

### 4.2 Loops: `while` & `for..in`

```aether
# While Loop
let mut n = 5
while n > 0:
    println("Countdown:", n)
    n = n - 1

# For-in Loop with Range
for i in range(1, 6):
    if i == 3:
        continue # Skip 3
    println("Step:", i)
```

### 4.3 Exhaustive Pattern Matching (`match`)

AETHER features structural expression pattern matching with jump-table efficiency:

```aether
let status_code = 404

let status_msg = match status_code:
    200 => "OK - Resource retrieved successfully"
    301 => "Moved Permanently"
    400 => "Bad Request"
    404 => "Not Found - The requested endpoint does not exist"
    500 => "Internal Server Error"
    _   => "Unknown HTTP Status"

println("Response:", status_msg)
```

---
