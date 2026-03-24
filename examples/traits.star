trait Greetable
  fn greet(self): String
end

type Dog = { name: String }

type Cat = { name: String, lives: Int }

impl Greetable for Dog
  fn greet(self): String = "Woof! I'm #{self.name}"
end

impl Greetable for Cat
  fn greet(self): String = "Meow! I'm #{self.name} with #{to_string(self.lives)} lives"
end

impl Dog
  fn bark(self): String = "BARK BARK!"
end

fn main() = do
  let dog = Dog { name: "Rex" }
  let cat = Cat { name: "Whiskers", lives: 9 }
  println(dog.greet())
  println(cat.greet())
  println(dog.bark())
end
