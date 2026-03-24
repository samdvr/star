# Networking in Star
# TCP, UDP, DNS, URL parsing — all using Rust std::net (no external crates)

# ── URL Parsing ─────────────────────────────────────────

fn demo_url_parse() =
  do
    println("=== URL Parsing ===")

    let result = url_parse("http://example.com:8080/api/users?name=star#top")
    debug(result)

    let simple = url_parse("http://localhost/index.html")
    debug(simple)

    let with_query = url_parse("https://api.example.com/v2/search?q=hello&limit=10")
    debug(with_query)
  end

# ── DNS Lookup ──────────────────────────────────────────

fn demo_dns() =
  do
    println("")
    println("=== DNS Lookup ===")

    # dns_lookup needs "host:port" format
    let addrs = dns_lookup("127.0.0.1:80")
    debug(addrs)
  end

# ── TCP ─────────────────────────────────────────────────

fn demo_tcp() =
  do
    println("")
    println("=== TCP ===")

    # Show API availability (actual sockets may be blocked in sandbox)
    let listener = tcp_listen("127.0.0.1:0")
    println("tcp_listen result: #{is_ok(listener)}")

    let conn = tcp_connect("127.0.0.1:1")
    println("tcp_connect to port 1: #{is_err(conn)}")

    # Full API:
    # tcp_connect(addr) -> Result<TcpStream, String>
    # tcp_listen(addr) -> Result<TcpListener, String>
    # tcp_accept(listener) -> Result<(TcpStream, String), String>
    # tcp_read(stream, max_bytes) -> Result<String, String>
    # tcp_write(stream, data) -> Result<Int, String>
    # tcp_read_line(stream) -> Result<String, String>
    # tcp_write_line(stream, data) -> Result<(), String>
    # tcp_set_timeout(stream, ms) -> Result<(), String>
    # tcp_close(stream) -> Result<(), String>
    println("All TCP operations available")
  end

# ── UDP ─────────────────────────────────────────────────

fn demo_udp() =
  do
    println("")
    println("=== UDP ===")

    let sock = udp_bind("127.0.0.1:0")
    println("udp_bind result: #{is_ok(sock)}")

    # Full API:
    # udp_bind(addr) -> Result<UdpSocket, String>
    # udp_send_to(socket, data, addr) -> Result<Int, String>
    # udp_recv_from(socket, max_bytes) -> Result<(String, String), String>
    println("All UDP operations available")
  end

# ── HTTP ────────────────────────────────────────────────

fn demo_http() =
  do
    println("")
    println("=== HTTP ===")

    # All HTTP functions use raw TCP sockets — http:// only (no TLS)

    # Convenience shorthand for GET
    # http_get(url) -> Result<String, String>
    println("http_get(url): available")

    # Generic HTTP — supports any method
    # http(method, url) -> Result<String, String>
    # http(method, url, body) -> Result<String, String>
    println("http(method, url): available")
    println("http(method, url, body): available")

    # Full control with custom headers
    # http_with_headers(method, url, headers, body) -> Result<String, String>
    println("http_with_headers(method, url, headers, body): available")

    # Examples (commented out — require a running HTTP server):
    # let resp = http_get("http://localhost:8080/")
    # let resp = http("POST", "http://localhost:8080/api", "{\"key\": \"value\"}")
    # let resp = http("DELETE", "http://localhost:8080/api/1")
    # let resp = http_with_headers("POST", "http://localhost:8080/api",
    #   ["Content-Type: application/json", "Authorization: Bearer token"],
    #   "{\"key\": \"value\"}")
  end

# ── Main ────────────────────────────────────────────────

fn main() =
  do
    demo_url_parse()
    demo_dns()
    demo_tcp()
    demo_udp()
    demo_http()
    println("")
    println("All networking demos complete!")
  end
