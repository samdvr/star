# Star standard library showcase
# No more rust!() — everything uses native Star builtins

fn main() =
  do
    # ── I/O ──────────────────────────────────────────────
    println("=== Star Standard Library ===")
    println()

    # ── String interpolation ─────────────────────────────
    let name = "Star"
    let version = 1
    println("Welcome to #{name} v#{version}!")
    println()

    # ── List operations with pipes ───────────────────────
    println("--- List Operations ---")
    let nums = range(1, 11)
    println("range(1, 11) = #{nums}")

    let evens = nums
      |> filter(fn(x) => x % 2 == 0)
    println("evens = #{evens}")

    let doubled = evens
      |> map(fn(x) => x * 2)
    println("doubled = #{doubled}")

    let total = doubled |> sum
    println("sum = #{total}")

    let product_val = [1, 2, 3, 4, 5] |> product
    println("product([1..5]) = #{product_val}")
    println()

    # ── Fold ─────────────────────────────────────────────
    println("--- Fold ---")
    let factorial = range(1, 6)
      |> fold(1, fn(acc, x) => acc * x)
    println("5! = #{factorial}")
    println()

    # ── Sort and reverse ─────────────────────────────────
    println("--- Sort & Reverse ---")
    let unsorted = [5, 2, 8, 1, 9, 3]
    println("unsorted = #{unsorted}")
    println("sorted   = #{sort(unsorted)}")
    println("reversed = #{reverse(unsorted)}")
    println()

    # ── String operations ────────────────────────────────
    println("--- String Operations ---")
    let greeting = "  Hello, World!  "
    println("trim:      '#{trim(greeting)}'")
    println("uppercase: #{uppercase(trim(greeting))}")
    println("lowercase: #{lowercase(trim(greeting))}")

    let csv = "apple,banana,cherry"
    let fruits = split(csv, ",")
    println("split:     #{fruits}")
    println("join:      #{join(fruits, " | ")}")
    println("contains:  #{contains(csv, "banana")}")
    println("replace:   #{replace(csv, ",", " & ")}")
    println()

    # ── Head, tail, last ─────────────────────────────────
    println("--- Head / Tail / Last ---")
    let items = [10, 20, 30, 40, 50]
    debug(head(items))
    debug(tail(items))
    debug(last(items))
    debug(init(items))
    println()

    # ── Take, drop, zip ─────────────────────────────────
    println("--- Take / Drop / Zip ---")
    println("take(3): #{take(items, 3)}")
    println("drop(3): #{drop(items, 3)}")
    let zipped = zip([1, 2, 3], ["a", "b", "c"])
    debug(zipped)
    println()

    # ── Any, all, find ───────────────────────────────────
    println("--- Any / All / Find ---")
    let has_big = items |> any(fn(x) => x > 40)
    let all_pos = items |> all(fn(x) => x > 0)
    let found = items |> find(fn(x) => x > 25)
    println("any > 40:  #{has_big}")
    println("all > 0:   #{all_pos}")
    debug(found)
    println()

    # ── Enumerate ────────────────────────────────────────
    println("--- Enumerate ---")
    let indexed = ["a", "b", "c"] |> enumerate
    debug(indexed)
    println()

    # ── Math ─────────────────────────────────────────────
    println("--- Math ---")
    println("abs(-42) = #{abs(-42)}")
    println("min(3, 7) = #{min(3, 7)}")
    println("max(3, 7) = #{max(3, 7)}")
    println("clamp(15, 0, 10) = #{clamp(15, 0, 10)}")
    println()

    # ── Conversions ──────────────────────────────────────
    println("--- Conversions ---")
    println("to_string(42) = '#{to_string(42)}'")
    println("to_int(\"123\") = #{to_int("123")}")
    println("length(\"hello\") = #{length("hello")}")
    println("length([1,2,3]) = #{length([1, 2, 3])}")
    println()

    # ── Flatten and flat_map ─────────────────────────────
    println("--- Flatten ---")
    let nested = [[1, 2], [3, 4], [5]]
    println("flatten: #{flatten(nested)}")
    println()

    # ── Push and concat ──────────────────────────────────
    println("--- Push & Concat ---")
    let base = [1, 2, 3]
    println("push(4):   #{push(base, 4)}")
    println("concat:    #{concat(base, [4, 5, 6])}")
    println()

    # ── Each (side effects in pipe) ──────────────────────
    println("--- Each ---")
    [1, 2, 3] |> each(fn(x) => println("  item: #{x}"))
    println()

    println("=== Done! ===")
  end
