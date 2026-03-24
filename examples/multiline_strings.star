# Multi-line strings example

# Basic multi-line string with automatic dedent
let poem = """
  Roses are red,
  Violets are blue,
  Star has multi-line strings,
  And dedent too.
  """

println(poem)

# Interpolation works inside triple-quoted strings
let lang = "Star"
let year = 2026
let message = """
  Welcome to #{lang}!
  It's #{year} and multi-line strings are here.
  Enjoy writing readable text.
  """

println(message)

# Mixed indentation — only the common prefix is stripped
let code_block = """
  fn main() =
    println("hello")
    let x = 42
    x + 1
  """

println(code_block)
