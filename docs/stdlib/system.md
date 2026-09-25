# Standard Library: `std::system` & `std::os`

## Overview
Platform access, process control, environment inspection, and system metrics.

---

## Functions

### `env_get(key: String) -> String | Nil`
Retrieves the value of an environment variable. Returns `nil` if not defined.
```aether
let home = env_get("HOME");
```

### `env_set(key: String, val: String) -> Nil`
Sets the value of an environment variable for the current process.

### `exit(code: Int) -> Nil`
Terminates the current process immediately with the specified integer exit code.

### `args() -> List<String>`
Returns the command-line arguments passed to the script.