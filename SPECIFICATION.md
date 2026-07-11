# AETHER Language Specification
## Version 0.1.0-alpha

---

## 1. Overview
AETHER is a post-quantum, intent-driven, self-healing, and multiverse-aware programming language. Its design optimizes for human cognitive expression while natively supporting quantum mechanics, many-worlds branching, decentralized swarm networks, and brain-computer interfaces.

---

## 2. Lexical Structure

### 2.1 Character Set
All AETHER source files must be encoded in UTF-8.

### 2.2 Comments
AETHER supports single-line and multi-line block comments:
```aether
// This is a single-line comment

/*
   This is a multi-line
   block comment.
*/
```

### 2.3 Identifiers
Identifiers are case-sensitive and must match the regular expression:
`[a-zA-Z_][a-zA-Z0-9_]*`

### 2.4 Literals
- **Integer**: Base-10 integers, e.g., `42`, `-105`.
- **Float**: Base-10 decimals, e.g., `3.14159`, `-0.007`.
- **String**: UTF-8 character sequences enclosed in double quotes, supporting backslash escapes.
- **Boolean**: `true` or `false`.
- **Null**: `null` represents the absence of a value.

### 2.5 Operators
AETHER defines the following core operators:
- **Arithmetic**: `+`, `-`, `*`, `/`, `%`
- **Comparison**: `==`, `!=`, `<`, `>`, `<=`, `>=`
- **Logical**: `&&`, `||`, `!`
- **Assignment & Bindings**: `=`, `=>`

---

## 3. Grammar (EBNF)

```ebnf
Program         ::= (IntentDecl | Statement)*
IntentDecl      ::= 'intent' Identifier '{' (SchemaDecl | ConstraintDecl | UiDecl | Statement)* '}'
SchemaDecl      ::= 'schema' '{' FieldDecl* '}'
FieldDecl       ::= Identifier ':' TypeExpr ('=' Expression)? Annotation* ';'
ConstraintDecl  ::= 'constraint' Expression ';'
UiDecl          ::= 'ui' '{' UiNode '}'

FnDecl          ::= 'fn' Identifier '(' ParamList? ')' BlockStmt
ParamList       ::= Param (',' Param)*
Param           ::= Identifier ':' TypeExpr

BlockStmt       ::= '{' Statement* '}'
Statement       ::= LetStmt
                  | AssignStmt
                  | ExprStmt
                  | IfStmt
                  | MatchStmt
                  | WhileStmt
                  | ForStmt
                  | ReturnStmt
                  | QuantumStmt
                  | MultiverseStmt
                  | SwarmStmt
                  | BciStmt
                  | DbStmt
                  | BlockStmt

LetStmt         ::= ('let' | 'var') Identifier (':' TypeExpr)? ('=' Expression)? ';'
AssignStmt      ::= Expression '=' Expression ';'?
ExprStmt        ::= Expression ';'?
IfStmt          ::= 'if' Expression BlockStmt ('else' (IfStmt | BlockStmt))?
MatchStmt       ::= 'match' Expression '{' MatchArm* '}'
MatchArm        ::= (Literal | '_') '=>' Statement

WhileStmt       ::= 'while' Expression BlockStmt
ForStmt         ::= 'for' Identifier 'in' Expression '..' Expression BlockStmt
ReturnStmt      ::= 'return' Expression? ';'

QuantumStmt     ::= 'qubit' Identifier ';'
                  | 'entangle' '(' Identifier (',' Identifier)* ')' ';'
                  | 'measure' '(' Identifier ')' '=>' Identifier ';'

MultiverseStmt  ::= 'branch_reality' BlockStmt ';'?
                  | 'observe_timeline' '(' Identifier ')' ';'
                  | 'merge_universe' '(' Identifier ')' ';'
                  | 'ManyWorldsPathfind' '(' 'graph' ':' (Identifier | Literal) ',' 'dest' ':' (Identifier | Literal) ')' ';'

SwarmStmt       ::= 'swarm_spawn' '(' (Identifier | Integer) ')' ';'
                  | 'hive_mind' BlockStmt ';'?
                  | 'von_neumann_replicate' '(' Identifier ')' ';'
                  | 'QuantumSwarmConsensus' '(' 'nodes' ':' ('(' | '[') Identifier (',' Identifier)* (')' | ']') ')' ';'

BciStmt         ::= 'cortex_bind' 'neural_stream' '(' StringLiteral ')' '{' ('thought_intent' '(' StringLiteral ')' '=>' Statement (',' | ';')?)* '}' ';'?
                  | 'hologram' Identifier '(' ('spatial_anchor' | 'depth_mesh') ':' Identifier (',' ('spatial_anchor' | 'depth_mesh') ':' Identifier)* ')' ';'
                  | 'QuantumMeshOptimize' '(' 'target' ':' Identifier ',' 'qubits' ':' ('(' | '[') Identifier (',' Identifier)* (')' | ']') ')' ';'

DbStmt          ::= 'db' '{' 'query' ':' StringLiteral ';' '}' ';'?

Expression      ::= UnaryExpr | BinaryExpr | CallExpr | MemberExpr | Literal | Identifier
UnaryExpr       ::= ('!' | '-') Expression
BinaryExpr      ::= Expression BinaryOp Expression
BinaryOp        ::= '+' | '-' | '*' | '/' | '%' | '==' | '!=' | '<' | '>' | '<=' | '>=' | '&&' | '||'
CallExpr        ::= Expression '(' ArgList? ')'
MemberExpr      ::= Expression '.' Identifier
ArgList         ::= Expression (',' Expression)*
TypeExpr        ::= Identifier ('<' TypeExpr (',' TypeExpr)* '>')?
```

---

## 4. Type System

### 4.1 Primitive Types
- `Int`: 64-bit signed integer.
- `Float`: 64-bit IEEE 754 double-precision float.
- `String`: Immutable UTF-8 string.
- `Bool`: Boolean value (`true`/`false`).
- `Void`: The empty tuple type, denoting no return value.

### 4.2 Composite & Container Types
- `Array<T>`: Dynamically sized contiguous array of element type `T`.
- `Map<K, V>`: Hash map of key type `K` to value type `V`.
- `Set<T>`: Hash set of unique elements of type `T`.
- `Queue<T>` / `Stack<T>`: standard FIFO / LIFO structures.

### 4.3 Quantum Types
- `qubit`: Declares a quantum bit reference within the current register context.
- `EntangledPair`: Implicit state link between two qubits.

### 4.4 Tensor & Neural Model Types
- `tensor`: First-class multi-dimensional arrays, defined as `tensor name: float(dims...) = initial_value;`.
- `model`: Multi-layer neural network configuration declarations (e.g., layers `Dense`, `Conv2D`, `MaxPool`).

---

## 5. Operator Precedence Table

The following table lists AETHER operators from highest to lowest precedence:

| Precedence | Operators | Description | Associativity |
|------------|-----------|-------------|---------------|
| 7 (highest)| `()` `[]` `.` | Function call, array index, member access | Left |
| 6          | `!` `-` (unary) | Logical NOT, arithmetic negation | Right |
| 5          | `*` `/` `%` | Multiplication, division, remainder | Left |
| 4          | `+` `-` | Addition, subtraction | Left |
| 3          | `<` `>` `<=` `>=` | Ordering comparisons | Left |
| 2          | `==` `!=` | Equality comparisons | Left |
| 1          | `&&` | Logical AND | Left |
| 0 (lowest) | `\|\|` `=` `=>` | Logical OR, assignment, match arrow | Right |

---

## 6. Memory Model

### 6.1 Stack vs Heap
- Basic primitive variables (`Int`, `Float`, `Bool`) are stack-allocated.
- Heap allocation is handled automatically by the **Fluid Memory Allocator**.

### 6.2 Fluid Memory Allocator
- Operates as a **Zero-Stop-The-World (Zero-STW)** epoch-based allocator.
- Utilizes hazard pointers and local allocation pages to guarantee lock-free garbage collection and zero GC latency.

### 6.3 Quantum Memory
- Declaring a `qubit` dynamically registers a quantum index.
- Qubits are simulated within a classical complex-valued state-vector matrix during testing/simulation, and will map to physical QPUs in hardware target compilation.

---

## 7. Concurrency Model

- **Read-Copy-Update (RCU)**: All shared context mutations in concurrent states happen via lock-free RCU protocols.
- **Swarm Concurrency**: Spawning agents via `swarm_spawn(N)` generates distributed lightweight execution routines communicating using Conflict-Free Replicated Data Types (CRDTs).

---

## 8. Quantum Semantics

1. **Register Allocation**: `qubit q;` initializes state `|0⟩`.
2. **Superposition**: `superpose(q)` applies a Hadamard gate, placing the qubit in state `1/√2 (|00⟩ + |11⟩)`.
3. **Entanglement**: `entangle(q1, q2)` applies a CNOT gate relative to `q1`, producing a Bell State:
   $$\vert\Psi^+\rangle = \frac{1}{\sqrt{2}}(\vert00\rangle + \vert11\rangle)$$
4. **Measurement**: `measure(q) => v;` collapses the state vector with respect to the probability amplitude, writing a classical `1` or `0` to the identifier `v`.

---

## 9. Multiverse Semantics

AETHER treats timeline branching as speculative execution trees:
1. **`branch_reality`**: Creates a copy of the current Unified Context Graph (UCG).
2. **`observe_timeline`**: Queries the final execution state of a specific branch.
3. **`merge_universe`**: Collapses the selected branches back to the main timeline, applying all mutations matching the chosen path.

---

## 10. Intent Semantics

An `intent` forms the primary compile-time contract of AETHER:
- Represents both data schema and logical execution methods.
- Validates properties defined in its `schema` block on each mutation.
- Includes a dedicated `intent Test_<Name>` block containing unit assertions.

---

## 11. Built-in Function Registry

The following built-ins are available in standard scope:
- `println(String) -> Void`: Prints string to stdout.
- `readln() -> String`: Reads line from stdin.
- `sqrt(Float) -> Float`: Square root.
- `pow(Float, Float) -> Float`: Exponentiation.
- `abs(Float) -> Float`: Absolute value.
- `floor(Float) -> Int`: Largest integer less than or equal to input.
- `ceil(Float) -> Int`: Smallest integer greater than or equal to input.
- `to_string(Any) -> String`: Formats type to string.
- `to_int(String) -> Int`: Parses string to integer.
- `assert(Bool) -> Void`: Throws exception if false.
