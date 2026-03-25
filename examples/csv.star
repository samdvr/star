# CSV parsing and encoding example

fn main() =
  do
    # Parse CSV data
    let csv_data = "name,age,city\nAlice,30,Portland\nBob,25,Seattle\n\"Eve, Jr.\",22,\"New York\""
    let rows = csv_parse(csv_data)

    println("Parsed CSV rows:")
    for row in rows do
      println("  #{to_string(row)}")
    end

    println("")

    # Encode back to CSV
    let data = [["product", "price", "quantity"], ["Widget", "9.99", "100"], ["Gadget", "24.50", "50"]]
    let encoded = csv_encode(data)
    println("Encoded CSV:")
    println(encoded)

    # Round-trip: parse then encode
    let roundtrip = csv_encode(csv_parse("a,b,c\n1,2,3\n4,5,6"))
    println("Round-trip:")
    println(roundtrip)
  end
