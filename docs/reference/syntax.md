# AETHER 2.0 Language Reference: Syntax & Grammar (EBNF)

## 1. Formal Grammar Specification

```ebnf
Program         ::= (Statement)* EOF

Statement       ::= IntentDecl
                  | ClassDecl
                  | FunctionDecl
                  | VarDecl
                  | ReturnStmt
                  | IfStmt
                  | WhileStmt
                  | ForStmt
                  | TryCatchStmt
                  | ExprStmt

IntentDecl      ::= "intent" IDENTIFIER "{" SchemaBlock? RequireBlock? EnsureBlock? (FunctionDecl)* "}"
SchemaBlock     ::= "schema" "{" (IDENTIFIER (":" Type)? ("=" Expression)? ";")* "}"
RequireBlock    ::= "require" "{" (Expression ";")* "}"
EnsureBlock     ::= "ensure" "{" (Expression ";")* "}"

ClassDecl       ::= "class" IDENTIFIER ("extends" IDENTIFIER)? "{" (FunctionDecl)* "}"
FunctionDecl    ::= "fn" IDENTIFIER "(" ParameterList? ")" Block
ParameterList   ::= IDENTIFIER ("," IDENTIFIER)*

VarDecl         ::= "let" IDENTIFIER (":" Type)? "=" Expression ";"
ReturnStmt      ::= "return" Expression? ";"
IfStmt          ::= "if" Expression Block ("elif" Expression Block)* ("else" Block)?
WhileStmt       ::= "while" Expression Block
ForStmt         ::= "for" IDENTIFIER "in" Expression Block
TryCatchStmt    ::= "try" Block "catch" "(" IDENTIFIER ")" Block ("finally" Block)?

Block           ::= "{" (Statement)* "}"
ExprStmt        ::= Expression ";"

Expression      ::= Assignment
Assignment      ::= (IDENTIFIER | MemberAccess | IndexAccess) "=" Assignment | LogicalOr
LogicalOr       ::= LogicalAnd ("or" LogicalAnd)*
LogicalAnd      ::= Equality ("and" Equality)*
Equality        ::= Relational (("==" | "!=") Relational)*
Relational      ::= Additive (("<" | "<=" | ">" | ">=") Additive)*
Additive        ::= Multiplicative (("+" | "-") Multiplicative)*
Multiplicative  ::= Unary (("*" | "/" | "%") Unary)*
Unary           ::= ("!" | "-" | "not") Unary | Call
Call            ::= Primary ( "(" ArgList? ")" | "[" Expression "]" | "." IDENTIFIER )*
Primary         ::= NUMBER | STRING | "true" | "false" | "nil" | IDENTIFIER 
                  | "(" Expression ")" 
                  | ListLiteral 
                  | MapLiteral 
                  | Comprehension
```