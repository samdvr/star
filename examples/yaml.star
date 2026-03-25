# YAML parsing and encoding example

fn main() =
  do
    let yaml_data = "name: My App\nversion: 1.0\ndebug: true\nport: 8080\ntags:\n- web\n- api\n- v2"

    println("Parsing YAML:")
    match yaml_parse(yaml_data)
    | Ok(result) => println("  #{result}")
    | Err(e) => println("  Error: #{e}")
    end

    println("")

    println("Encoding YAML:")
    let encoded = yaml_encode(yaml_data)
    println(encoded)
  end
