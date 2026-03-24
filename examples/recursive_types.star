# Recursive types — auto Box insertion

type Expr =
  | Num(Int)
  | Add(Expr, Expr)
  | Neg(Expr)

fn eval(e: Expr): Int =
  match e
  | Num(n) => n
  | Add(a, b) => eval(a) + eval(b)
  | Neg(x) => 0 - eval(x)
  end

fn main() =
  let e = Add(Num(1), Add(Num(2), Num(3)))
  println(to_string(eval(e)))
