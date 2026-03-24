# Star example: FizzBuzz with pattern matching and pipes

fn fizzbuzz(n: Int): String =
  match (n % 3, n % 5)
  | (0, 0) => "FizzBuzz"
  | (0, _) => "Fizz"
  | (_, 0) => "Buzz"
  | _ => "number"
  end

fn main() =
  fizzbuzz(15)
