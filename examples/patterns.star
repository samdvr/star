# Advanced Pattern Matching Examples

type Shape =
  | Circle(Float)
  | Rect(Float, Float)
  | Triangle(Float, Float, Float)

# Or-patterns: match multiple patterns in one arm
fn describe_shape(s: Shape): String = match s
  | Circle(_) => "round"
  | Rect(_, _) | Triangle(_, _, _) => "angular"
  end

# Range patterns: match a range of integers
fn classify(n: Int): String = match n
  | 0 => "zero"
  | 1..9 => "single digit"
  | 10..99 => "double digit"
  | _ => "big number"
  end

# String patterns
fn greet(lang: String): String = match lang
  | "en" => "Hello!"
  | "es" => "Hola!"
  | "fr" => "Bonjour!"
  | _ => "Hi!"
  end

# Combined: or-patterns with integers
fn parity_name(n: Int): String = match n
  | 0 | 2 | 4 | 6 | 8 => "even digit"
  | 1 | 3 | 5 | 7 | 9 => "odd digit"
  | _ => "not a digit"
  end

fn main() = do
  println(describe_shape(Circle(5.0)))
  println(describe_shape(Rect(3.0, 4.0)))
  println(classify(0))
  println(classify(7))
  println(classify(42))
  println(classify(100))
  println(greet("en"))
  println(greet("es"))
  println(greet("de"))
  println(parity_name(3))
  println(parity_name(4))
  println(parity_name(10))
end
