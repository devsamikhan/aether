# AETHER 2.0 Language Reference: Lexical Structure

## 1. Character Set & Encoding
Source files must be encoded in UTF-8. Identifiers and string literals may contain valid Unicode characters.

---

## 2. Comments
- **Line Comments**: Begin with `#` or `//` and extend to the end of the line:
  ```aether
  # Single line comment
  // Also supported single line comment
  ```
- **Block Comments**: Delimited by `/* ... */` and can be nested:
  ```aether
  /* Multi-line
     documentation block */
  ```

---

## 3. Keywords
The following reserved keywords cannot be used as variable identifiers:
`let`, `fn`, `return`, `if`, `elif`, `else`, `while`, `for`, `in`, `try`, `catch`, `finally`, `throw`, `class`, `intent`, `schema`, `require`, `ensure`, `use`, `as`, `true`, `false`, `nil`, `and`, `or`, `not`, `match`.

---

## 4. Literals
- **Integer**: Decimal `123`, Hexadecimal `0x1F`, Binary `0b1011`.
- **Float**: `12.34`, `1.5e-3`.
- **String**: Quoted with double quotes: `"hello\n"`.
- **Boolean**: `true`, `false`.
- **Nil**: `nil`.