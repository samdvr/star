# Star

A functional-first language with Ruby-like syntax that compiles to idiomatic Rust.

Star gives you the expressiveness of a high-level language with the performance of native code. Write clean, functional code and get optimized Rust binaries.

```star
fn main() =
  [1, 2, 3, 4, 5]
    |> filter(fn(x) => x > 2)
    |> map(fn(x) => x * 10)
    |> each(fn(x) => println(to_string(x)))
```

## Features

- **Functional-first** — immutable by default, pattern matching, pipe operator, lambdas
- **Ruby-like syntax** — `fn`, `do...end`, `if...then...else...end`, `match`
- **Compiles to Rust** — full type inference (Hindley-Milner), generates idiomatic `.rs` files
- **300+ built-in functions** — I/O, networking, crypto, collections, concurrency, CSV, TOML, JSON
- **Algebraic data types** — enums, structs, generics, traits, recursive types with auto-boxing
- **Module system** — multi-file projects with `use`, inline `module` blocks, `pub` visibility
- **Tooling** — formatter (`star fmt`), test runner (`star test`), REPL (`star repl`), watch mode (`--watch`), LSP server, VS Code extension

## Quick Start

```sh
# Install from source (requires Rust 1.85+)
git clone https://github.com/example/star.git
cd star
cargo install --path .

# Create and run a project
star new hello
cd hello
star run
```

## Language Overview

### Functions and Pipes

```star
fn square(x: Int): Int = x * x

fn main() =
  println(to_string(square(5)))
```

The pipe operator `|>` chains operations:

```star
fn main() =
  "hello world"
    |> uppercase()
    |> println()
```

### Pattern Matching

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

### Error Handling

```star
fn safe_divide(a: Float, b: Float): Result<Float, String> =
  if b == 0.0 then Err("division by zero")
  else Ok(a / b)
  end

fn main() =
  match safe_divide(10.0, 3.0)
  | Ok(result) => println("Result: #{to_string(result)}")
  | Err(msg) => println("Error: #{msg}")
  end
```

### Do Blocks and Loops

```star
fn main() =
  do
    let mut total = 0
    for x in [1, 2, 3, 4, 5] do
      total += x
    end
    println("Sum: #{to_string(total)}")
  end
```

## CLI

```
star build [file.star]       Compile a Star program
star run [file.star]         Compile and run
star check [file.star]       Type-check only
star test [file.star]        Run test functions
star fmt [file.star]         Format source code
star repl                    Interactive REPL
star new <name>              Create a new project
star lsp                     Start LSP server

Options:
  --release                  Build in release mode
  --watch                    Recompile on file changes (build/run)
  --filter <pattern>         Filter tests by name
  --verbose, -v              Verbose test output
```

## Documentation

- [Getting Started](docs/getting_started.md) — tutorial with examples
- [Language Reference](docs/language_reference.md) — complete syntax documentation
- [Standard Library Reference](docs/stdlib_reference.md) — all built-in functions

## Editor Support

- **VS Code** — syntax highlighting extension in [`editor/vscode/star-lang/`](editor/vscode/star-lang/)
- **LSP** — `star lsp` provides diagnostics, hover, completion, go-to-definition, formatting, and semantic tokens

## License

MIT
