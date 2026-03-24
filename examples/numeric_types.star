# Numeric Type Widths
# Star supports multiple integer and float widths beyond the default Int (i64) and Float (f64).

# Signed integers of various widths
fn add_bytes(a: Int8, b: Int8): Int8 = a + b
fn add_shorts(a: Int16, b: Int16): Int16 = a + b
fn add_ints32(a: Int32, b: Int32): Int32 = a + b

# Unsigned integers
fn add_ubytes(a: UInt8, b: UInt8): UInt8 = a + b
fn add_ushorts(a: UInt16, b: UInt16): UInt16 = a + b
fn add_uints32(a: UInt32, b: UInt32): UInt32 = a + b
fn add_uints(a: UInt, b: UInt): UInt = a + b

# 32-bit float
fn add_floats32(a: Float32, b: Float32): Float32 = a + b

# Struct with mixed numeric types
type Sensor = {
  id: UInt32,
  value: Float32,
  raw: Int16
}

fn main() = do
  println("Numeric type widths demo")

  # Default types still work
  let x: Int = 42
  let y: Float = 3.14
  println("Int: #{x}, Float: #{y}")

  println("Done!")
end
