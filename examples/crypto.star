# Star Cryptography & Security showcase
# Demonstrates hashing, secure random generation, and UUIDs

fn main() =
  do
    println("=== Star Cryptography & Security ===")
    println()

    # ── Hashing ─────────────────────────────────────────
    println("--- SHA-256 ---")
    let h1 = sha256("hello")
    println("sha256('hello'): #{h1}")

    let h2 = sha256("hello world")
    println("sha256('hello world'): #{h2}")

    let h3 = sha256("")
    println("sha256(''): #{h3}")
    println()

    println("--- SHA-512 ---")
    let h4 = sha512("hello")
    println("sha512('hello'): #{h4}")
    println()

    println("--- MD5 ---")
    let h5 = md5("hello")
    println("md5('hello'): #{h5}")

    let h6 = md5("The quick brown fox jumps over the lazy dog")
    println("md5('The quick...'): #{h6}")
    println()

    println("--- Hash (fast, non-crypto) ---")
    let h7 = hash_bytes("hello")
    println("hash_bytes('hello'): #{h7}")

    let h8 = hash_bytes("hello")
    println("hash_bytes('hello') again: #{h8}")
    assert_eq(h7, h8)
    println("same input = same hash (deterministic)")
    println()

    # ── Secure Random ───────────────────────────────────
    println("--- Secure Random ---")
    let bytes = secure_random_bytes(8)
    println("8 random bytes: #{bytes}")

    let hex1 = secure_random_hex(16)
    println("16 random bytes as hex: #{hex1}")
    assert_eq(length(hex1), 32)

    let hex2 = secure_random_hex(16)
    println("another 16 bytes hex:   #{hex2}")
    assert_ne(hex1, hex2)
    println("two calls produce different values")
    println()

    # ── UUID v4 ─────────────────────────────────────────
    println("--- UUID v4 ---")
    let id1 = uuid_v4()
    println("uuid: #{id1}")
    assert_eq(length(id1), 36)

    let id2 = uuid_v4()
    println("uuid: #{id2}")
    assert_ne(id1, id2)
    println("each UUID is unique")
    println()

    # ── Practical: integrity check ──────────────────────
    println("--- Integrity Check ---")
    let data = "important data payload"
    let checksum = sha256(data)
    println("data: #{data}")
    println("checksum: #{checksum}")

    # Verify integrity
    let verified = sha256(data) == checksum
    println("integrity check: #{verified}")
    assert(verified)
    println()

    println("=== Done! ===")
  end
