# CRDT Usage in AETHER

AETHER language runtime provides native support for CRDT primitives within its distributed execution blocks.

## 1. Grow-only Counter Example
We declare a swarm block and compile to state-based merges:
```aether
swarm_spawn(10) {
    crdt_counter: GCounter;
    crdt_counter.increment();
}

let total = hive_mind.sum();
println("Total: {total}"); // Guaranteed eventual convergence!
```

## 2. Observed-Remove Set Example
Used for distributed tracking of system resources without central locks:
```aether
swarm_spawn(3) {
    active_users: ORSet;
    active_users.add("Alice");
}
```
*Note: In-process, the JIT toolchain maps these to the `aether::crdt::ORSet` Rust type representation.*
