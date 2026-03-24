# Star Collections Algorithms & Utilities showcase
# Demonstrates searching, sorting, windowing, transformations, and aggregations

fn main() =
  do
    println("=== Star Collection Algorithms ===")
    println()

    let nums = [5, 3, 8, 1, 9, 2, 7, 4, 6]

    # ── Searching ───────────────────────────────────────
    println("--- Searching ---")
    let sorted = sort(nums)
    println("sorted: #{sorted}")

    let found = binary_search(sorted, 7)
    println("binary_search(sorted, 7): #{unwrap(found)}")

    let pos = position(nums, fn(x) => x == 8)
    println("position of 8: #{unwrap(pos)}")

    println("contains 5: #{contains_element(nums, 5)}")
    println("contains 99: #{contains_element(nums, 99)}")
    println()

    # ── Sorting ─────────────────────────────────────────
    println("--- Sorting ---")
    println("sort_desc: #{sort_desc(nums)}")

    let words = ["banana", "apple", "cherry", "date"]
    let by_len = sort_by_key(words, fn(s: String) => length(s))
    println("sort by length: #{by_len}")

    println("is_sorted [1,2,3]: #{is_sorted([1, 2, 3])}")
    println("is_sorted [3,1,2]: #{is_sorted([3, 1, 2])}")
    println()

    # ── Slicing & Windowing ─────────────────────────────
    println("--- Slicing & Windowing ---")
    let data = [1, 2, 3, 4, 5, 6, 7, 8]

    let ch = chunks(data, 3)
    println("chunks(3): #{ch}")

    let win = windows(data, 3)
    println("windows(3): #{win}")

    let third = nth(data, 2)
    println("nth(2): #{unwrap(third)}")

    let tw = take_while(data, fn(x) => x < 5)
    println("take_while(< 5): #{tw}")

    let dw = drop_while(data, fn(x) => x < 5)
    println("drop_while(< 5): #{dw}")

    let (left, right) = split_at(data, 4)
    println("split_at(4): #{left} | #{right}")
    println()

    # ── Transformations ─────────────────────────────────
    println("--- Transformations ---")

    # scan: running sum
    let running = scan([1, 2, 3, 4, 5], 0, fn(acc, x) => acc + x)
    println("scan (running sum): #{running}")

    # reduce: no initial value
    let total = reduce([1, 2, 3, 4, 5], fn(a, b) => a + b)
    println("reduce (sum): #{unwrap(total)}")

    # partition: split by predicate
    let (evens, odds) = partition([1, 2, 3, 4, 5, 6], fn(x) => x % 2 == 0)
    println("evens: #{evens}")
    println("odds:  #{odds}")

    # group_by: group by key
    let groups = group_by([1, 2, 3, 4, 5, 6], fn(x) => x % 3)
    println("group_by (mod 3): #{groups}")

    # unique
    let u = unique([1, 3, 2, 3, 1, 4, 2, 5])
    println("unique: #{u}")

    # intersperse
    let inter = intersperse([10, 20, 30], 0)
    println("intersperse(0): #{inter}")
    println()

    # ── Aggregations ────────────────────────────────────
    println("--- Aggregations ---")
    println("min_of: #{unwrap(min_of(nums))}")
    println("max_of: #{unwrap(max_of(nums))}")

    let floats = [1.5, 2.5, 3.0, 4.0]
    println("sum_float: #{sum_float(floats)}")
    println("product_float: #{product_float(floats)}")
    println()

    # ── Zip Utilities ───────────────────────────────────
    println("--- Zip Utilities ---")
    let pairs = [(1, "a"), (2, "b"), (3, "c")]
    let (nums2, letters) = unzip(pairs)
    println("unzip nums: #{nums2}")
    println("unzip letters: #{letters}")

    let sums = zip_with([1, 2, 3], [10, 20, 30], fn(a, b) => a + b)
    println("zip_with (+): #{sums}")

    let products = zip_with([2, 3, 4], [5, 6, 7], fn(a, b) => a * b)
    println("zip_with (*): #{products}")
    println()

    println("=== Done! ===")
  end
