# Star example: ADTs, pattern matching, and pipes

type Shape =
  | Circle(Float)
  | Rectangle(Float, Float)
  | Point

fn area(s: Shape): Float =
  match s
  | Circle(r) => 3.14159 * r * r
  | Rectangle(w, h) => w * h
  | Point => 0.0
  end

fn describe(s: Shape): String =
  match s
  | Circle(_) => "circle"
  | Rectangle(_, _) => "rectangle"
  | Point => "point"
  end

fn main() =
  do
    println("Circle area: #{area(Circle(5.0))}")
    println("Rectangle area: #{area(Rectangle(3.0, 4.0))}")
    println("#{describe(Point)} has area #{area(Point)}")
  end
