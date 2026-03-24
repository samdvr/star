# Generics and Parametric Polymorphism
# Demonstrates generic functions and types

fn identity<T>(x: T): T = x

type Pair<A, B> = {
  fst: A,
  snd: B
}

type Stack<T> = {
  items: List<T>
}

fn main() = do
  # Identity with different types
  println("identity(42) = #{identity(42)}")
  println("identity(hello) = #{identity("hello")}")
  println("identity(true) = #{identity(true)}")

  # Generic struct
  let int_pair = Pair { fst: 1, snd: 2 }
  println("Pair: (#{int_pair.fst}, #{int_pair.snd})")

  let mixed_pair = Pair { fst: "hello", snd: 42 }
  println("Mixed pair: (#{mixed_pair.fst}, #{mixed_pair.snd})")

  # Stack operations
  let s = Stack { items: [] }
  let s = Stack { items: push(s.items, 1) }
  let s = Stack { items: push(s.items, 2) }
  let s = Stack { items: push(s.items, 3) }
  println("Stack size: #{length(s.items)}")
  println("Stack empty: #{length(s.items) == 0}")

  # Generic list operations
  let nums = [3, 1, 4, 1, 5, 9, 2, 6]
  let sorted = sort(nums)
  println("Sorted: #{sorted}")

  let strings = ["banana", "apple", "cherry"]
  let sorted_strings = sort(strings)
  println("Sorted strings: #{sorted_strings}")

  # Tuples
  let t = (1, "hello", true)
  println("Tuple: #{t}")

  # Nested generics
  let nested: List<List<Int>> = [[1, 2], [3, 4], [5, 6]]
  println("Nested: #{nested}")
  println("Flattened: #{flatten(nested)}")

  println("Done!")
end
