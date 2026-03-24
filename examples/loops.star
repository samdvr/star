# Loops and compound assignment examples

fn main() =
  # For loop with sum
  let mut total = 0
  for x in [1, 2, 3, 4, 5] do
    total += x
  end
  println(to_string(total))

  # While loop: compute power of 2
  let mut n = 1
  while n < 100 do
    n *= 2
  end
  println(to_string(n))

  # For loop iteration
  for name in ["Alice", "Bob", "Charlie"] do
    println(name)
  end

  # Index access
  let items = [10, 20, 30]
  println(to_string(items[1]))

  # Compound assignment
  let mut x = 100
  x -= 30
  x /= 2
  x %= 13
  println(to_string(x))
