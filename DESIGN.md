# Star (★) Language Design

## 1. Philosophy & Overview

Star is a functional-first language with Ruby-like syntax that compiles to idiomatic Rust.

**Core principles:**
- **Simplicity over cleverness** — minimal syntax, maximum clarity
- **Zero-cost compilation** — Star features map directly to Rust constructs with no runtime overhead
- **Delegate the hard parts** — ownership/borrowing are Rust's problem; Star generates code that lets `rustc` enforce safety
- **Expression-oriented** — everything is an expression, everything returns a value

**Design tradeoffs:**

| Decision | Star | Why |
|----------|------|-----|
| Ownership | Implicit (clone-by-default, opt-in borrowing) | Ergonomics over control |
| Mutability | Explicit `mut` keyword | Safety by default |
| GC | None | Compiles to Rust's ownership model |
| Runtime | None | Zero-cost, pure compilation |
| Null | None — `Option` only | Compile-time safety |
| Exceptions | None — `Result` only | Explicit error handling |

**Comparison:**

| Feature | Star | Rust | OCaml | Ruby |
|---------|------|------|-------|------|
| Syntax noise | Low | High | Medium | Low |
| Type safety | Strong | Strong | Strong | Weak |
| Ownership | Implicit | Explicit | GC | GC |
| Pattern matching | Yes | Yes | Yes | Limited |
| First-class functions | Yes | Yes | Yes | Yes |
| Zero-cost abstractions | Yes | Yes | No | No |
| Compilation target | Rust | Native | Native/Bytecode | Interpreted |

---

## 2. Syntax Specification

### 2.1 Grammar (EBNF)

```ebnf
program        = { module_decl | use_decl | type_decl | fn_decl | expr } ;

(* Modules *)
module_decl    = "module" IDENT block "end" ;
use_decl       = "use" module_path [ "::" "{" ident_list "}" ] ;
module_path    = IDENT { "::" IDENT } ;

(* Type declarations *)
type_decl      = "type" IDENT [ type_params ] "=" type_body ;
type_params    = "<" IDENT { "," IDENT } ">" ;
type_body      = variant { "|" variant }
               | struct_body ;
variant        = IDENT [ "(" type_list ")" ] ;
struct_body    = "{" field_decl { "," field_decl } "}" ;
field_decl     = IDENT ":" type_expr ;

(* Functions *)
fn_decl        = [ "pub" ] "fn" IDENT [ type_params ] "(" param_list ")" [ ":" type_expr ] "=" expr_block ;
param_list     = [ param { "," param } ] ;
param          = IDENT [ ":" type_expr ] ;
lambda         = "fn" "(" param_list ")" "=>" expr ;

(* Expressions *)
expr_block     = expr
               | NEWLINE INDENT { statement NEWLINE } expr NEWLINE DEDENT ;
statement      = let_binding | expr ;
let_binding    = "let" [ "mut" ] pattern [ ":" type_expr ] "=" expr ;

expr           = pipe_expr ;
pipe_expr      = logic_expr { "|>" logic_expr } ;
logic_expr     = comp_expr { ( "and" | "or" ) comp_expr } ;
comp_expr      = add_expr { ( "==" | "!=" | "<" | ">" | "<=" | ">=" ) add_expr } ;
add_expr       = mul_expr { ( "+" | "-" ) mul_expr } ;
mul_expr       = unary_expr { ( "*" | "/" | "%" ) unary_expr } ;
unary_expr     = [ "not" | "-" ] call_expr ;
call_expr      = primary { "(" arg_list ")" | "." IDENT } ;
primary        = IDENT | literal | "(" expr ")" | if_expr | match_expr
               | list_expr | lambda | do_block | rust_block ;

(* Control flow *)
if_expr        = "if" expr "then" expr_block [ "else" expr_block ] "end" ;
match_expr     = "match" expr NEWLINE { match_arm } "end" ;
match_arm      = "|" pattern [ "when" expr ] "=>" expr_block NEWLINE ;

(* Patterns *)
pattern        = "_"
               | IDENT
               | literal
               | IDENT "(" pattern_list ")"
               | "(" pattern_list ")"
               | "[" pattern_list [ "|" IDENT ] "]"
               | pattern "as" IDENT ;

(* Types *)
type_expr      = IDENT [ "<" type_list ">" ]
               | "fn" "(" type_list ")" "->" type_expr
               | "(" type_list ")"  ;
type_list      = [ type_expr { "," type_expr } ] ;

(* Literals *)
literal        = INT | FLOAT | STRING | "true" | "false" | "nil" ;
list_expr      = "[" [ expr { "," expr } ] "]" ;

(* Interop *)
do_block       = "do" NEWLINE { statement NEWLINE } "end" ;
rust_block     = "rust!" STRING ;

(* Misc *)
ident_list     = IDENT { "," IDENT } ;
arg_list       = [ expr { "," expr } ] ;
pattern_list   = [ pattern { "," pattern } ] ;
```

### 2.2 Indentation

Star uses **significant newlines** but not significant indentation for blocks. Blocks are delimited by keywords (`end`, `=>`). Single-expression bodies can be inline.

### 2.3 Core Constructs — Examples

```star
# Variable binding
let name = "Star"
let mut counter = 0

# Function
fn greet(name: String): String =
  "Hello, " + name + "!"

# Multi-line function
fn factorial(n: Int): Int =
  if n <= 1 then 1
  else n * factorial(n - 1)
  end

# Lambda
let double = fn(x) => x * 2

# Pipe operator
let result = [1, 2, 3, 4, 5]
  |> map(fn(x) => x * 2)
  |> filter(fn(x) => x > 4)
  |> fold(0, fn(acc, x) => acc + x)

# Pattern matching
fn describe(shape: Shape): String =
  match shape
  | Circle(r) => "Circle with radius " + r.to_string()
  | Rectangle(w, h) => "Rectangle " + w.to_string() + "x" + h.to_string()
  | Point => "A point"
  end

# If expression
let status = if count > 0 then "has items" else "empty" end

# Algebraic data types
type Shape =
  | Circle(Float)
  | Rectangle(Float, Float)
  | Point

# Generic type
type Option<T> =
  | Some(T)
  | None

# Struct-like type
type Person = {
  name: String,
  age: Int
}

# Module
module Math
  pub fn square(x: Int): Int = x * x
  pub fn cube(x: Int): Int = x * x * x
end

# Using modules
use Math::{square, cube}

# Do block (imperative)
do
  let mut total = 0
  total = total + square(3)
  total = total + cube(2)
  total
end

# Rust interop
rust!("println!(\"raw rust here\");")
```

---

## 3. Type System Design

### 3.1 Type Inference Strategy

Star uses **Hindley-Milner type inference** (Algorithm W) extended with:
- Literal type defaulting (`42` → `Int`, `3.14` → `Float`)
- Return type inference from function body
- Generic instantiation at call sites

**Rules:**
1. All function parameters in public APIs must have type annotations
2. Local bindings are fully inferred
3. Function return types are inferred but can be annotated
4. Generic type parameters are inferred at call sites

### 3.2 Primitive Types

| Star Type | Rust Type |
|-----------|-----------|
| `Int` | `i64` |
| `Float` | `f64` |
| `Bool` | `bool` |
| `String` | `String` |
| `Char` | `char` |
| `()` | `()` |

### 3.3 ADT Representation

Star ADTs map directly to Rust enums:

```star
type Result<T, E> =
  | Ok(T)
  | Err(E)
```

Generates:

```rust
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

Struct types map to Rust structs:

```star
type Point = { x: Float, y: Float }
```

Generates:

```rust
#[derive(Debug, Clone, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}
```

### 3.4 Generics

Star generics map 1:1 to Rust generics. Trait bounds are inferred from usage or annotated explicitly:

```star
fn max<T: Ord>(a: T, b: T): T =
  if a > b then a else b end
```

Generates:

```rust
fn max<T: Ord>(a: T, b: T) -> T {
    if a > b { a } else { b }
}
```

### 3.5 Ownership Strategy

Star uses a **clone-by-default** strategy for simplicity:
- Values are cloned when passed to functions (unless the compiler can prove a move is safe)
- `&` prefix opts into borrowing: `fn len(s: &String): Int`
- `&mut` for mutable borrows: `fn push(list: &mut List<T>, item: T)`
- The generated Rust is always valid — `rustc` is the final arbiter of safety

This means Star code might clone more than hand-written Rust, but:
1. The compiler optimizes away unnecessary clones
2. Users can opt into borrowing for hot paths
3. Correctness is never compromised

---

## 4. Core Features — Side-by-Side Examples

### 4.1 Functions

**Star:**
```star
fn add(a: Int, b: Int): Int = a + b

fn apply_twice(f: fn(Int) -> Int, x: Int): Int =
  f(f(x))

let result = apply_twice(fn(x) => x + 1, 5)
```

**Rust output:**
```rust
fn add(a: i64, b: i64) -> i64 {
    a + b
}

fn apply_twice(f: impl Fn(i64) -> i64, x: i64) -> i64 {
    f(f(x))
}

fn main() {
    let result: i64 = apply_twice(|x| x + 1, 5);
}
```

### 4.2 Pipe Operator

**Star:**
```star
fn double(x: Int): Int = x * 2
fn add_one(x: Int): Int = x + 1

let result = 5 |> double |> add_one |> double
```

**Rust output:**
```rust
fn double(x: i64) -> i64 { x * 2 }
fn add_one(x: i64) -> i64 { x + 1 }

fn main() {
    let result: i64 = double(add_one(double(5)));
}
```

### 4.3 Pattern Matching

**Star:**
```star
type Expr =
  | Num(Int)
  | Add(Expr, Expr)
  | Mul(Expr, Expr)

fn eval(e: Expr): Int =
  match e
  | Num(n) => n
  | Add(a, b) => eval(a) + eval(b)
  | Mul(a, b) => eval(a) * eval(b)
  end
```

**Rust output:**
```rust
enum Expr {
    Num(i64),
    Add(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
}

fn eval(e: Expr) -> i64 {
    match e {
        Expr::Num(n) => n,
        Expr::Add(a, b) => eval(*a) + eval(*b),
        Expr::Mul(a, b) => eval(*a) * eval(*b),
    }
}
```

Note: Star automatically boxes recursive enum variants.

### 4.4 Modules

**Star:**
```star
module StringUtils
  pub fn upcase(s: String): String =
    s.to_uppercase()

  pub fn words(s: String): List<String> =
    s.split(" ").collect()
end

use StringUtils::{upcase, words}

let title = "hello world" |> upcase
```

**Rust output:**
```rust
mod string_utils {
    pub fn upcase(s: String) -> String {
        s.to_uppercase()
    }

    pub fn words(s: String) -> Vec<String> {
        s.split(" ").map(|s| s.to_string()).collect()
    }
}

use string_utils::{upcase, words};

fn main() {
    let title: String = upcase("hello world".to_string());
}
```

### 4.5 List Operations

**Star:**
```star
let numbers = [1, 2, 3, 4, 5]

let evens = numbers
  |> filter(fn(x) => x % 2 == 0)
  |> map(fn(x) => x * 10)

let sum = evens |> fold(0, fn(acc, x) => acc + x)
```

**Rust output:**
```rust
fn main() {
    let numbers: Vec<i64> = vec![1, 2, 3, 4, 5];

    let evens: Vec<i64> = numbers.iter()
        .filter(|x| *x % 2 == 0)
        .map(|x| x * 10)
        .collect();

    let sum: i64 = evens.iter().fold(0, |acc, x| acc + x);
}
```

---

## 5. Compiler Architecture

```
Source (.star)
     │
     ▼
  ┌──────┐
  │ Lexer │  → Token stream
  └──┬───┘
     ▼
  ┌───────┐
  │ Parser │  → AST (concrete syntax tree)
  └──┬────┘
     ▼
  ┌──────────┐
  │ Resolver  │  → Name resolution, module linking
  └──┬───────┘
     ▼
  ┌─────────────┐
  │ Type Checker │  → Typed AST (Hindley-Milner inference)
  └──┬──────────┘
     ▼
  ┌─────────┐
  │ Lowering │  → HIR (ownership insertion, Box insertion, clone insertion)
  └──┬──────┘
     ▼
  ┌──────────┐
  │ Rust      │  → Rust source code (.rs files)
  │ Codegen   │
  └──┬───────┘
     ▼
  ┌───────┐
  │ rustc  │  → Binary
  └───────┘
```

**Key decisions:**

1. **Borrow checking is delegated to `rustc`** — Star inserts clones conservatively, then relies on Rust's compiler for safety. This means Star never rejects a program that Rust would accept.

2. **Box insertion** — Recursive types are automatically boxed. The type checker detects cycles in type definitions and inserts `Box` at the appropriate positions.

3. **Clone insertion** — By default, values are cloned when used in multiple places. An optimization pass removes unnecessary clones when it can prove the value is used linearly.

4. **Pipe desugaring** — `a |> f` desugars to `f(a)` and `a |> f(b)` desugars to `f(a, b)` (first-argument insertion) during parsing.

---

## 6. Rust Interop

### 6.1 Calling Rust from Star

Use `rust!` for inline Rust expressions:

```star
fn now(): Int =
  rust!("std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_secs() as i64")
```

### 6.2 External Rust functions

```star
extern fn std_fs_read_to_string(path: String): Result<String, String>
```

Maps to calling a Rust function directly in the generated code.

### 6.3 Embedding raw Rust blocks

For larger interop needs:

```star
rust! {
  use std::collections::HashMap;

  fn custom_hash_thing() -> HashMap<String, i64> {
      let mut m = HashMap::new();
      m.insert("key".to_string(), 42);
      m
  }
}
```

---

## 7. Error Handling

Star uses `Result<T, E>` and `Option<T>` — no exceptions, no panics (by default).

### 7.1 The `?` operator

```star
fn read_file(path: String): Result<String, Error> =
  let content = File.read(path)?
  content |> trim
```

Generates standard Rust `?` operator usage.

### 7.2 Pattern matching on errors

```star
match File.read("config.toml")
| Ok(content) => parse(content)
| Err(e) => default_config()
end
```

### 7.3 `or_else` chains

```star
let config = File.read("config.toml")
  |> or_else(fn(_) => File.read("default.toml"))
  |> unwrap_or(DEFAULT_CONFIG)
```

---

## 8. Tooling

### 8.1 CLI Design

```
star build [file.star]      # Compile to Rust and build
star check [file.star]      # Type-check only
star emit-rust [file.star]  # Output generated Rust to stdout
star run [file.star]        # Build and run
star fmt [file.star]        # Format source code
star new [project-name]     # Create new project
```

### 8.2 Build System

Star projects use a `Star.toml` manifest:

```toml
[package]
name = "my-project"
version = "0.1.0"

[dependencies]
# Star packages
http = "1.0"

[rust-dependencies]
# Direct Rust crate dependencies
serde = { version = "1.0", features = ["derive"] }
```

The compiler generates a Cargo project in `.star-build/`, manages the `Cargo.toml`, and invokes `cargo build`.

### 8.3 Project Structure

```
my-project/
  Star.toml
  src/
    main.star
    utils.star
  .star-build/          # Generated — gitignored
    Cargo.toml
    src/
      main.rs
      utils.rs
```

---

## 9. Bonus Features

### 9.1 Async/Await

```star
async fn fetch(url: String): Result<String, Error> =
  let response = Http.get(url).await?
  response.body()

async fn main() =
  let data = fetch("https://example.com").await?
  println(data)
```

Generates Rust `async fn` with `tokio` runtime:

```rust
async fn fetch(url: String) -> Result<String, Box<dyn std::error::Error>> {
    let response = reqwest::get(&url).await?;
    Ok(response.text().await?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = fetch("https://example.com".to_string()).await?;
    println!("{}", data);
    Ok(())
}
```

### 9.2 Ownership Annotations

When performance matters, Star supports explicit ownership control:

```star
# Borrow (read-only reference)
fn len(s: &String): Int = s.len()

# Mutable borrow
fn push(list: &mut List<Int>, item: Int) =
  list.push(item)

# Move (take ownership — no clone)
fn consume(s: ~String) =
  println(s)
```

The `~` prefix means "move, do not clone" — useful in hot paths.

### 9.3 Complete Example Program

**Star source (`main.star`):**

```star
type Task = {
  title: String,
  done: Bool,
  priority: Int
}

fn new_task(title: String, priority: Int): Task =
  Task { title: title, done: false, priority: priority }

fn complete(task: Task): Task =
  Task { done: true, ..task }

fn high_priority(task: &Task): Bool =
  task.priority >= 5

fn format_task(task: &Task): String =
  let status = if task.done then "✓" else "○" end
  status + " [" + task.priority.to_string() + "] " + task.title

fn main() =
  let tasks = [
    new_task("Design Star language", 10),
    new_task("Write parser", 8),
    new_task("Buy groceries", 2),
    new_task("Implement codegen", 9)
  ]

  let important = tasks
    |> filter(high_priority)
    |> map(complete)
    |> map(format_task)

  important |> each(println)
```

**Generated Rust:**

```rust
#[derive(Debug, Clone, PartialEq)]
struct Task {
    title: String,
    done: bool,
    priority: i64,
}

fn new_task(title: String, priority: i64) -> Task {
    Task {
        title,
        done: false,
        priority,
    }
}

fn complete(task: Task) -> Task {
    Task {
        done: true,
        ..task
    }
}

fn high_priority(task: &Task) -> bool {
    task.priority >= 5
}

fn format_task(task: &Task) -> String {
    let status = if task.done { "✓" } else { "○" };
    format!("{} [{}] {}", status, task.priority, task.title)
}

fn main() {
    let tasks: Vec<Task> = vec![
        new_task("Design Star language".to_string(), 10),
        new_task("Write parser".to_string(), 8),
        new_task("Buy groceries".to_string(), 2),
        new_task("Implement codegen".to_string(), 9),
    ];

    let important: Vec<String> = tasks
        .into_iter()
        .filter(|t| high_priority(t))
        .map(|t| complete(t))
        .map(|t| format_task(&t))
        .collect();

    for item in &important {
        println!("{}", item);
    }
}
```

Standard Library Features to Add:
1. Core Data Types & Collections

Fundamental building blocks for any program.
	•	Primitive types: integers, floats, booleans, strings, chars
	•	Collections:
	•	Arrays / slices
	•	Lists / vectors
	•	Hash maps / dictionaries
	•	Sets
	•	Queues / stacks / heaps (priority queues)
	•	Iterators / sequences: lazy or eager traversal abstractions

⸻

2. String & Text Processing

Almost every language invests heavily here.
	•	String manipulation (split, join, trim, replace)
	•	Encoding/decoding (UTF-8, UTF-16, base64)
	•	Regular expressions
	•	Formatting / templating (e.g., printf, interpolation)

⸻

3. I/O (Input / Output)

Abstractions over reading/writing data.
	•	File system operations (read/write files, directories)
	•	Streams (buffered I/O, stdin/stdout/stderr)
	•	Serialization formats:
	•	JSON
	•	XML
	•	CSV
	•	Binary encoding

⸻

4. Concurrency & Parallelism

Varies a lot by language, but usually present.
	•	Threads / thread pools
	•	Async/await or futures/promises
	•	Synchronization primitives:
	•	Mutexes
	•	RW locks
	•	Channels / message passing
	•	Timers / scheduling

⸻

5. Error Handling

Mechanisms to deal with failure.
	•	Exceptions (Java, Python)
	•	Result types (Rust, Swift)
	•	Option/Maybe types
	•	Stack traces / error propagation utilities

⸻

6. Math & Numeric Utilities

Core numeric functionality.
	•	Basic math (sin, cos, sqrt, log, etc.)
	•	Random number generation
	•	Big integers / arbitrary precision (in some languages)
	•	Statistics / numeric helpers (sometimes minimal)

⸻

7. Date, Time & Timezones

Handling time is universally needed.
	•	Timestamps, durations
	•	Formatting/parsing dates
	•	Timezones and conversions
	•	Clocks (system, monotonic)

⸻

8. Networking

Basic networking primitives.
	•	TCP/UDP sockets
	•	HTTP clients (sometimes servers)
	•	DNS resolution
	•	URL parsing

⸻

9. OS & Environment Interaction

Bridge to the operating system.
	•	Environment variables
	•	Process management (spawn, kill, pipes)
	•	Signals
	•	File permissions / metadata

⸻

10. Reflection / Introspection (language-dependent)

More common in dynamic languages.
	•	Inspect types at runtime
	•	Metadata about functions/classes
	•	Dynamic invocation

⸻

11. Memory & Resource Management

Usually partially exposed.
	•	Garbage collection APIs (if GC language)
	•	Smart pointers / ownership helpers (e.g., Rust’s Rc, Arc)
	•	Resource cleanup (RAII, defer, finally)

⸻

12. Testing & Debugging Utilities

Often bundled or semi-standard.
	•	Assertions
	•	Unit testing frameworks
	•	Logging
	•	Profiling hooks

⸻

13. Collections Algorithms & Utilities

Higher-level operations on data.
	•	Sorting, searching
	•	Map/filter/reduce
	•	Aggregations
	•	Comparators

⸻

14. Cryptography & Security (sometimes std, sometimes separate)

Varies by philosophy.
	•	Hashing (SHA, MD5)
	•	Random secure generators
	•	TLS/SSL (often not in core stdlib, but close)

⸻

15. Configuration & CLI Utilities

Common in modern languages.
	•	Argument parsing
	•	Config file parsing (env, JSON, TOML, YAML)
	•	Terminal utilities (colors, input)

⸻

Subtle Differences Across Languages
	•	Minimal stdlib (e.g., C, Rust core):
	•	Smaller, composable, relies on ecosystem crates
	•	Batteries-included (e.g., Python, Go):
	•	Rich networking, HTTP servers, parsing, etc.
	•	Enterprise-heavy (e.g., Java):
	•	Extensive APIs but sometimes verbose

⸻


The minimum viable stdlib usually includes:
	•	Collections
	•	String processing
	•	File I/O
	•	Error handling
	•	Basic concurrency
	•	Time/date
	•	JSON serialization
