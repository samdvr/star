# Star example: functional pipeline

fn double(x: Int): Int = x * 2

fn add_one(x: Int): Int = x + 1

fn is_even(x: Int): Bool = x % 2 == 0

fn main() =
  let result = 5 |> double |> add_one
  println("result = #{result}")
