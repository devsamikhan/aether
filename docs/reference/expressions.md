# AETHER 2.0 Language Reference: Expressions & Operators

## 1. Operator Hierarchy & Precedence
Expressions are evaluated in accordance with standard mathematical operator precedence (from highest binding to lowest):

1. **Member & Subscript**: `.member`, `[index]`, `(arguments)`
2. **Unary Operators**: `-` (negation), `!` / `not` (logical NOT), `~` (bitwise NOT)
3. **Multiplicative**: `*`, `/`, `%`
4. **Additive**: `+`, `-`
5. **Bitwise Shifts**: `<<`, `>>`
6. **Relational**: `<`, `<=`, `>`, `>=`
7. **Equality**: `==`, `!=`
8. **Bitwise AND**: `&`
9. **Bitwise XOR**: `^`
10. **Bitwise OR**: `|`
11. **Logical AND**: `and`, `&&`
12. **Logical OR**: `or`, `||`
13. **Assignment**: `=`, `+=`, `-=`, `*=`, `/=`

---

## 2. Comprehensions
AETHER features inline collection comprehensions:

```aether
# List Comprehension
let evens = [x for x in [1, 2, 3, 4, 5, 6] if x % 2 == 0];

# Map Transformation
let doubled = {k: v * 2 for (k, v) in {"a": 1, "b": 2}};
```

---

## 3. String Concatenation & Interpolation
Strings support concatenation using the `+` operator with implicit or explicit string conversion:

```aether
let message = "Total items: " + to_string(42);
```