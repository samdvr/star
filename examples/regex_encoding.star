# Star regex and encoding showcase
# Demonstrates regex matching and base64 encoding

fn main() =
  do
    println("=== Regex & Encoding ===")
    println()

    # ── Regex matching ──────────────────────────────────
    println("--- Regex ---")
    let text = "My phone is 555-1234 and office is 555-5678"

    println("match digits: #{regex_match(text, "[0-9]+")}")
    println("match alpha only: #{regex_match(text, "^[a-zA-Z]+$")}")

    let first = regex_find(text, "[0-9]{3}-[0-9]{4}")
    debug(first)

    let all_phones = regex_find_all(text, "[0-9]{3}-[0-9]{4}")
    println("all phones: #{all_phones}")

    let redacted = regex_replace(text, "[0-9]{3}-[0-9]{4}", "XXX-XXXX")
    println("redacted: #{redacted}")
    println()

    # ── Email validation example ────────────────────────
    println("--- Email Validation ---")
    let emails = ["user@example.com", "bad@", "test.user@domain.co.uk", "no-at-sign"]
    let pattern = "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
    each(emails, fn(e) => println("  #{e}: #{regex_match(e, pattern)}"))
    println()

    # ── Extract numbers from text ───────────────────────
    println("--- Extract Numbers ---")
    let report = "Sales: 150 units at $23.50 each, total $3525.00"
    let numbers = regex_find_all(report, "[0-9]+\\.?[0-9]*")
    println("numbers in text: #{numbers}")
    println()

    # ── Base64 encoding ─────────────────────────────────
    println("--- Base64 ---")
    let original = "Hello, Star!"
    let encoded = encode_base64(original)
    println("original: #{original}")
    println("encoded:  #{encoded}")

    let decoded = decode_base64(encoded)
    debug(decoded)
    println()

    println("=== Done! ===")
  end
