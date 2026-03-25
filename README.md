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
- **300+ built-in functions** — I/O, networking, crypto, collections, concurrency, CSV, TOML, YAML, JSON, signals
- **Algebraic data types** — enums, structs, generics, traits, recursive types with auto-boxing
- **Module system** — multi-file projects with `use`, inline `module` blocks, `pub` visibility
- **Tooling** — formatter, test runner, REPL, watch mode, incremental builds, LSP server (with signature help & inlay hints), VS Code extension
- **Smart pointers** — `Rc`/`Arc` builtins for shared ownership, lazy iterators for efficient pipelines

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

### Lazy Iterators

Chain operations without intermediate allocations:

```star
fn main() =
  do
    let nums = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    let result = lazy_filter(nums, fn(x) => x > 3)
    let result = lazy_map(result, fn(x) => x * 10)
    let result = lazy_take(result, 3)
    println(to_string(result))  # [40, 50, 60]
  end
```

### Data Formats

Parse and encode CSV, TOML, YAML, and JSON:

```star
fn main() =
  do
    let rows = csv_parse("name,age\nAlice,30\nBob,25")
    println(to_string(rows))

    match toml_parse("title = \"My App\"\nport = 8080")
    | Ok(data) => println(data)
    | Err(e) => println("Error: #{e}")
    end

    match yaml_parse("name: Star\nversion: 1.0")
    | Ok(data) => println(data)
    | Err(e) => println("Error: #{e}")
    end
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
star init                    Initialize project in current directory
star clean                   Remove build artifacts
star lsp                     Start LSP server

Options:
  --release                  Build in release mode (optimized)
  --watch                    Recompile on file changes (build/run)
  --filter <pattern>         Filter tests by name
  --verbose, -v              Verbose test output
```

Builds are incremental — unchanged code skips recompilation automatically.

## Documentation

- [Getting Started](docs/getting_started.md) — tutorial with examples
- [Language Reference](docs/language_reference.md) — complete syntax documentation
- [Standard Library Reference](docs/stdlib_reference.md) — all built-in functions

## Editor Support

- **VS Code** — syntax highlighting extension in [`editor/vscode/star-lang/`](editor/vscode/star-lang/)
- **LSP** — `star lsp` provides diagnostics, hover, completion, go-to-definition, signature help, inlay hints, formatting, and semantic tokens

## Standard Library Highlights

| Category | Functions |
|----------|-----------|
| **I/O** | println, read_line, read_file, write_file, list_dir, ... |
| **Collections** | map, filter, fold, sort, group_by, zip, partition, ... |
| **Lazy iterators** | lazy_map, lazy_filter, lazy_take, lazy_skip, lazy_chain, ... |
| **Strings** | split, join, trim, replace, uppercase, regex_match, ... |
| **Math** | abs, sqrt, sin, cos, log, random, gcd, lcm, ... |
| **Networking** | http_get, http, tcp_connect, dns_lookup, url_parse, ... |
| **Data formats** | json_parse, json_encode, csv_parse, toml_parse, yaml_parse, ... |
| **Crypto** | sha256, sha512, md5, uuid_v4, secure_random_hex, ... |
| **Concurrency** | spawn, channel, mutex_new, atomic_new, parallel_map, ... |
| **Smart pointers** | rc_new, rc_clone, arc_new, arc_clone, ... |
| **Signals** | on_signal, ignore_signal, reset_signal |
| **Date/Time** | now, format_timestamp, sleep_ms, elapsed, ... |
| **Error handling** | unwrap, map_result, and_then, ok_or, transpose, ... |

## License

MIT
