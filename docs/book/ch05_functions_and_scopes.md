# Chapter 5: Functions, Closures, Scopes & Unpacking

### 5.1 Function Declarations (`fn` or `def`)

AETHER accepts both modern curly-brace `fn` and Pythonic indented `def` declarations:

```aether
# Modern C-Style
fn calculate_area(width, height) {
    return width * height
}

# Pythonic Style
def greet(name, title="Engineer"):
    return "Greetings, " + title + " " + name

println(greet("Sami", title="Lead Architect"))
```

### 5.2 Multiple Return Values & Tuple Unpacking

```aether
def get_coordinates():
    return 33.6844, 73.0479

let lat, lon = get_coordinates()
println("Latitude:", lat, "Longitude:", lon)
```

### 5.3 Closures & Higher-Order Functions

```aether
def make_multiplier(factor):
    def multiplier(n):
        return n * factor
    return multiplier

let double = make_multiplier(2)
let triple = make_multiplier(3)

println("Double 15:", double(15)) # 30
println("Triple 15:", triple(15)) # 45
```

---
