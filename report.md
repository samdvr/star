# Star v1 Ship Readiness Report

**Date:** 2026-03-23
**Scope:** Full audit of DESIGN.md vs implementation, identifying all gaps for a v1 release.

---

## Executive Summary

Star is a functional-first language with Ruby-like syntax that compiles to idiomatic Rust. The compiler is **P0-complete** — core language features (type inference, pattern matching, modules, loops, 290+ stdlib builtins) all work end-to-end with 615 tests passing.

To ship v1, the project needs work in three areas:

1. **Language completeness** — Traits, generics, and ownership features are parsed but partially wired up
2. **Tooling & DX** — No LSP, no REPL, no watch mode, no package registry
3. **Production hardening** — Error messages need polish, pattern exhaustiveness is basic, no warnings system

---

## 1. Language Features — Gaps vs DESIGN.md

### 1.1 Trait System (HIGH — blocks real-world usage)

**Current state:** Traits are parsed into AST, registered in the type checker with method signatures, and impl blocks are codegen'd. But:

- [ ] **Trait bound enforcement** — `fn max<T: Ord>(a: T, b: T)` parses, but the type checker does not verify that `T` actually satisfies `Ord` at call sites. Bounds are cosmetic.
- [ ] **Method dispatch on trait objects** — `dyn Trait` is parsed but there's no vtable or dynamic dispatch in codegen. Calling a method through a trait reference will likely fail or generate invalid Rust.
- [ ] **Associated types** — Parsed in AST (`TraitItem::AssociatedType`) but never resolved or substituted during type checking or codegen.
- [ ] **Default method bodies** — Tracked in the traits registry (`has_default: bool`) but unclear if defaults are properly inherited when an impl omits them.
- [ ] **Trait coherence** — No orphan rule checking, no overlap detection for impl blocks.
- [ ] **Super-traits** — No `trait Foo: Bar` inheritance syntax or semantics.

### 1.2 Generics (MEDIUM — works for simple cases, breaks on complex ones)

- [ ] **Trait bound validation** — Generic functions accept bounds syntactically but don't enforce them. `fn foo<T: Clone + Debug>(x: T)` will generate the Rust signature, but the type checker won't catch violations.
- [ ] **Higher-kinded types** — Not supported (no `F<_>` or `impl<T> Trait for F<T>` patterns).
- [ ] **Const generics** — Not supported.
- [ ] **Where clauses** — Not parsed or supported. Complex bounds require `where` syntax in Rust.
- [ ] **Generic method calls** — e.g., `list.into::<Vec<String>>()` turbofish syntax not supported.

### 1.3 Ownership & Borrowing (MEDIUM — clone-by-default works, explicit control incomplete)

- [ ] **Mutable references (`&mut`)** — Parsed in AST/typeck but codegen support is limited. Real mutable-borrow patterns likely generate invalid Rust.
- [ ] **Move semantics (`~T`)** — The tilde prefix is parsed but likely not generating correct move-only code in Rust.
- [ ] **Lifetime annotations** — Parsed by the lexer/parser but never emitted in generated Rust. Functions returning references will fail to compile.
- [ ] **Borrow checker feedback** — When rustc rejects generated code due to borrowing issues, Star gives no actionable feedback (user sees raw Rust errors).

### 1.4 Pattern Matching (LOW-MEDIUM — works, but incomplete)

- [ ] **Exhaustiveness checking** — Basic coverage only. No warnings for non-exhaustive matches on enums. Rust's `rustc` will catch these, but Star should report them first.
- [ ] **Match guards (`when`)** — The `when` keyword exists in the lexer and guards are parsed, but validation and codegen need verification.
- [ ] **Nested or-patterns** — `| Foo(A | B)` — unclear if fully supported in codegen.
- [ ] **Literal patterns in nested positions** — e.g., matching on `Some(42)` — needs verification.

### 1.5 Error Handling (LOW — mostly works)

- [ ] **`?` operator propagation** — In DESIGN.md but needs verification that codegen correctly handles `?` in all positions (nested expressions, closures, etc.).
- [ ] **Custom error types** — No way to define errors that implement `std::error::Error`. Users must use `Result<T, String>`.
- [ ] **Error chaining** — No `.context()` or source-chain equivalent.

### 1.6 Missing Syntax from DESIGN.md

- [ ] **Struct update syntax** — `Task { done: true, ..task }` is shown in DESIGN.md examples. Verify codegen handles this correctly.
- [ ] **Selective use imports** — `use Math::{square, cube}` shown in DESIGN.md. Currently `use Module` emits `use module::*;` (glob). Selective imports may not work.
- [ ] **Range expressions** — `1..10`, `1..=10` — not clear if supported beyond `range()` builtin.
- [ ] **String raw literals** — No `r"..."` syntax.
- [ ] **Byte/char literals** — No `b"..."` or `'c'` character literal syntax.

---

## 2. Type System Gaps

### 2.1 Type Inference Limitations

- [ ] **Generic return type inference** — Functions returning generic types may not infer correctly without explicit annotations.
- [ ] **Closure type inference** — Lambda parameter types in complex positions (nested pipes, higher-order functions) may fail to infer.
- [ ] **Recursive function typing** — Mutual recursion between polymorphic functions may have edge cases.

### 2.2 Type System Missing Features

- [ ] **Type classes / interfaces** — Trait bounds don't propagate through the type checker. This means generic code that calls trait methods won't be validated until Rust compilation.
- [ ] **Variance tracking** — No covariance/contravariance analysis. Not critical for v1 but affects soundness of generic container types.
- [ ] **Numeric coercion** — Int and Float are distinct with no implicit coercion. DESIGN.md doesn't address mixed arithmetic — need to decide: error or auto-coerce?

---

## 3. Standard Library Gaps

### 3.1 Documented in DESIGN.md but Missing

| Category | DESIGN.md Mentions | Status |
|---|---|---|
| JSON | "Serialization formats: JSON" | ✅ Implemented (json_parse, json_encode, json_get, json_object, json_array) |
| CSV | "Serialization formats: CSV" | ❌ Not implemented |
| XML | "Serialization formats: XML" | ❌ Not implemented |
| Binary encoding | "Binary encoding" | ❌ Not implemented |
| Timezones | "Timezones and conversions" | ❌ Only UTC timestamps, no timezone support |
| Signals | "Signals" | ❌ Not implemented |
| Big integers | "Big integers / arbitrary precision" | ❌ Not implemented |
| Arg parsing (rich) | "Argument parsing" | ⚠️ Basic (arg_get, arg_count) — no flag parsing, help generation |
| Config file parsing | "Config file parsing (env, JSON, TOML, YAML)" | ⚠️ JSON only |
| Terminal utilities | "Terminal utilities (colors, input)" | ⚠️ ANSI colors exist, no raw terminal/TUI |

### 3.2 Recommended for v1 Minimum

- [ ] **CSV parsing** — Very commonly needed. A `csv_parse(text)` and `csv_encode(rows)` would cover most use cases.
- [ ] **TOML parsing** — Star uses Star.toml, so supporting TOML in user programs is natural.
- [ ] **Rich argument parsing** — A `parse_args()` that returns structured flags/options, or a declarative API.
- [ ] **Timezone support** — At minimum, UTC offset and format with timezone.

### 3.3 Stdlib Quality Issues

- [ ] **No documentation** — 290+ builtins have no user-facing docs. Users must read examples or source.
- [ ] **No type signatures exposed** — Builtins are recognized by name in codegen; users can't discover them via tooling.
- [ ] **Error messages for wrong arity** — Type checker validates arity, but error messages for builtins could be more helpful (e.g., "map takes 2 arguments: a list and a function").

---

## 4. Tooling & Developer Experience

### 4.1 Critical for v1

- [ ] **Error message quality** — When Rust compilation fails, users see raw `cargo build` output with Rust line numbers that don't map back to Star source. Need source-mapping or at minimum a wrapper that translates common errors.
- [ ] **Warnings system** — No unused variable warnings, no unreachable code warnings, no deprecation warnings. The compiler is silent unless something is a hard error.
- [ ] **`star test` improvements** — Currently runs `test_*` functions but has no assertion failure reporting with source locations, no test filtering, no `--verbose` output.

### 4.2 Important for v1

- [ ] **LSP server** — No language server protocol implementation. Without this, IDE support (autocomplete, go-to-definition, inline errors) is impossible. This is table-stakes for modern language adoption.
- [ ] **REPL** — No interactive mode. `star repl` would be extremely useful for exploration and learning.
- [ ] **Watch mode** — No `star watch` or `star run --watch` for auto-rebuild on file changes.
- [ ] **Source maps** — When rustc errors occur, no mapping from generated Rust line numbers back to Star source lines.
- [ ] **Documentation generation** — No `star doc` command. No doc-comment syntax (`##` or `///` equivalent).

### 4.3 Nice-to-have for v1

- [ ] **Incremental compilation** — Currently rebuilds everything on every change. For larger projects this will be slow.
- [ ] **Library output** — Only executables are supported. No way to build a `.star` library that other projects can depend on.
- [ ] **Dependency resolution** — `Star.toml` has a `[dependencies]` section but there's no package registry, no dependency fetching, no version resolution.
- [ ] **Playground** — A web-based playground (à la Rust Playground) for trying Star without installing.
- [ ] **Syntax highlighting** — TextMate grammar, tree-sitter grammar, or VS Code extension for Star files.

---

## 5. Formatter Gaps

- [ ] **Comment preservation** — Comments are discarded during parsing, so `star fmt` strips all comments. This is a **blocker** for real-world usage.
- [ ] **Configuration** — No options for indentation style, line width, trailing commas, etc.
- [ ] **Idempotency verification** — No guarantee that formatting is stable (fmt(fmt(x)) == fmt(x)).

---

## 6. Build System & Project Management

- [ ] **Star package dependencies** — `[dependencies]` in Star.toml is parsed but there's no registry or resolution mechanism. Star packages can't depend on other Star packages.
- [ ] **Dev dependencies** — No `[dev-dependencies]` section for test-only dependencies.
- [ ] **Build scripts** — No pre/post-build hooks.
- [ ] **Multi-binary projects** — No support for multiple entry points in one project.
- [ ] **Workspace support** — No monorepo / multi-package workspace.
- [ ] **Manifest completeness** — Only `name` and `version` fields; no authors, description, license, homepage, repository fields.
- [ ] **Lock file** — No Star.lock for reproducible builds.

---

## 7. Documentation

- [ ] **Language reference** — DESIGN.md is the only spec. Need a proper language reference covering all syntax, semantics, and edge cases.
- [ ] **Standard library reference** — 290+ builtins with no API docs. Need a searchable reference.
- [ ] **Tutorial / Getting Started** — No guided introduction for new users.
- [ ] **Error catalog** — No list of all compiler errors with explanations and fixes.
- [ ] **Migration guide** — Tips for Rust/Ruby/OCaml developers coming to Star.

---

## 8. Testing & Quality

### Current: 615 tests (592 unit + 23 integration)

- [ ] **End-to-end test coverage** — Integration tests compile Star to Rust and check output, but don't cover all 290+ builtins or edge cases.
- [ ] **Fuzzing** — No fuzz testing for parser or type checker. Important for a compiler.
- [ ] **Property-based tests** — No quickcheck/proptest for type inference or codegen.
- [ ] **Benchmark suite** — No compile-time or runtime benchmarks. Need baselines before optimizing.
- [ ] **Error message tests** — No snapshot tests for error message quality.
- [ ] **Formatter round-trip tests** — No tests verifying `parse(format(parse(src))) == parse(src)`.

---

## 9. Prioritized v1 Checklist

### P0 — Must ship (blocks v1)

| # | Feature | Effort | Why |
|---|---------|--------|-----|
| 1 | **Comment preservation in formatter** | Medium | `star fmt` destroying comments is unusable |
| 2 | **Rust error translation** | Medium | Users seeing raw Rust errors is confusing |
| 3 | **Trait bound enforcement** | Large | Generic code silently generates invalid Rust |
| 4 | **Pattern exhaustiveness warnings** | Medium | Silent non-exhaustive matches cause runtime panics |
| 5 | **Warnings system** (unused vars, unreachable code) | Medium | Silent compiler feels broken |
| 6 | **Stdlib documentation** | Medium | 290+ undocumented functions are undiscoverable |
| 7 | **Language reference documentation** | Large | No spec beyond DESIGN.md |
| 8 | **`&mut` codegen completion** | Medium | Mutable borrows are in the design but don't fully work |
| 9 | **Selective use imports** | Small | `use Module::{a, b}` is in DESIGN.md but may not work |
| 10 | **Match guard codegen** | Small | `when` guards are parsed but need verification |

### P1 — Should ship (significantly improves quality)

| # | Feature | Effort | Why |
|---|---------|--------|-----|
| 11 | **LSP server** (basic: errors, go-to-def) | Large | Table-stakes for language adoption |
| 12 | **REPL** | Medium | Essential for learning and exploration |
| 13 | **Watch mode** | Small | `star run --watch` for fast iteration |
| 14 | **Syntax highlighting** (VS Code extension) | Small | Basic syntax coloring for .star files |
| 15 | **`star test` improvements** | Small | Test filtering, better failure output |
| 16 | **Struct update syntax verification** | Small | `{ field: val, ..base }` from DESIGN.md |
| 17 | **Numeric type coercion story** | Small | Decide and document Int/Float interaction |
| 18 | **Tutorial / Getting Started guide** | Medium | First thing new users need |
| 19 | **CSV stdlib** | Small | Very commonly needed |
| 20 | **TOML stdlib** | Small | Natural fit given Star.toml |

### P2 — Nice to have (can ship after v1)

| # | Feature | Effort | Why |
|---|---------|--------|-----|
| 21 | Package registry & dependency resolution | Very Large | Ecosystem feature, not needed for single-project use |
| 22 | Incremental compilation | Large | Only matters for large projects |
| 23 | Library output (not just executables) | Medium | Needed for packages ecosystem |
| 24 | Web playground | Medium | Marketing / adoption |
| 25 | Fuzzing & property-based tests | Medium | Quality, not user-facing |
| 26 | Associated types | Medium | Advanced trait feature |
| 27 | Super-traits | Medium | Advanced trait feature |
| 28 | Lifetime annotations in codegen | Large | Only needed for zero-copy perf paths |
| 29 | Move semantics (`~T`) | Medium | Perf optimization, clone-by-default works |
| 30 | Higher-kinded types | Very Large | Academic, not practical for v1 |
| 31 | Timezone support | Medium | Can use raw timestamps for now |
| 32 | Signal handling | Small | Niche use case |
| 33 | Big integers | Medium | Niche use case |
| 34 | Formatter configuration | Medium | Opinionated defaults are fine for v1 |
| 35 | Workspace / monorepo support | Large | Single-project is fine for v1 |

---

## 10. Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Users hit raw Rust errors they can't understand | HIGH | HIGH | P0 #2: Error translation layer |
| Trait-heavy code silently generates broken Rust | HIGH | MEDIUM | P0 #3: Trait bound enforcement |
| Formatter destroys comments in real codebases | HIGH | HIGH | P0 #1: Comment preservation |
| No IDE support limits adoption to CLI enthusiasts | MEDIUM | HIGH | P1 #11: Basic LSP |
| 290+ builtins are undiscoverable without docs | MEDIUM | HIGH | P0 #6: Stdlib docs |
| Complex generic code fails at Rust compile time | MEDIUM | MEDIUM | P0 #3: Better type checking |

---

## Appendix: Files Audited

| File | Lines | Status |
|------|-------|--------|
| src/ast.rs | 303 | Complete for current features |
| src/lexer.rs | 1,334 | Complete, well-tested |
| src/parser.rs | 2,625 | Complete with error recovery |
| src/typeck.rs | 2,701 | Core inference works; trait/generic checking incomplete |
| src/codegen.rs | 6,685 | Largest file; 290+ builtins; needs &mut and trait dispatch |
| src/resolver.rs | 247 | Works for file-based modules; no package resolution |
| src/optimize.rs | 218 | Conservative clone elimination; correct but limited |
| src/borrow.rs | 1,026 | String/Vec inference works; limited scope |
| src/formatter.rs | 1,349 | Full AST formatting; no comment preservation |
| src/manifest.rs | 479 | Basic Star.toml; no package deps resolution |
| src/main.rs | 358 | 11 CLI commands; no watch/repl |
| src/error.rs | 99 | Basic span tracking |
| tests/integration.rs | 537 | 23 integration tests |
| **Total** | **~17,961** | |
