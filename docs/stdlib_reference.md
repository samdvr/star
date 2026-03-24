# Star Standard Library Reference

Star provides a comprehensive standard library of built-in functions that are recognized at compile time and translated directly into idiomatic Rust. No imports are needed -- all functions listed here are available in every Star program.

Types used in signatures: `Int` (64-bit integer), `Float` (64-bit float), `String`, `Bool`, `List<T>`, `Option<T>`, `Result<T, E>`, `Map<K, V>`, `Set<T>`, `Deque<T>`, `Heap<T>`, `Instant`, `Duration`, `TcpStream`, `TcpListener`, `UdpSocket`, `Sender<T>`, `Receiver<T>`, `Mutex<T>`, `RwLock<T>`, `Atomic`, `JoinHandle<T>`.

---

## I/O

### `println(args...)`
Print values to stdout followed by a newline. Accepts zero or more arguments.

### `print(args...)`
Print values to stdout without a trailing newline. Accepts zero or more arguments.

### `eprintln(args...)`
Print values to stderr followed by a newline. Accepts zero or more arguments.

### `debug(args...)`
Print debug representations of values to stderr. Accepts zero or more arguments.

### `read_line(): String`
Read a line from stdin, trimming the trailing newline.

### `read_all_stdin(): String`
Read all remaining input from stdin as a single string.

---

## File System

### `read_file(path: String): Result<String, String>`
Read the entire contents of a file as a string.

### `write_file(path: String, content: String): Result<(), String>`
Write a string to a file, creating or overwriting it.

### `append_file(path: String, content: String): Result<(), String>`
Append a string to a file, creating it if it does not exist.

### `file_exists(path: String): Bool`
Check whether a file exists at the given path.

### `delete_file(path: String): Result<(), String>`
Delete a file at the given path.

### `rename_file(from: String, to: String): Result<(), String>`
Rename or move a file from one path to another.

### `copy_file(src: String, dst: String): Result<(), String>`
Copy a file from source to destination.

### `file_size(path: String): Result<Int, String>`
Get the size of a file in bytes.

### `read_lines(path: String): Result<List<String>, String>`
Read a file and return its contents as a list of lines.

---

## Directories

### `list_dir(path: String): Result<List<String>, String>`
List the names of entries in a directory.

### `create_dir(path: String): Result<(), String>`
Create a single directory.

### `create_dir_all(path: String): Result<(), String>`
Create a directory and all missing parent directories.

### `delete_dir(path: String): Result<(), String>`
Recursively delete a directory and all its contents.

### `dir_exists(path: String): Bool`
Check whether a directory exists at the given path.

---

## Path Operations

### `path_join(base: String, child: String): String`
Join two path components together.

### `path_parent(path: String): Option<String>`
Return the parent directory of a path.

### `path_filename(path: String): Option<String>`
Return the file name component of a path.

### `path_extension(path: String): Option<String>`
Return the file extension of a path.

### `path_stem(path: String): Option<String>`
Return the file name without its extension.

### `path_is_absolute(path: String): Bool`
Check whether a path is absolute.

### `path_is_relative(path: String): Bool`
Check whether a path is relative.

---

## Environment & Process

### `env_get(key: String): Option<String>`
Get the value of an environment variable.

### `env_set(key: String, value: String)`
Set an environment variable.

### `env_remove(key: String)`
Remove an environment variable.

### `env_vars(): List<(String, String)>`
Return all environment variables as key-value pairs.

### `current_dir(): Result<String, String>`
Get the current working directory.

### `set_current_dir(path: String): Result<(), String>`
Change the current working directory.

### `args(): List<String>`
Get all command-line arguments including the program name.

### `command(cmd: String): Result<Int, String>`
Run a shell command and return its exit code.

### `command_output(cmd: String): Result<(String, String, Int), String>`
Run a shell command and return (stdout, stderr, exit_code).

### `command_with_stdin(cmd: String, stdin: String): Result<(String, String, Int), String>`
Run a shell command with data piped to stdin, returning (stdout, stderr, exit_code).

### `command_with_args(program: String, args: List<String>): Result<Int, String>`
Run a program with explicit arguments and return its exit code.

### `command_with_args_output(program: String, args: List<String>): Result<(String, String, Int), String>`
Run a program with explicit arguments and return (stdout, stderr, exit_code).

### `process_id(): Int`
Get the current process ID.

### `kill_process(pid: Int): Result<(), String>`
Kill a process by PID (Unix only, sends SIGKILL).

### `exit(code: Int)`
Terminate the process with the given exit code.

### `panic(message: String)`
Abort the program with a panic message.

---

## File Metadata

### `is_file(path: String): Bool`
Check whether the path points to a regular file.

### `is_dir(path: String): Bool`
Check whether the path points to a directory.

### `is_symlink(path: String): Bool`
Check whether the path points to a symbolic link.

### `file_modified(path: String): Result<Int, String>`
Get the last modification time as a Unix timestamp in seconds.

### `file_created(path: String): Result<Int, String>`
Get the creation time as a Unix timestamp in seconds.

### `file_readonly(path: String): Result<Bool, String>`
Check whether a file is read-only.

### `set_readonly(path: String, readonly: Bool): Result<(), String>`
Set or clear the read-only permission on a file.

### `symlink(src: String, dst: String): Result<(), String>`
Create a symbolic link (Unix only).

### `read_link(path: String): Result<String, String>`
Read the target of a symbolic link.

### `canonicalize(path: String): Result<String, String>`
Resolve a path to its absolute canonical form.

### `temp_dir(): String`
Get the system temporary directory path.

### `exe_path(): Result<String, String>`
Get the path of the currently running executable.

---

## List Operations

### `map(list: List<T>, f: fn(T) => U): List<U>`
Transform each element of a list by applying a function.

### `filter(list: List<T>, f: fn(T) => Bool): List<T>`
Keep only elements for which the predicate returns true.

### `fold(list: List<T>, init: U, f: fn(U, T) => U): U`
Reduce a list to a single value by applying a function with an accumulator.

### `each(list: List<T>, f: fn(T) => ())`
Execute a function for each element (side effects only).

### `flat_map(list: List<T>, f: fn(T) => List<U>): List<U>`
Map each element to a list and flatten the results.

### `any(list: List<T>, f: fn(T) => Bool): Bool`
Check if any element satisfies the predicate.

### `all(list: List<T>, f: fn(T) => Bool): Bool`
Check if all elements satisfy the predicate.

### `find(list: List<T>, f: fn(T) => Bool): Option<T>`
Return the first element that satisfies the predicate.

### `enumerate(list: List<T>): List<(Int, T)>`
Pair each element with its index.

### `take(list: List<T>, n: Int): List<T>`
Return the first n elements.

### `drop(list: List<T>, n: Int): List<T>`
Skip the first n elements and return the rest.

### `zip(a: List<T>, b: List<U>): List<(T, U)>`
Combine two lists into a list of pairs, truncating to the shorter length.

### `flatten(list: List<List<T>>): List<T>`
Flatten a list of lists into a single list.

### `reverse(list: List<T>): List<T>`
Return a new list with elements in reverse order.

### `sort(list: List<T>): List<T>`
Return a new list sorted in ascending order.

### `sort_by(list: List<T>, cmp: fn(T, T) => Ordering): List<T>`
Return a new list sorted using a custom comparator.

### `head(list: List<T>): Option<T>`
Return the first element, or None if empty.

### `tail(list: List<T>): List<T>`
Return all elements except the first.

### `last(list: List<T>): Option<T>`
Return the last element, or None if empty.

### `init(list: List<T>): List<T>`
Return all elements except the last.

### `push(list: List<T>, elem: T): List<T>`
Return a new list with the element appended.

### `concat(a: List<T>, b: List<T>): List<T>`
Concatenate two lists into a new list.

### `dedup(list: List<T>): List<T>`
Remove consecutive duplicate elements.

### `sum(list: List<Int>): Int`
Sum all integers in the list.

### `product(list: List<Int>): Int`
Multiply all integers in the list.

### `count(list: List<T>): Int`
Return the number of elements in the list.

### `min_by(list: List<T>, f: fn(T) => K): Option<T>`
Return the element with the minimum value of the key function.

### `max_by(list: List<T>, f: fn(T) => K): Option<T>`
Return the element with the maximum value of the key function.

---

## Collection Algorithms

### `binary_search(list: List<T>, value: T): Option<Int>`
Search a sorted list for a value, returning its index if found.

### `position(list: List<T>, f: fn(T) => Bool): Option<Int>`
Return the index of the first element satisfying the predicate.

### `contains_element(list: List<T>, value: T): Bool`
Check whether the list contains the given value.

### `sort_desc(list: List<T>): List<T>`
Return a new list sorted in descending order.

### `sort_by_key(list: List<T>, f: fn(T) => K): List<T>`
Sort a list by the result of a key function.

### `is_sorted(list: List<T>): Bool`
Check whether the list is sorted in ascending order.

### `chunks(list: List<T>, n: Int): List<List<T>>`
Split a list into sublists of at most n elements.

### `windows(list: List<T>, n: Int): List<List<T>>`
Return all contiguous sublists of length n.

### `nth(list: List<T>, n: Int): Option<T>`
Return the element at index n, or None if out of bounds.

### `take_while(list: List<T>, f: fn(T) => Bool): List<T>`
Take elements from the front while the predicate holds.

### `drop_while(list: List<T>, f: fn(T) => Bool): List<T>`
Drop elements from the front while the predicate holds.

### `split_at(list: List<T>, n: Int): (List<T>, List<T>)`
Split a list into two at index n.

### `scan(list: List<T>, init: U, f: fn(U, T) => U): List<U>`
Like fold, but collects all intermediate accumulator values.

### `reduce(list: List<T>, f: fn(T, T) => T): Option<T>`
Fold without an initial value, using the first element as the accumulator.

### `partition(list: List<T>, f: fn(T) => Bool): (List<T>, List<T>)`
Split a list into two: elements that match the predicate and those that do not.

### `group_by(list: List<T>, f: fn(T) => K): List<(K, List<T>)>`
Group elements by the result of a key function.

### `unique(list: List<T>): List<T>`
Remove duplicate elements, preserving the first occurrence.

### `intersperse(list: List<T>, sep: T): List<T>`
Insert a separator between every pair of elements.

### `min_of(list: List<T>): Option<T>`
Return the minimum element of a list.

### `max_of(list: List<T>): Option<T>`
Return the maximum element of a list.

### `sum_float(list: List<Float>): Float`
Sum all floats in the list.

### `product_float(list: List<Float>): Float`
Multiply all floats in the list.

### `unzip(list: List<(A, B)>): (List<A>, List<B>)`
Split a list of pairs into two separate lists.

### `zip_with(a: List<T>, b: List<U>, f: fn(T, U) => V): List<V>`
Combine two lists element-wise using a function.

---

## String Operations

### `to_string(value: T): String`
Convert any value to its string representation.

### `trim(s: String): String`
Remove leading and trailing whitespace.

### `trim_start(s: String): String`
Remove leading whitespace.

### `trim_end(s: String): String`
Remove trailing whitespace.

### `split(s: String, delimiter: String): List<String>`
Split a string by a delimiter.

### `join(parts: List<String>, separator: String): String`
Join a list of strings with a separator.

### `contains(s: String, substring: String): Bool`
Check whether a string contains a substring.

### `replace(s: String, from: String, to: String): String`
Replace all occurrences of a substring.

### `replace_first(s: String, from: String, to: String): String`
Replace only the first occurrence of a substring.

### `uppercase(s: String): String`
Convert a string to uppercase.

### `lowercase(s: String): String`
Convert a string to lowercase.

### `capitalize(s: String): String`
Capitalize the first character of a string.

### `starts_with(s: String, prefix: String): Bool`
Check whether a string starts with the given prefix.

### `ends_with(s: String, suffix: String): Bool`
Check whether a string ends with the given suffix.

### `chars(s: String): List<String>`
Split a string into a list of single-character strings.

### `char_at(s: String, index: Int): Option<Char>`
Return the character at the given index.

### `substring(s: String, start: Int): String`
Return the substring from start to the end.

### `substring(s: String, start: Int, end: Int): String`
Return the substring from start (inclusive) to end (exclusive).

### `index_of(s: String, substring: String): Option<Int>`
Return the byte index of the first occurrence of a substring.

### `last_index_of(s: String, substring: String): Option<Int>`
Return the byte index of the last occurrence of a substring.

### `pad_left(s: String, width: Int): String`
Pad a string on the left with spaces to the given width.

### `pad_left(s: String, width: Int, fill: String): String`
Pad a string on the left with a fill character to the given width.

### `pad_right(s: String, width: Int): String`
Pad a string on the right with spaces to the given width.

### `pad_right(s: String, width: Int, fill: String): String`
Pad a string on the right with a fill character to the given width.

### `repeat(s: String, n: Int): String`
Repeat a string n times.

### `is_empty(s: String): Bool`
Check whether a string is empty.

### `is_blank(s: String): Bool`
Check whether a string is empty or contains only whitespace.

### `reverse_string(s: String): String`
Reverse the characters in a string.

### `lines(s: String): List<String>`
Split a string into lines.

### `words(s: String): List<String>`
Split a string into whitespace-separated words.

### `strip_prefix(s: String, prefix: String): Option<String>`
Remove a prefix from a string if present.

### `strip_suffix(s: String, suffix: String): Option<String>`
Remove a suffix from a string if present.

### `is_numeric(s: String): Bool`
Check whether all characters are ASCII digits.

### `is_alphabetic(s: String): Bool`
Check whether all characters are alphabetic.

### `is_alphanumeric(s: String): Bool`
Check whether all characters are alphanumeric.

---

## Regex & Encoding

### `regex_match(s: String, pattern: String): Bool`
Check whether a string matches a regular expression. Requires the `regex` crate (auto-detected).

### `regex_find(s: String, pattern: String): Option<String>`
Find the first match of a regex in a string.

### `regex_find_all(s: String, pattern: String): List<String>`
Find all matches of a regex in a string.

### `regex_replace(s: String, pattern: String, replacement: String): String`
Replace all regex matches in a string.

### `bytes(s: String): List<Int>`
Return the UTF-8 bytes of a string as a list of integers.

### `from_bytes(b: List<Int>): String`
Convert a list of byte values back into a UTF-8 string.

### `encode_base64(s: String): String`
Encode a string as Base64. Requires the `base64` crate (auto-detected).

### `decode_base64(s: String): Option<String>`
Decode a Base64 string, returning None on failure.

### `char_code(s: String): Int`
Return the Unicode code point of the first character.

### `from_char_code(code: Int): String`
Convert a Unicode code point to a single-character string.

### `format(fmt: String, args...): String`
Format a string using Rust-style format placeholders (`{}`, `{:?}`, etc.).

---

## Cryptography

### `sha256(data: String): String`
Compute the SHA-256 hash of a string, returned as a hex digest.

### `sha512(data: String): String`
Compute the SHA-512 hash of a string, returned as a hex digest.

### `md5(data: String): String`
Compute the MD5 hash of a string, returned as a hex digest.

### `hash_bytes(data: String): String`
Compute a fast non-cryptographic hash, returned as a 16-character hex string.

### `secure_random_bytes(n: Int): List<Int>`
Generate n cryptographically secure random bytes from /dev/urandom.

### `secure_random_hex(n: Int): String`
Generate n random bytes and return them as a hex string.

### `uuid_v4(): String`
Generate a random UUID v4 string.

---

## Math

### `abs(x: Int): Int`
Return the absolute value.

### `min(a: Int, b: Int): Int`
Return the smaller of two values.

### `max(a: Int, b: Int): Int`
Return the larger of two values.

### `pow(base: Float, exp: Float): Float`
Raise a number to a power.

### `sqrt(x: Float): Float`
Compute the square root.

### `clamp(x: T, min: T, max: T): T`
Restrict a value to a given range.

### `sin(x: Float): Float`
Compute the sine (argument in radians).

### `cos(x: Float): Float`
Compute the cosine (argument in radians).

### `tan(x: Float): Float`
Compute the tangent (argument in radians).

### `asin(x: Float): Float`
Compute the arc sine.

### `acos(x: Float): Float`
Compute the arc cosine.

### `atan(x: Float): Float`
Compute the arc tangent.

### `atan2(y: Float, x: Float): Float`
Compute the two-argument arc tangent.

### `floor(x: Float): Float`
Round down to the nearest integer.

### `ceil(x: Float): Float`
Round up to the nearest integer.

### `round(x: Float): Float`
Round to the nearest integer.

### `truncate(x: Float): Float`
Remove the fractional part.

### `log(x: Float): Float`
Compute the natural logarithm (base e).

### `log2(x: Float): Float`
Compute the base-2 logarithm.

### `log10(x: Float): Float`
Compute the base-10 logarithm.

### `exp(x: Float): Float`
Compute e raised to the given power.

### `exp2(x: Float): Float`
Compute 2 raised to the given power.

### `signum(x: Float): Float`
Return the sign of a number (-1.0, 0.0, or 1.0).

### `hypot(x: Float, y: Float): Float`
Compute the length of the hypotenuse (sqrt(x^2 + y^2)) without overflow.

### `cbrt(x: Float): Float`
Compute the cube root.

### `pi(): Float`
Return the constant pi (3.14159...).

### `e_const(): Float`
Return Euler's number e (2.71828...).

### `infinity(): Float`
Return positive infinity.

### `neg_infinity(): Float`
Return negative infinity.

### `nan(): Float`
Return NaN (not a number).

### `is_nan(x: Float): Bool`
Check whether a value is NaN.

### `is_infinite(x: Float): Bool`
Check whether a value is positive or negative infinity.

### `is_finite(x: Float): Bool`
Check whether a value is finite (not NaN or infinity).

### `to_radians(degrees: Float): Float`
Convert degrees to radians.

### `to_degrees(radians: Float): Float`
Convert radians to degrees.

### `random(): Int`
Generate a pseudo-random integer.

### `random_range(min: Int, max: Int): Int`
Generate a pseudo-random integer in [min, max).

### `random_float(): Float`
Generate a pseudo-random float in [0.0, 1.0).

### `gcd(a: Int, b: Int): Int`
Compute the greatest common divisor of two integers.

### `lcm(a: Int, b: Int): Int`
Compute the least common multiple of two integers.

---

## Ranges

### `range(start: Int, end: Int): List<Int>`
Generate a list of integers from start (inclusive) to end (exclusive).

### `range_inclusive(start: Int, end: Int): List<Int>`
Generate a list of integers from start (inclusive) to end (inclusive).

---

## Date & Time

### `now(): Int`
Return the current Unix timestamp in seconds.

### `now_ms(): Int`
Return the current Unix timestamp in milliseconds.

### `now_ns(): Int`
Return the current Unix timestamp in nanoseconds.

### `monotonic(): Instant`
Return a monotonic clock instant for measuring durations.

### `elapsed(start: Instant): Float`
Return the seconds elapsed since an instant.

### `elapsed_ms(start: Instant): Int`
Return the milliseconds elapsed since an instant.

### `monotonic_elapsed_ms(start: Instant, end: Instant): Int`
Return the milliseconds between two instants.

### `timestamp_secs(secs: Int): SystemTime`
Create a SystemTime from a Unix timestamp in seconds.

### `timestamp_millis(ms: Int): SystemTime`
Create a SystemTime from a Unix timestamp in milliseconds.

### `format_timestamp(unix_secs: Int): String`
Format a Unix timestamp as an ISO 8601 UTC string (e.g. "2024-01-15T12:30:00Z").

### `parse_timestamp(iso: String): Result<Int, String>`
Parse an ISO 8601 date-time string into a Unix timestamp in seconds.

### `duration_secs(secs: Int): Duration`
Create a Duration from a number of seconds.

### `duration_ms(ms: Int): Duration`
Create a Duration from a number of milliseconds.

### `sleep_secs(secs: Int)`
Block the current thread for the given number of seconds.

### `sleep_millis(ms: Int)`
Block the current thread for the given number of milliseconds.

---

## Networking

### `tcp_connect(addr: String): Result<TcpStream, String>`
Open a TCP connection to the given address (e.g. "127.0.0.1:8080").

### `tcp_listen(addr: String): Result<TcpListener, String>`
Bind a TCP listener to the given address.

### `tcp_accept(listener: TcpListener): Result<(TcpStream, String), String>`
Accept an incoming TCP connection, returning the stream and peer address.

### `tcp_read(stream: TcpStream, max_bytes: Int): Result<String, String>`
Read up to max_bytes from a TCP stream as a UTF-8 string.

### `tcp_write(stream: TcpStream, data: String): Result<Int, String>`
Write a string to a TCP stream, returning the number of bytes written.

### `tcp_close(stream: TcpStream): Result<(), String>`
Shut down a TCP connection.

### `tcp_read_line(stream: TcpStream): Result<String, String>`
Read a single line from a TCP stream.

### `tcp_write_line(stream: TcpStream, data: String): Result<(), String>`
Write a string followed by a newline to a TCP stream.

### `tcp_set_timeout(stream: TcpStream, ms: Int): Result<(), String>`
Set the read and write timeout on a TCP stream in milliseconds.

### `udp_bind(addr: String): Result<UdpSocket, String>`
Bind a UDP socket to the given address.

### `udp_send_to(socket: UdpSocket, data: String, addr: String): Result<Int, String>`
Send data to a specific address over UDP.

### `udp_recv_from(socket: UdpSocket, max_bytes: Int): Result<(String, String), String>`
Receive data from a UDP socket, returning (data, sender_address).

### `dns_lookup(host: String): Result<List<String>, String>`
Resolve a hostname to a list of IP addresses.

### `url_parse(url: String): Result<Map<String, String>, String>`
Parse a URL into its components (scheme, host, port, path, query, fragment).

### `http_get(url: String): Result<String, String>`
Perform an HTTP/HTTPS GET request and return the response body.

### `http(method: String, url: String): Result<String, String>`
Perform an HTTP/HTTPS request with the given method and no body.

### `http(method: String, url: String, body: String): Result<String, String>`
Perform an HTTP/HTTPS request with the given method and body.

### `http_with_headers(method: String, url: String, headers: List<String>, body: String): Result<String, String>`
Perform an HTTP/HTTPS request with custom headers (e.g. ["Content-Type: application/json"]).

---

## Type Conversions

### `to_int(s: String): Int`
Parse a string as an integer, returning 0 on failure.

### `to_float(s: String): Float`
Parse a string as a float, returning 0.0 on failure.

### `length(collection: T): Int`
Return the length of a string, list, or other collection.

---

## Testing & Debugging

### `assert(condition: Bool)`
Panic if the condition is false.

### `assert_msg(condition: Bool, message: String)`
Panic with a custom message if the condition is false.

### `assert_eq(a: T, b: T)`
Panic if the two values are not equal.

### `assert_ne(a: T, b: T)`
Panic if the two values are equal.

### `log_debug(value: T)`
Print a value to stderr with a [DEBUG] prefix.

### `log_info(value: T)`
Print a value to stderr with an [INFO] prefix.

### `log_warn(value: T)`
Print a value to stderr with a [WARN] prefix.

### `log_error(value: T)`
Print a value to stderr with an [ERROR] prefix.

### `time_fn(f: fn() => T): (T, Int)`
Execute a function and return (result, elapsed_milliseconds).

### `bench(n: Int, f: fn() => T): Float`
Run a function n times and return the average milliseconds per iteration.

### `dbg(x: T): T`
Print a value to stderr with a [dbg] prefix and return it.

### `type_name_of(x: T): String`
Return the Rust type name of a value.

### `todo()`
Panic with a "not yet implemented" message.

### `todo_msg(message: String)`
Panic with a custom "not yet implemented" message.

### `unreachable_msg(message: String)`
Panic indicating unreachable code was reached.

---

## CLI & Arguments

### `arg_get(n: Int): Option<String>`
Get the nth command-line argument (0 = first after program name).

### `arg_count(): Int`
Return the number of arguments (excluding the program name).

### `arg_has(flag: String): Bool`
Check whether a flag (e.g. "--verbose") is present in the arguments.

### `arg_value(flag: String): Option<String>`
Get the value following a flag (supports `--key value` and `--key=value`).

### `arg_pairs(): List<(String, String)>`
Parse all `--key=value` and `--key value` pairs from arguments.

---

## JSON

### `json_get(json: String, key: String): Option<String>`
Extract the value for a key from a JSON object string.

### `json_object(pairs: List<(String, String)>): String`
Build a JSON object string from key-value pairs.

### `json_array(items: List<String>): String`
Build a JSON array string from a list of string values.

### `json_escape(s: String): String`
Escape a string for safe use inside JSON.

### `json_parse(s: String): Result<String, String>`
Parse a JSON string and return a normalized string representation.

### `json_encode(value: T): String`
Encode a value as a JSON string using its debug representation.

---

## Environment Files

### `parse_env_string(content: String): List<(String, String)>`
Parse a .env-formatted string into key-value pairs.

### `load_env_file(path: String): Result<List<(String, String)>, String>`
Load and parse a .env file into key-value pairs.

---

## Colors & Terminal

### `color_red(s: String): String`
Wrap a string in ANSI red color codes.

### `color_green(s: String): String`
Wrap a string in ANSI green color codes.

### `color_blue(s: String): String`
Wrap a string in ANSI blue color codes.

### `color_yellow(s: String): String`
Wrap a string in ANSI yellow color codes.

### `color_cyan(s: String): String`
Wrap a string in ANSI cyan color codes.

### `color_magenta(s: String): String`
Wrap a string in ANSI magenta color codes.

### `bold(s: String): String`
Wrap a string in ANSI bold codes.

### `dim(s: String): String`
Wrap a string in ANSI dim codes.

### `underline(s: String): String`
Wrap a string in ANSI underline codes.

### `strip_ansi(s: String): String`
Remove all ANSI escape sequences from a string.

### `prompt(message: String): String`
Print a message and read a line of input from the user.

### `confirm(message: String): Bool`
Print a message with [y/n] and return true if the user answers yes.

### `clear_screen()`
Clear the terminal screen.

### `cursor_up(n: Int)`
Move the cursor up by n lines.

### `cursor_down(n: Int)`
Move the cursor down by n lines.

---

## Result & Option

### `unwrap(x: Result<T, E> | Option<T>): T`
Extract the value, panicking if it is Err or None.

### `unwrap_or(x: Result<T, E> | Option<T>, default: T): T`
Extract the value, returning a default if it is Err or None.

### `unwrap_or_else(x: Result<T, E>, f: fn(E) => T): T`
Extract the value, computing a default from the error if it is Err.

### `expect(x: Result<T, E> | Option<T>, message: String): T`
Extract the value, panicking with a custom message if it is Err or None.

### `unwrap_err(x: Result<T, E>): E`
Extract the error value, panicking if it is Ok.

### `map_result(x: Result<T, E>, f: fn(T) => U): Result<U, E>`
Transform the Ok value of a Result.

### `map_option(x: Option<T>, f: fn(T) => U): Option<U>`
Transform the Some value of an Option.

### `map_err(x: Result<T, E>, f: fn(E) => F): Result<T, F>`
Transform the Err value of a Result.

### `and_then(x: Result<T, E> | Option<T>, f: fn(T) => Result<U, E> | Option<U>): Result<U, E> | Option<U>`
Chain a computation that may fail (monadic bind).

### `or_else(x: Result<T, E>, f: fn(E) => Result<T, E>): Result<T, E>`
Try an alternative computation if the first one failed.

### `map_or(x: Result<T, E> | Option<T>, default: U, f: fn(T) => U): U`
Apply a function to the inner value or return a default.

### `or_default(x: Result<T, E> | Option<T>): T`
Extract the value or return the type's default value.

### `ok(x: Result<T, E>): Option<T>`
Convert a Result to an Option, discarding the error.

### `err(x: Result<T, E>): Option<E>`
Convert a Result to an Option of its error.

### `is_ok(x: Result<T, E>): Bool`
Check whether a Result is Ok.

### `is_err(x: Result<T, E>): Bool`
Check whether a Result is Err.

### `is_some(x: Option<T>): Bool`
Check whether an Option is Some.

### `is_none(x: Option<T>): Bool`
Check whether an Option is None.

### `some(value: T): Option<T>`
Wrap a value in Some.

### `none(): Option<T>`
Return None.

### `ok_or(x: Option<T>, error: E): Result<T, E>`
Convert an Option to a Result, using the given error for None.

### `ok_or_else(x: Option<T>, f: fn() => E): Result<T, E>`
Convert an Option to a Result, lazily computing the error for None.

### `flatten_result(x: Result<Result<T, E>, E>): Result<T, E>`
Flatten a nested Result.

### `flatten_option(x: Option<Option<T>>): Option<T>`
Flatten a nested Option.

### `transpose(x: Option<Result<T, E>>): Result<Option<T>, E>`
Transpose an Option of a Result into a Result of an Option.

---

## HashMap

### `map_new(): Map<K, V>`
Create a new empty HashMap.

### `map_from_list(pairs: List<(K, V)>): Map<K, V>`
Create a HashMap from a list of key-value pairs.

### `map_insert(m: Map<K, V>, key: K, value: V): Map<K, V>`
Return a new map with the key-value pair inserted.

### `map_remove(m: Map<K, V>, key: K): Map<K, V>`
Return a new map with the key removed.

### `map_get(m: Map<K, V>, key: K): Option<V>`
Look up a value by key.

### `map_contains_key(m: Map<K, V>, key: K): Bool`
Check whether the map contains a key.

### `map_keys(m: Map<K, V>): List<K>`
Return all keys as a list.

### `map_values(m: Map<K, V>): List<V>`
Return all values as a list.

### `map_entries(m: Map<K, V>): List<(K, V)>`
Return all key-value pairs as a list.

### `map_size(m: Map<K, V>): Int`
Return the number of entries in the map.

### `map_merge(a: Map<K, V>, b: Map<K, V>): Map<K, V>`
Merge two maps; entries from the second override those in the first.

---

## HashSet

### `set_new(): Set<T>`
Create a new empty HashSet.

### `set_from_list(items: List<T>): Set<T>`
Create a HashSet from a list of values.

### `set_insert(s: Set<T>, value: T): Set<T>`
Return a new set with the value added.

### `set_remove(s: Set<T>, value: T): Set<T>`
Return a new set with the value removed.

### `set_contains(s: Set<T>, value: T): Bool`
Check whether the set contains a value.

### `set_union(a: Set<T>, b: Set<T>): Set<T>`
Return the union of two sets.

### `set_intersection(a: Set<T>, b: Set<T>): Set<T>`
Return the intersection of two sets.

### `set_difference(a: Set<T>, b: Set<T>): Set<T>`
Return elements in the first set that are not in the second.

### `set_size(s: Set<T>): Int`
Return the number of elements in the set.

### `set_to_list(s: Set<T>): List<T>`
Convert a set to a list.

---

## Deque

### `deque_new(): Deque<T>`
Create a new empty double-ended queue.

### `deque_from_list(items: List<T>): Deque<T>`
Create a deque from a list of values.

### `deque_push_front(d: Deque<T>, value: T): Deque<T>`
Return a new deque with the value added to the front.

### `deque_push_back(d: Deque<T>, value: T): Deque<T>`
Return a new deque with the value added to the back.

### `deque_pop_front(d: Deque<T>): (Option<T>, Deque<T>)`
Remove and return the front element along with the updated deque.

### `deque_pop_back(d: Deque<T>): (Option<T>, Deque<T>)`
Remove and return the back element along with the updated deque.

### `deque_size(d: Deque<T>): Int`
Return the number of elements in the deque.

### `deque_to_list(d: Deque<T>): List<T>`
Convert a deque to a list.

---

## Heap

### `heap_new(): Heap<T>`
Create a new empty max-heap.

### `heap_from_list(items: List<T>): Heap<T>`
Create a max-heap from a list of values.

### `heap_push(h: Heap<T>, value: T): Heap<T>`
Return a new heap with the value inserted.

### `heap_pop(h: Heap<T>): (Option<T>, Heap<T>)`
Remove and return the maximum element along with the updated heap.

### `heap_peek(h: Heap<T>): Option<T>`
Return the maximum element without removing it.

### `heap_size(h: Heap<T>): Int`
Return the number of elements in the heap.

### `heap_to_list(h: Heap<T>): List<T>`
Convert a heap to a sorted list.

---

## Concurrency

### `spawn(f: fn() => T): JoinHandle<T>`
Run a function in a new OS thread and return a handle.

### `spawn_join(f: fn() => T): Result<T, String>`
Spawn a thread and immediately join it, returning the result.

### `channel(): (Sender<T>, Receiver<T>)`
Create an unbounded multi-producer, single-consumer channel.

### `send(sender: Sender<T>, value: T): Result<(), String>`
Send a value through a channel.

### `recv(receiver: Receiver<T>): Result<T, String>`
Receive a value from a channel, blocking until one is available.

### `try_recv(receiver: Receiver<T>): Option<T>`
Try to receive a value from a channel without blocking.

### `mutex_new(value: T): Mutex<T>`
Create a new mutex wrapping a value (shared via Arc).

### `mutex_lock(m: Mutex<T>): T`
Lock the mutex and return a clone of the inner value.

### `rwlock_new(value: T): RwLock<T>`
Create a new read-write lock wrapping a value (shared via Arc).

### `rwlock_read(rw: RwLock<T>): T`
Acquire a read lock and return a clone of the inner value.

### `rwlock_write(rw: RwLock<T>): T`
Acquire a write lock and return a clone of the inner value.

### `atomic_new(value: Int): Atomic`
Create a new atomic integer (shared via Arc).

### `atomic_get(a: Atomic): Int`
Read the current value of an atomic integer.

### `atomic_set(a: Atomic, value: Int)`
Set the value of an atomic integer.

### `atomic_add(a: Atomic, delta: Int): Int`
Atomically add to an atomic integer and return the previous value.

### `sleep(seconds: Int)`
Async sleep for the given number of seconds (requires tokio).

### `sleep_ms(ms: Int)`
Async sleep for the given number of milliseconds (requires tokio).

### `timeout(seconds: Int, future: Future<T>): Result<T, String>`
Run an async operation with a timeout (requires tokio).

### `spawn_async(f: fn() => Future<T>): JoinHandle<T>`
Spawn an async task on the tokio runtime.

### `spawn_blocking(f: fn() => T): JoinHandle<T>`
Run a blocking function on the tokio blocking thread pool.

### `parallel_map(list: List<T>, f: fn(T) => U): List<U>`
Map a function over a list in parallel using OS threads.
