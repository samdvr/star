# Star Language for VS Code

Syntax highlighting and language support for the [Star programming language](https://github.com/example/star).

## Features

- Syntax highlighting for `.star` files
- String interpolation `#{}` support
- Comment toggling (`#` line comments)
- Auto-closing brackets, quotes, and `do...end` blocks
- Code folding for blocks (`do`/`end`, `module`/`end`, etc.)

## Installation

### From Source

1. Copy this folder to your VS Code extensions directory:

   ```sh
   # macOS/Linux
   cp -r editor/vscode/star-lang ~/.vscode/extensions/star-lang

   # Windows
   xcopy /E editor\vscode\star-lang %USERPROFILE%\.vscode\extensions\star-lang
   ```

2. Restart VS Code.

3. Open any `.star` file to see syntax highlighting.

### From VSIX (if packaged)

```sh
cd editor/vscode/star-lang
npx @vscode/vsce package
code --install-extension star-lang-0.1.0.vsix
```

## Language Overview

Star is a functional-first language with Ruby-like syntax that compiles to idiomatic Rust:

```star
# Functions use = for the body
fn greet(name: String): String =
  "Hello, #{name}!"

# Pipe operator for chaining
fn main() =
  [1, 2, 3, 4, 5]
    |> filter(fn(x) => x > 2)
    |> map(fn(x) => x * 10)
    |> each(fn(x) => println(to_string(x)))
```
