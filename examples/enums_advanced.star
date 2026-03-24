# Advanced Enum Patterns
# Demonstrates complex enums, pattern matching, and recursive types

type Color =
  | Red
  | Green
  | Blue
  | Custom(Int, Int, Int)

fn color_name(c: Color): String = match c
  | Red => "red"
  | Green => "green"
  | Blue => "blue"
  | Custom(r, g, b) => "rgb(#{r}, #{g}, #{b})"
  end

type Shape =
  | Circle(Float)
  | Rectangle(Float, Float)
  | Triangle(Float, Float, Float)
  | Point

fn area(s: Shape): Float = match s
  | Circle(r) => 3.14159 * r * r
  | Rectangle(w, h) => w * h
  | Triangle(a, b, c) => do
    let s = (a + b + c) / 2.0
    sqrt(s * (s - a) * (s - b) * (s - c))
  end
  | Point => 0.0
  end

fn describe(s: Shape): String = match s
  | Circle(_) => "circle"
  | Rectangle(_, _) => "rectangle"
  | Triangle(_, _, _) => "triangle"
  | Point => "point"
  end

# Recursive type: expression tree
type Expr =
  | Lit(Int)
  | Add(Expr, Expr)
  | Mul(Expr, Expr)
  | Neg(Expr)

fn eval(e: Expr): Int = match e
  | Lit(n) => n
  | Add(a, b) => eval(a) + eval(b)
  | Mul(a, b) => eval(a) * eval(b)
  | Neg(a) => 0 - eval(a)
  end

fn show_expr(e: Expr): String = match e
  | Lit(n) => to_string(n)
  | Add(a, b) => "(#{show_expr(a)} + #{show_expr(b)})"
  | Mul(a, b) => "(#{show_expr(a)} * #{show_expr(b)})"
  | Neg(a) => "(-#{show_expr(a)})"
  end

# Option-like type
type Maybe =
  | Just(Int)
  | Nothing

fn safe_div(a: Int, b: Int): Maybe =
  if b == 0 then Nothing else Just(a / b) end

fn maybe_to_string(m: Maybe): String = match m
  | Just(x) => "Just(#{x})"
  | Nothing => "Nothing"
  end

fn main() = do
  # Colors
  println("Colors:")
  println("  #{color_name(Red)}")
  println("  #{color_name(Green)}")
  println("  #{color_name(Custom(255, 128, 0))}")

  # Shapes
  println("\nShapes:")
  let shapes = [Circle(5.0), Rectangle(3.0, 4.0), Triangle(3.0, 4.0, 5.0), Point]
  for s in shapes do
    println("  #{describe(s)}: area = #{area(s)}")
  end

  # Expression tree
  println("\nExpressions:")
  let e1 = Add(Lit(1), Mul(Lit(2), Lit(3)))
  println("  #{show_expr(e1)} = #{eval(e1)}")

  let e2 = Mul(Add(Lit(1), Lit(2)), Neg(Lit(4)))
  println("  #{show_expr(e2)} = #{eval(e2)}")

  # Safe division
  println("\nSafe division:")
  println("  10 / 3 = #{maybe_to_string(safe_div(10, 3))}")
  println("  10 / 0 = #{maybe_to_string(safe_div(10, 0))}")

  println("\nDone!")
end
