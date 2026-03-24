# Control Flow
# Demonstrates if/else, match, for, while, break, continue

fn classify(n: Int): String =
  if n > 0 then "positive"
  else if n < 0 then "negative"
  else "zero"
  end end

fn day_name(n: Int): String = match n
  | 1 => "Monday"
  | 2 => "Tuesday"
  | 3 => "Wednesday"
  | 4 => "Thursday"
  | 5 => "Friday"
  | 6 => "Saturday"
  | 7 => "Sunday"
  | _ => "Unknown"
  end

fn main() = do
  # Number classification
  println("=== Classification ===")
  println("5: #{classify(5)}")
  println("0: #{classify(0)}")
  println("-3: #{classify(-3)}")

  # Pattern matching
  println("\n=== Days ===")
  for d in [1, 2, 3, 4, 5, 6, 7] do
    println("#{d} = #{day_name(d)}")
  end

  # Simple for loop
  println("\n=== For Loop ===")
  for x in [1, 2, 3, 4, 5] do
    println("x = #{x}")
  end

  # While loop
  println("\n=== While Loop ===")
  let mut i = 1
  while i <= 5 do
    println("i = #{i}")
    i += 1
  end

  # For loop with break
  println("\n=== Break ===")
  for x in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10] do
    if x > 5 then break end
    println("#{x}")
  end

  # For loop with continue
  println("\n=== Continue (skip evens) ===")
  for x in [1, 2, 3, 4, 5, 6, 7, 8] do
    if x % 2 == 0 then continue end
    println("#{x}")
  end

  # Compound assignment
  println("\n=== Compound Assignment ===")
  let mut val = 100
  val += 50
  println("After += 50: #{val}")
  val -= 30
  println("After -= 30: #{val}")
  val *= 2
  println("After *= 2: #{val}")

  println("\nDone!")
end
