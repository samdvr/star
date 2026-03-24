# Functional Programming Patterns
# Demonstrates functional style with Star's stdlib

fn is_even(n: Int): Bool = n % 2 == 0
fn square(n: Int): Int = n * n
fn double(n: Int): Int = n * 2

fn main() = do
  let nums = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

  # Pipeline transformations
  println("=== Pipeline Operations ===")
  let evens = nums |> filter(fn(x) => is_even(x))
  println("Evens: #{evens}")

  let doubled_evens = nums
    |> filter(fn(x) => is_even(x))
    |> map(fn(x) => double(x))
  println("Doubled evens: #{doubled_evens}")

  let sum_of_squares = nums
    |> map(fn(x) => square(x))
    |> fold(0, fn(acc, x) => acc + x)
  println("Sum of squares: #{sum_of_squares}")

  # List operations
  println("\n=== List Operations ===")
  println("Head: #{head(nums)}")
  println("Tail: #{tail(nums)}")
  println("Last: #{last(nums)}")
  println("Init: #{init(nums)}")
  println("Take 3: #{take(nums, 3)}")
  println("Drop 7: #{drop(nums, 7)}")
  println("Reversed: #{reverse(nums)}")

  # Zip and enumerate
  println("\n=== Zip & Enumerate ===")
  let letters = ["a", "b", "c", "d", "e"]
  let zipped = zip(take(nums, 5), letters)
  println("Zipped: #{zipped}")

  let enumerated = enumerate(letters)
  println("Enumerated: #{enumerated}")

  # Aggregations
  println("\n=== Aggregations ===")
  println("Sum: #{sum(nums)}")
  println("Product of [1,2,3,4,5]: #{product([1, 2, 3, 4, 5])}")
  println("Any > 5: #{any(nums, fn(x) => x > 5)}")
  println("All > 0: #{all(nums, fn(x) => x > 0)}")

  # Find
  let found = find(nums, fn(x) => x > 7)
  println("First > 7: #{found}")

  # Sort
  println("\n=== Sorting ===")
  let unsorted = [5, 3, 8, 1, 9, 2, 7, 4, 6, 10]
  println("Sorted: #{sort(unsorted)}")

  # Dedup
  let with_dups = [1, 1, 2, 2, 3, 3, 2, 2, 1, 1]
  println("Deduped (sorted first): #{dedup(sort(with_dups))}")

  # Flatten
  let nested = [[1, 2], [3, 4], [5, 6]]
  println("Flattened: #{flatten(nested)}")

  # String operations
  println("\n=== String Ops ===")
  let words_list = ["hello", "world", "from", "star"]
  println("Joined: #{join(words_list, " ")}")

  let sentence = "the quick brown fox"
  println("Words: #{words(sentence)}")
  println("Uppercase: #{uppercase(sentence)}")

  println("\nDone!")
end
