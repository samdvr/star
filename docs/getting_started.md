# Getting Started with Star

Star is a functional-first language with Ruby-like syntax that compiles to idiomatic Rust. It gives you the expressiveness of a high-level language with the performance of native code.

## Installation

Build from source (requires Rust 1.85+):

```sh
git clone https://github.com/example/star.git
cd star
cargo install --path .
```

Verify the installation:

```sh
star --version
```

## Your First Program

Create a file called `hello.star`:

```star
fn main() =
  println("Hello from Star!")
```

Run it:

```sh
star run hello.star
```

This compiles your Star code to Rust, builds it, and runs the resulting binary. You can also just compile without running:

```sh
star build hello.star
```

## Creating a Project

For anything beyond a single file, use the project system:

```sh
star new my-project
cd my-project
```

This creates:

```
my-project/
  Star.toml        # Project manifest
  src/main.star    # Entry point
  .gitignore       # Ignores .star-build/
```

Build and run with:

```sh
star build    # Compile
star run      # Compile and run
star check    # Type-check only (fast)
```

## Language Basics

### Variables

```star
let name = "Alice"
let age = 30
let pi = 3.14
let items = [1, 2, 3]
```

Variables are immutable by default. Use `let mut` inside `do` blocks for mutable bindings.

### Functions

Functions are defined with `fn`, use `=` for the body, and the last expression is the return value:

```star
fn greet(name: String): String =
  "Hello, #{name}!"

fn add(a: Int, b: Int): Int =
  a + b

fn main() =
  println(greet("World"))
```

Star infers types in most cases, but you can annotate parameters and return types for clarity.

### Multi-Statement Functions

When a function needs multiple steps, each statement is part of an implicit block:

```star
fn describe(x: Int): String =
  let doubled = x * 2
  let label = if doubled > 10 then "big" else "small" end
  "#{x} doubled is #{doubled} (#{label})"
```

### If Expressions

`if` is an expression that returns a value:

```star
let status = if age >= 18 then "adult" else "minor" end
```

Multi-line form:

```star
let message = if score > 90 then
  "excellent"
else if score > 70 then
  "good"
else
  "keep trying"
end
```

### Pattern Matching

`match` is Star's most powerful control flow, similar to Rust's `match` or ML's `case`:

```star
type Shape =
  | Circle(Float)
  | Rectangle(Float, Float)

fn area(s: Shape): Float =
  match s
  | Circle(r) => 3.14159 * r * r
  | Rectangle(w, h) => w * h
  end
```

Matching on values with guards:

```star
fn classify(n: Int): String =
  match n
  | 0 => "zero"
  | 1 | 2 | 3 => "small"
  | n when n < 0 => "negative"
  | _ => "large"
  end
```

### Lists and Pipes

Lists are the primary collection type:

```star
let numbers = [1, 2, 3, 4, 5]

let result = numbers
  |> filter(fn(x) => x > 2)
  |> map(fn(x) => x * 10)
  |> sum()
```

The pipe operator `|>` feeds the result of the left side as the first argument to the right side. It's the idiomatic way to chain operations.

### Lambdas

Anonymous functions use `fn(params) => body`:

```star
let double = fn(x) => x * 2
let add = fn(a, b) => a + b

[1, 2, 3] |> map(fn(x) => x * x)
```

### Do Blocks

When you need imperative-style code with multiple statements and mutation:

```star
fn compute(): Int =
  do
    let mut total = 0
    for x in [1, 2, 3, 4, 5] do
      total += x
    end
    total
  end
```

### Loops

For loops iterate over collections:

```star
for item in [1, 2, 3] do
  println(to_string(item))
end
```

While loops for conditional repetition:

```star
let mut n = 10
while n > 0 do
  println(to_string(n))
  n -= 1
end
```

Both support `break` and `continue`.

### String Interpolation

Embed expressions in strings with `#{}`:

```star
let name = "Star"
let version = 1
println("Welcome to #{name} v#{version}!")
println("1 + 2 = #{1 + 2}")
```

Triple-quoted strings for multi-line text:

```star
let html = """
  <html>
    <body>#{content}</body>
  </html>
  """
```

## Type System

### Algebraic Data Types

Define sum types (enums) and product types (structs):

```star
# Sum type (enum)
type Result<T, E> =
  | Ok(T)
  | Err(E)

type Color =
  | Red
  | Green
  | Blue
  | Custom(Int, Int, Int)

# Product type (struct)
type User = {
  name: String,
  email: String,
  age: Int
}
```

Construct and use them:

```star
let user = User { name: "Alice", email: "alice@example.com", age: 30 }
println(user.name)

let color = Custom(255, 128, 0)
```

### Type Parameters

Functions and types can be generic:

```star
fn first<T>(list: List<T>): T =
  head(list)

type Pair<A, B> = {
  left: A,
  right: B
}
```

### Primitive Types

| Star Type | Rust Type | Description |
|-----------|-----------|-------------|
| `Int`     | `i64`     | 64-bit signed integer |
| `Float`   | `f64`     | 64-bit float |
| `String`  | `String`  | UTF-8 string |
| `Bool`    | `bool`    | Boolean |
| `List<T>` | `Vec<T>`  | Dynamic array |

Additional numeric types: `Int8`, `Int16`, `Int32`, `UInt`, `UInt8`, `UInt16`, `UInt32`, `Float32`.

Collection types: `Map<K,V>`, `Set<T>`, `Deque<T>`, `Heap<T>`.

## Traits and Implementations

Define interfaces with `trait` and implement them with `impl`:

```star
trait Describable
  fn describe(self): String
end

type Dog = { name: String, breed: String }

impl Describable for Dog
  fn describe(self): String =
    "#{self.name} the #{self.breed}"
end

fn main() =
  let dog = Dog { name: "Rex", breed: "Labrador" }
  println(dog.describe())
```

Inherent methods (no trait):

```star
impl Dog
  fn bark(self): String = "Woof!"
end
```

## Error Handling

Star uses `Result<T, E>` and `Option<T>` for error handling, not exceptions:

```star
fn divide(a: Float, b: Float): Result<Float, String> =
  if b == 0.0 then
    Err("division by zero")
  else
    Ok(a / b)
  end

fn main() =
  match divide(10.0, 3.0)
  | Ok(result) => println("Result: #{to_string(result)}")
  | Err(msg) => println("Error: #{msg}")
  end
```

Convenience functions: `unwrap`, `unwrap_or`, `map_result`, `and_then`, `is_ok`, `is_err`.

The try operator `?` propagates errors:

```star
fn process(): Result<String, String> =
  let data = read_file("input.txt")?
  Ok(uppercase(data))
```

## Modules

### Multi-File Projects

Create a module in a separate file:

```star
# math.star
pub fn square(x: Int): Int = x * x
pub fn cube(x: Int): Int = x * x * x
```

Import and use it:

```star
# main.star
use Math

fn main() =
  println(to_string(square(5)))
```

The `use Math` declaration looks for `math.star` in the same directory. Only `pub` functions are accessible.

### Inline Modules

```star
module Helpers
  pub fn double(x: Int): Int = x * 2
end

fn main() =
  println(to_string(Helpers::double(5)))
```

## Testing

Write test functions with the `test_` prefix:

```star
fn add(a: Int, b: Int): Int = a + b

fn test_add() =
  assert_eq(add(2, 3), 5)

fn test_add_negative() =
  assert_eq(add(-1, 1), 0)
```

Run tests:

```sh
star test my_tests.star
```

Filter tests by name:

```sh
star test --filter add
```

Verbose output with per-test timing:

```sh
star test --verbose
```

Output:

```
running 2 tests...
  running test_add...
  PASS: test_add (0ms)
  running test_add_negative...
  PASS: test_add_negative (0ms)

2 passed, 0 failed (1ms)
```

## Project Manifest (Star.toml)

```toml
[package]
name = "my-project"
version = "0.1.0"
description = "My Star project"
authors = ["Alice"]
license = "MIT"

[dependencies]
serde = "1"
tokio = { version = "1", features = ["full"] }

[dev-dependencies]
criterion = "0.5"
```

Dependencies are Rust crates — they're included in the generated Cargo.toml. Common crates like `regex`, `base64`, and `tokio` are auto-detected and added without manual declaration.

Dev-dependencies are only included when running `star test`.

## CLI Reference

```
star build [file.star]       Compile a Star program
star run [file.star]         Compile and run
star check [file.star]       Type-check only
star emit-rust [file.star]   Print generated Rust code
star test [file.star]        Run test functions
star fmt [file.star]         Format source code
star new <name>              Create a new project
star init                    Initialize project in current directory
star clean                   Remove build artifacts
star repl                    Start an interactive REPL session
star lsp                     Start the Language Server Protocol server

Options:
  --release                  Build in release mode
  --watch                    Recompile on file changes (build/run)
  --filter <pattern>         Filter tests by name substring
  --verbose, -v              Verbose test output with timing
  -h, --help                 Show help
  -V, --version              Show version
```

## The Pipe Operator

The pipe operator `|>` is central to Star's style. It takes the result of the left-hand side and passes it as the first argument to the function on the right-hand side.

### Basic Piping

```star
fn main() =
  let result = "hello world"
    |> uppercase()
    |> split(" ")
    |> head()
  println(result)
```

Each step feeds into the next, reading top to bottom like a series of transformations.

### Multi-Line Pipes

Pipes work naturally across multiple lines. Star treats `|>` at the start of a line as a continuation of the previous expression:

```star
fn main() =
  let stats = [4, 7, 2, 9, 1, 8, 3, 6, 5]
    |> filter(fn(x) => x > 3)
    |> sort()
    |> map(fn(x) => x * 10)
    |> reverse()
  println(to_string(stats))
```

### Writing Pipe-Friendly Functions

To work well with pipes, design functions so the "data" argument comes first:

```star
fn keep_above(items: List<Int>, threshold: Int): List<Int> =
  filter(items, fn(x) => x > threshold)

fn label(items: List<Int>, prefix: String): List<String> =
  map(items, fn(x) => "#{prefix}: #{to_string(x)}")

fn main() =
  [1, 2, 3, 4, 5]
    |> keep_above(3)
    |> label("value")
    |> each(fn(s) => println(s))
```

When `keep_above(3)` appears on the right side of `|>`, the piped list fills in the first argument, so it becomes `keep_above(list, 3)`.

### Combining Pipes with Pattern Matching

Pipes compose well with other Star features:

```star
fn classify(n: Int): String =
  match n
  | 0 => "zero"
  | n when n > 0 => "positive"
  | _ => "negative"
  end

fn main() =
  [-3, 0, 5, -1, 7]
    |> map(fn(x) => classify(x))
    |> each(fn(s) => println(s))
```

## Module System

Star supports both inline modules and external file modules. All modules use `pub` to control visibility.

### Inline Modules

Define a module directly in your source file with `module...end`:

```star
module StringUtils
  pub fn shout(s: String): String =
    uppercase(s)

  pub fn whisper(s: String): String =
    lowercase(s)
end

fn main() =
  println(StringUtils::shout("hello"))
  println(StringUtils::whisper("HELLO"))
```

Functions without `pub` are private to the module and cannot be called from outside.

### External File Modules

Split code across files by creating a separate `.star` file and importing it with `use`:

```star
# utils.star
pub fn double(x: Int): Int = x * 2
pub fn triple(x: Int): Int = x * 3
```

```star
# main.star
use Utils

fn main() =
  println(to_string(double(5)))
  println(to_string(triple(5)))
```

The `use Utils` declaration looks for `utils.star` in the same directory (lowercased). All `pub` items from the module are brought into scope.

### Pub Visibility

Only `pub`-marked functions and types are accessible outside their module:

```star
module Auth
  # Private — only callable within Auth
  fn hash_password(pw: String): String =
    sha256(pw)

  # Public — callable from outside
  pub fn create_user(name: String, pw: String): String =
    "#{name}:#{hash_password(pw)}"
end

fn main() =
  println(Auth::create_user("alice", "secret123"))
  # Auth::hash_password("x")  -- this would be an error
```

### Nested Modules and Transitive Imports

External modules can themselves `use` other modules. Star resolves these transitively and detects circular dependencies:

```star
# math.star
pub fn square(x: Int): Int = x * x

# geometry.star
use Math
pub fn circle_area(r: Float): Float = 3.14159 * to_float(square(to_int(r)))

# main.star
use Geometry
fn main() =
  println(to_string(circle_area(5.0)))
```

## Formatting

The `star fmt` command automatically formats your Star source code with consistent indentation and style.

### Usage

Format a single file:

```sh
star fmt myfile.star
```

Format the entry point of a project (when a `Star.toml` is present):

```sh
star fmt
```

This reads `src/main.star`, parses it, and writes the formatted output back in place. Comments are preserved in their original positions.

### What It Does

The formatter normalizes:

- Indentation (two spaces per level)
- Spacing around operators and keywords
- Blank lines between top-level items
- Consistent formatting of `fn`, `type`, `match`, `if`, `do`, `module`, `trait`, and `impl` blocks

It does not change the semantics of your code -- only whitespace and layout.

### Integration Tips

Run `star fmt` before committing to keep diffs clean. It pairs well with CI checks:

```sh
# In a CI script: format and check for differences
star fmt src/main.star
git diff --exit-code src/main.star
```

## Project Structure

For larger Star projects, the recommended layout is:

```
my-project/
  Star.toml            # Project manifest (name, version, dependencies)
  Star.lock            # Lockfile for reproducible builds (auto-generated)
  src/
    main.star          # Entry point
    utils.star         # Shared utilities (use Utils)
    models.star        # Data types (use Models)
  examples/
    demo.star          # Example programs
  tests/
    math_test.star     # Test files
  .gitignore           # Ignores .star-build/
  .star-build/         # Build artifacts (auto-generated, gitignored)
```

### Star.toml

The project manifest declares metadata and Rust crate dependencies:

```toml
[package]
name = "my-project"
version = "0.1.0"
description = "A Star project"
authors = ["Your Name"]
license = "MIT"

[dependencies]
serde = "1"

[dev-dependencies]
criterion = "0.5"
```

Dependencies listed here are Rust crates included in the generated `Cargo.toml`. Common crates like `regex`, `base64`, and `tokio` are auto-detected from your code and added without manual declaration.

### Build Output

All compilation artifacts go into `.star-build/`, which contains a generated Cargo project. This directory is created automatically and should be gitignored. Use `star clean` to remove it.

### Creating a Project

Use `star new` to scaffold a new project:

```sh
star new my-project
cd my-project
star run
```

Or initialize in an existing directory:

```sh
mkdir my-project && cd my-project
star init
```

## REPL

Star includes an interactive REPL (Read-Eval-Print Loop) for experimenting with expressions and building up code incrementally.

### Starting the REPL

```sh
star repl
```

You will see a prompt where you can type expressions and definitions:

```
Star REPL v0.1.0
Type expressions to evaluate. Commands: :quit, :reset, :history

star>
```

### Evaluating Expressions

Type any expression and it will be evaluated and printed automatically:

```
star> 1 + 2
3
star> "hello" |> uppercase()
HELLO
star> [1, 2, 3] |> map(fn(x) => x * x)
[1, 4, 9]
```

Side-effect statements like `let` bindings and `println` calls are executed without extra printing:

```
star> let name = "Star"
star> println("Hello, #{name}!")
Hello, Star!
```

### Defining Functions and Types

Top-level definitions persist across inputs within the session:

```
star> fn square(x: Int): Int = x * x
star> square(7)
49
star> type Color = | Red | Blue | Green
star> Red
Red
```

### Multi-Line Input

The REPL detects incomplete expressions automatically. Lines ending with `do`, `=`, open brackets, or commas continue on the next line:

```
star> fn factorial(n: Int): Int =
  ...   if n <= 1 then 1
  ...   else n * factorial(n - 1)
  ...   end
star> factorial(10)
3628800
```

### REPL Commands

| Command              | Description                            |
|----------------------|----------------------------------------|
| `:quit` or `:q`     | Exit the REPL                          |
| `:reset`             | Clear all definitions and start fresh  |
| `:history` or `:h`  | Show all inputs entered this session   |

If an expression causes a compilation error, the REPL discards that input and lets you try again without losing previous definitions.

## Watch Mode

Watch mode automatically recompiles and reruns your program when source files change.

### Usage

```sh
star run --watch
star run myfile.star --watch
star build --watch
```

### How It Works

When `--watch` is active, Star:

1. Compiles and runs your program immediately.
2. Watches all `.star` files in the `src/` directory (and the directory of the entry file) plus `Star.toml`.
3. When any watched file changes, clears the screen and recompiles.
4. Continues watching until you press Ctrl+C.

```
[watch] Compiling...
Hello, World!
[watch] Watching for changes... (Ctrl+C to stop)
```

Edit a file, save, and the output updates automatically:

```
[watch] Recompiling...
Hello, Star!
[watch] Watching for changes... (Ctrl+C to stop)
```

### Combining with Release Mode

You can combine `--watch` with `--release` for optimized rebuilds:

```sh
star run --watch --release
```

Watch mode works with both `star run` and `star build`. It does not apply to `star check`, `star test`, or other commands.

## Next Steps

- Read the [Language Reference](language_reference.md) for complete syntax documentation
- Read the [Standard Library Reference](stdlib_reference.md) for all built-in functions
- Explore the `examples/` directory for working programs covering every feature
