# Standard Library: `std::io` & `std::fs`

## Overview
Filesystem operations, streaming buffers, and standard I/O.

---

## Filesystem Functions

### `file_read(path: String) -> String`
Reads the entire UTF-8 text content of a file into memory.
```aether
let content = file_read("config.json");
```

### `file_write(path: String, content: String) -> Nil`
Creates or truncates a file, writing the provided string content.
```aether
file_write("output.txt", "Execution success\n");
```

### `file_append(path: String, content: String) -> Nil`
Appends content to the end of an existing file.

### `file_exists(path: String) -> Bool`
Returns `true` if a file or directory exists at the given path.
```aether
if file_exists("data.bin") {
    println("File ready for processing");
}
```

### `dir_create(path: String) -> Nil`
Creates directory path recursively.