# Math & Numeric Utilities in Star

fn demo_trig() =
  do
    println("=== Trigonometry ===")

    let angle = to_radians(45.0)
    println("sin(45°): #{sin(angle)}")
    println("cos(45°): #{cos(angle)}")
    println("tan(45°): #{tan(angle)}")

    println("asin(1.0): #{to_degrees(asin(1.0))}°")
    println("acos(0.0): #{to_degrees(acos(0.0))}°")
    println("atan(1.0): #{to_degrees(atan(1.0))}°")
    println("atan2(1, 1): #{to_degrees(atan2(1.0, 1.0))}°")
  end

fn demo_rounding() =
  do
    println("")
    println("=== Rounding ===")

    println("floor(3.7):    #{floor(3.7)}")
    println("ceil(3.2):     #{ceil(3.2)}")
    println("round(3.5):    #{round(3.5)}")
    println("round(3.4):    #{round(3.4)}")
    println("truncate(3.9): #{truncate(3.9)}")
    println("truncate(-3.9): #{truncate(-3.9)}")
  end

fn demo_logarithms() =
  do
    println("")
    println("=== Logarithms & Exponentials ===")

    println("log(e):     #{log(e_const())}")
    println("log2(8):    #{log2(8.0)}")
    println("log10(100): #{log10(100.0)}")
    println("exp(1):     #{exp(1.0)}")
    println("exp2(3):    #{exp2(3.0)}")
  end

fn demo_misc_math() =
  do
    println("")
    println("=== Misc Math ===")

    println("sqrt(144):    #{sqrt(144.0)}")
    println("cbrt(27):     #{cbrt(27.0)}")
    println("pow(2, 10):   #{pow(2.0, 10.0)}")
    println("hypot(3, 4):  #{hypot(3.0, 4.0)}")
    println("abs(-42):     #{abs(-42)}")
    println("signum(-5):   #{signum(-5.0)}")
    println("signum(0):    #{signum(0.0)}")
    println("signum(5):    #{signum(5.0)}")
    println("min(3, 7):    #{min(3, 7)}")
    println("max(3, 7):    #{max(3, 7)}")
    println("clamp(15, 0, 10): #{clamp(15, 0, 10)}")
  end

fn demo_constants() =
  do
    println("")
    println("=== Constants ===")

    println("pi:   #{pi()}")
    println("e:    #{e_const()}")
  end

fn demo_predicates() =
  do
    println("")
    println("=== Float Predicates ===")

    println("is_nan(0.0 / 0.0):  #{is_nan(nan())}")
    println("is_nan(1.0):        #{is_nan(1.0)}")
    println("is_infinite(1/0):   #{is_infinite(infinity())}")
    println("is_finite(42.0):    #{is_finite(42.0)}")
  end

fn demo_integer_math() =
  do
    println("")
    println("=== Integer Math ===")

    println("gcd(12, 8):   #{gcd(12, 8)}")
    println("gcd(54, 24):  #{gcd(54, 24)}")
    println("lcm(4, 6):    #{lcm(4, 6)}")
    println("lcm(12, 18):  #{lcm(12, 18)}")
  end

fn demo_random() =
  do
    println("")
    println("=== Random ===")

    println("random():          #{random()}")
    println("random_range(1,100): #{random_range(1, 100)}")
    println("random_float():    #{random_float()}")
  end

fn main() =
  do
    demo_trig()
    demo_rounding()
    demo_logarithms()
    demo_misc_math()
    demo_constants()
    demo_predicates()
    demo_integer_math()
    demo_random()
    println("")
    println("All math demos complete!")
  end
