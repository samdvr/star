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
