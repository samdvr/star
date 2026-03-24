# Closures and Higher-Order Functions
# Demonstrates lambdas, closures, and functional patterns

fn apply(f: fn(Int) -> Int, x: Int): Int = f(x)

fn compose(f: fn(Int) -> Int, g: fn(Int) -> Int): fn(Int) -> Int =
  fn(x) => f(g(x))

fn main() = do
  # Basic lambda
  let double = fn(x) => x * 2
  let add_one = fn(x) => x + 1
  println("double(5) = #{apply(double, 5)}")
  println("add_one(5) = #{apply(add_one, 5)}")

  # Compose functions
  let double_then_add = compose(add_one, double)
  println("double_then_add(5) = #{apply(double_then_add, 5)}")

  # Lambda with multiple params
  let add = fn(a: Int, b: Int) => a + b
  println("add(3, 4) = #{add(3, 4)}")

  # Lambda in pipe
  let result = [1, 2, 3, 4, 5]
    |> map(fn(x) => x * x)
    |> filter(fn(x) => x > 5)
    |> fold(0, fn(acc, x) => acc + x)
  println("Sum of squares > 5: #{result}")

  # Closure capturing variables
  let multiplier = 3
  let times_three = fn(x) => x * multiplier
  println("times_three(7) = #{times_three(7)}")

  # Returning closures from functions
  let nums = [10, 20, 30, 40, 50]
  let evens = filter(nums, fn(x) => x % 2 == 0)
  println("Even numbers: #{evens}")

  # Chained operations
  let processed = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    |> filter(fn(x) => x % 2 == 0)
    |> map(fn(x) => x * 10)
  println("Processed: #{processed}")

  # Each (for side effects)
  print("Counting: ")
  each([1, 2, 3, 4, 5], fn(x) => print("#{x} "))
  println("")

  # Flat map
  let nested = [1, 2, 3]
    |> flat_map(fn(x) => [x, x * 10])
  println("Flat mapped: #{nested}")

  # Any / All
  let has_negative = any([1, -2, 3], fn(x) => x < 0)
  let all_positive = all([1, 2, 3], fn(x) => x > 0)
  println("Has negative: #{has_negative}")
  println("All positive: #{all_positive}")

  println("Done!")
end
