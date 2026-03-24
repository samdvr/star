# JSON serialization and parsing demo

fn main() = do
  # Parse a JSON object
  let obj = json_parse("{\"name\": \"Star\", \"version\": 1, \"active\": true}")
  println("Parsed object: #{obj}")

  # Parse a JSON array
  let arr = json_parse("[1, 2, 3, \"hello\", null]")
  println("Parsed array: #{arr}")

  # Parse nested JSON
  let nested = json_parse("{\"user\": {\"name\": \"Alice\", \"tags\": [\"admin\", \"dev\"]}}")
  println("Parsed nested: #{nested}")

  # json_encode a value to a debug string
  let encoded = json_encode("hello world")
  println("Encoded string: #{encoded}")

  let encoded_num = json_encode(42)
  println("Encoded number: #{encoded_num}")

  # Use json_get to extract fields
  let json_str = "{\"language\": \"Star\", \"version\": \"0.1\"}"
  let lang = json_get(json_str, "language")
  println("Language: #{lang}")

  # Build JSON objects from data
  let pairs = [("name", "Star"), ("type", "language")]
  let built = json_object(pairs)
  println("Built object: #{built}")

  # Build JSON arrays
  let items = ["alpha", "beta", "gamma"]
  let built_arr = json_array(items)
  println("Built array: #{built_arr}")

  # Escape special characters in JSON strings
  let escaped = json_escape("hello \"world\" \n tab\t")
  println("Escaped: #{escaped}")

  println("JSON demo complete!")
end
