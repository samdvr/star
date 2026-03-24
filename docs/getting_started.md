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

Options:
  --release                  Build in release mode
  --filter <pattern>         Filter tests by name substring
  --verbose, -v              Verbose test output with timing
  -h, --help                 Show help
  -V, --version              Show version
```

## Next Steps

- Read the [Language Reference](language_reference.md) for complete syntax documentation
- Read the [Standard Library Reference](stdlib_reference.md) for all built-in functions
- Explore the `examples/` directory for working programs covering every feature
