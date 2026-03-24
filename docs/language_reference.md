# Star Language Reference

Complete syntax and semantics reference for the Star programming language.

## Table of Contents

- [Lexical Structure](#lexical-structure)
- [Types](#types)
- [Expressions](#expressions)
- [Declarations](#declarations)
- [Pattern Matching](#pattern-matching)
- [Control Flow](#control-flow)
- [Modules](#modules)
- [Traits and Implementations](#traits-and-implementations)
- [Error Handling](#error-handling)
- [Concurrency](#concurrency)
- [Interoperability](#interoperability)

---

## Lexical Structure

### Comments

```star
# Single-line comment
let x = 42  # Inline comment
```

### Identifiers

**Lowercase identifiers** — variables, functions, parameters:
- Start with `a-z` or `_`
- Continue with `a-z`, `0-9`, `_`
- Examples: `x`, `my_func`, `_private`

**Uppercase identifiers** — types, enum variants, modules:
- Start with `A-Z`
- Continue with `A-Z`, `a-z`, `0-9`, `_`
- Examples: `Option`, `MyType`, `Some`

### Keywords

```
fn  let  mut  type  module  use  pub  trait  impl  extern  async  await
if  then  else  end  match  when  for  in  while  do  break  continue
and  or  not  true  false  as  dyn  move
```

### Operators

| Category | Operators |
|----------|-----------|
| Arithmetic | `+`  `-`  `*`  `/`  `%` |
| Comparison | `==`  `!=`  `<`  `>`  `<=`  `>=` |
| Logical | `and`  `or`  `not` |
| Bitwise | `band`  `bor`  `bxor`  `<<`  `>>` |
| Assignment | `=`  `+=`  `-=`  `*=`  `/=`  `%=` |
| Special | `\|>`  `?`  `..`  `::` |

### Operator Precedence (highest to lowest)

1. Postfix: `.field`, `.method()`, `[index]`, `?`, `.await`
2. Unary: `-x`, `not x`
3. Multiplicative: `*`, `/`, `%`
4. Additive: `+`, `-`
5. Bitwise: `band`, `bor`, `bxor`, `<<`, `>>`
6. Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
7. Logical AND: `and`
8. Logical OR: `or`
9. Pipe: `|>`

### Literals

**Integers:**
```star
42
0
-5
```
All integer literals compile to `i64` by default.

**Floats:**
```star
3.14
0.0
-2.5
```
All float literals compile to `f64` by default.

**Strings:**
```star
"hello"
"escape: \n \t \\ \""
"interpolation: #{expr}"
```

**Triple-quoted strings** (multi-line, auto-dedented):
```star
"""
  First line
  Second line with #{value}
  """
```
Leading whitespace is stripped based on the indentation of the closing `"""`.

**String interpolation** embeds arbitrary expressions:
```star
"name: #{user.name}, age: #{user.age}"
"result: #{if x > 0 then "positive" else "negative" end}"
```

**Booleans:**
```star
true
false
```

**Lists:**
```star
[1, 2, 3]
["a", "b", "c"]
[]
```

**Tuples:**
```star
(1, "hello", true)
(x, y)
()                    # Unit (empty tuple)
```

A single expression in parentheses is grouping, not a tuple: `(x + y)`.

---

## Types

### Primitive Types

| Star | Rust | Description |
|------|------|-------------|
| `Int` | `i64` | 64-bit signed integer |
| `Int8` | `i8` | 8-bit signed integer |
| `Int16` | `i16` | 16-bit signed integer |
| `Int32` | `i32` | 32-bit signed integer |
| `UInt` | `u64` | 64-bit unsigned integer |
| `UInt8` | `u8` | 8-bit unsigned integer |
| `UInt16` | `u16` | 16-bit unsigned integer |
| `UInt32` | `u32` | 32-bit unsigned integer |
| `Float` | `f64` | 64-bit float |
| `Float32` | `f32` | 32-bit float |
| `Bool` | `bool` | Boolean |
| `String` | `String` | UTF-8 string |
| `Char` | `char` | Unicode character |

### Collection Types

| Star | Rust | Description |
|------|------|-------------|
| `List<T>` | `Vec<T>` | Dynamic array |
| `Map<K, V>` | `HashMap<K, V>` | Hash map |
| `Set<T>` | `HashSet<T>` | Hash set |
| `Deque<T>` | `VecDeque<T>` | Double-ended queue |
| `Heap<T>` | `BinaryHeap<T>` | Max-heap |

### Type Annotations

Type annotations use `:` after the name:

```star
let x: Int = 42
fn add(a: Int, b: Int): Int = a + b
```

Type annotations are optional when the compiler can infer the type.

### Generic Types

```star
List<Int>
Map<String, Int>
Result<String, String>
Option<User>
```

### Function Types

```star
fn(Int, Int) -> Bool
fn(String) -> Int
fn() -> String
```

### Reference Types

```star
&T          # Immutable reference
&mut T      # Mutable reference
~T          # Move/ownership transfer
```

### Tuple Types

```star
(Int, String)
(Bool, Int, Float)
```

### Dynamic Trait Objects

```star
dyn Display
```

### Type Aliases

```star
type StringList = List<String>
type Callback = fn(Int) -> String
type Pair = (Int, Int)
```

### Enum Types (Sum Types / Algebraic Data Types)

```star
type Option<T> =
  | None
  | Some(T)

type Shape =
  | Circle(Float)
  | Rectangle(Float, Float)
  | Triangle(Float, Float, Float)

type Color =
  | Red
  | Green
  | Blue
```

Variants can carry zero or more fields. Recursive types are automatically boxed by the compiler:

```star
type Expr =
  | Num(Int)
  | Add(Expr, Expr)    # Automatically wrapped in Box
  | Mul(Expr, Expr)
```

### Struct Types (Product Types)

```star
type User = {
  name: String,
  email: String,
  age: Int
}
```

### Struct Construction and Field Access

```star
let user = User { name: "Alice", email: "a@b.com", age: 30 }
println(user.name)
```

**Struct update syntax** (spread):
```star
let updated = User { name: "Bob", ..user }
```

---

## Expressions

### Function Calls

```star
println("hello")
add(1, 2)
map(list, fn(x) => x * 2)
```

### Method Calls

```star
user.describe()
list.length()
"hello".contains("ell")
```

### Lambdas

```star
fn(x) => x * 2
fn(x, y) => x + y
fn(x: Int): Int => x * 2
move fn(x) => x + captured_var
```

### Pipe Operator

The pipe `|>` passes the left-hand value as the first argument to the right-hand function:

```star
5 |> double |> to_string

# Equivalent to:
to_string(double(5))
```

With arguments:

```star
[1, 2, 3, 4, 5]
  |> filter(fn(x) => x > 2)
  |> map(fn(x) => x * 10)
  |> sum()
```

The pipe works across newlines, which enables readable data pipelines.

### Index Access

```star
list[0]
list[i]
```

Index expressions are cast to `usize` automatically.

### Try Operator

```star
let data = read_file("config.txt")?
```

Unwraps a `Result` or `Option`, returning early with the error/`None` if present.

### Field Access

```star
user.name
point.x
```

### If Expressions

```star
if condition then value1 else value2 end

if x > 0 then
  "positive"
else if x < 0 then
  "negative"
else
  "zero"
end
```

`if` without `else` is allowed for side effects:

```star
if debug then println("debugging") end
```

### Match Expressions

```star
match value
| pattern1 => body1
| pattern2 => body2
| pattern3 when guard => body3
| _ => default
end
```

See [Pattern Matching](#pattern-matching) for pattern syntax.

### Do Blocks

Multi-statement blocks where the last expression is the return value:

```star
do
  let x = compute()
  let y = transform(x)
  combine(x, y)
end
```

**Statements allowed in do blocks:**

```star
do
  # Let bindings
  let x = 10
  let mut counter = 0

  # Assignment
  counter = counter + 1

  # Compound assignment
  counter += 1
  counter -= 1
  counter *= 2
  counter /= 2
  counter %= 3

  # Index assignment
  list[0] = 42

  # Expressions (side effects)
  println("hello")

  # Last expression is the return value
  counter
end
```

### For Loops

```star
for x in [1, 2, 3] do
  println(to_string(x))
end

for (key, value) in entries do
  println("#{key}: #{value}")
end
```

### While Loops

```star
while condition do
  body
end
```

### Break and Continue

```star
for x in items do
  if x < 0 then continue end
  if x > 100 then break end
  process(x)
end
```

### Range Expressions

```star
range(1, 10)             # [1, 2, ..., 9]
range_inclusive(1, 10)    # [1, 2, ..., 10]
```

### Await

```star
let result = fetch(url).await
let result = await fetch(url)
```

### Inline Rust

```star
rust!("println!(\"raw Rust code\");")
```

---

## Declarations

### Function Declarations

```star
fn name(param1: Type1, param2: Type2): ReturnType = body
```

**Examples:**

```star
fn add(a: Int, b: Int): Int = a + b

fn greet(name: String): String =
  "Hello, #{name}!"

fn process(items: List<Int>): List<Int> =
  items
    |> filter(fn(x) => x > 0)
    |> map(fn(x) => x * 2)
```

**Public functions:**
```star
pub fn public_api(x: Int): String = to_string(x)
```

**Generic functions:**
```star
fn identity<T>(x: T): T = x
fn first<T>(list: List<T>): T = head(list)
```

**Type parameter bounds:**
```star
fn sort_items<T: Clone + Ord>(items: List<T>): List<T> = sort(items)
```

**Async functions:**
```star
async fn fetch_data(url: String): String =
  http_get(url)
```

### Type Declarations

**Enum (sum type):**
```star
type Shape =
  | Circle(Float)
  | Rectangle(Float, Float)
```

**Struct (product type):**
```star
type Point = { x: Float, y: Float }
```

**Alias:**
```star
type Name = String
```

**Generic:**
```star
type Tree<T> =
  | Leaf(T)
  | Node(Tree<T>, T, Tree<T>)
```

### Let Bindings

Top-level constants:

```star
let PI = 3.14159
pub let MAX_SIZE: Int = 1000
```

Inside function bodies and do blocks:

```star
let x = 42
let mut counter = 0
let (a, b) = (1, 2)
```

### Extern Declarations

Declare functions implemented in Rust:

```star
extern fn my_rust_fn(x: Int): String

# With explicit Rust path
extern fn get_pid(): Int = "std::process::id"
```

### Annotations

```star
@[inline]
fn hot_path(x: Int): Int = x * 2

@[cfg(test)]
fn test_only(): Bool = true
```

---

## Pattern Matching

### Pattern Forms

**Wildcard:**
```star
| _ => "anything"
```

**Variable binding:**
```star
| x => x + 1
```

**Literals:**
```star
| 0 => "zero"
| 3.14 => "pi"
| "hello" => "greeting"
| true => "yes"
```

**Constructor (variant):**
```star
| Some(x) => x
| None => default_value
| Ok(value) => value
| Err(msg) => panic(msg)
| Circle(r) => 3.14 * r * r
```

**Tuple:**
```star
| (x, y) => x + y
| (_, y, _) => y
```

**List:**
```star
| [] => "empty"
| [x] => "single"
| [x, y] => "pair"
| [head | tail] => head     # Head and rest
```

**Or-patterns:**
```star
| 1 | 2 | 3 => "small"
| Red | Green | Blue => "primary"
```

**Range patterns:**
```star
| 1..10 => "single digit"
| 0..100 => "percentage"
```

**Binding with `as`:**
```star
| Some(x) as opt => process(opt, x)
```

### Guards

Add conditions with `when`:

```star
match value
| x when x > 0 => "positive"
| x when x < 0 => "negative"
| _ => "zero"
end
```

### Destructuring in Let

```star
let (x, y) = get_point()
let [first, second | rest] = items
```

---

## Control Flow

### If / Else

```star
# Expression form (returns a value)
let result = if x > 0 then "positive" else "negative" end

# Multi-branch
if condition1 then
  action1
else if condition2 then
  action2
else
  action3
end

# Statement form (no else, for side effects)
if debug then println("debug mode") end
```

### Match

```star
match expr
| pattern1 => body1
| pattern2 when guard => body2
| _ => default_body
end
```

Match is exhaustive — the compiler expects all cases to be handled (use `_` as a catch-all).

### For Loops

```star
for x in collection do
  body
end
```

The collection can be any iterable: lists, ranges, map entries, etc.

**Destructuring in for:**
```star
for (index, value) in enumerate(list) do
  println("#{to_string(index)}: #{value}")
end
```

### While Loops

```star
while condition do
  body
end
```

### Break and Continue

Both `break` and `continue` work in `for` and `while` loops:

```star
for x in items do
  if should_skip(x) then continue end
  if should_stop(x) then break end
  process(x)
end
```

---

## Modules

### File-Based Modules

Each `.star` file is a module. Import with `use`:

```star
# Imports math.star from the same directory
use Math
```

This makes all `pub` functions from `math.star` available in the current file.

**Selective imports:**
```star
use Math::{square, cube}
```

**Nested modules:**
```star
use Parent::Child
```

### Inline Modules

```star
module Utils
  pub fn helper(x: Int): Int = x * 2

  fn private_fn(): String = "internal"
end
```

Access with `::`:
```star
Utils::helper(5)
```

### Visibility

- `pub fn` — Public, accessible from other modules
- `fn` — Private, only accessible within the same module

Functions without `pub` in a file module are not exported.

---

## Traits and Implementations

### Trait Declarations

```star
trait Describable
  fn describe(self): String
end
```

**With default methods:**
```star
trait Printable
  fn to_display(self): String

  fn print(self) =
    println(self.to_display())
end
```

**Generic traits:**
```star
trait Container<T>
  fn get(self, index: Int): Option<T>
  fn size(self): Int
end
```

**With associated types:**
```star
trait Iterator
  type Item
  fn next(self): Option<Self::Item>
end
```

### Impl Blocks

**Trait implementation:**
```star
impl Describable for User
  fn describe(self): String =
    "#{self.name} (#{to_string(self.age)})"
end
```

**Inherent implementation (methods on a type, no trait):**
```star
impl User
  fn full_name(self): String =
    "#{self.first} #{self.last}"

  fn is_adult(self): Bool =
    self.age >= 18
end
```

**Operator overloading:**
```star
type Vec2 = { x: Float, y: Float }

impl Add for Vec2
  fn add(self, other: Vec2): Vec2 =
    Vec2 { x: self.x + other.x, y: self.y + other.y }
end

impl Display for Vec2
  fn fmt(self): String =
    "(#{to_string(self.x)}, #{to_string(self.y)})"
end
```

Supported operator traits: `Add`, `Sub`, `Mul`, `Div`, `Rem`, `Neg`, `Not`, `PartialEq`, `PartialOrd`, `Index`, `Display`.

---

## Error Handling

### Result Type

```star
# Functions that can fail return Result<T, E>
fn parse_number(s: String): Result<Int, String> =
  if is_numeric(s) then
    Ok(to_int(s))
  else
    Err("not a number: #{s}")
  end
```

### Option Type

```star
fn find_user(id: Int): Option<User> =
  if id > 0 then
    Some(lookup(id))
  else
    None
  end
```

### Try Operator

Propagate errors with `?`:

```star
fn process(): Result<String, String> =
  let content = read_file("data.txt")?
  let parsed = parse_data(content)?
  Ok(format_output(parsed))
```

### Pattern Matching on Results

```star
match read_file("config.txt")
| Ok(content) => println(content)
| Err(e) => eprintln("Error: #{e}")
end
```

### Convenience Functions

```star
unwrap(result)                    # Panic on Err/None
unwrap_or(result, default)        # Default on Err/None
unwrap_or_else(result, fn(e) => fallback(e))
expect(result, "error message")   # Panic with message on Err/None
map_result(result, fn(x) => x * 2)
map_option(option, fn(x) => x + 1)
and_then(result, fn(x) => next_step(x))
or_else(result, fn(e) => recovery(e))
is_ok(result)                     # Bool
is_err(result)                    # Bool
is_some(option)                   # Bool
is_none(option)                   # Bool
ok_or(option, "error")            # Option<T> -> Result<T, E>
flatten_result(nested)            # Result<Result<T,E>,E> -> Result<T,E>
flatten_option(nested)            # Option<Option<T>> -> Option<T>
transpose(result)                 # Result<Option<T>,E> -> Option<Result<T,E>>
```

---

## Concurrency

### Threads

```star
# Spawn and join
let result = spawn_join(fn() => expensive_computation())

# Spawn detached
spawn(fn() => background_task())
```

### Channels

```star
let (tx, rx) = channel()
send(tx, "hello")
let msg = recv(rx)
let maybe = try_recv(rx)
```

### Synchronization

```star
# Mutex
let m = mutex_new(0)
let val = mutex_lock(m)

# Read-Write Lock
let rw = rwlock_new(0)
let val = rwlock_read(rw)
rwlock_write(rw)

# Atomics
let a = atomic_new(0)
atomic_set(a, 42)
let val = atomic_get(a)
atomic_add(a, 1)
```

### Parallel Map

```star
let results = parallel_map([1, 2, 3, 4], fn(x) => expensive(x))
```

### Async/Await

```star
async fn fetch(url: String): String =
  http_get(url)

async fn main() =
  let data = fetch("https://example.com").await
  println(data)
```

Async main functions automatically use `#[tokio::main]`.

---

## Interoperability

### Extern Functions

Declare Rust functions for direct use:

```star
extern fn custom_hash(data: String): Int = "my_crate::hash"
```

### Inline Rust

Embed raw Rust code:

```star
rust!("let now = std::time::Instant::now();")
```

### Auto-Detected Dependencies

The compiler automatically adds Cargo dependencies when you use certain features:

| Feature Used | Crate Added |
|---|---|
| `regex_*` functions | `regex = "1"` |
| `encode_base64` / `decode_base64` | `base64 = "0.22"` |
| `async` functions | `tokio = { version = "1", features = ["full"] }` |
| `http_*` functions | `native-tls = "0.2"` |

### Star.toml Dependencies

For other crates, add them to your manifest:

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
```

---

## Testing

### Test Functions

Any function starting with `test_` is collected by the test runner:

```star
fn add(a: Int, b: Int): Int = a + b

fn test_basic_add() =
  assert_eq(add(1, 2), 3)

fn test_negative() =
  assert_eq(add(-1, -2), -3)

fn test_identity() =
  assert_eq(add(0, 5), 5)
```

### Running Tests

```sh
star test file.star           # Run all tests
star test                     # Run tests in project (Star.toml)
star test --filter negative   # Run only matching tests
star test --verbose           # Show timing and progress
star test --release           # Run with optimizations
```

### Assertion Functions

```star
assert(condition)                    # Panic if false
assert_eq(actual, expected)          # Panic if not equal
assert_ne(actual, expected)          # Panic if equal
assert_msg(condition, "message")     # Panic with message if false
```

### Dev Dependencies

Dependencies only needed for testing go in `[dev-dependencies]`:

```toml
[dev-dependencies]
criterion = "0.5"
```

These are merged into the build only when running `star test`.

---

## Build System

### Star.lock

After the first build of a project, Star creates a `Star.lock` file that pins dependency versions. This file is preserved across builds for reproducibility and should be committed to version control.

### Build Artifacts

Build output goes to `.star-build/`, which contains the generated Rust project and compiled binary. Clean with:

```sh
star clean
```

### Emitting Rust

Inspect the generated Rust code:

```sh
star emit-rust file.star
```

This is useful for debugging codegen issues or understanding what Star produces.

### Formatting

Format Star source files:

```sh
star fmt file.star
star fmt                # Format src/main.star in a project
```
