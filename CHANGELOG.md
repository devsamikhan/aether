# Changelog

All notable changes to Project AETHER will be documented in this file.

## [0.2.0] - 2026-07-13

### Added
- Full type checker with Hindley-Milner static inference (Int, Float, Bool, String, Char, Unit).
- Unification algorithms with occurs checks to prevent infinite/recursive types.
- Subtyping variance rules (covariant, contravariant, invariant).
- Module namespace system featuring module path resolution, selective, wildcard, and renamed imports, and shadowing rules.
- High-resolution diagnostics reporting formatting source code lines with caret indicators.
- 47 new documentation manuals and example programs.

### Changed
- Integrated module (mod) and import (use) statement structures inside the parser.
- Improved error recovery loop limits on lexer, parser, and type checking compilation passes.

## [0.1.0] - 2026-07-11
### Added
- Complete compiler core.
- CRDT library (GCounter, GSet, PNCounter, ORSet) with join-semilattice validation tests.
- Toolchain CLI supporting init, build, run, test, and benchmark commands.
