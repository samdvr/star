# Star collections showcase
# Demonstrates HashMap, HashSet, Deque, and Heap

fn main() =
  do
    println("=== Star Collections ===")
    println()

    # ── HashMap ─────────────────────────────────────────
    println("--- HashMap ---")

    # Build a map by inserting key-value pairs
    let scores = map_new()
      |> map_insert("Alice", 95)
      |> map_insert("Bob", 87)
      |> map_insert("Carol", 92)

    println("scores size: #{map_size(scores)}")

    # Lookup
    let alice = map_get(scores, "Alice")
    debug(alice)

    let missing = map_get(scores, "Dave")
    debug(missing)

    # Check key existence
    println("has Bob: #{map_contains_key(scores, "Bob")}")
    println("has Eve: #{map_contains_key(scores, "Eve")}")

    # Keys and values
    let keys = map_keys(scores)
    let vals = map_values(scores)
    println("keys: #{sort(keys)}")
    println("values: #{sort(vals)}")

    # Entries as list of tuples
    let entries = map_entries(scores)
    debug(entries)

    # Remove a key
    let fewer = map_remove(scores, "Bob")
    println("after remove Bob: size = #{map_size(fewer)}")

    # Merge two maps
    let extra = map_new()
      |> map_insert("Dave", 88)
      |> map_insert("Eve", 91)
    let merged = map_merge(scores, extra)
    println("merged size: #{map_size(merged)}")

    # Build map from list of tuples
    let from_pairs = map_from_list([(1, "one"), (2, "two"), (3, "three")])
    println("from pairs: #{from_pairs}")
    println()

    # ── HashSet ─────────────────────────────────────────
    println("--- HashSet ---")

    # Build from list (deduplicates automatically)
    let fruits = set_from_list(["apple", "banana", "cherry", "apple"])
    println("fruits (deduped): size = #{set_size(fruits)}")

    # Insert and contains
    let more_fruits = set_insert(fruits, "date")
    println("after insert date: size = #{set_size(more_fruits)}")
    println("has banana: #{set_contains(more_fruits, "banana")}")
    println("has grape: #{set_contains(more_fruits, "grape")}")

    # Remove
    let less_fruits = set_remove(more_fruits, "banana")
    println("after remove banana: size = #{set_size(less_fruits)}")

    # Set operations
    let a = set_from_list([1, 2, 3, 4, 5])
    let b = set_from_list([3, 4, 5, 6, 7])

    let u = set_union(a, b)
    println("union size: #{set_size(u)}")

    let i = set_intersection(a, b)
    let inter_list = set_to_list(i) |> sort
    println("intersection: #{inter_list}")

    let d = set_difference(a, b)
    let diff_list = set_to_list(d) |> sort
    println("a - b: #{diff_list}")
    println()

    # ── Deque (double-ended queue) ──────────────────────
    println("--- Deque ---")

    let dq = deque_new()
      |> deque_push_back(1)
      |> deque_push_back(2)
      |> deque_push_back(3)
    println("deque: #{deque_to_list(dq)}")
    println("size: #{deque_size(dq)}")

    # Push front
    let dq2 = deque_push_front(dq, 0)
    println("after push_front(0): #{deque_to_list(dq2)}")

    # Pop from both ends
    let result_back = deque_pop_back(dq2)
    debug(result_back)

    let result_front = deque_pop_front(dq2)
    debug(result_front)

    # Build from list
    let dq3 = deque_from_list([10, 20, 30])
    println("from list: #{deque_to_list(dq3)}")
    println()

    # ── Heap (max-heap / priority queue) ────────────────
    println("--- Heap (Priority Queue) ---")

    let h = heap_new()
      |> heap_push(3)
      |> heap_push(1)
      |> heap_push(4)
      |> heap_push(1)
      |> heap_push(5)
      |> heap_push(9)
      |> heap_push(2)
      |> heap_push(6)
    println("heap size: #{heap_size(h)}")

    # Peek at the max
    let top = heap_peek(h)
    debug(top)

    # Pop the max
    let popped = heap_pop(h)
    debug(popped)

    # Sorted extraction
    let sorted = heap_to_list(h)
    println("sorted: #{sorted}")

    # Build from list
    let h2 = heap_from_list([50, 10, 40, 20, 30])
    println("from list sorted: #{heap_to_list(h2)}")
    println("peek: #{unwrap(heap_peek(h2))}")
    println()

    println("=== Done! ===")
  end
