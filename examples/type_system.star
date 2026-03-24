# Type System Showcase
# Demonstrates structs, traits, impls, and type features

type Point = {
  x: Float,
  y: Float
}

impl Point
  fn distance(self, other: Point): Float =
    sqrt(pow(self.x - other.x, 2.0) + pow(self.y - other.y, 2.0))

  fn translate(self, dx: Float, dy: Float): Point =
    Point { x: self.x + dx, y: self.y + dy }

  fn scale(self, factor: Float): Point =
    Point { x: self.x * factor, y: self.y * factor }

  fn to_string(self): String =
    "(#{self.x}, #{self.y})"
end

type Rectangle = {
  origin: Point,
  width: Float,
  height: Float
}

impl Rectangle
  fn area(self): Float = self.width * self.height
  fn perimeter(self): Float = 2.0 * (self.width + self.height)
end

# Traits
trait Describable
  fn describe(self): String
end

impl Describable for Point
  fn describe(self): String = "Point at #{self.to_string()}"
end

impl Describable for Rectangle
  fn describe(self): String =
    "Rectangle at #{self.origin.to_string()}, #{self.width}x#{self.height}"
end

# Struct update syntax
type Config = {
  debug: Bool,
  verbose: Bool,
  max_retries: Int,
  timeout: Float
}

fn default_config(): Config =
  Config { debug: false, verbose: false, max_retries: 3, timeout: 30.0 }

fn debug_config(): Config =
  Config { debug: true, verbose: true, ..default_config() }

fn main() = do
  # Points
  println("=== Points ===")
  let p1 = Point { x: 0.0, y: 0.0 }
  let p2 = Point { x: 3.0, y: 4.0 }
  println("p1 = #{p1.to_string()}")
  println("p2 = #{p2.to_string()}")
  println("Distance: #{p1.distance(p2)}")

  let p3 = p2.translate(1.0, -1.0)
  println("p2 translated: #{p3.to_string()}")

  let p4 = p2.scale(2.0)
  println("p2 scaled: #{p4.to_string()}")

  # Rectangle
  println("\n=== Rectangle ===")
  let rect = Rectangle { origin: Point { x: 0.0, y: 0.0 }, width: 10.0, height: 5.0 }
  println("Area: #{rect.area()}")
  println("Perimeter: #{rect.perimeter()}")

  # Trait method calls
  println("\n=== Describable ===")
  println(p1.describe())
  println(rect.describe())

  # Config with struct update
  println("\n=== Config ===")
  let cfg = default_config()
  println("Default debug: #{cfg.debug}, timeout: #{cfg.timeout}")

  let dbg_cfg = debug_config()
  println("Debug debug: #{dbg_cfg.debug}, verbose: #{dbg_cfg.verbose}")
  println("Debug retries: #{dbg_cfg.max_retries}, timeout: #{dbg_cfg.timeout}")

  println("\nDone!")
end
