# TOML parsing and encoding example

fn main() =
  do
    let toml_data = "title = \"My App\"\nversion = \"1.0\"\ndebug = true\nmax_connections = 100\npi = 3.14\n\n[database]\nhost = \"localhost\"\nport = 5432\n\n[server]\naddress = \"0.0.0.0\"\nworkers = 4"

    println("Parsing TOML:")
    match toml_parse(toml_data)
    | Ok(result) => println("  #{result}")
    | Err(e) => println("  Error: #{e}")
    end

    println("")

    # Encode TOML back
    println("Encoding TOML:")
    let encoded = toml_encode(toml_data)
    println(encoded)
  end
