# Date & Time in Star
# Uses Rust's std::time — no external crates needed

fn demo_timestamps() =
  do
    println("=== Timestamps ===")

    let secs = now()
    let ms = now_ms()
    println("Unix timestamp (secs): #{secs}")
    println("Unix timestamp (ms):   #{ms}")

    # Format a timestamp as ISO 8601
    let formatted = format_timestamp(secs)
    println("Formatted: #{formatted}")

    # Known timestamp: 2024-01-01 00:00:00 UTC = 1704067200
    let y2024 = format_timestamp(1704067200)
    println("2024-01-01: #{y2024}")

    # Unix epoch
    let epoch = format_timestamp(0)
    println("Epoch: #{epoch}")
  end

fn demo_parsing() =
  do
    println("")
    println("=== Parsing ===")

    let parsed = parse_timestamp("2024-06-15T12:30:45Z")
    debug(parsed)

    # Round-trip: format -> parse -> format
    let ts = 1718451045
    let fmt = format_timestamp(ts)
    let reparsed = parse_timestamp(fmt)
    debug(reparsed)
  end

fn demo_monotonic() =
  do
    println("")
    println("=== Monotonic Clock ===")

    # Measure execution time
    let start = monotonic()
    sleep_millis(10)
    let elapsed_time = elapsed_ms(start)
    println("Elapsed: #{elapsed_time} ms")

    let elapsed_secs = elapsed(start)
    println("Elapsed (f64): #{elapsed_secs} secs")
  end

fn demo_durations() =
  do
    println("")
    println("=== Durations ===")

    let d1 = duration_secs(5)
    let d2 = duration_ms(2500)
    debug(d1)
    debug(d2)
  end

fn demo_arithmetic() =
  do
    println("")
    println("=== Time Arithmetic ===")

    let current = now()
    let one_day = 86400
    let one_hour = 3600

    let tomorrow = current + one_day
    let yesterday = current - one_day
    let in_one_hour = current + one_hour

    println("Now:       #{format_timestamp(current)}")
    println("Tomorrow:  #{format_timestamp(tomorrow)}")
    println("Yesterday: #{format_timestamp(yesterday)}")
    println("+1 hour:   #{format_timestamp(in_one_hour)}")
  end

fn main() =
  do
    demo_timestamps()
    demo_parsing()
    demo_monotonic()
    demo_durations()
    demo_arithmetic()
    println("")
    println("All date/time demos complete!")
  end
