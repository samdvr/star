# Extern function example
# Declares a Star function that delegates to a Rust function path

extern fn fast_sqrt(x: Float): Float = "f64::sqrt"

fn main() = do
  let result = fast_sqrt(144.0)
  println(to_string(result))
end
