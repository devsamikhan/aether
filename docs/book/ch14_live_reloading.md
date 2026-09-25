# Chapter 14: Erlang-Grade Live Code Swapping & Zero-Downtime Reloading

AETHER features in-place bytecode swapping. Long-running servers can be updated on-the-fly without dropping active socket connections or erasing in-memory state.

### 14.1 Running in Live-Reload Mode

```bash
aether live server.ae 250
```

When you edit `server.ae` in your editor and save:
1. The compiler re-parses the AST in memory.
2. Invariant safety checks verify the new code.
3. The VM updates function and method pointers atomically.
4. Active heap state, variables, and client sessions are 100% preserved!

---
