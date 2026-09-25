# Standard Library: `std::crypto`

## Overview
Pure Rust cryptographic primitives, zero-allocation hashing, and secure random generators.

---

## Functions

### `sha256(data: String) -> String`
Computes the SHA-256 cryptographic digest of a string, returning a lowercase hexadecimal string.
```aether
let hash = sha256("admin_secret");
println(hash);
```

### `random_int(min: Int, max: Int) -> Int`
Generates a cryptographically strong pseudo-random integer in the range `[min, max]`.

### `uuid() -> String`
Generates a random UUIDv4 string conforming to RFC 4122.
```aether
let id = uuid(); # e.g. "c9b1f7d2-4e2b-4d0a-9d2a-89a1c1d4b678"
```