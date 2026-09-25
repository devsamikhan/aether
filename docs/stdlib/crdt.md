# Standard Library: `std::crdt`

## Overview
Conflict-Free Replicated Data Types for peer-to-peer eventual consistency without central coordinators.

---

## Primitives

### `GCounter(node_id: String)`
Grow-only distributed counter supporting commutative merges.
- `increment(delta: Int)`
- `value() -> Int`
- `merge(other: GCounter)`

### `PNCounter(node_id: String)`
Positive-Negative distributed counter supporting increments and decrements.
```aether
let c1 = PNCounter("node_1");
let c2 = PNCounter("node_2");

c1.increment(20);
c2.increment(30);
c1.decrement(5);

c1.merge(c2);
println(c1.value()); # 45
```

### `ORSet(node_id: String)`
Observed-Remove Set with tombstone tracking for conflict-free set unions and deletions.
- `add(item: Any)`
- `remove(item: Any)`
- `contains(item: Any) -> Bool`
- `merge(other: ORSet)`