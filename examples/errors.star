# Error Handling in Star
# Errors are values: Result<T, E> and Option<T>, just like Rust

# ── Option basics ───────────────────────────────────────

fn demo_option() =
  do
    println("=== Option ===")

    # Constructors
    let x = some(42)
    debug(x)

    # Querying
    println("is_some(x): #{is_some(x)}")
    println("is_none(x): #{is_none(x)}")

    # Extracting values
    println("unwrap(x):       #{unwrap(x)}")
    println("unwrap_or(x, 0): #{unwrap_or(x, 0)}")

    # Transforming
    let doubled = map_option(x, fn(v) => v * 2)
    debug(doubled)

    # Chaining — and_then returns a new Option
    let chained = and_then(x, fn(v) => some(v + 10))
    debug(chained)

    # Flattening nested Options
    let nested = some(some(99))
    let flat = flatten_option(nested)
    debug(flat)
  end

# ── Result basics ───────────────────────────────────────

fn safe_divide(a: Int, b: Int): Result<Int, String> =
  if b == 0 then Err("division by zero")
  else Ok(a / b)
  end

fn demo_result() =
  do
    println("")
    println("=== Result ===")

    let good = safe_divide(10, 2)
    let bad = safe_divide(10, 0)

    debug(good)
    debug(bad)

    # Querying
    println("is_ok(good): #{is_ok(good)}")
    println("is_err(bad): #{is_err(bad)}")

    # Extracting
    println("unwrap(good):       #{unwrap(good)}")
    println("unwrap_or(bad, -1): #{unwrap_or(bad, -1)}")

    # Transforming Ok value with map_result
    let mapped = map_result(good, fn(v) => v * 10)
    debug(mapped)

    # Chaining Results with and_then
    let chained = and_then(good, fn(v) => safe_divide(v, 2))
    debug(chained)

    # Convert Result -> Option
    let opt = ok(good)
    debug(opt)

    let err_opt = err(bad)
    debug(err_opt)
  end

# ── Option <-> Result conversion ────────────────────────

fn demo_conversion() =
  do
    println("")
    println("=== Conversions ===")

    # Option -> Result with ok_or
    let x = some(42)

    let r1 = ok_or(x, "missing")
    debug(r1)
  end

# ── Pattern matching on errors ──────────────────────────

fn describe_result(r: Result<Int, String>): String =
  match r
  | Ok(v) => "Success: #{v}"
  | Err(e) => "Error: #{e}"
  end

fn demo_matching() =
  do
    println("")
    println("=== Pattern Matching ===")

    let good = safe_divide(20, 4)
    let bad = safe_divide(20, 0)

    println(describe_result(good))
    println(describe_result(bad))
  end

# ── map_or (default + transform) ────────────────────────

fn demo_map_or() =
  do
    println("")
    println("=== map_or ===")

    let x = some(3)

    let r1 = map_or(x, 0, fn(v) => v * 10)
    println("map_or(Some(3), 0, *10): #{r1}")
  end

# ── or_else (recover from errors) ──────────────────────

fn demo_or_else() =
  do
    println("")
    println("=== or_else ===")

    let bad = safe_divide(10, 0)
    let recovered = or_else(bad, fn(e) => safe_divide(10, 1))
    debug(recovered)
  end

# ── expect (unwrap with message) ────────────────────────

fn demo_expect() =
  do
    println("")
    println("=== expect ===")

    let x = some(42)
    let val = expect(x, "should have a value")
    println("expect(Some(42)): #{val}")
  end

# ── Main ────────────────────────────────────────────────

fn main() =
  do
    demo_option()
    demo_result()
    demo_conversion()
    demo_matching()
    demo_map_or()
    demo_or_else()
    demo_expect()
    println("")
    println("All error handling demos complete!")
  end
