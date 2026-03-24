# Star Testing & Debugging Utilities showcase
# Demonstrates assertions, logging, profiling, and debug helpers

fn fib(n: Int): Int =
  if n <= 1 then n
  else fib(n - 1) + fib(n - 2)
  end

fn main() =
  do
    println("=== Star Testing & Debugging ===")
    println()

    # ── Assertions ──────────────────────────────────────
    println("--- Assertions ---")
    assert(1 + 1 == 2)
    println("assert(1 + 1 == 2) passed")

    assert_eq(4 * 5, 20)
    println("assert_eq(4 * 5, 20) passed")

    assert_ne("hello", "world")
    println("assert_ne(hello, world) passed")

    assert_msg(10 > 5, "ten should be greater than five")
    println("assert_msg with custom message passed")

    assert_eq(fib(10), 55)
    println("assert_eq(fib(10), 55) passed")
    println()

    # ── Logging ─────────────────────────────────────────
    println("--- Logging (goes to stderr) ---")
    log_debug("this is a debug message")
    log_info("application started")
    log_warn("disk space getting low")
    log_error("connection failed")
    println("(check stderr for log output)")
    println()

    # ── Debug Utilities ─────────────────────────────────
    println("--- Debug Utilities ---")

    # dbg prints to stderr and returns the value
    let x = dbg(42 + 8)
    println("dbg returned: #{x}")

    # type_name_of returns the Rust type name
    let t1 = type_name_of(42)
    println("type of 42: #{t1}")

    let t2 = type_name_of("hello")
    println("type of string: #{t2}")

    let t3 = type_name_of([1, 2, 3])
    println("type of list: #{t3}")

    let t4 = type_name_of(true)
    println("type of bool: #{t4}")

    let t5 = type_name_of(3.14)
    println("type of float: #{t5}")
    println()

    # ── Profiling ───────────────────────────────────────
    println("--- Profiling ---")

    # time_fn runs a closure and returns (result, elapsed_ms)
    let (result, ms) = time_fn(fn() => fib(30))
    println("fib(30) = #{result}, took #{ms}ms")

    # bench runs a closure N times and returns average ms
    let avg = bench(100, fn() => fib(20))
    println("fib(20) avg over 100 runs: #{avg}ms")
    println()

    println("=== All tests passed! ===")
  end
