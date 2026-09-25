# Chapter 3: Collections & Composite Data Structures

AETHER provides rich, expressive collection data structures right in the language core.

### 3.1 Lists (Dynamic Arrays)

Lists are ordered, mutable sequences of values:

```aether
let fruits = ["Apple", "Banana", "Cherry"]

# Indexing & slicing
println("First fruit:", fruits[0]) # Apple
println("Last fruit: ", fruits[-1]) # Cherry

# Appending & modifying
fruits.push("Date")
fruits[1] = "Blueberry"
println("Updated fruits:", fruits)

# Array length
println("Total items:", len(fruits))
```

### 3.2 Dictionaries (HashMaps)

Dictionaries store associative key-value pairs with $O(1)$ lookups:

```aether
let config = {
    "host": "127.0.0.1",
    "port": 8080,
    "ssl": true,
    "max_connections": 10000
}

println("Connecting to:", config["host"] + ":" + str(config["port"]))

# Adding new keys
config["timeout_ms"] = 5000
```

### 3.3 List & Dictionary Comprehensions

AETHER supports Pythonic list and dictionary comprehensions:

```aether
let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

# List comprehension with filtering
let evens_squared = [x * x for x in numbers if x % 2 == 0]
println("Even squares:", evens_squared) # [4, 16, 36, 64, 100]

# Dictionary comprehension
let squares_map = {x: x * x for x in numbers if x <= 5}
println("Squares map:", squares_map)
```

---
