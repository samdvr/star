# Star showcase: a complete working program
# Demonstrates: types, pattern matching, pipes, lambdas, do blocks

type Expr =
  | Num(Int)
  | Add(Int, Int)
  | Mul(Int, Int)

fn eval(e: Expr): Int =
  match e
  | Num(n) => n
  | Add(a, b) => a + b
  | Mul(a, b) => a * b
  end

fn double(x: Int): Int = x * 2
fn add_one(x: Int): Int = x + 1

fn main() =
  do
    # Pattern matching
    let e1 = Num(42)
    let e2 = Add(10, 20)
    let e3 = Mul(3, 7)
    println("Num(42) = #{eval(e1)}")
    println("Add(10, 20) = #{eval(e2)}")
    println("Mul(3, 7) = #{eval(e3)}")

    # Pipe operator
    let result = 5 |> double |> add_one |> double
    println("5 |> double |> add_one |> double = #{result}")

    # If expressions
    let x = 10
    let sign = if x > 0 then "positive" else "non-positive" end
    println("#{x} is #{sign}")

    # Lists
    let nums = [1, 2, 3, 4, 5]
    println("nums = #{nums}")

    # Lambda
    let sq = fn(n: Int) => n * n
    println("sq(7) = #{sq(7)}")
  end
