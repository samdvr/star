pub fn square(x: Int): Int = x * x

pub fn cube(x: Int): Int = x * x * x

pub fn factorial(n: Int): Int =
  if n <= 1 then 1 else n * factorial(n - 1) end

fn internal_helper(x: Int): Int = x + 1
