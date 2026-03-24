use Math
use Utils

fn main() = do
  println("=== Module System Demo ===")

  println("Math.square(5) = #{square(5)}")
  println("Math.cube(3) = #{cube(3)}")
  println("Math.factorial(6) = #{factorial(6)}")

  println(greet("Star"))
  println(repeat_str("*", 10))
  println(shout("modules work"))
end
