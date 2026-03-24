# Star Configuration & CLI Utilities showcase
# Demonstrates argument parsing, JSON, env files, and terminal colors

fn main() =
  do
    println("=== Star Configuration & CLI ===")
    println()

    # ── Terminal Colors ─────────────────────────────────
    println("--- Terminal Colors ---")
    println(color_red("This is red"))
    println(color_green("This is green"))
    println(color_blue("This is blue"))
    println(color_yellow("This is yellow"))
    println(color_cyan("This is cyan"))
    println(color_magenta("This is magenta"))
    println(bold("This is bold"))
    println(dim("This is dim"))
    println(underline("This is underlined"))
    println()

    # Combine styles
    println(bold(color_red("Bold red error!")))
    println(bold(color_green("Bold green success!")))
    println()

    # Strip ANSI
    let colored = color_red("hello")
    let plain = strip_ansi(colored)
    println("colored length: #{length(colored)}")
    println("plain length:   #{length(plain)}")
    println("stripped: #{plain}")
    println()

    # ── Argument Parsing ────────────────────────────────
    println("--- Argument Parsing ---")
    let ac = arg_count()
    println("arg count: #{ac}")

    let first = arg_get(0)
    println("first arg: #{unwrap_or(first, "(none)")}")

    let has_v = arg_has("--verbose")
    println("has --verbose: #{has_v}")

    let name_val = arg_value("--name")
    println("--name value: #{unwrap_or(name_val, "(not set)")}")

    let pairs = arg_pairs()
    println("arg pairs: #{pairs}")
    println()

    # ── JSON ────────────────────────────────────────────
    println("--- JSON ---")

    # Build JSON
    let obj = json_object([("name", "Star"), ("version", "0.1.0"), ("type", "language")])
    println("built: #{obj}")

    let arr = json_array(["hello", "world", "star"])
    println("array: #{arr}")

    # Parse JSON
    let json = "{\"name\": \"Star\", \"version\": \"0.1.0\", \"stars\": 42, \"active\": true}"
    println("source: #{json}")

    let name = json_get(json, "name")
    println("name: #{unwrap(name)}")

    let version = json_get(json, "version")
    println("version: #{unwrap(version)}")

    let stars = json_get(json, "stars")
    println("stars: #{unwrap(stars)}")

    let active = json_get(json, "active")
    println("active: #{unwrap(active)}")

    # Nested JSON
    let nested = "{\"user\": {\"name\": \"Alice\", \"age\": 30}, \"tags\": [\"a\", \"b\"]}"
    let user = json_get(nested, "user")
    println("nested user: #{unwrap(user)}")

    let tags = json_get(nested, "tags")
    println("nested tags: #{unwrap(tags)}")

    # JSON escape
    let raw = "hello \"world\" \\ test"
    let escaped = json_escape(raw)
    println("escaped: #{escaped}")
    println()

    # ── ENV File Parsing ────────────────────────────────
    println("--- ENV File Parsing ---")
    let env_content = "# Database config\nDB_HOST=localhost\nDB_PORT=5432\nDB_NAME=\"myapp\"\n\n# API\nAPI_KEY='secret123'\nDEBUG=true"
    let vars = parse_env_string(env_content)
    each(vars, fn(pair) => println("  #{pair}"))
    println()

    # Write and load an env file
    let env_path = "/tmp/star_test.env"
    let _ = write_file(env_path, env_content)
    let loaded = load_env_file(env_path)
    println("loaded from file: #{unwrap(loaded)}")
    let _ = delete_file(env_path)
    println()

    # ── Practical: colored status output ────────────────
    println("--- Status Report ---")
    println("  [#{color_green("PASS")}] Compiler")
    println("  [#{color_green("PASS")}] Tests")
    println("  [#{color_red("FAIL")}] Lint")
    println("  [#{color_green("PASS")}] Deploy")
    println()

    println("=== Done! ===")
  end
