use std::io::Write;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Helper: write Star source to a temp file, run `cargo run -- emit-rust <file>`,
/// return (success, stdout, stderr).
fn emit_rust(star_src: &str) -> (bool, String, String) {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let tid = std::thread::current().id();
    let dir = std::env::temp_dir().join("star_integration_tests");
    std::fs::create_dir_all(&dir).unwrap();
    let filename = format!("test_{:?}_{}.star", tid, id);
    let file = dir.join(filename);
    let mut f = std::fs::File::create(&file).unwrap();
    f.write_all(star_src.as_bytes()).unwrap();

    // Use the pre-built test binary directly to avoid recompilation races
    let bin = env!("CARGO_BIN_EXE_star");
    let output = Command::new(bin)
        .args(["emit-rust", file.to_str().unwrap()])
        .output()
        .expect("Failed to run star binary");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !success {
        eprintln!("emit-rust failed:\nstderr: {stderr}\nstdout: {stdout}");
    }
    (success, stdout, stderr)
}

// ── Empty file ──────────────────────────────────────────────────

#[test]
fn test_empty_file() {
    let (success, stdout, _) = emit_rust("");
    assert!(success, "Empty file should compile");
    // Should at least produce the star_display helper
    assert!(stdout.contains("fn star_display"));
}

// ── Hello world ─────────────────────────────────────────────────

#[test]
fn test_hello_world() {
    let (success, stdout, _) = emit_rust(r#"fn main() = println("Hello, world!")"#);
    assert!(success);
    assert!(stdout.contains("fn main()"));
    assert!(stdout.contains("println!"));
}

// ── Simple function ─────────────────────────────────────────────

#[test]
fn test_function_definition() {
    let (success, stdout, _) = emit_rust("fn add(a: Int, b: Int): Int = a + b");
    assert!(success);
    assert!(stdout.contains("fn add(a: i64, b: i64) -> i64"));
}

// ── Type declarations ───────────────────────────────────────────

#[test]
fn test_enum_type() {
    let (success, stdout, _) = emit_rust("type Color =\n  | Red\n  | Green\n  | Blue");
    assert!(success);
    assert!(stdout.contains("enum Color"));
    assert!(stdout.contains("Red,"));
}

#[test]
fn test_struct_type() {
    let (success, stdout, _) = emit_rust("type Point = {\n  x: Float,\n  y: Float\n}");
    assert!(success);
    assert!(stdout.contains("struct Point"));
    assert!(stdout.contains("x: f64"));
}

// ── Match expression ────────────────────────────────────────────

#[test]
fn test_match_expression() {
    let src = r#"fn describe(x: Int): String = match x
  | 0 => "zero"
  | 1 => "one"
  | _ => "other"
  end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("match x"));
}

// ── Pipe operator ───────────────────────────────────────────────

#[test]
fn test_pipe_operator() {
    let (success, stdout, _) = emit_rust("fn double(x: Int): Int = x * 2\nfn main() = 5 |> double");
    assert!(success);
    assert!(stdout.contains("double(5i64)"));
}

// ── Loops ───────────────────────────────────────────────────────

#[test]
fn test_for_loop() {
    let src = "fn main() = for x in [1, 2, 3] do\n  println(x)\nend";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("for"));
}

#[test]
fn test_while_loop() {
    let src = "fn main() = do\n  let mut x = 0\n  while x < 10 do\n    x += 1\n  end\n  x\nend";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("while"));
}

// ── Trait declarations ──────────────────────────────────────────

#[test]
fn test_trait_decl() {
    let src = "trait Printable\n  fn to_str(self): String\nend";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("trait Printable"));
}

// ── Extern fn ───────────────────────────────────────────────────

#[test]
fn test_extern_fn() {
    let src = "extern fn libc_exit(code: Int)";
    let (success, _stdout, _) = emit_rust(src);
    assert!(success);
}

// ── Comments only file ──────────────────────────────────────────

#[test]
fn test_comments_only() {
    let src = "# This file has nothing but comments\n# Another comment";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("fn star_display"));
}

// ── Unicode in string literals ──────────────────────────────────

#[test]
fn test_unicode_strings() {
    let src = r#"fn main() = println("Hello, 世界! 🌍")"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("Hello"));
}

// ── Deeply nested expressions ───────────────────────────────────

#[test]
fn test_deeply_nested() {
    let src = "fn main() = ((((((1 + 2) + 3) + 4) + 5) + 6) + 7)";
    let (success, _stdout, _) = emit_rust(src);
    assert!(success);
}

// ── Long identifiers ───────────────────────────────────────────

#[test]
fn test_long_identifier() {
    let src = "fn this_is_a_very_long_function_name_that_should_still_work(x: Int): Int = x";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("this_is_a_very_long_function_name_that_should_still_work"));
}

// ── Multiple items ──────────────────────────────────────────────

#[test]
fn test_multiple_functions() {
    let src = "fn double(x: Int): Int = x * 2\n\nfn main() = println(double(21))";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("fn double"));
    assert!(stdout.contains("fn main"));
}

// ── Lambda ──────────────────────────────────────────────────────

#[test]
fn test_lambda_in_pipe() {
    let src = "fn main() = [1, 2, 3] |> map(fn(x) => x * 2)";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains(".map("));
}

// ── Let bindings ────────────────────────────────────────────────

#[test]
fn test_let_binding() {
    let src = "fn main() = do\n  let x = 42\n  println(x)\nend";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("let x"));
}

// ── Type alias ──────────────────────────────────────────────────

#[test]
fn test_type_alias() {
    let src = "type Numbers = List<Int>";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("type Numbers = Vec<i64>"));
}

// ── Async function ──────────────────────────────────────────────

#[test]
fn test_async_function() {
    let src = "async fn fetch() = 42";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("async fn fetch"));
}

// ── Invalid syntax should fail ──────────────────────────────────

#[test]
fn test_invalid_syntax_fails() {
    let (success, _, _) = emit_rust("fn foo(x: Int x + 1");
    assert!(!success);
}

// ── Compound assignment ─────────────────────────────────────────

#[test]
fn test_compound_assignment() {
    let src = "fn main() = do\n  let mut x = 0\n  x += 10\n  x -= 3\n  x *= 2\n  println(x)\nend";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("+=") || stdout.contains("x = (x + "));
}

// ── String interpolation ────────────────────────────────────────

#[test]
fn test_string_interpolation() {
    let src = r#"fn main() = do
  let name = "world"
  println("hello #{name}")
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("format!"));
}

// ── Try operator (?) ───────────────────────────────────────────

#[test]
fn test_try_operator() {
    let src = r#"fn parse_num(s: String): Result<Int, String> = do
  let n = to_int(s)?
  ok(n)
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("?"), "Generated Rust should contain ? operator");
}

// ── Struct update syntax (..base) ──────────────────────────────

#[test]
fn test_struct_update_syntax() {
    let src = r#"type Config = {
  debug: Bool,
  verbose: Bool,
  level: Int
}

fn with_debug(c: Config): Config =
  Config { debug: true, ..c }"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains(".."), "Generated Rust should contain struct update syntax (..)");
}

// ── Bitwise operators ──────────────────────────────────────────

#[test]
fn test_bitwise_operators() {
    let src = "fn main() = do\n  let a = 255 band 15\n  let b = a bor 48\n  let c = b bxor 255\n  let d = 1 << 4\n  let e = 16 >> 2\n  println(c)\nend";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains(" & "), "Should contain bitwise AND");
    assert!(stdout.contains(" | "), "Should contain bitwise OR");
    assert!(stdout.contains(" ^ "), "Should contain bitwise XOR");
    assert!(stdout.contains(" << "), "Should contain left shift");
    assert!(stdout.contains(" >> "), "Should contain right shift");
}

// ── Type parameter bounds ──────────────────────────────────────

#[test]
fn test_type_param_bounds() {
    let src = "fn max_val<T: Ord>(a: T, b: T): T =\n  if a > b then a else b end";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("T: Ord"), "Should contain type bound");
}

#[test]
fn test_type_param_multiple_bounds() {
    let src = "fn show<T: Debug + Clone>(x: T): String = debug(x)";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("T: Debug + Clone"), "Should contain multiple bounds");
}

// ── Trait objects ──────────────────────────────────────────────

#[test]
fn test_dyn_trait() {
    let src = "trait Drawable\n  fn draw(self): String\nend\n\nfn render(obj: dyn Drawable): String = obj.draw()";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("Box<dyn Drawable>"), "Should contain Box<dyn Drawable>");
}

// ── Destructuring in function parameters ───────────────────────

#[test]
fn test_destructuring_params() {
    let src = "fn swap((a, b): (Int, Int)): (Int, Int) = (b, a)";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("(a, b): (i64, i64)"), "Should contain destructured param");
}

// ── Multi-line lambda ──────────────────────────────────────────

#[test]
fn test_multiline_lambda() {
    let src = r#"fn main() = do
  let f = fn(x: Int) => do
    let y = x * 2
    y + 1
  end
  println(f(5))
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("|x: i64|"), "Should contain lambda with type annotation");
}

// ── Trait impl with method calls ───────────────────────────────

#[test]
fn test_trait_impl_method_call() {
    let src = r#"type Circle = { radius: Float }

impl Circle
  fn area(self): Float = 3.14159 * self.radius * self.radius
end

fn main() = do
  let c = Circle { radius: 5.0 }
  println(c.area())
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("impl Circle"));
    assert!(stdout.contains("fn area"));
}

// ── Recursive types ────────────────────────────────────────────

#[test]
fn test_recursive_enum() {
    let src = r#"type Tree =
  | Leaf(Int)
  | Node(Tree, Tree)

fn sum_tree(t: Tree): Int =
  match t
  | Leaf(n) => n
  | Node(l, r) => sum_tree(l) + sum_tree(r)
  end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("Box<"), "Recursive type should be auto-boxed");
}

// ── String interpolation with expressions ──────────────────────

#[test]
fn test_interpolation_with_expr() {
    let src = r#"fn main() = do
  let x = 42
  println("The answer is #{x + 1}!")
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("format!"), "Should use format! for interpolation");
}

// ── Module declaration ─────────────────────────────────────────

#[test]
fn test_module_decl() {
    let src = "module Math\n  pub fn double(x: Int): Int = x * 2\nend\n\nuse Math\n\nfn main() = println(double(5))";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("mod math"), "Should generate Rust module");
}

// ── Impl block (inherent) ──────────────────────────────────────

#[test]
fn test_inherent_impl() {
    let src = r#"type Counter = { val: Int }

impl Counter
  fn new(): Counter = Counter { val: 0 }
  fn increment(self): Counter = Counter { val: self.val + 1 }
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("impl Counter"));
}

// ── Module-level constants ───────────────────────────────────────

#[test]
fn test_module_level_constant() {
    let src = "let MAX_SIZE: Int = 100\n\nfn main() = println(MAX_SIZE)";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("const MAX_SIZE: i64 = 100i64"), "Should contain const declaration");
}

#[test]
fn test_module_level_string_constant() {
    let src = r#"let greeting = "hello"

fn main() = println(greeting)"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("const greeting: &str = \"hello\""), "Should contain string const declaration");
}

// ── Move semantics for lambda captures ──────────────────────────

#[test]
fn test_move_lambda() {
    let src = r#"fn main() = do
  let x = 42
  let f = move fn() => x
  println(f())
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("move |"), "Should contain move closure");
}

// ── Operator overloading ────────────────────────────────────────

#[test]
fn test_operator_overloading_add() {
    let src = r#"type Vec2 = { x: Float, y: Float }

impl Add for Vec2
  fn add(self, other: Vec2): Vec2 = Vec2 { x: self.x + other.x, y: self.y + other.y }
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("std::ops::Add"), "Should contain std::ops::Add trait impl");
}

// ── Associated types in traits ──────────────────────────────────

#[test]
fn test_associated_type_in_trait() {
    let src = r#"trait Container
  type Item
  fn get(self): Item
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("type Item;"), "Should contain associated type declaration");
}

#[test]
fn test_associated_type_in_impl() {
    let src = r#"trait Container
  type Item
  fn get(self): Item
end

type IntBox = { val: Int }

impl Container for IntBox
  type Item = Int
  fn get(self): Int = self.val
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("type Item = i64;"), "Should contain associated type definition");
}

// ── Lifetime annotations ────────────────────────────────────────

#[test]
fn test_lifetime_in_function() {
    let src = "fn first(items: &List<Int>): &Int = head(items)";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("&"), "Should contain reference types");
}

// ── Conditional compilation ─────────────────────────────────────

#[test]
fn test_annotation_cfg() {
    let src = r#"@[cfg(target_os = "linux")]
fn linux_only(): String = "linux"

fn main() = println("hello")"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("#[cfg(target_os"), "Should contain #[cfg(...)] attribute");
}

// ═══════════════════════════════════════════════════════════════════
// ERROR CASES — programs that should fail to compile
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_error_missing_closing_end() {
    let (success, _, _) = emit_rust("fn main() = do\n  let x = 1\n  println(x)");
    assert!(!success, "Missing 'end' should fail");
}

#[test]
fn test_error_unclosed_string() {
    let (success, _, _) = emit_rust(r#"fn main() = println("hello)"#);
    assert!(!success, "Unclosed string literal should fail");
}

#[test]
fn test_error_missing_return_type_colon() {
    // Missing colon before return type
    let (success, _, _) = emit_rust("fn foo(x: Int) Int = x");
    assert!(!success, "Missing colon before return type should fail");
}

#[test]
fn test_error_unexpected_token_in_type() {
    let (success, _, _) = emit_rust("type Foo = {\n  x: ,\n}");
    assert!(!success, "Unexpected comma in type field should fail");
}

#[test]
fn test_error_empty_match() {
    // An empty match without arms — may or may not be valid
    let (success, stdout, _) = emit_rust("fn main() = match 42\nend");
    // If it compiles, just verify it produces a match; if it fails, that's fine too
    if success {
        assert!(stdout.contains("match"));
    }
}

#[test]
fn test_error_double_comma_in_params() {
    let (success, _, _) = emit_rust("fn foo(a: Int,, b: Int) = a");
    assert!(!success, "Double comma in params should fail");
}

// ═══════════════════════════════════════════════════════════════════
// EDGE CASES — valid but tricky programs
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_nested_match() {
    let src = r#"fn describe(x: Int, y: Int): String = match x
  | 0 => match y
    | 0 => "origin"
    | _ => "x-axis"
    end
  | _ => "elsewhere"
  end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success, "Nested match should compile");
    assert!(stdout.contains("match"));
}

#[test]
fn test_chained_pipes() {
    let src = "fn double(x: Int): Int = x * 2\nfn add_one(x: Int): Int = x + 1\nfn main() = 5 |> double |> add_one |> double";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("double(add_one(double(5i64)))"));
}

#[test]
fn test_complex_string_interpolation() {
    let src = r#"fn main() = do
  let x = 10
  let y = 20
  println("sum = #{x + y}, product = #{x * y}")
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("format!"));
}

#[test]
fn test_nested_if_expressions() {
    let src = r#"fn classify(x: Int): String =
  if x > 0 then
    if x > 100 then "big" else "small" end
  else
    if x < -100 then "very negative" else "negative" end
  end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("if"));
}

#[test]
fn test_list_of_tuples() {
    let src = "fn main() = [(1, 2), (3, 4), (5, 6)]";
    let (success, _stdout, _) = emit_rust(src);
    assert!(success);
}

#[test]
fn test_enum_with_multiple_fields() {
    let src = "type Expr =\n  | Lit(Int)\n  | Add(Expr, Expr)\n  | Mul(Expr, Expr)\n  | Neg(Expr)";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("enum Expr"));
    assert!(stdout.contains("Box<"), "Recursive Expr should be auto-boxed");
}

#[test]
fn test_struct_with_generic() {
    let src = "type Wrapper<T> = { value: T }";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("struct Wrapper<T>"));
    assert!(stdout.contains("value: T"));
}

#[test]
fn test_trait_with_default_method() {
    let src = r#"trait Greetable
  fn greet(self): String = "hello"
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("trait Greetable"));
    assert!(stdout.contains("fn greet"));
}

#[test]
fn test_impl_for_trait() {
    let src = r#"type Dog = { name: String }

trait Animal
  fn speak(self): String
end

impl Animal for Dog
  fn speak(self): String = "woof"
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("impl Animal for Dog"));
}

#[test]
fn test_multiple_type_params() {
    let src = "fn pair<A, B>(a: A, b: B): (A, B) = (a, b)";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("<A, B>"));
}

#[test]
fn test_index_access() {
    let src = "fn main() = do\n  let list = [10, 20, 30]\n  println(list[1])\nend";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("as usize]"), "Index should cast to usize");
}

#[test]
fn test_mutable_variable() {
    let src = "fn main() = do\n  let mut x = 0\n  x = 42\n  println(x)\nend";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("let mut x"));
}

#[test]
fn test_tuple_destructuring() {
    let src = "fn main() = do\n  let (a, b) = (1, 2)\n  println(a + b)\nend";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("let (a, b)"));
}

#[test]
fn test_match_with_guard_like_patterns() {
    let src = r#"fn fizzle(n: Int): String = match n % 3
  | 0 => "fizz"
  | 1 => "one"
  | _ => "other"
  end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("match"));
}

#[test]
fn test_multiline_pipe() {
    let src = "fn double(x: Int): Int = x * 2\nfn add_one(x: Int): Int = x + 1\nfn main() = 1\n  |> double\n  |> add_one";
    let (success, _stdout, _) = emit_rust(src);
    assert!(success);
}

#[test]
fn test_empty_struct() {
    let src = "type Unit = {}";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("struct Unit"));
}

#[test]
fn test_single_variant_enum() {
    let src = "type Wrapper = | Value(Int)";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("enum Wrapper"));
}

#[test]
fn test_nested_generics() {
    let src = "fn main() = do\n  let x: List<List<Int>> = [[1, 2], [3, 4]]\n  println(x)\nend";
    let (success, _stdout, _) = emit_rust(src);
    assert!(success);
}

#[test]
fn test_if_without_else() {
    let src = "fn main() = if true then println(42) end";
    let (success, _stdout, _) = emit_rust(src);
    assert!(success);
}

#[test]
fn test_do_block_multiple_statements() {
    let src = r#"fn main() = do
  let a = 1
  let b = 2
  let c = a + b
  println(c)
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("let a"));
    assert!(stdout.contains("let b"));
    assert!(stdout.contains("let c"));
}

#[test]
fn test_lambda_with_type_annotation() {
    let src = "fn main() = do\n  let f = fn(x: Int): Int => x * x\n  println(f(5))\nend";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("|x: i64|"), "Lambda should have type annotation");
}

#[test]
fn test_multiple_traits() {
    let src = r#"trait Foo
  fn foo(self): Int
end

trait Bar
  fn bar(self): String
end

type Baz = { x: Int }

impl Foo for Baz
  fn foo(self): Int = self.x
end

impl Bar for Baz
  fn bar(self): String = "baz"
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("impl Foo for Baz"));
    assert!(stdout.contains("impl Bar for Baz"));
}

#[test]
fn test_enum_match_all_variants() {
    let src = r#"type Direction =
  | North
  | South
  | East
  | West

fn to_string(d: Direction): String = match d
  | North => "N"
  | South => "S"
  | East => "E"
  | West => "W"
  end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("Direction::North"));
}

#[test]
fn test_nested_function_calls() {
    let src = "fn add(a: Int, b: Int): Int = a + b\nfn main() = println(add(add(1, 2), add(3, 4)))";
    let (success, _stdout, _) = emit_rust(src);
    assert!(success);
}

#[test]
fn test_method_chaining_in_do_block() {
    let src = "fn main() = do\n  let s = \"hello world\"\n  let result = uppercase(s)\n  println(result)\nend";
    let (success, _stdout, _) = emit_rust(src);
    assert!(success);
}

#[test]
fn test_multiple_constants() {
    let src = r#"let PI: Float = 3.14159
let TAU: Float = 6.28318
let NAME = "star"

fn main() = println(PI)"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("const PI: f64"));
    assert!(stdout.contains("const TAU: f64"));
    assert!(stdout.contains("const NAME"));
}

#[test]
fn test_break_and_continue() {
    let src = "fn main() = do\n  for x in [1, 2, 3, 4, 5] do\n    if x == 3 then continue end\n    if x == 5 then break end\n    println(x)\n  end\nend";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("continue"));
    assert!(stdout.contains("break"));
}

#[test]
fn test_pub_function() {
    let src = "pub fn add(a: Int, b: Int): Int = a + b";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("pub fn add"));
}

#[test]
fn test_module_with_pub_functions() {
    let src = r#"module Utils
  pub fn double(x: Int): Int = x * 2
  pub fn triple(x: Int): Int = x * 3
end

use Utils

fn main() = println(double(5))"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("mod utils"));
}

#[test]
fn test_operator_overloading_sub() {
    let src = "type Vec2 = { x: Float, y: Float }\n\nimpl Sub for Vec2\n  fn sub(self, other: Vec2): Vec2 = Vec2 { x: self.x - other.x, y: self.y - other.y }\nend";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("std::ops::Sub"), "Should contain std::ops::Sub");
}

#[test]
fn test_string_escape_sequences() {
    let src = r#"fn main() = println("tab:\there\nnewline")"#;
    let (success, _stdout, _) = emit_rust(src);
    assert!(success);
}

#[test]
fn test_float_literals() {
    let src = "fn main() = do\n  let a = 1.0\n  let b = 0.5\n  let c = 100.001\n  println(a + b + c)\nend";
    let (success, _stdout, _) = emit_rust(src);
    assert!(success);
}

#[test]
fn test_negative_numbers() {
    let src = "fn main() = do\n  let x = -42\n  let y = -3.14\n  println(x)\nend";
    let (success, _stdout, _) = emit_rust(src);
    assert!(success);
}

#[test]
fn test_boolean_expressions() {
    let src = "fn check(a: Bool, b: Bool): Bool = (a and b) or (not a and not b)";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("&&") || stdout.contains("and"));
    assert!(stdout.contains("||") || stdout.contains("or"));
}

#[test]
fn test_map_collection_type() {
    let src = "fn main() = do\n  let m = map_new()\n  let m2 = map_insert(m, \"key\", 42)\n  println(m2)\nend";
    let (success, _stdout, _) = emit_rust(src);
    assert!(success);
}

#[test]
fn test_if_else_as_expression() {
    let src = "fn abs(x: Int): Int = if x >= 0 then x else 0 - x end";
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("if"));
    assert!(stdout.contains("else"));
}

#[test]
fn test_wildcard_pattern() {
    let src = r#"fn ignore(x: Int): String = match x
  | _ => "ignored"
  end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("_ =>"));
}

// ── Type system: return type mismatch detected ──────────────

#[test]
fn test_return_type_mismatch_error() {
    let src = r#"fn wrong(x: Int): String = x"#;
    let (success, _stdout, stderr) = emit_rust(src);
    assert!(!success, "Should fail: Int returned where String expected");
    assert!(stderr.contains("type") || stderr.contains("mismatch") || stderr.contains("expected"),
        "Error should mention type mismatch: {stderr}");
}

#[test]
fn test_unknown_identifier_error() {
    let src = r#"fn main() = nonexistent_function(42)"#;
    let (success, _stdout, stderr) = emit_rust(src);
    assert!(!success, "Should fail: unknown function");
    assert!(stderr.contains("Unknown") || stderr.contains("unknown"),
        "Error should mention unknown identifier: {stderr}");
}

#[test]
fn test_did_you_mean_suggestion() {
    let src = r#"fn main() = prinln("hello")"#;
    let (success, _stdout, stderr) = emit_rust(src);
    assert!(!success, "Should fail: misspelled builtin");
    assert!(stderr.contains("println"), "Should suggest 'println': {stderr}");
}

// ── Type system: arg type mismatch ──────────────────────────

#[test]
fn test_arg_type_mismatch() {
    let src = r#"fn greet(name: String): String = name
fn main() = greet(42)"#;
    let (success, _stdout, stderr) = emit_rust(src);
    assert!(!success, "Should fail: Int arg where String expected");
    assert!(!stderr.is_empty());
}

#[test]
fn test_arg_count_mismatch() {
    let src = r#"fn add(a: Int, b: Int): Int = a + b
fn main() = add(1)"#;
    let (success, _stdout, stderr) = emit_rust(src);
    assert!(!success, "Should fail: wrong number of arguments");
    assert!(!stderr.is_empty());
}

// ── Type system: if branch mismatch ─────────────────────────

#[test]
fn test_if_branch_type_mismatch() {
    let src = r#"fn bad(x: Bool): Int = if x then 1 else "oops" end"#;
    let (success, _stdout, stderr) = emit_rust(src);
    assert!(!success, "Should fail: then=Int, else=String");
    assert!(!stderr.is_empty());
}

// ── Type system: list element mismatch ──────────────────────

#[test]
fn test_list_element_type_mismatch() {
    let src = r#"fn main() = [1, "two", 3]"#;
    let (success, _stdout, stderr) = emit_rust(src);
    assert!(!success, "Should fail: mixed list element types");
    assert!(!stderr.is_empty());
}

// ── Type system: struct missing field ───────────────────────

#[test]
fn test_struct_missing_field_error() {
    let src = r#"type Point = { x: Int, y: Int }
fn main() = Point { x: 1 }"#;
    let (success, _stdout, stderr) = emit_rust(src);
    assert!(!success, "Should fail: missing field y");
    assert!(!stderr.is_empty());
}

#[test]
fn test_struct_wrong_field_type_error() {
    let src = r#"type Point = { x: Int, y: Int }
fn main() = Point { x: 1, y: "two" }"#;
    let (success, _stdout, stderr) = emit_rust(src);
    assert!(!success, "Should fail: String where Int expected");
    assert!(!stderr.is_empty());
}

// ── Type system: enum constructor arity ─────────────────────

#[test]
fn test_enum_constructor_wrong_arity() {
    let src = r#"type Wrapper =
  | Wrap(Int)

fn main() = Wrap(1, 2)"#;
    let (success, _stdout, stderr) = emit_rust(src);
    assert!(!success, "Should fail: Wrap takes 1 arg, got 2");
    assert!(!stderr.is_empty());
}

// ── Type system: match arm mismatch ─────────────────────────

#[test]
fn test_match_arm_type_mismatch() {
    let src = r#"fn bad(x: Int) = match x
  | 0 => "zero"
  | _ => 42
  end"#;
    let (success, _stdout, stderr) = emit_rust(src);
    assert!(!success, "Should fail: arm returns String vs Int");
    assert!(!stderr.is_empty());
}

// ── Type system: const type mismatch ────────────────────────

#[test]
fn test_const_with_type_annotation() {
    let src = r#"let X: Int = 42"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success, "Const with matching type should compile");
    assert!(stdout.contains("const X: i64 = 42i64"));
}

// ── Correct programs compile without errors ─────────────────

#[test]
fn test_recursive_type_auto_boxing() {
    let src = r#"type Tree =
  | Leaf(Int)
  | Node(Tree, Tree)

fn sum(t: Tree): Int = match t
  | Leaf(n) => n
  | Node(l, r) => sum(l) + sum(r)
  end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("Box<Tree>"));
}

#[test]
fn test_generic_enum_compiles() {
    let src = r#"type Maybe<T> =
  | Just(T)
  | Nothing

fn unwrap_or<T>(m: Maybe<T>, default: T): T = match m
  | Just(x) => x
  | Nothing => default
  end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("Maybe<T>") || stdout.contains("enum Maybe"));
}

#[test]
fn test_trait_impl_compiles() {
    let src = r#"type Dog = { name: String }

trait Speak
  fn speak(self): String
end

impl Speak for Dog
  fn speak(self): String = "Woof!"
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("impl Speak for Dog"));
}

#[test]
fn test_nested_match_compiles() {
    let src = r#"type Expr =
  | Num(Int)
  | Add(Expr, Expr)

fn eval(e: Expr): Int = match e
  | Num(n) => n
  | Add(a, b) => eval(a) + eval(b)
  end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("Box<Expr>"));
    assert!(stdout.contains("fn eval"));
}

#[test]
fn test_higher_order_function() {
    let src = r#"fn apply(f: fn(Int) -> Int, x: Int): Int = f(x)
fn double(x: Int): Int = x * 2
fn main() = println(apply(double, 21))"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("impl Fn(i64) -> i64") || stdout.contains("Fn(i64)"));
}

#[test]
fn test_string_interpolation_complex() {
    let src = r#"fn greet(name: String, age: Int): String = "Hello #{name}, you are #{age} years old""#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("format!"));
}

#[test]
fn test_for_loop_with_break() {
    let src = r#"fn find_first_even(xs: List<Int>): Int = do
  let mut result = 0
  for x in xs do
    if x % 2 == 0 then do
      result = x
      break
    end end
  end
  result
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("break"));
    assert!(stdout.contains("for x in"));
}

#[test]
fn test_while_with_compound_assign() {
    let src = r#"fn countdown(n: Int): Int = do
  let mut x = n
  let mut sum = 0
  while x > 0 do
    sum += x
    x -= 1
  end
  sum
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("+="));
    assert!(stdout.contains("-="));
}

#[test]
fn test_generic_swap_function() {
    let src = r#"fn first<A, B>(a: A, b: B): A = a"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("<A, B>"));
}

#[test]
fn test_list_of_tuples_typed() {
    let src = r#"fn pairs(): List<(Int, String)> = [(1, "one"), (2, "two")]"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("Vec<(i64, String)>"));
}

#[test]
fn test_result_type_return() {
    let src = r#"fn safe_div(a: Int, b: Int): Result<Int, String> =
  if b == 0 then err("division by zero") else ok(a / b) end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("Result<i64, String>"));
}

#[test]
fn test_chained_method_calls() {
    let src = r#"type Builder = { val: Int }

impl Builder
  fn set(self, v: Int): Builder = Builder { val: v }
  fn get(self): Int = self.val
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("impl Builder"));
}

#[test]
fn test_module_with_pub_fn() {
    let src = r#"module Math
  pub fn square(x: Int): Int = x * x
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("pub fn square"));
}

#[test]
fn test_async_function_compiles() {
    let src = r#"async fn fetch_data(): String = "data""#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("async fn fetch_data"));
}

#[test]
fn test_lambda_in_let_binding() {
    let src = r#"fn main() = do
  let double = fn(x: Int) => x * 2
  println(double(21))
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("|x: i64|"));
}

#[test]
fn test_nested_if_expression() {
    let src = r#"fn classify(x: Int): String =
  if x > 0 then
    if x > 100 then "big" else "small" end
  else
    "negative"
  end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("if") && stdout.contains("else"));
}

#[test]
fn test_pattern_destructuring() {
    let src = r#"fn sum_pair(p: (Int, Int)): Int = do
  let (a, b) = p
  a + b
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("let (a, b)"));
}

#[test]
fn test_empty_list_compiles() {
    let src = r#"fn empty(): List<Int> = []"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("vec![]"));
}

#[test]
fn test_const_declarations() {
    let src = r#"let X: Int = 10
let Y: Int = 20
let NAME = "star""#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("const X: i64 = 10i64"));
    assert!(stdout.contains("const Y: i64 = 20i64"));
    assert!(stdout.contains("const NAME"));
}

#[test]
fn test_enum_shape_variants() {
    let src = r#"type Shape =
  | Circle(Float)
  | Rect(Float, Float)
  | Point"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("Circle(f64)"));
    assert!(stdout.contains("Rect(f64, f64)"));
    assert!(stdout.contains("Point,"));
}

#[test]
fn test_do_block_with_let_mut() {
    let src = r#"fn counter(): Int = do
  let mut count = 0
  count += 1
  count += 1
  count += 1
  count
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("let mut count"));
    assert!(stdout.contains("count += 1"));
}

#[test]
fn test_index_access_compiles() {
    let src = r#"fn first(xs: List<Int>): Int = xs[0]"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("as usize"));
}

#[test]
fn test_type_bounds_multiple() {
    let src = r#"fn show<T: Debug + Clone>(x: T): T = x"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("Debug") && stdout.contains("Clone"));
}

#[test]
fn test_dyn_trait_param() {
    let src = r#"trait Drawable
  fn draw(self): String
end

fn render(obj: dyn Drawable): String = obj.draw()"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("Box<dyn Drawable>"));
}

#[test]
fn test_trait_default_method_body() {
    let src = r#"trait Greet
  fn name(self): String
  fn greet(self): String = "Hello!"
end"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("trait Greet"));
    assert!(stdout.contains("fn greet"));
}

#[test]
fn test_map_collection_operations() {
    let src = r#"fn main() = do
  let m = map_new()
  let m2 = map_insert(m, "a", 1)
  let v = map_get(m2, "a")
  println(v)
end"#;
    let (success, _stdout, _) = emit_rust(src);
    assert!(success);
}

#[test]
fn test_set_collection_operations() {
    let src = r#"fn main() = do
  let s = set_new()
  let s2 = set_insert(s, 1)
  let s3 = set_insert(s2, 2)
  println(set_len(s3))
end"#;
    let (success, _stdout, _) = emit_rust(src);
    assert!(success);
}

#[test]
fn test_pipe_with_multi_arg_builtin() {
    let src = r#"fn main() = [3, 1, 2] |> sort |> join(", ")"#;
    let (success, _stdout, _) = emit_rust(src);
    assert!(success);
}

#[test]
fn test_annotations_passthrough() {
    let src = r#"@[cfg(test)]
fn test_helper(): Int = 42"#;
    let (success, stdout, _) = emit_rust(src);
    assert!(success);
    assert!(stdout.contains("#[cfg(test)]"));
}

// ── End-to-end compile + run tests ────────────────────────────────────

/// Helper: write Star source to a temp file, compile it via `star build`, then run the binary.
/// Returns (success, stdout, stderr) from the *run* step.
fn compile_and_run(star_src: &str) -> (bool, String, String) {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let tid = std::thread::current().id();
    let dir = std::env::temp_dir().join("star_e2e_tests").join(format!("{:?}_{}", tid, id));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("main.star");
    std::fs::write(&file, star_src).unwrap();

    let bin = env!("CARGO_BIN_EXE_star");

    // Build
    let build_output = Command::new(bin)
        .args(["build", file.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .expect("Failed to run star build");

    if !build_output.status.success() {
        let stderr = String::from_utf8_lossy(&build_output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&build_output.stdout).to_string();
        return (false, stdout, format!("Build failed: {stderr}"));
    }

    // Run
    let run_output = Command::new(bin)
        .args(["run", file.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .expect("Failed to run star run");

    let success = run_output.status.success();
    let stdout = String::from_utf8_lossy(&run_output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&run_output.stderr).to_string();
    (success, stdout, stderr)
}

#[test]
fn test_e2e_hello_world() {
    let (success, stdout, stderr) = compile_and_run(r#"fn main() = println("Hello from Star!")"#);
    assert!(success, "Hello world should compile and run: {stderr}");
    assert!(stdout.contains("Hello from Star!"), "Should print greeting: {stdout}");
}

#[test]
fn test_e2e_arithmetic() {
    let (success, stdout, stderr) = compile_and_run(r#"
fn add(a: Int, b: Int): Int = a + b
fn main() = println(add(40, 2).to_string())
"#);
    assert!(success, "Arithmetic should work: {stderr}");
    assert!(stdout.contains("42"), "Should compute 42: {stdout}");
}

#[test]
fn test_e2e_pattern_matching() {
    let (success, stdout, stderr) = compile_and_run(r#"
type Shape =
  | Circle(Float)
  | Rect(Float, Float)

fn area(s: Shape): Float =
  match s
  | Circle(r) => 3.14 * r * r
  | Rect(w, h) => w * h
  end

fn main() = println(area(Rect(3.0, 4.0)).to_string())
"#);
    assert!(success, "Pattern matching should work: {stderr}");
    assert!(stdout.contains("12"), "Should compute area 12: {stdout}");
}

#[test]
fn test_e2e_list_operations() {
    let (success, stdout, stderr) = compile_and_run(r#"
fn main() =
  let result = [1, 2, 3, 4, 5]
    |> filter(fn(x) => x % 2 == 0)
    |> map(fn(x) => x * 10)
    |> fold(0, fn(acc, x) => acc + x)
  println(result.to_string())
"#);
    assert!(success, "List ops should work: {stderr}");
    assert!(stdout.contains("60"), "filter evens [2,4], *10 = [20,40], sum = 60: {stdout}");
}

#[test]
fn test_e2e_string_interpolation() {
    let (success, stdout, stderr) = compile_and_run(r#"
fn main() =
  let name = "Star"
  let version = 1
  println("Hello #{name} v#{version}!")
"#);
    assert!(success, "String interpolation should work: {stderr}");
    assert!(stdout.contains("Hello Star v1!"), "Should interpolate: {stdout}");
}

#[test]
fn test_e2e_recursive_type() {
    let (success, stdout, stderr) = compile_and_run(r#"
type Tree =
  | Leaf(Int)
  | Node(Tree, Tree)

fn tree_sum(t: Tree): Int =
  match t
  | Leaf(n) => n
  | Node(l, r) => tree_sum(l) + tree_sum(r)
  end

fn main() =
  let t = Node(Node(Leaf(1), Leaf(2)), Leaf(3))
  println(tree_sum(t).to_string())
"#);
    assert!(success, "Recursive types should work: {stderr}");
    assert!(stdout.contains("6"), "Tree sum should be 6: {stdout}");
}

#[test]
fn test_e2e_for_loop() {
    let (success, stdout, stderr) = compile_and_run(r#"
fn main() = do
  let mut total = 0
  for x in [10, 20, 30] do
    total += x
  end
  println(total.to_string())
end
"#);
    assert!(success, "For loop should work: {stderr}");
    assert!(stdout.contains("60"), "Sum should be 60: {stdout}");
}

#[test]
fn test_e2e_struct_type() {
    let (success, stdout, stderr) = compile_and_run(r#"
type Point = { x: Float, y: Float }

fn dist(p: Point): Float = sqrt(p.x * p.x + p.y * p.y)

fn main() = println(dist(Point { x: 3.0, y: 4.0 }).to_string())
"#);
    assert!(success, "Struct types should work: {stderr}");
    assert!(stdout.contains("5"), "Distance should be 5: {stdout}");
}

// ── CLI tests ─────────────────────────────────────────────────────────

#[test]
fn test_cli_version() {
    let bin = env!("CARGO_BIN_EXE_star");
    let output = Command::new(bin)
        .arg("--version")
        .output()
        .expect("Failed to run star --version");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("star"), "Version output should contain 'star': {stdout}");
}

#[test]
fn test_cli_help() {
    let bin = env!("CARGO_BIN_EXE_star");
    let output = Command::new(bin)
        .arg("--help")
        .output()
        .expect("Failed to run star --help");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("build"), "Help should mention build: {stdout}");
    assert!(stdout.contains("run"), "Help should mention run: {stdout}");
    assert!(stdout.contains("check"), "Help should mention check: {stdout}");
    assert!(stdout.contains("emit-rust"), "Help should mention emit-rust: {stdout}");
    assert!(stdout.contains("fmt"), "Help should mention fmt: {stdout}");
}

#[test]
fn test_cli_check() {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join("star_cli_tests");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("check_{}.star", id));
    std::fs::write(&file, r#"fn main() = println("ok")"#).unwrap();

    let bin = env!("CARGO_BIN_EXE_star");
    let output = Command::new(bin)
        .args(["check", file.to_str().unwrap()])
        .output()
        .expect("Failed to run star check");
    assert!(output.status.success(), "check should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("OK"), "check should print OK: {stdout}");
}

#[test]
fn test_cli_check_type_error() {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join("star_cli_tests");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("check_err_{}.star", id));
    std::fs::write(&file, r#"fn main(): Int = "not an int""#).unwrap();

    let bin = env!("CARGO_BIN_EXE_star");
    let output = Command::new(bin)
        .args(["check", file.to_str().unwrap()])
        .output()
        .expect("Failed to run star check");
    assert!(!output.status.success(), "check should fail on type error");
}

#[test]
fn test_cli_emit_rust() {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join("star_cli_tests");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("emit_{}.star", id));
    std::fs::write(&file, r#"fn add(a: Int, b: Int): Int = a + b"#).unwrap();

    let bin = env!("CARGO_BIN_EXE_star");
    let output = Command::new(bin)
        .args(["emit-rust", file.to_str().unwrap()])
        .output()
        .expect("Failed to run star emit-rust");
    assert!(output.status.success(), "emit-rust should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("fn add("), "Should emit Rust function: {stdout}");
    assert!(stdout.contains("i64"), "Should use i64 for Int: {stdout}");
}

#[test]
fn test_cli_fmt() {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join("star_cli_tests");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("fmt_{}.star", id));
    std::fs::write(&file, "fn   add(a:Int,b:Int):Int=a+b").unwrap();

    let bin = env!("CARGO_BIN_EXE_star");
    let output = Command::new(bin)
        .args(["fmt", file.to_str().unwrap()])
        .output()
        .expect("Failed to run star fmt");
    assert!(output.status.success(), "fmt should succeed: {}", String::from_utf8_lossy(&output.stderr));
    // Verify file was reformatted
    let content = std::fs::read_to_string(&file).unwrap();
    assert!(content.contains("fn add"), "Formatted file should contain function: {content}");
}

#[test]
fn test_cli_new_project() {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join("star_cli_tests").join(format!("new_{}", id));
    // Clean up if exists
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let project_name = "test-project";

    let bin = env!("CARGO_BIN_EXE_star");
    let output = Command::new(bin)
        .args(["new", project_name])
        .current_dir(&dir)
        .output()
        .expect("Failed to run star new");
    assert!(output.status.success(), "new should succeed: {}", String::from_utf8_lossy(&output.stderr));

    // Verify project structure
    assert!(dir.join(project_name).join("Star.toml").exists(), "Star.toml should exist");
    assert!(dir.join(project_name).join("src/main.star").exists(), "src/main.star should exist");
    assert!(dir.join(project_name).join(".gitignore").exists(), ".gitignore should exist");
}

#[test]
fn test_cli_unknown_command() {
    let bin = env!("CARGO_BIN_EXE_star");
    let output = Command::new(bin)
        .arg("nonexistent")
        .output()
        .expect("Failed to run star");
    assert!(!output.status.success(), "Unknown command should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown command"), "Should say unknown command: {stderr}");
}

#[test]
fn test_cli_no_args() {
    let bin = env!("CARGO_BIN_EXE_star");
    let output = Command::new(bin)
        .output()
        .expect("Failed to run star");
    assert!(!output.status.success(), "No args should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage"), "Should show usage: {stderr}");
}
