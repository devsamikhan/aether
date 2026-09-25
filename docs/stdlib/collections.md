# Standard Library: `std::collections`

## Overview
High-performance sequence and associative map data structures.

---

## Dynamic Array Methods

### `len(list) -> Int`
Returns the element count of the collection.
```aether
let items = [1, 2, 3];
println(len(items)); # 3
```

### `push(list, element) -> Nil`
Appends an element to the end of the list.
```aether
items.push(4);
```

### `pop(list) -> Any`
Removes and returns the last element.
```aether
let last = items.pop(); # 4
```

### `insert(list, index: Int, element) -> Nil`
Inserts an element at the specified index.

### `contains(list, element) -> Bool`
Returns `true` if element exists within the collection.

---

## Map / Dictionary Methods

### `keys(map) -> List`
Returns a list of all keys in the dictionary.

### `values(map) -> List`
Returns a list of all values in the dictionary.

### `has_key(map, key) -> Bool`
Checks if key exists with \(O(1)\) time complexity.