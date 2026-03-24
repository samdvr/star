# Star string processing showcase
# Comprehensive string operations without external crates

fn main() =
  do
    println("=== Star String Processing ===")
    println()

    # ── Trimming ────────────────────────────────────────
    println("--- Trimming ---")
    let padded = "  hello world  "
    println("original:   '#{padded}'")
    println("trim:       '#{trim(padded)}'")
    println("trim_start: '#{trim_start(padded)}'")
    println("trim_end:   '#{trim_end(padded)}'")
    println()

    # ── Case conversion ─────────────────────────────────
    println("--- Case Conversion ---")
    let greeting = "hello world"
    println("uppercase:  #{uppercase(greeting)}")
    println("lowercase:  #{lowercase("HELLO")}")
    println("capitalize: #{capitalize(greeting)}")
    println()

    # ── Substring & Indexing ─────────────────────────────
    println("--- Substring & Indexing ---")
    let text = "Hello, World!"
    println("substring(1, 5): #{substring(text, 1, 5)}")
    println("substring(7):    #{substring(text, 7)}")
    println("char_at(0):      #{unwrap(char_at(text, 0))}")
    println("char_at(7):      #{unwrap(char_at(text, 7))}")
    println()

    # ── Search ──────────────────────────────────────────
    println("--- Search ---")
    let sentence = "the quick brown fox jumps over the lazy dog"
    println("index_of 'fox':      #{unwrap(index_of(sentence, "fox"))}")
    println("index_of 'cat':      #{is_none(index_of(sentence, "cat"))}")
    println("last_index_of 'the': #{unwrap(last_index_of(sentence, "the"))}")
    println("contains 'quick':    #{contains(sentence, "quick")}")
    println("starts_with 'the':   #{starts_with(sentence, "the")}")
    println("ends_with 'dog':     #{ends_with(sentence, "dog")}")
    println()

    # ── Replace ─────────────────────────────────────────
    println("--- Replace ---")
    let csv = "apple,banana,apple,cherry"
    println("replace all:   #{replace(csv, "apple", "APPLE")}")
    println("replace first: #{replace_first(csv, "apple", "APPLE")}")
    println()

    # ── Strip prefix/suffix ─────────────────────────────
    println("--- Strip Prefix / Suffix ---")
    let path = "/usr/local/bin"
    debug(strip_prefix(path, "/usr"))
    debug(strip_suffix(path, "/bin"))
    debug(strip_prefix(path, "/etc"))
    println()

    # ── Splitting & Joining ─────────────────────────────
    println("--- Split / Join / Lines / Words ---")
    let multi = "line one\nline two\nline three"
    println("lines: #{lines(multi)}")
    println("words: #{words("  hello   world  foo  ")}")
    let parts = split("a::b::c", "::")
    println("split: #{parts}")
    println("join:  #{join(parts, " -> ")}")
    println()

    # ── Padding ─────────────────────────────────────────
    println("--- Padding ---")
    println("pad_left:  '#{pad_left("42", 6)}'")
    println("pad_right: '#{pad_right("hi", 6)}'")
    println("pad_left0: '#{pad_left("42", 6, "0")}'")
    println("pad_right*: '#{pad_right("hi", 6, "*")}'")
    println()

    # ── Repeat & Reverse ────────────────────────────────
    println("--- Repeat & Reverse ---")
    println("repeat:  #{repeat("ab", 4)}")
    println("repeat:  #{repeat("-", 20)}")
    println("reverse: #{reverse_string("Star!")}")
    println()

    # ── Character operations ────────────────────────────
    println("--- Characters ---")
    let s = "Hello"
    println("chars: #{chars(s)}")
    println("length: #{length(s)}")
    println("char_code('A'): #{char_code("A")}")
    println("from_char_code(65): #{from_char_code(65)}")
    println("from_char_code(9733): #{from_char_code(9733)}")
    println()

    # ── Bytes ───────────────────────────────────────────
    println("--- Bytes ---")
    let b = bytes("Hi!")
    println("bytes: #{b}")
    println("from_bytes: #{from_bytes(b)}")
    println()

    # ── Predicates ──────────────────────────────────────
    println("--- Predicates ---")
    println("is_empty(\"\"):     #{is_empty("")}")
    println("is_empty(\"hi\"):   #{is_empty("hi")}")
    println("is_blank(\"  \"):   #{is_blank("  ")}")
    println("is_blank(\" a \"):  #{is_blank(" a ")}")
    println("is_numeric(\"123\"): #{is_numeric("123")}")
    println("is_numeric(\"12a\"): #{is_numeric("12a")}")
    println("is_alphabetic(\"abc\"): #{is_alphabetic("abc")}")
    println("is_alphanumeric(\"a1\"): #{is_alphanumeric("a1")}")
    println()

    # ── Practical example: slug generator ───────────────
    println("--- Practical: Slug Generator ---")
    let title = "  Hello, World! This is Star.  "
    let slug = title
      |> trim
      |> lowercase
      |> replace(",", "")
      |> replace(".", "")
      |> replace("!", "")
      |> words
      |> join("-")
    println("title: '#{title}'")
    println("slug:  '#{slug}'")
    println()

    # ── Practical example: CSV parser ───────────────────
    println("--- Practical: CSV Line Parser ---")
    let header = "name,age,city"
    let row = "Alice,30,New York"
    let fields = split(header, ",")
    let values = split(row, ",")
    let pairs = zip(fields, values)
    debug(pairs)
    println()

    println("=== Done! ===")
  end
