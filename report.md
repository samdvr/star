# Star Language — V1 Ship Readiness Report

Audit of the Star compiler codebase against DESIGN.md, covering all modules: lexer, parser, AST, type checker, codegen, optimizer, borrow inference, resolver, manifest, formatter, LSP, CLI, error reporting, tests, and examples.

---

## Executive Summary

Star is an impressive compiler with ~25,000 lines of Rust, 912 unit tests, 129 integration tests, 37 example programs, and a 290+ function standard library. The core compilation pipeline (lex → parse → typecheck → codegen → optimize → borrow-infer → cargo build) works end-to-end. However, several categories of work remain before a v1 release.

**Overall readiness: ~75%**

The gaps fall into five buckets:
1. **Language features promised in DESIGN.md but missing** (high priority)
2. **Compiler correctness issues** (high priority)
3. **Developer experience & tooling** (medium priority)
4. **Standard library gaps** (medium priority)
5. **Quality, testing, and polish** (lower priority)

---

## 1. Missing Language Features (from DESIGN.md)

### 1.1 Expression-Level Borrowing & Dereferencing
**Status:** Tokens exist (`&`, `*`, `~`) but expression-level parsing is missing.

- `&expr` — create a reference at the expression level (only works in type annotations today)
- `*expr` — dereference operator (completely absent; `UnaryOp` only has `Neg` and `Not`)
- `~expr` — move prefix on general expressions (only works on lambdas as `move fn()`)

**Impact:** Users cannot write `&my_string` or `*boxed_val` in expressions. The design doc shows `~String` move syntax for hot paths — not usable today.

### 1.2 Struct Spread Syntax
**Status:** Parsed but needs codegen verification.

- `Task { done: true, ..task }` — the design doc shows this as a core feature
- Parser has `StructLiteral` with a `spread` field, but codegen handling needs audit

### 1.3 Where Clauses
**Status:** Not implemented.

- No `where T: Trait` syntax on functions or impl blocks
- Only inline bounds `<T: Ord>` are supported
- Limits expressiveness for complex generic constraints

### 1.4 Associated Types in Trait Bounds
**Status:** Not implemented.

- `T: Iterator<Item=Int>` not supported
- No inference of associated type values from impl context
- No type substitution when using trait bounds with associated types

### 1.5 Lifetime Parameters in Functions
**Status:** Partial — tokens and type nodes exist, but no binding or validation.

- `fn foo<'a>(x: &'a T)` — unclear if it parses correctly end-to-end
- Lifetime annotations are essentially ignored throughout the compiler

### 1.6 Character Literals
**Status:** Not implemented.

- No `'c'` literal syntax (conflicts with lifetime token `'a`)
- `Char` is a declared primitive type but has no literal form

### 1.7 Named-Field Enum Variants
**Status:** Not implemented.

- Only tuple variants `| Variant(Type, Type)` are supported
- `| Variant { field: Type }` struct-like variants are missing

### 1.8 Struct Field Visibility
**Status:** Not implemented.

- No `pub` modifier on individual struct fields
- All fields are implicitly public in generated Rust

### 1.9 Module Aliasing
**Status:** Not implemented.

- No `use Foo as Bar` syntax
- No nested module paths `use Foo::Bar::Baz` beyond selective imports

### 1.10 Loop Enhancements
**Status:** Missing advanced forms.

- No labeled loops (`'label: for`)
- No `break` with value (`break 42`)
- No `loop` keyword (infinite loop sugar)

---

## 2. Compiler Correctness Issues

### 2.1 Mutual Recursion Not Detected (Critical)
**Severity:** High — causes stack overflows at runtime.

The recursive type detection in codegen only checks direct self-reference. If enum `A` contains enum `B` and `B` contains `A`, neither is auto-boxed. This will produce Rust code that fails to compile (infinite size type) or cause stack overflows.

**Fix:** Implement transitive cycle detection using a fixed-point algorithm across all type definitions.

### 2.2 Trait Bounds Parsed But Never Validated
**Severity:** High — silent type errors.

`fn<T: Clone>(x: T)` parses and type-checks without verifying `Clone` is satisfied by the concrete type. Star relies entirely on rustc to catch these, which means error messages point to generated Rust code — not Star source.

### 2.3 Pattern Match Exhaustiveness is Incomplete
**Severity:** Medium — non-exhaustive matches compile but panic at runtime.

The type checker only warns about missing wildcard/catch-all arms. It doesn't verify that all enum variants are covered, doesn't account for guards, and doesn't detect overlapping patterns.

### 2.4 Method Call Type Checking Deferred
**Severity:** Medium — type errors surface only at rustc stage.

`MethodCall` expressions always return a fresh type variable. The type checker never validates that the method exists on the receiver type. All checking is delegated to rustc.

### 2.5 Let-Polymorphism Incomplete
**Severity:** Medium — generic let-bindings may not instantiate correctly.

Generalization happens but type schemes are not properly instantiated at all use sites. This can lead to incorrect type unification for polymorphic values used in multiple contexts.

### 2.6 Module Visibility Leaks
**Severity:** Low — all modules emit `use super::*;`.

Generated Rust modules import everything from the parent scope, including private functions. This breaks encapsulation guarantees that `pub` visibility is supposed to enforce.

---

## 3. Developer Experience & Tooling

### 3.1 LSP Server (Needs Significant Work)
**Current state:** Foundation-level with 6 request handlers.

**Working:**
- Hover (type info), completion (keywords/builtins/symbols), go-to-definition (single file), formatting, document symbols, semantic tokens

**Missing for v1:**
- Cross-file go-to-definition
- Workspace support (multi-file projects)
- Signature help for function calls
- Find all references
- Rename refactoring
- Inlay hints for inferred types
- Incremental document analysis
- Diagnostic quick-fixes / code actions

### 3.2 No Incremental Compilation
Every `star build` recompiles everything from scratch. For projects beyond a few files, this will be noticeably slow.

### 3.3 No Watch Mode
No `star watch` command for automatic recompilation on file changes. Essential for iterative development.

### 3.4 No REPL
No interactive mode for exploring the language. Important for onboarding new users.

### 3.5 No Documentation Generator
No `star doc` command to generate API documentation from source annotations/comments.

### 3.6 Error Reporting Gaps
- Single-line spans only — no multi-line error regions
- No error chaining or "caused by" context
- No suggestion/fix hints in error output
- ANSI colors hardcoded (no `--no-color` flag for CI)
- No machine-readable JSON error format
- Errors in generated Rust reference line numbers in `.rs` files, not `.star` source

### 3.7 No Package Manager / Registry
DESIGN.md shows `[dependencies]` in Star.toml for Star packages, but there's no package registry, no `star install`, and no dependency resolution for Star-native packages.

---

## 4. Standard Library Gaps

### 4.1 JSON Serialization/Deserialization
**Status:** Not implemented.

DESIGN.md lists JSON as part of the minimum viable stdlib. There are no `json_parse`, `json_stringify`, or related builtins. This is a hard requirement for v1 — nearly every modern language ships with JSON support.

### 4.2 CSV / TOML / YAML Parsing
**Status:** Not implemented.

Common serialization formats have no stdlib support. At minimum, TOML parsing would be useful since Star.toml is the project manifest format.

### 4.3 Argument Parsing
**Status:** Not implemented.

DESIGN.md section 15 lists CLI argument parsing as a standard feature. Currently only `args()` returns raw strings — no structured parsing.

### 4.4 Terminal Utilities
**Status:** Not implemented.

No ANSI color helpers, no terminal size detection, no interactive input beyond `read_line`.

### 4.5 Signal Handling
**Status:** Not implemented.

DESIGN.md section 9 lists signals as part of OS interaction. No `on_signal`, `trap`, or similar.

### 4.6 Smart Pointers / Resource Management
**Status:** Not implemented.

No `Rc`, `Arc`, `Weak`, or RAII/`defer` patterns exposed. DESIGN.md section 11 lists these as expected.

### 4.7 Streaming / Lazy Evaluation
**Status:** Not implemented.

No lazy iterators, generators, or stream abstractions. All collection operations are eager (clone + collect).

---

## 5. Quality, Testing & Polish

### 5.1 Untested Modules
| Module | Tests | Risk |
|--------|-------|------|
| `ast.rs` | 0 | Low (data types) |
| `error.rs` | 0 | Medium (formatting bugs) |
| `main.rs` | 0 | **High** (CLI correctness) |
| `lsp.rs` | 0 | **High** (IDE experience) |

### 5.2 Weak Test Coverage
| Module | Tests | Lines | Concern |
|--------|-------|-------|---------|
| `borrow.rs` | 19 | 1,026 | Complex heuristics undertested |
| `resolver.rs` | 14 | 377 | Multi-file resolution edge cases |
| `manifest.rs` | 16 | 722 | Cargo.toml generation not directly tested |

### 5.3 No End-to-End Runtime Tests
Integration tests verify codegen output but never compile+run the generated Rust. A program could generate syntactically valid but semantically broken Rust that only fails at cargo build time.

### 5.4 Generated Rust Code Quality
- Pervasive `.clone().into_iter()` pattern (150+ instances) — generates Clippy warnings
- `&*var` pattern used instead of `var.as_str()`
- No `#[allow(dead_code)]` on generated helpers — warning noise
- All I/O errors converted to `String` via `map_err(|e| e.to_string())` — loses error type info
- No `#[inline]` hints on hot stdlib wrappers

### 5.5 Builtin System Unmaintainable
290+ builtins hardcoded in a single match expression spanning ~2,700 lines in `codegen.rs`, with parallel arity tables in `typeck.rs`. Adding a new builtin requires editing two files in sync. Should be table-driven or extracted to a registry module.

### 5.6 PascalCase Module Conversion
`HTTPClient` → `h_t_t_p_client` instead of `http_client`. The snake_case conversion doesn't handle acronyms.

---

## Prioritized V1 Roadmap

### P0 — Must Fix Before V1

| # | Item | Category | Effort |
|---|------|----------|--------|
| 1 | Fix mutual recursion detection (auto-boxing) | Correctness | Medium |
| 2 | Add JSON serialize/deserialize builtins | Stdlib | Medium |
| 3 | Implement `*expr` dereference operator | Language | Small |
| 4 | Implement `&expr` borrow operator | Language | Small |
| 5 | Validate trait bounds at type-check time | Correctness | Large |
| 6 | Improve pattern match exhaustiveness checking | Correctness | Medium |
| 7 | Add character literals | Language | Small |
| 8 | Add end-to-end compile+run integration tests | Testing | Medium |
| 9 | Add CLI tests for all subcommands | Testing | Medium |
| 10 | Map Star source spans through to rustc errors | DX | Large |

### P1 — Should Have for V1

| # | Item | Category | Effort |
|---|------|----------|--------|
| 11 | Cross-file go-to-definition in LSP | Tooling | Medium |
| 12 | LSP workspace support | Tooling | Medium |
| 13 | `star watch` command | Tooling | Small |
| 14 | Where clauses | Language | Medium |
| 15 | Named-field enum variants | Language | Medium |
| 16 | Struct spread in codegen | Language | Small |
| 17 | Module aliasing (`use Foo as Bar`) | Language | Small |
| 18 | CLI argument parsing builtin | Stdlib | Small |
| 19 | `--no-color` and `--json` error output flags | DX | Small |
| 20 | Fix module visibility leaks (`use super::*`) | Correctness | Medium |

### P2 — Nice to Have for V1

| # | Item | Category | Effort |
|---|------|----------|--------|
| 21 | REPL / interactive mode | Tooling | Large |
| 22 | Incremental compilation | Tooling | Large |
| 23 | LSP signature help & inlay hints | Tooling | Medium |
| 24 | Lazy iterators / streaming | Language | Large |
| 25 | `Rc`/`Arc` smart pointer builtins | Stdlib | Medium |
| 26 | CSV/TOML/YAML parsing builtins | Stdlib | Medium |
| 27 | Signal handling | Stdlib | Small |
| 28 | Terminal color/formatting helpers | Stdlib | Small |
| 29 | Extract builtin registry from codegen.rs | Code quality | Large |
| 30 | Documentation generator (`star doc`) | Tooling | Large |
| 31 | Struct field visibility (`pub` fields) | Language | Small |
| 32 | Loop labels and `break` with value | Language | Small |
| 33 | Move prefix `~` on general expressions | Language | Small |
| 34 | Package registry / `star install` | Ecosystem | Very Large |

### Post-V1

| Item | Notes |
|------|-------|
| Associated types | Complex type system feature |
| Lifetime validation | Currently delegated entirely to rustc |
| Let-polymorphism fix | Subtle HM inference issue |
| Const generics | Niche feature |
| Generic type defaults | Low demand |
| `async` closures | Rust itself only recently stabilized these |
| Workspace / multi-package builds | Needed for large projects |

---

## Module Health Summary

| Module | Lines | Tests | Completeness | Health |
|--------|-------|-------|--------------|--------|
| `lexer.rs` | 1,623 | 90 | 98% | Excellent |
| `parser.rs` | 3,127 | 80 | 85% | Good |
| `ast.rs` | ~800 | 0 | 95% | Good (data-only) |
| `typeck.rs` | 4,270 | 144 | 70% | Needs work |
| `codegen.rs` | 7,693 | 496 | 85% | Good but sprawling |
| `optimize.rs` | 338 | 30 | 95% | Excellent |
| `borrow.rs` | 1,026 | 19 | 75% | Adequate |
| `resolver.rs` | 377 | 14 | 85% | Good |
| `manifest.rs` | 722 | 16 | 80% | Good |
| `formatter.rs` | 1,382 | 24 | 90% | Good |
| `lsp.rs` | ~600 | 0 | 40% | Needs significant work |
| `main.rs` | ~500 | 0 | 85% | Good but untested |
| `error.rs` | ~150 | 0 | 70% | Adequate |

---

## Conclusion

Star's core compilation pipeline is solid and the standard library is remarkably rich for a young language. The main blockers for v1 are:

1. **Correctness** — mutual recursion detection, trait bound validation, and exhaustiveness checking need to be fixed to prevent confusing runtime failures
2. **JSON support** — a non-negotiable stdlib feature for any modern language
3. **Expression-level `&`/`*` operators** — fundamental for a language that generates Rust
4. **Error experience** — users hitting rustc errors instead of Star errors is the biggest DX gap
5. **Testing** — CLI and LSP have zero tests; end-to-end tests don't actually compile the generated Rust

Fixing the P0 items would make Star credible as a v1 release. The P1 items would make it competitive.
