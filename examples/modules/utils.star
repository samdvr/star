pub fn greet(name: String): String = "Hello, #{name}!"

pub fn repeat_str(s: String, n: Int): String =
  if n <= 0 then "" else "#{s}#{repeat_str(s, n - 1)}" end

pub fn shout(msg: String): String = "#{uppercase(msg)}!"
