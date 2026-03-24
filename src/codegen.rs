use crate::ast::*;
use std::collections::{HashMap, HashSet};

fn fmt_type_params(tps: &[TypeParam]) -> String {
    if tps.is_empty() {
        String::new()
    } else {
        let parts: Vec<String> = tps.iter().map(|tp| {
            if tp.bounds.is_empty() {
                tp.name.clone()
            } else {
                format!("{}: {}", tp.name, tp.bounds.join(" + "))
            }
        }).collect();
        format!("<{}>", parts.join(", "))
    }
}

pub fn generate(program: &Program, test_mode: bool) -> String {
    let mut cg = CodeGen::new();
    cg.test_mode = test_mode;
    cg.collect_variants(program);
    cg.detect_recursive_types(program);
    cg.emit_program(program);
    cg.output
}

struct CodeGen {
    output: String,
    indent: usize,
    // Maps variant name -> enum name for qualified access
    variant_to_enum: HashMap<String, String>,
    // Set of known builtin function names
    builtins: HashSet<&'static str>,
    // Maps type_name -> (variant_name -> set of field indices that need Box<>)
    recursive_types: HashMap<String, HashMap<String, HashSet<usize>>>,
    // When true, generate a test harness instead of the user's main()
    test_mode: bool,
}

impl CodeGen {
    /// Inline HTTP helper function emitted into generated code blocks.
    /// Supports HTTP and HTTPS (via native-tls), all methods, custom headers, and request bodies.
    const HTTP_HELPER: &str = concat!(
        "fn _star_http(_method: &str, _url: &str, _headers: &[&str], _body: &str) -> Result<String, String> { ",
        "let (_scheme, _rest) = if _url.starts_with(\"https://\") { (\"https\", &_url[8..]) } ",
        "else if _url.starts_with(\"http://\") { (\"http\", &_url[7..]) } ",
        "else { return Err(\"unsupported URL scheme (use http:// or https://)\".into()); }; ",
        "let (_host_port, _path) = _rest.split_once('/').unwrap_or((_rest, \"\")); ",
        "let _path = format!(\"/{}\", _path); ",
        "let _host = _host_port.split(':').next().unwrap_or(_host_port); ",
        "let _default_port: u16 = if _scheme == \"https\" { 443 } else { 80 }; ",
        "let _addr = if _host_port.contains(':') { _host_port.to_string() } else { format!(\"{}:{}\", _host_port, _default_port) }; ",
        "let _tcp = std::net::TcpStream::connect(&_addr).map_err(|e| e.to_string())?; ",
        "let mut _req = format!(\"{} {} HTTP/1.1\\r\\nHost: {}\\r\\nConnection: close\\r\\n\", _method, _path, _host); ",
        "for _h in _headers { _req.push_str(_h); _req.push_str(\"\\r\\n\"); } ",
        "if !_body.is_empty() { _req.push_str(&format!(\"Content-Length: {}\\r\\n\", _body.len())); } ",
        "_req.push_str(\"\\r\\n\"); ",
        "if !_body.is_empty() { _req.push_str(_body); } ",
        "let _resp = if _scheme == \"https\" { ",
        "let _connector = native_tls::TlsConnector::new().map_err(|e| e.to_string())?; ",
        "let mut _tls = _connector.connect(_host, _tcp).map_err(|e| e.to_string())?; ",
        "use std::io::{Write, Read}; ",
        "_tls.write_all(_req.as_bytes()).map_err(|e| e.to_string())?; ",
        "let mut _r = String::new(); _tls.read_to_string(&mut _r).map_err(|e| e.to_string())?; _r ",
        "} else { ",
        "use std::io::{Write, Read}; ",
        "let mut _tcp = _tcp; ",
        "_tcp.write_all(_req.as_bytes()).map_err(|e| e.to_string())?; ",
        "let mut _r = String::new(); _tcp.read_to_string(&mut _r).map_err(|e| e.to_string())?; _r ",
        "}; ",
        "if let Some((_hdr, _body)) = _resp.split_once(\"\\r\\n\\r\\n\") { Ok(_body.to_string()) } else { Ok(_resp) } ",
        "}"
    );

    /// Inline JSON value type and parser/encoder emitted into generated code.
    const JSON_HELPER: &str = concat!(
        "#[derive(Clone, Debug)] ",
        "enum _StarJson { Null, Bool(bool), Num(f64), Str(String), Array(Vec<_StarJson>), Object(Vec<(String, _StarJson)>) } ",
        "impl std::fmt::Display for _StarJson { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match self { ",
        "_StarJson::Null => write!(f, \"null\"), ",
        "_StarJson::Bool(b) => write!(f, \"{}\", b), ",
        "_StarJson::Num(n) => { if *n == (*n as i64) as f64 { write!(f, \"{}\", *n as i64) } else { write!(f, \"{}\", n) } } ",
        "_StarJson::Str(s) => write!(f, \"\\\"{}\\\"\", s.replace('\\\\', \"\\\\\\\\\").replace('\"', \"\\\\\\\"\")), ",
        "_StarJson::Array(arr) => { write!(f, \"[\")?; for (i, v) in arr.iter().enumerate() { if i > 0 { write!(f, \", \")?; } write!(f, \"{}\", v)?; } write!(f, \"]\") } ",
        "_StarJson::Object(pairs) => { write!(f, \"{{\")?; for (i, (k, v)) in pairs.iter().enumerate() { if i > 0 { write!(f, \", \")?; } write!(f, \"\\\"{}\\\": {}\", k.replace('\\\\', \"\\\\\\\\\").replace('\"', \"\\\\\\\"\"), v)?; } write!(f, \"}}\") } ",
        "} } } ",
        "fn _star_json_parse(input: &str) -> Result<String, String> { ",
        "let bytes = input.as_bytes(); let len = bytes.len(); ",
        "fn skip_ws(b: &[u8], mut i: usize) -> usize { while i < b.len() && matches!(b[i], b' ' | b'\\t' | b'\\n' | b'\\r') { i += 1; } i } ",
        "fn parse_value(b: &[u8], i: usize, len: usize) -> Result<(_StarJson, usize), String> { ",
        "let i = skip_ws(b, i); if i >= len { return Err(\"unexpected end of input\".into()); } ",
        "match b[i] { ",
        "b'\"' => parse_string(b, i, len), ",
        "b'{' => parse_object(b, i, len), ",
        "b'[' => parse_array(b, i, len), ",
        "b't' => { if i + 4 <= len && &b[i..i+4] == b\"true\" { Ok((_StarJson::Bool(true), i + 4)) } else { Err(\"invalid token\".into()) } } ",
        "b'f' => { if i + 5 <= len && &b[i..i+5] == b\"false\" { Ok((_StarJson::Bool(false), i + 5)) } else { Err(\"invalid token\".into()) } } ",
        "b'n' => { if i + 4 <= len && &b[i..i+4] == b\"null\" { Ok((_StarJson::Null, i + 4)) } else { Err(\"invalid token\".into()) } } ",
        "b'-' | b'0'..=b'9' => parse_number(b, i, len), ",
        "c => Err(format!(\"unexpected character '{}'\", c as char)) } } ",
        "fn parse_string(b: &[u8], i: usize, len: usize) -> Result<(_StarJson, usize), String> { ",
        "let mut s = String::new(); let mut j = i + 1; ",
        "while j < len { match b[j] { ",
        "b'\"' => return Ok((_StarJson::Str(s), j + 1)), ",
        "b'\\\\' => { j += 1; if j >= len { return Err(\"unterminated string\".into()); } ",
        "match b[j] { b'\"' => s.push('\"'), b'\\\\' => s.push('\\\\'), b'/' => s.push('/'), b'n' => s.push('\\n'), b'r' => s.push('\\r'), b't' => s.push('\\t'), b'b' => s.push('\\x08'), b'f' => s.push('\\x0C'), ",
        "b'u' => { if j + 4 >= len { return Err(\"incomplete unicode escape\".into()); } ",
        "let hex = std::str::from_utf8(&b[j+1..j+5]).map_err(|e| e.to_string())?; ",
        "let cp = u32::from_str_radix(hex, 16).map_err(|e| e.to_string())?; ",
        "if let Some(c) = char::from_u32(cp) { s.push(c); } else { s.push('\\u{FFFD}'); } j += 4; } ",
        "_ => { s.push('\\\\'); s.push(b[j] as char); } } j += 1; } ",
        "c => { s.push(c as char); j += 1; } } } Err(\"unterminated string\".into()) } ",
        "fn parse_number(b: &[u8], i: usize, len: usize) -> Result<(_StarJson, usize), String> { ",
        "let mut j = i; if j < len && b[j] == b'-' { j += 1; } ",
        "while j < len && b[j].is_ascii_digit() { j += 1; } ",
        "if j < len && b[j] == b'.' { j += 1; while j < len && b[j].is_ascii_digit() { j += 1; } } ",
        "if j < len && (b[j] == b'e' || b[j] == b'E') { j += 1; if j < len && (b[j] == b'+' || b[j] == b'-') { j += 1; } while j < len && b[j].is_ascii_digit() { j += 1; } } ",
        "let s = std::str::from_utf8(&b[i..j]).map_err(|e| e.to_string())?; ",
        "let n: f64 = s.parse().map_err(|e: std::num::ParseFloatError| e.to_string())?; ",
        "Ok((_StarJson::Num(n), j)) } ",
        "fn parse_array(b: &[u8], i: usize, len: usize) -> Result<(_StarJson, usize), String> { ",
        "let mut arr = Vec::new(); let mut j = skip_ws(b, i + 1); ",
        "if j < len && b[j] == b']' { return Ok((_StarJson::Array(arr), j + 1)); } ",
        "loop { let (val, next) = parse_value(b, j, len)?; arr.push(val); j = skip_ws(b, next); ",
        "if j >= len { return Err(\"unterminated array\".into()); } ",
        "if b[j] == b']' { return Ok((_StarJson::Array(arr), j + 1)); } ",
        "if b[j] != b',' { return Err(\"expected ',' or ']'\".into()); } j += 1; } } ",
        "fn parse_object(b: &[u8], i: usize, len: usize) -> Result<(_StarJson, usize), String> { ",
        "let mut pairs = Vec::new(); let mut j = skip_ws(b, i + 1); ",
        "if j < len && b[j] == b'}' { return Ok((_StarJson::Object(pairs), j + 1)); } ",
        "loop { j = skip_ws(b, j); if j >= len || b[j] != b'\"' { return Err(\"expected string key\".into()); } ",
        "let (key_val, next) = parse_string(b, j, len)?; ",
        "let key = if let _StarJson::Str(s) = key_val { s } else { unreachable!() }; ",
        "j = skip_ws(b, next); if j >= len || b[j] != b':' { return Err(\"expected ':'\".into()); } j += 1; ",
        "let (val, next) = parse_value(b, j, len)?; pairs.push((key, val)); j = skip_ws(b, next); ",
        "if j >= len { return Err(\"unterminated object\".into()); } ",
        "if b[j] == b'}' { return Ok((_StarJson::Object(pairs), j + 1)); } ",
        "if b[j] != b',' { return Err(\"expected ',' or '}'\".into()); } j += 1; } } ",
        "let (val, end) = parse_value(bytes, 0, len)?; ",
        "let rest = skip_ws(bytes, end); ",
        "if rest < len { return Err(format!(\"trailing data at position {}\", rest)); } ",
        "Ok(format!(\"{}\", val)) } ",
        "fn _star_json_encode<T: std::fmt::Debug + std::any::Any>(val: &T) -> String { ",
        "use std::any::Any; ",
        "let a = val as &dyn Any; ",
        "if let Some(s) = a.downcast_ref::<String>() { ",
        "let mut out = String::from('\"'); ",
        "for c in s.chars() { match c { ",
        "'\\\\' => out.push_str(\"\\\\\\\\\"), ",
        "'\"' => out.push_str(\"\\\\\\\"\"), ",
        "'\\n' => out.push_str(\"\\\\n\"), ",
        "'\\r' => out.push_str(\"\\\\r\"), ",
        "'\\t' => out.push_str(\"\\\\t\"), ",
        "c if (c as u32) < 0x20 => out.push_str(&format!(\"\\\\u{:04x}\", c as u32)), ",
        "c => out.push(c), ",
        "} } out.push('\"'); return out; } ",
        "if let Some(n) = a.downcast_ref::<i64>() { return format!(\"{}\", n); } ",
        "if let Some(n) = a.downcast_ref::<f64>() { ",
        "if n.is_nan() || n.is_infinite() { return \"null\".to_string(); } ",
        "if *n == (*n as i64) as f64 { return format!(\"{}\", *n as i64); } ",
        "return format!(\"{}\", n); } ",
        "if let Some(b) = a.downcast_ref::<bool>() { return format!(\"{}\", b); } ",
        "let dbg = format!(\"{:?}\", val); ",
        "if dbg == \"()\" { return \"null\".to_string(); } ",
        "dbg } ",
        "\n"
    );

    fn new() -> Self {
        let builtins: HashSet<&'static str> = [
            // I/O
            "println", "print", "eprintln", "debug",
            "read_line", "read_all_stdin",
            // File system
            "read_file", "write_file", "append_file",
            "file_exists", "delete_file", "rename_file", "copy_file",
            "file_size", "read_lines",
            // Directories
            "list_dir", "create_dir", "create_dir_all",
            "delete_dir", "dir_exists",
            // Path operations
            "path_join", "path_parent", "path_filename",
            "path_extension", "path_stem",
            "path_is_absolute", "path_is_relative",
            // Environment & process
            "env_get", "env_set", "env_vars", "env_remove",
            "current_dir", "set_current_dir", "args",
            "command", "command_output",
            "command_with_stdin", "command_with_args", "command_with_args_output",
            "process_id", "kill_process",
            // File metadata & permissions
            "is_file", "is_dir", "is_symlink",
            "file_modified", "file_created",
            "file_readonly", "set_readonly",
            "symlink", "read_link", "canonicalize",
            "temp_dir", "exe_path",
            // List operations
            "map", "filter", "fold", "each", "flat_map",
            "any", "all", "find", "enumerate",
            "take", "drop", "zip", "flatten",
            "reverse", "sort", "sort_by",
            "head", "tail", "last", "init",
            "push", "concat", "dedup",
            "sum", "product", "count",
            "min_by", "max_by",
            // Collection algorithms & utilities
            "binary_search", "position", "contains_element",
            "sort_desc", "sort_by_key", "is_sorted",
            "chunks", "windows", "nth", "take_while", "drop_while", "split_at",
            "scan", "reduce", "partition", "group_by", "unique", "intersperse",
            "min_of", "max_of", "sum_float", "product_float",
            "unzip", "zip_with",
            // String operations
            "to_string", "trim", "trim_start", "trim_end",
            "split", "join", "contains", "replace", "replace_first",
            "uppercase", "lowercase", "capitalize",
            "starts_with", "ends_with", "chars",
            "char_at", "substring", "index_of", "last_index_of",
            "pad_left", "pad_right", "repeat",
            "is_empty", "is_blank",
            "reverse_string", "lines", "words",
            "strip_prefix", "strip_suffix",
            "is_numeric", "is_alphabetic", "is_alphanumeric",
            "regex_match", "regex_find", "regex_find_all", "regex_replace",
            "bytes", "from_bytes",
            "encode_base64", "decode_base64",
            "char_code", "from_char_code",
            "format",
            // Cryptography & security
            "sha256", "sha512", "md5", "hash_bytes",
            "secure_random_bytes", "secure_random_hex", "uuid_v4",
            // Constructors / ranges
            "range", "range_inclusive",
            // Math
            "abs", "min", "max", "pow", "sqrt", "clamp",
            "sin", "cos", "tan", "asin", "acos", "atan", "atan2",
            "floor", "ceil", "round", "truncate",
            "log", "log2", "log10", "exp", "exp2",
            "signum", "hypot", "cbrt",
            "pi", "e_const", "infinity", "neg_infinity", "nan",
            "is_nan", "is_infinite", "is_finite",
            "to_radians", "to_degrees",
            "random", "random_range", "random_float",
            "gcd", "lcm",
            // Date & Time
            "now", "now_ms", "now_ns",
            "elapsed", "elapsed_ms",
            "monotonic", "monotonic_elapsed_ms",
            "timestamp_secs", "timestamp_millis",
            "format_timestamp", "parse_timestamp",
            "duration_secs", "duration_ms",
            "sleep_secs", "sleep_millis",
            // Networking — TCP
            "tcp_connect", "tcp_listen", "tcp_accept",
            "tcp_read", "tcp_write", "tcp_close",
            "tcp_read_line", "tcp_write_line",
            "tcp_set_timeout",
            // Networking — UDP
            "udp_bind", "udp_send_to", "udp_recv_from",
            // Networking — DNS & URL
            "dns_lookup", "url_parse",
            // Networking — HTTP (simple, std-only)
            "http_get",
            "http", "http_with_headers",
            // Conversions
            "to_int", "to_float",
            // Utility
            "length",
            // Process
            "exit", "panic",
            // Testing & debugging
            "assert", "assert_eq", "assert_ne", "assert_msg",
            "log_debug", "log_info", "log_warn", "log_error",
            "time_fn", "bench",
            "dbg", "type_name_of",
            "todo", "todo_msg", "unreachable_msg",
            // Configuration & CLI
            "arg_get", "arg_count", "arg_has", "arg_value", "arg_pairs",
            "json_get", "json_object", "json_array", "json_escape",
            "json_parse", "json_encode",
            "parse_env_string", "load_env_file",
            "color_red", "color_green", "color_blue", "color_yellow",
            "color_cyan", "color_magenta",
            "bold", "dim", "underline",
            "strip_ansi", "prompt", "confirm",
            "clear_screen", "cursor_up", "cursor_down",
            // Option/Result helpers
            "unwrap", "unwrap_or", "unwrap_or_else", "or_else", "map_or",
            "is_some", "is_none", "is_ok", "is_err",
            "ok", "err",
            "expect", "unwrap_err",
            "map_result", "map_option", "map_err",
            "and_then", "or_default",
            "flatten_result", "flatten_option",
            "ok_or", "ok_or_else",
            "some", "none",
            "transpose",
            // HashMap operations
            "map_new", "map_from_list", "map_insert", "map_remove",
            "map_get", "map_contains_key", "map_keys", "map_values",
            "map_entries", "map_size", "map_merge",
            // HashSet operations
            "set_new", "set_from_list", "set_insert", "set_remove",
            "set_contains", "set_union", "set_intersection",
            "set_difference", "set_size", "set_to_list",
            // Deque operations
            "deque_new", "deque_from_list",
            "deque_push_back", "deque_push_front",
            "deque_pop_back", "deque_pop_front",
            "deque_size", "deque_to_list",
            // Heap operations
            "heap_new", "heap_from_list",
            "heap_push", "heap_pop", "heap_peek",
            "heap_size", "heap_to_list",
            // Concurrency — threads
            "spawn", "spawn_join",
            // Concurrency — channels
            "channel", "send", "recv", "try_recv",
            // Concurrency — sync primitives
            "mutex_new", "mutex_lock", "rwlock_new", "rwlock_read", "rwlock_write",
            "atomic_new", "atomic_get", "atomic_set", "atomic_add",
            // Concurrency — async/tokio
            "sleep", "sleep_ms", "timeout",
            "spawn_async", "spawn_blocking",
            // Concurrency — parallel helpers
            "parallel_map",
        ]
        .into_iter()
        .collect();

        Self {
            output: String::new(),
            indent: 0,
            variant_to_enum: HashMap::new(),
            builtins,
            recursive_types: HashMap::new(),
            test_mode: false,
        }
    }

    fn collect_variants(&mut self, program: &Program) {
        for item in &program.items {
            match item {
                Item::TypeDecl(td) => self.collect_type_variants(td),
                Item::ModuleDecl(m) => {
                    for item in &m.items {
                        if let Item::TypeDecl(td) = item {
                            self.collect_type_variants(td);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_type_variants(&mut self, td: &TypeDecl) {
        if let TypeBody::Enum(variants) = &td.body {
            for variant in variants {
                self.variant_to_enum
                    .insert(variant.name.clone(), td.name.clone());
            }
        }
    }

    fn detect_recursive_types(&mut self, program: &Program) {
        for item in &program.items {
            if let Item::TypeDecl(td) = item {
                self.check_type_recursion(td);
            }
            if let Item::ModuleDecl(m) = item {
                for item in &m.items {
                    if let Item::TypeDecl(td) = item {
                        self.check_type_recursion(td);
                    }
                }
            }
        }
    }

    fn check_type_recursion(&mut self, td: &TypeDecl) {
        if let TypeBody::Enum(variants) = &td.body {
            let mut variant_map: HashMap<String, HashSet<usize>> = HashMap::new();
            for variant in variants {
                let mut boxed_fields = HashSet::new();
                for (fi, field) in variant.fields.iter().enumerate() {
                    if self.type_refers_to(field, &td.name) {
                        boxed_fields.insert(fi);
                    }
                }
                if !boxed_fields.is_empty() {
                    variant_map.insert(variant.name.clone(), boxed_fields);
                }
            }
            if !variant_map.is_empty() {
                self.recursive_types.insert(td.name.clone(), variant_map);
            }
        }
    }

    fn type_refers_to(&self, ty: &TypeExpr, name: &str) -> bool {
        match ty {
            TypeExpr::Named(n, args) => {
                if n == name {
                    return true;
                }
                args.iter().any(|a| self.type_refers_to(a, name))
            }
            TypeExpr::Function(params, ret) => {
                params.iter().any(|p| self.type_refers_to(p, name))
                    || self.type_refers_to(ret, name)
            }
            TypeExpr::Tuple(types) => types.iter().any(|t| self.type_refers_to(t, name)),
            TypeExpr::Ref(inner) | TypeExpr::MutRef(inner) | TypeExpr::Move(inner) => {
                self.type_refers_to(inner, name)
            }
            TypeExpr::Dyn(_) | TypeExpr::Lifetime(_) => false,
        }
    }

    fn is_boxed_field(&self, type_name: &str, variant_name: &str, field_idx: usize) -> bool {
        self.recursive_types
            .get(type_name)
            .and_then(|vm| vm.get(variant_name))
            .map_or(false, |fields| fields.contains(&field_idx))
    }

    fn qualify_variant(&self, name: &str) -> String {
        if let Some(enum_name) = self.variant_to_enum.get(name) {
            format!("{enum_name}::{name}")
        } else {
            name.to_string()
        }
    }

    fn is_builtin(&self, name: &str) -> bool {
        self.builtins.contains(name)
    }

    fn line(&mut self, s: &str) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn push(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn indent(&mut self) {
        self.indent += 1;
    }

    fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    fn emit_prelude(&mut self) {
        // Helper function: uses Any to display strings cleanly, Debug for everything else
        self.line("fn star_display<T: std::fmt::Debug + std::any::Any>(val: &T) -> String {");
        self.indent();
        self.line("if let Some(s) = (val as &dyn std::any::Any).downcast_ref::<String>() {");
        self.indent();
        self.line("s.clone()");
        self.dedent();
        self.line("} else {");
        self.indent();
        self.line("format!(\"{:?}\", val)");
        self.dedent();
        self.line("}");
        self.dedent();
        self.line("}");
        self.output.push('\n');
    }

    fn emit_indent(&mut self) {
        for _ in 0..self.indent {
            self.push("    ");
        }
    }

    // ── Program ─────────────────────────────────────────────────────────────

    fn emit_program(&mut self, program: &Program) {
        // Emit prelude for Display helpers
        self.emit_prelude();

        // Collect items by kind for ordering
        let mut types = Vec::new();
        let mut functions = Vec::new();
        let mut modules = Vec::new();
        let mut uses = Vec::new();
        let mut top_exprs = Vec::new();
        let mut extern_fns = Vec::new();
        let mut traits = Vec::new();
        let mut impls = Vec::new();
        let mut consts = Vec::new();

        for item in &program.items {
            match item {
                Item::TypeDecl(td) => types.push(td),
                Item::Function(f) => functions.push(f),
                Item::ModuleDecl(m) => modules.push(m),
                Item::UseDecl(u) => uses.push(u),
                Item::Expr(e) => top_exprs.push(e),
                Item::ExternFn(ef) => extern_fns.push(ef),
                Item::TraitDecl(t) => traits.push(t),
                Item::ImplBlock(imp) => impls.push(imp),
                Item::Const(c) => consts.push(c),
            }
        }

        // Emit use declarations
        for u in &uses {
            self.emit_use(u);
        }
        if !uses.is_empty() {
            self.output.push('\n');
        }

        // Emit type declarations
        for td in &types {
            self.emit_type_decl(td);
            self.output.push('\n');
        }

        // Emit modules
        for m in &modules {
            self.emit_module(m);
            self.output.push('\n');
        }

        // Emit constants
        for c in &consts {
            self.emit_const(c);
            self.output.push('\n');
        }

        // Emit traits
        for t in &traits {
            self.emit_trait_decl(t);
            self.output.push('\n');
        }

        // Emit impl blocks
        for imp in &impls {
            self.emit_impl_block(imp);
            self.output.push('\n');
        }

        // Emit extern fn wrappers
        for ef in &extern_fns {
            self.emit_extern_fn(ef);
            self.output.push('\n');
        }

        // Emit functions
        let has_main = functions.iter().any(|f| f.name == "main");

        // Collect test function names for test mode
        let test_fn_names: Vec<String> = if self.test_mode {
            functions.iter()
                .filter(|f| f.name.starts_with("test_"))
                .map(|f| f.name.clone())
                .collect()
        } else {
            Vec::new()
        };

        for f in &functions {
            if f.name == "main" && self.test_mode {
                // Skip user's main() in test mode
                continue;
            } else if f.name == "main" {
                self.emit_main_function(f);
            } else {
                self.emit_function(f);
            }
            self.output.push('\n');
        }

        // In test mode, generate a test harness main()
        if self.test_mode {
            self.line("fn main() {");
            self.indent();
            self.line("let _filter = std::env::var(\"STAR_TEST_FILTER\").ok();");
            self.line("let _verbose = std::env::var(\"STAR_TEST_VERBOSE\").is_ok();");
            self.line("let mut _passed = 0u32;");
            self.line("let mut _failed = 0u32;");
            self.line("let mut _skipped = 0u32;");
            self.line("let _all_tests: Vec<(&str, fn())> = vec![");
            self.indent();
            for name in &test_fn_names {
                self.line(&format!("(\"{}\", {} as fn()),", name, name));
            }
            self.dedent();
            self.line("];");
            self.line("let _tests: Vec<&(&str, fn())> = _all_tests.iter().filter(|(name, _)| {");
            self.indent();
            self.line("match &_filter { Some(f) => name.contains(f.as_str()), None => true }");
            self.dedent();
            self.line("}).collect();");
            self.line("_skipped = (_all_tests.len() - _tests.len()) as u32;");
            self.line("println!(\"running {} test{}...\", _tests.len(), if _tests.len() == 1 { \"\" } else { \"s\" });");
            self.line("let _suite_start = std::time::Instant::now();");
            self.line("for (_name, _test_fn) in &_tests {");
            self.indent();
            self.line("if _verbose { println!(\"  running {}...\", _name); }");
            self.line("let _start = std::time::Instant::now();");
            self.line("match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| _test_fn())) {");
            self.indent();
            self.line("Ok(_) => { let _ms = _start.elapsed().as_millis(); println!(\"  PASS: {} ({}ms)\", _name, _ms); _passed += 1; }");
            self.line("Err(_) => { let _ms = _start.elapsed().as_millis(); println!(\"  FAIL: {} ({}ms)\", _name, _ms); _failed += 1; }");
            self.dedent();
            self.line("}");
            self.dedent();
            self.line("}");
            self.line("let _total_ms = _suite_start.elapsed().as_millis();");
            self.line("println!();");
            self.line("if _skipped > 0 {");
            self.indent();
            self.line("println!(\"{} passed, {} failed, {} skipped ({}ms)\", _passed, _failed, _skipped, _total_ms);");
            self.dedent();
            self.line("} else {");
            self.indent();
            self.line("println!(\"{} passed, {} failed ({}ms)\", _passed, _failed, _total_ms);");
            self.dedent();
            self.line("}");
            self.line("if _failed > 0 { std::process::exit(1); }");
            self.dedent();
            self.line("}");
        }

        // If there are top-level expressions but no main, generate one (not in test mode)
        if !self.test_mode && !top_exprs.is_empty() && !has_main {
            self.line("fn main() {");
            self.indent();
            for expr in &top_exprs {
                self.emit_indent();
                self.emit_expr(expr);
                self.push(";\n");
            }
            self.dedent();
            self.line("}");
        }
    }

    // ── Use ─────────────────────────────────────────────────────────────────

    fn emit_use(&mut self, u: &UseDecl) {
        let path = u
            .path
            .iter()
            .map(|s| to_snake_case(s))
            .collect::<Vec<_>>()
            .join("::");

        match &u.imports {
            Some(imports) => {
                let names = imports.join(", ");
                self.line(&format!("use {path}::{{{names}}};"));
            }
            None => {
                self.line(&format!("use {path}::*;"));
            }
        }
    }

    // ── Types ───────────────────────────────────────────────────────────────

    fn emit_type_decl(&mut self, td: &TypeDecl) {
        let type_params = fmt_type_params(&td.type_params);

        match &td.body {
            TypeBody::Enum(variants) => {
                self.line("#[derive(Debug, Clone, PartialEq)]");
                self.line(&format!("enum {}{} {{", td.name, type_params));
                self.indent();
                for variant in variants {
                    if variant.fields.is_empty() {
                        self.line(&format!("{},", variant.name));
                    } else {
                        let fields: Vec<String> = variant
                            .fields
                            .iter()
                            .enumerate()
                            .map(|(fi, f)| {
                                let ty = self.type_to_rust(f, &td.name);
                                if self.is_boxed_field(&td.name, &variant.name, fi) {
                                    format!("Box<{ty}>")
                                } else {
                                    ty
                                }
                            })
                            .collect();
                        self.line(&format!("{}({}),", variant.name, fields.join(", ")));
                    }
                }
                self.dedent();
                self.line("}");
            }
            TypeBody::Struct(fields) => {
                self.line("#[derive(Debug, Clone, PartialEq)]");
                self.line(&format!("struct {}{} {{", td.name, type_params));
                self.indent();
                for field in fields {
                    let ty = self.type_to_rust(&field.ty, &td.name);
                    self.line(&format!("{}: {},", field.name, ty));
                }
                self.dedent();
                self.line("}");
            }
            TypeBody::Alias(ty) => {
                let rust_ty = self.type_to_rust(ty, &td.name);
                self.line(&format!("type {}{} = {};", td.name, type_params, rust_ty));
            }
        }
    }

    fn type_to_rust(&self, ty: &TypeExpr, _parent_type: &str) -> String {
        match ty {
            TypeExpr::Named(name, args) => {
                let rust_name = match name.as_str() {
                    "Int" => "i64".to_string(),
                    "Int8" => "i8".to_string(),
                    "Int16" => "i16".to_string(),
                    "Int32" => "i32".to_string(),
                    "UInt" => "u64".to_string(),
                    "UInt8" => "u8".to_string(),
                    "UInt16" => "u16".to_string(),
                    "UInt32" => "u32".to_string(),
                    "Float" => "f64".to_string(),
                    "Float32" => "f32".to_string(),
                    "Bool" => "bool".to_string(),
                    "String" => "String".to_string(),
                    "Char" => "char".to_string(),
                    "List" => {
                        if args.len() == 1 {
                            let inner = self.type_to_rust(&args[0], _parent_type);
                            return format!("Vec<{inner}>");
                        }
                        "Vec<_>".to_string()
                    }
                    "Map" => {
                        if args.len() == 2 {
                            let k = self.type_to_rust(&args[0], _parent_type);
                            let v = self.type_to_rust(&args[1], _parent_type);
                            return format!("std::collections::HashMap<{k}, {v}>");
                        }
                        "std::collections::HashMap<_, _>".to_string()
                    }
                    "Set" => {
                        if args.len() == 1 {
                            let inner = self.type_to_rust(&args[0], _parent_type);
                            return format!("std::collections::HashSet<{inner}>");
                        }
                        "std::collections::HashSet<_>".to_string()
                    }
                    "Deque" => {
                        if args.len() == 1 {
                            let inner = self.type_to_rust(&args[0], _parent_type);
                            return format!("std::collections::VecDeque<{inner}>");
                        }
                        "std::collections::VecDeque<_>".to_string()
                    }
                    "Heap" => {
                        if args.len() == 1 {
                            let inner = self.type_to_rust(&args[0], _parent_type);
                            return format!("std::collections::BinaryHeap<{inner}>");
                        }
                        "std::collections::BinaryHeap<_>".to_string()
                    }
                    other => other.to_string(),
                };
                if args.is_empty() || name == "List" {
                    rust_name
                } else {
                    let type_args: Vec<String> = args
                        .iter()
                        .map(|a| self.type_to_rust(a, _parent_type))
                        .collect();
                    format!("{rust_name}<{}>", type_args.join(", "))
                }
            }
            TypeExpr::Function(params, ret) => {
                let ps: Vec<String> = params
                    .iter()
                    .map(|p| self.type_to_rust(p, _parent_type))
                    .collect();
                let r = self.type_to_rust(ret, _parent_type);
                format!("impl Fn({}) -> {r}", ps.join(", "))
            }
            TypeExpr::Tuple(types) => {
                let ts: Vec<String> = types
                    .iter()
                    .map(|t| self.type_to_rust(t, _parent_type))
                    .collect();
                format!("({})", ts.join(", "))
            }
            TypeExpr::Ref(inner) => {
                let i = self.type_to_rust(inner, _parent_type);
                format!("&{i}")
            }
            TypeExpr::MutRef(inner) => {
                let i = self.type_to_rust(inner, _parent_type);
                format!("&mut {i}")
            }
            TypeExpr::Move(inner) => self.type_to_rust(inner, _parent_type),
            TypeExpr::Dyn(trait_name) => format!("Box<dyn {trait_name}>"),
            TypeExpr::Lifetime(name) => format!("'{name}"),
        }
    }

    // ── Modules ─────────────────────────────────────────────────────────────

    fn emit_module(&mut self, m: &ModuleDecl) {
        let name = to_snake_case(&m.name);
        self.line(&format!("mod {name} {{"));
        self.indent();
        self.line("use super::*;");
        self.push("\n");
        for item in &m.items {
            match item {
                Item::TypeDecl(td) => self.emit_type_decl(td),
                Item::Function(f) => self.emit_function(f),
                Item::TraitDecl(t) => self.emit_trait_decl(t),
                Item::ImplBlock(imp) => self.emit_impl_block(imp),
                Item::ExternFn(ef) => self.emit_extern_fn(ef),
                _ => {}
            }
            self.output.push('\n');
        }
        self.dedent();
        self.line("}");
    }

    // ── Traits ──────────────────────────────────────────────────────────────

    fn emit_trait_decl(&mut self, t: &TraitDecl) {
        let type_params = fmt_type_params(&t.type_params);
        self.line(&format!("trait {}{type_params} {{", t.name));
        self.indent();
        // Emit associated types
        for assoc_ty in &t.associated_types {
            self.line(&format!("type {assoc_ty};"));
        }
        for method in &t.methods {
            let params: Vec<String> = method
                .params
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    if i == 0 && p.name == "self" {
                        "&self".to_string()
                    } else {
                        let ty = p
                            .ty
                            .as_ref()
                            .map(|t| self.type_to_rust(t, ""))
                            .unwrap_or_else(|| "impl std::any::Any".to_string());
                        format!("{}: {ty}", to_snake_case(&p.name))
                    }
                })
                .collect();
            let ret = method
                .return_type
                .as_ref()
                .map(|t| format!(" -> {}", self.type_to_rust(t, "")))
                .unwrap_or_default();
            if let Some(body) = &method.default_body {
                self.line(&format!("fn {}({}){ret} {{", to_snake_case(&method.name), params.join(", ")));
                self.indent();
                self.emit_indent();
                self.emit_expr(body);
                self.push("\n");
                self.dedent();
                self.line("}");
            } else {
                self.line(&format!("fn {}({}){ret};", to_snake_case(&method.name), params.join(", ")));
            }
        }
        self.dedent();
        self.line("}");
    }

    fn emit_impl_block(&mut self, imp: &ImplBlock) {
        // Check for operator overloading: well-known trait names get special treatment
        let operator_traits = [
            "Add", "Sub", "Mul", "Div", "Rem",
            "PartialEq", "PartialOrd",
            "Neg", "Not",
            "Index", "Display",
        ];
        let type_params = fmt_type_params(&imp.type_params);
        let is_operator_trait = imp.trait_name.as_ref()
            .map(|t| operator_traits.contains(&t.as_str()))
            .unwrap_or(false);

        let target = if let Some(trait_name) = &imp.trait_name {
            if is_operator_trait {
                // For operator traits, emit the full std::ops path
                let rust_trait = match trait_name.as_str() {
                    "Add" => "std::ops::Add",
                    "Sub" => "std::ops::Sub",
                    "Mul" => "std::ops::Mul",
                    "Div" => "std::ops::Div",
                    "Rem" => "std::ops::Rem",
                    "Neg" => "std::ops::Neg",
                    "Not" => "std::ops::Not",
                    "Index" => "std::ops::Index",
                    "PartialEq" => "PartialEq",
                    "PartialOrd" => "PartialOrd",
                    "Display" => "std::fmt::Display",
                    _ => trait_name,
                };
                format!("impl{type_params} {rust_trait} for {}", imp.type_name)
            } else {
                format!("impl{type_params} {} for {}", trait_name, imp.type_name)
            }
        } else {
            format!("impl{type_params} {}", imp.type_name)
        };
        self.line(&format!("{target} {{"));
        self.indent();
        // Emit associated type definitions
        for (name, ty) in &imp.associated_types {
            let rust_ty = self.type_to_rust(ty, "");
            self.line(&format!("type {name} = {rust_ty};"));
        }
        for method in &imp.methods {
            self.emit_function(method);
            self.output.push('\n');
        }
        self.dedent();
        self.line("}");
    }

    fn emit_const(&mut self, c: &ConstDecl) {
        let vis = if c.is_pub { "pub " } else { "" };
        // For string literals, use static &str (not .to_string())
        let is_string = matches!(&c.value.kind, ExprKind::StringLit(_));
        let ty = c.ty.as_ref()
            .map(|t| self.type_to_rust(t, ""))
            .unwrap_or_else(|| self.infer_const_type(&c.value));
        let name = &c.name;
        self.emit_indent();
        if is_string {
            // For strings, emit as const &str (no .to_string())
            self.push(&format!("{vis}const {name}: &str = "));
            if let ExprKind::StringLit(s) = &c.value.kind {
                let escaped = escape_rust_string(s);
                self.push(&format!("\"{escaped}\""));
            }
        } else {
            self.push(&format!("{vis}const {name}: {ty} = "));
            self.emit_expr(&c.value);
        }
        self.push(";\n");
    }

    fn infer_const_type(&self, expr: &Expr) -> String {
        match &expr.kind {
            ExprKind::IntLit(_) => "i64".to_string(),
            ExprKind::FloatLit(_) => "f64".to_string(),
            ExprKind::BoolLit(_) => "bool".to_string(),
            ExprKind::StringLit(_) | ExprKind::StringInterp(_) => "&'static str".to_string(),
            _ => "i64".to_string(), // fallback
        }
    }

    fn emit_extern_fn(&mut self, ef: &ExternFn) {
        let rust_name = ef.rust_name.as_deref().unwrap_or(&ef.name);
        let params: Vec<String> = ef
            .params
            .iter()
            .map(|p| {
                let ty = p
                    .ty
                    .as_ref()
                    .map(|t| self.type_to_rust(t, ""))
                    .unwrap_or_else(|| "impl std::any::Any".to_string());
                format!("{}: {ty}", to_snake_case(&p.name))
            })
            .collect();
        let ret = ef
            .return_type
            .as_ref()
            .map(|t| format!(" -> {}", self.type_to_rust(t, "")))
            .unwrap_or_default();
        // Generate a wrapper function that calls the Rust path
        self.line(&format!(
            "fn {}({}){ret} {{",
            to_snake_case(&ef.name),
            params.join(", ")
        ));
        self.indent();
        let args: Vec<String> = ef.params.iter().map(|p| to_snake_case(&p.name)).collect();
        self.line(&format!("{rust_name}({})", args.join(", ")));
        self.dedent();
        self.line("}");
    }

    // ── Functions ───────────────────────────────────────────────────────────

    fn emit_function(&mut self, f: &Function) {
        // Emit annotations (e.g., #[cfg(...)])
        for ann in &f.annotations {
            self.line(&format!("#[{ann}]"));
        }
        let vis = if f.is_pub { "pub " } else { "" };
        let async_kw = if f.is_async { "async " } else { "" };
        let type_params = fmt_type_params(&f.type_params);

        let params: Vec<String> = f
            .params
            .iter()
            .map(|p| {
                if p.name == "self" {
                    return "&self".to_string();
                }
                let ty = p
                    .ty
                    .as_ref()
                    .map(|t| self.type_to_rust(t, ""))
                    .unwrap_or_else(|| "i64".to_string());
                if let Some(pattern) = &p.destructure {
                    let pat_str = self.pattern_to_rust(pattern);
                    format!("{pat_str}: {ty}")
                } else {
                    format!("{}: {ty}", p.name)
                }
            })
            .collect();

        let ret = f
            .return_type
            .as_ref()
            .map(|t| format!(" -> {}", self.type_to_rust(t, "")))
            .unwrap_or_default();

        self.line(&format!(
            "{vis}{async_kw}fn {}{type_params}({params}){ret} {{",
            f.name,
            params = params.join(", ")
        ));
        self.indent();
        self.emit_indent();
        self.emit_expr_as_return(&f.body);
        self.push("\n");
        self.dedent();
        self.line("}");
    }

    fn emit_main_function(&mut self, f: &Function) {
        if f.is_async {
            self.line("#[tokio::main]");
            self.line("async fn main() {");
        } else {
            self.line("fn main() {");
        }
        self.indent();
        self.emit_indent();
        self.emit_expr(&f.body);
        self.push(";\n");
        self.dedent();
        self.line("}");
    }

    // ── Builtin Functions ───────────────────────────────────────────────────

    /// Try to emit a builtin call. Returns true if it was handled.
    fn try_emit_builtin_call(&mut self, name: &str, args: &[Expr]) -> bool {
        match name {
            // ── I/O ─────────────────────────────────────────────────────
            "println" => {
                if args.is_empty() {
                    self.push("println!()");
                } else if args.len() == 1 {
                    self.push("println!(\"{}\", star_display(&(");
                    self.emit_expr(&args[0]);
                    self.push(")))");
                } else {
                    // println(format, args...) — pass through directly
                    self.push("println!(");
                    self.emit_expr(&args[0]);
                    for arg in &args[1..] {
                        self.push(", ");
                        self.emit_expr(arg);
                    }
                    self.push(")");
                }
                true
            }
            "print" => {
                if args.is_empty() {
                    self.push("print!(\"\")");
                } else if args.len() == 1 {
                    self.push("print!(\"{}\", star_display(&(");
                    self.emit_expr(&args[0]);
                    self.push(")))");
                } else {
                    self.push("print!(");
                    self.emit_expr(&args[0]);
                    for arg in &args[1..] {
                        self.push(", ");
                        self.emit_expr(arg);
                    }
                    self.push(")");
                }
                true
            }
            "eprintln" => {
                if args.len() == 1 {
                    self.push("eprintln!(\"{}\", star_display(&(");
                    self.emit_expr(&args[0]);
                    self.push(")))");
                } else {
                    self.push("eprintln!(");
                    if !args.is_empty() {
                        self.emit_expr(&args[0]);
                    }
                    for arg in args.iter().skip(1) {
                        self.push(", ");
                        self.emit_expr(arg);
                    }
                    self.push(")");
                }
                true
            }
            "debug" => {
                if args.len() == 1 {
                    self.push("println!(\"{:?}\", ");
                    self.emit_expr(&args[0]);
                    self.push(")");
                } else {
                    self.push("println!(\"");
                    for (i, _) in args.iter().enumerate() {
                        if i > 0 {
                            self.push(", ");
                        }
                        self.push("{:?}");
                    }
                    self.push("\"");
                    for arg in args {
                        self.push(", ");
                        self.emit_expr(arg);
                    }
                    self.push(")");
                }
                true
            }

            // ── Stdin ────────────────────────────────────────────────────
            "read_line" if args.is_empty() => {
                self.push("{ let mut _buf = String::new(); std::io::stdin().read_line(&mut _buf).unwrap(); _buf.trim_end_matches('\\n').trim_end_matches('\\r').to_string() }");
                true
            }
            "read_all_stdin" if args.is_empty() => {
                self.push("{ let mut _buf = String::new(); std::io::Read::read_to_string(&mut std::io::stdin(), &mut _buf).unwrap(); _buf }");
                true
            }

            // ── File system ──────────────────────────────────────────────
            "read_file" if args.len() == 1 => {
                // read_file(path) -> Result<String, String>
                self.push("std::fs::read_to_string(&*");
                self.emit_expr(&args[0]);
                self.push(").map_err(|e| e.to_string())");
                true
            }
            "write_file" if args.len() == 2 => {
                // write_file(path, content) -> Result<(), String>
                self.push("std::fs::write(&*");
                self.emit_expr(&args[0]);
                self.push(", &*");
                self.emit_expr(&args[1]);
                self.push(").map_err(|e| e.to_string())");
                true
            }
            "append_file" if args.len() == 2 => {
                // append_file(path, content) -> Result<(), String>
                self.push("{ use std::io::Write; std::fs::OpenOptions::new().append(true).create(true).open(&*");
                self.emit_expr(&args[0]);
                self.push(").and_then(|mut f| f.write_all(");
                self.emit_expr(&args[1]);
                self.push(".as_bytes())).map_err(|e| e.to_string()) }");
                true
            }
            "file_exists" if args.len() == 1 => {
                self.push("std::path::Path::new(&*");
                self.emit_expr(&args[0]);
                self.push(").exists()");
                true
            }
            "delete_file" if args.len() == 1 => {
                self.push("std::fs::remove_file(&*");
                self.emit_expr(&args[0]);
                self.push(").map_err(|e| e.to_string())");
                true
            }
            "rename_file" if args.len() == 2 => {
                self.push("std::fs::rename(&*");
                self.emit_expr(&args[0]);
                self.push(", &*");
                self.emit_expr(&args[1]);
                self.push(").map_err(|e| e.to_string())");
                true
            }
            "copy_file" if args.len() == 2 => {
                self.push("std::fs::copy(&*");
                self.emit_expr(&args[0]);
                self.push(", &*");
                self.emit_expr(&args[1]);
                self.push(").map(|_| ()).map_err(|e| e.to_string())");
                true
            }
            "file_size" if args.len() == 1 => {
                self.push("std::fs::metadata(&*");
                self.emit_expr(&args[0]);
                self.push(").map(|m| m.len() as i64).map_err(|e| e.to_string())");
                true
            }
            "read_lines" if args.len() == 1 => {
                // read_lines(path) -> Result<List<String>, String>
                self.push("std::fs::read_to_string(&*");
                self.emit_expr(&args[0]);
                self.push(").map(|s| s.lines().map(|l| l.to_string()).collect::<Vec<_>>()).map_err(|e| e.to_string())");
                true
            }

            // ── Directories ──────────────────────────────────────────────
            "list_dir" if args.len() == 1 => {
                // list_dir(path) -> Result<List<String>, String>
                self.push("std::fs::read_dir(&*");
                self.emit_expr(&args[0]);
                self.push(").map(|entries| entries.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).collect::<Vec<_>>()).map_err(|e| e.to_string())");
                true
            }
            "create_dir" if args.len() == 1 => {
                self.push("std::fs::create_dir(&*");
                self.emit_expr(&args[0]);
                self.push(").map_err(|e| e.to_string())");
                true
            }
            "create_dir_all" if args.len() == 1 => {
                self.push("std::fs::create_dir_all(&*");
                self.emit_expr(&args[0]);
                self.push(").map_err(|e| e.to_string())");
                true
            }
            "delete_dir" if args.len() == 1 => {
                self.push("std::fs::remove_dir_all(&*");
                self.emit_expr(&args[0]);
                self.push(").map_err(|e| e.to_string())");
                true
            }
            "dir_exists" if args.len() == 1 => {
                self.push("std::path::Path::new(&*");
                self.emit_expr(&args[0]);
                self.push(").is_dir()");
                true
            }

            // ── Path operations ──────────────────────────────────────────
            "path_join" if args.len() == 2 => {
                self.push("std::path::Path::new(&*");
                self.emit_expr(&args[0]);
                self.push(").join(&*");
                self.emit_expr(&args[1]);
                self.push(").to_string_lossy().to_string()");
                true
            }
            "path_parent" if args.len() == 1 => {
                self.push("std::path::Path::new(&*");
                self.emit_expr(&args[0]);
                self.push(").parent().map(|p| p.to_string_lossy().to_string())");
                true
            }
            "path_filename" if args.len() == 1 => {
                self.push("std::path::Path::new(&*");
                self.emit_expr(&args[0]);
                self.push(").file_name().map(|n| n.to_string_lossy().to_string())");
                true
            }
            "path_extension" if args.len() == 1 => {
                self.push("std::path::Path::new(&*");
                self.emit_expr(&args[0]);
                self.push(").extension().map(|e| e.to_string_lossy().to_string())");
                true
            }
            "path_stem" if args.len() == 1 => {
                self.push("std::path::Path::new(&*");
                self.emit_expr(&args[0]);
                self.push(").file_stem().map(|s| s.to_string_lossy().to_string())");
                true
            }
            "path_is_absolute" if args.len() == 1 => {
                self.push("std::path::Path::new(&*");
                self.emit_expr(&args[0]);
                self.push(").is_absolute()");
                true
            }
            "path_is_relative" if args.len() == 1 => {
                self.push("!std::path::Path::new(&*");
                self.emit_expr(&args[0]);
                self.push(").is_absolute()");
                true
            }

            // ── Environment & process ────────────────────────────────────
            "env_get" if args.len() == 1 => {
                self.push("std::env::var(&*");
                self.emit_expr(&args[0]);
                self.push(").ok()");
                true
            }
            "env_set" if args.len() == 2 => {
                self.push("unsafe { std::env::set_var(&*");
                self.emit_expr(&args[0]);
                self.push(", &*");
                self.emit_expr(&args[1]);
                self.push(") }");
                true
            }
            "env_remove" if args.len() == 1 => {
                self.push("unsafe { std::env::remove_var(&*");
                self.emit_expr(&args[0]);
                self.push(") }");
                true
            }
            "env_vars" if args.is_empty() => {
                self.push("std::env::vars().collect::<Vec<(String, String)>>()");
                true
            }
            "current_dir" if args.is_empty() => {
                self.push("std::env::current_dir().map(|p| p.to_string_lossy().to_string()).map_err(|e| e.to_string())");
                true
            }
            "set_current_dir" if args.len() == 1 => {
                self.push("std::env::set_current_dir(&*");
                self.emit_expr(&args[0]);
                self.push(").map_err(|e| e.to_string())");
                true
            }
            "args" if args.is_empty() => {
                self.push("std::env::args().collect::<Vec<String>>()");
                true
            }
            "command" if args.len() == 1 => {
                // command(cmd_string) -> Result<Int, String>  (exit code)
                self.push("std::process::Command::new(\"sh\").arg(\"-c\").arg(&*");
                self.emit_expr(&args[0]);
                self.push(").status().map(|s| s.code().unwrap_or(-1) as i64).map_err(|e| e.to_string())");
                true
            }
            "command_output" if args.len() == 1 => {
                // command_output(cmd_string) -> Result<{stdout, stderr, status}, String>
                // Returns a tuple: (stdout: String, stderr: String, exit_code: Int)
                self.push("std::process::Command::new(\"sh\").arg(\"-c\").arg(&*");
                self.emit_expr(&args[0]);
                self.push(").output().map(|o| (String::from_utf8_lossy(&o.stdout).to_string(), String::from_utf8_lossy(&o.stderr).to_string(), o.status.code().unwrap_or(-1) as i64)).map_err(|e| e.to_string())");
                true
            }
            "command_with_stdin" if args.len() == 2 => {
                // command_with_stdin(cmd, stdin_data) -> Result<(String, String, Int), String>
                self.push("{ let _cmd_str = ");
                self.emit_expr(&args[0]);
                self.push("; let _stdin_data = ");
                self.emit_expr(&args[1]);
                self.push("; (|| -> Result<(String, String, i64), String> { use std::io::Write; let mut child = std::process::Command::new(\"sh\").arg(\"-c\").arg(&*_cmd_str).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().map_err(|e| e.to_string())?; child.stdin.take().unwrap().write_all(_stdin_data.as_bytes()).map_err(|e| e.to_string())?; let o = child.wait_with_output().map_err(|e| e.to_string())?; Ok((String::from_utf8_lossy(&o.stdout).to_string(), String::from_utf8_lossy(&o.stderr).to_string(), o.status.code().unwrap_or(-1) as i64)) })() }");
                true
            }
            "command_with_args" if args.len() == 2 => {
                // command_with_args(program, args_list) -> Result<Int, String>
                self.push("{ let _prog = ");
                self.emit_expr(&args[0]);
                self.push("; let _args: Vec<String> = ");
                self.emit_expr(&args[1]);
                self.push("; std::process::Command::new(&*_prog).args(&_args).status().map(|s| s.code().unwrap_or(-1) as i64).map_err(|e| e.to_string()) }");
                true
            }
            "command_with_args_output" if args.len() == 2 => {
                // command_with_args_output(program, args_list) -> Result<(String, String, Int), String>
                self.push("{ let _prog = ");
                self.emit_expr(&args[0]);
                self.push("; let _args: Vec<String> = ");
                self.emit_expr(&args[1]);
                self.push("; std::process::Command::new(&*_prog).args(&_args).output().map(|o| (String::from_utf8_lossy(&o.stdout).to_string(), String::from_utf8_lossy(&o.stderr).to_string(), o.status.code().unwrap_or(-1) as i64)).map_err(|e| e.to_string()) }");
                true
            }
            "process_id" if args.is_empty() => {
                self.push("(std::process::id() as i64)");
                true
            }
            "kill_process" if args.len() == 1 => {
                // kill_process(pid) -> Result<(), String> — sends SIGKILL on unix
                self.push("{ let _pid = ");
                self.emit_expr(&args[0]);
                self.push(" as u32; ");
                self.push("(|| -> Result<(), String> { ");
                self.push("#[cfg(unix)] { use std::os::unix::process::CommandExt; ");
                self.push("std::process::Command::new(\"kill\").arg(\"-9\").arg(_pid.to_string()).status().map(|_| ()).map_err(|e| e.to_string()) }");
                self.push(" #[cfg(not(unix))] { Err(\"kill_process not supported on this platform\".to_string()) }");
                self.push(" })() }");
                true
            }

            // ── File metadata & permissions ──────────────────────────────
            "is_file" if args.len() == 1 => {
                self.push("std::path::Path::new(&*");
                self.emit_expr(&args[0]);
                self.push(").is_file()");
                true
            }
            "is_dir" if args.len() == 1 => {
                self.push("std::path::Path::new(&*");
                self.emit_expr(&args[0]);
                self.push(").is_dir()");
                true
            }
            "is_symlink" if args.len() == 1 => {
                self.push("std::path::Path::new(&*");
                self.emit_expr(&args[0]);
                self.push(").is_symlink()");
                true
            }
            "file_modified" if args.len() == 1 => {
                // file_modified(path) -> Result<Int, String> — unix timestamp seconds
                self.push("std::fs::metadata(&*");
                self.emit_expr(&args[0]);
                self.push(").and_then(|m| m.modified()).map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64).map_err(|e| e.to_string())");
                true
            }
            "file_created" if args.len() == 1 => {
                // file_created(path) -> Result<Int, String> — unix timestamp seconds
                self.push("std::fs::metadata(&*");
                self.emit_expr(&args[0]);
                self.push(").and_then(|m| m.created()).map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64).map_err(|e| e.to_string())");
                true
            }
            "file_readonly" if args.len() == 1 => {
                // file_readonly(path) -> Result<Bool, String>
                self.push("std::fs::metadata(&*");
                self.emit_expr(&args[0]);
                self.push(").map(|m| m.permissions().readonly()).map_err(|e| e.to_string())");
                true
            }
            "set_readonly" if args.len() == 2 => {
                // set_readonly(path, readonly) -> Result<(), String>
                self.push("{ let _path = (");
                self.emit_expr(&args[0]);
                self.push(").clone(); let _ro = ");
                self.emit_expr(&args[1]);
                self.push("; std::fs::metadata(&*_path).and_then(|m| { let mut perms = m.permissions(); perms.set_readonly(_ro); std::fs::set_permissions(&*_path, perms) }).map_err(|e| e.to_string()) }");
                true
            }
            "symlink" if args.len() == 2 => {
                // symlink(src, dst) -> Result<(), String>
                self.push("{ let _sym_src = (");
                self.emit_expr(&args[0]);
                self.push(").clone(); let _sym_dst = (");
                self.emit_expr(&args[1]);
                self.push(").clone(); ");
                self.push("#[cfg(unix)] { std::os::unix::fs::symlink(&*_sym_src, &*_sym_dst).map_err(|e| e.to_string()) }");
                self.push(" #[cfg(not(unix))] { Err(\"symlink not supported on this platform\".to_string()) }");
                self.push(" }");
                true
            }
            "read_link" if args.len() == 1 => {
                // read_link(path) -> Result<String, String>
                self.push("std::fs::read_link(&*");
                self.emit_expr(&args[0]);
                self.push(").map(|p| p.to_string_lossy().to_string()).map_err(|e| e.to_string())");
                true
            }
            "canonicalize" if args.len() == 1 => {
                // canonicalize(path) -> Result<String, String>
                self.push("std::fs::canonicalize(&*");
                self.emit_expr(&args[0]);
                self.push(").map(|p| p.to_string_lossy().to_string()).map_err(|e| e.to_string())");
                true
            }
            "temp_dir" if args.is_empty() => {
                self.push("std::env::temp_dir().to_string_lossy().to_string()");
                true
            }
            "exe_path" if args.is_empty() => {
                // exe_path() -> Result<String, String>
                self.push("std::env::current_exe().map(|p| p.to_string_lossy().to_string()).map_err(|e| e.to_string())");
                true
            }

            // ── Concurrency — threads ────────────────────────────────────
            "spawn" if args.len() == 1 => {
                // spawn(fn) -> JoinHandle — runs closure in a new thread
                self.push("std::thread::spawn(move || { (");
                self.emit_expr(&args[0]);
                self.push(")() })");
                true
            }
            "spawn_join" if args.len() == 1 => {
                // spawn_join(fn) -> Result — spawns and immediately joins
                self.push("std::thread::spawn(move || { (");
                self.emit_expr(&args[0]);
                self.push(")() }).join().map_err(|_| \"thread panicked\".to_string())");
                true
            }

            // ── Concurrency — channels ───────────────────────────────────
            "channel" if args.is_empty() => {
                // channel() -> (Sender, Receiver) — unbounded mpsc channel
                self.push("std::sync::mpsc::channel()");
                true
            }
            "send" if args.len() == 2 => {
                // send(sender, value) -> Result
                self.emit_expr(&args[0]);
                self.push(".send(");
                self.emit_expr(&args[1]);
                self.push(").map_err(|e| e.to_string())");
                true
            }
            "recv" if args.len() == 1 => {
                // recv(receiver) -> Result — blocking receive
                self.emit_expr(&args[0]);
                self.push(".recv().map_err(|e| e.to_string())");
                true
            }
            "try_recv" if args.len() == 1 => {
                // try_recv(receiver) -> Option — non-blocking receive
                self.emit_expr(&args[0]);
                self.push(".try_recv().ok()");
                true
            }

            // ── Concurrency — sync primitives ────────────────────────────
            "mutex_new" if args.len() == 1 => {
                // mutex_new(value) -> Arc<Mutex<T>>
                self.push("std::sync::Arc::new(std::sync::Mutex::new(");
                self.emit_expr(&args[0]);
                self.push("))");
                true
            }
            "mutex_lock" if args.len() == 1 => {
                // mutex_lock(mutex) -> MutexGuard (deref to inner value)
                self.emit_expr(&args[0]);
                self.push(".lock().unwrap().clone()");
                true
            }
            "rwlock_new" if args.len() == 1 => {
                self.push("std::sync::Arc::new(std::sync::RwLock::new(");
                self.emit_expr(&args[0]);
                self.push("))");
                true
            }
            "rwlock_read" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".read().unwrap().clone()");
                true
            }
            "rwlock_write" if args.len() == 1 => {
                // rwlock_write(rwlock) — returns clone of inner after acquiring write lock
                self.emit_expr(&args[0]);
                self.push(".write().unwrap().clone()");
                true
            }
            "atomic_new" if args.len() == 1 => {
                self.push("std::sync::Arc::new(std::sync::atomic::AtomicI64::new(");
                self.emit_expr(&args[0]);
                self.push("))");
                true
            }
            "atomic_get" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".load(std::sync::atomic::Ordering::SeqCst)");
                true
            }
            "atomic_set" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".store(");
                self.emit_expr(&args[1]);
                self.push(", std::sync::atomic::Ordering::SeqCst)");
                true
            }
            "atomic_add" if args.len() == 2 => {
                // atomic_add(atomic, delta) -> previous value
                self.emit_expr(&args[0]);
                self.push(".fetch_add(");
                self.emit_expr(&args[1]);
                self.push(", std::sync::atomic::Ordering::SeqCst)");
                true
            }

            // ── Concurrency — async/tokio ────────────────────────────────
            "sleep" if args.len() == 1 => {
                // sleep(seconds) — async sleep
                self.push("tokio::time::sleep(std::time::Duration::from_secs(");
                self.emit_expr(&args[0]);
                self.push(" as u64)).await");
                true
            }
            "sleep_ms" if args.len() == 1 => {
                self.push("tokio::time::sleep(std::time::Duration::from_millis(");
                self.emit_expr(&args[0]);
                self.push(" as u64)).await");
                true
            }
            "timeout" if args.len() == 2 => {
                // timeout(seconds, future) -> Result
                self.push("tokio::time::timeout(std::time::Duration::from_secs(");
                self.emit_expr(&args[0]);
                self.push(" as u64), ");
                self.emit_expr(&args[1]);
                self.push(").await.map_err(|_| \"timed out\".to_string())");
                true
            }
            "spawn_async" if args.len() == 1 => {
                // spawn_async(async_fn) -> JoinHandle
                self.push("tokio::spawn(async move { (");
                self.emit_expr(&args[0]);
                self.push(")().await })");
                true
            }
            "spawn_blocking" if args.len() == 1 => {
                // spawn_blocking(fn) -> JoinHandle — run blocking code on thread pool
                self.push("tokio::task::spawn_blocking(move || { (");
                self.emit_expr(&args[0]);
                self.push(")() })");
                true
            }

            // ── Concurrency — parallel helpers ───────────────────────────
            "parallel_map" if args.len() == 2 => {
                // parallel_map(list, fn) -> List — maps in parallel using threads
                self.push("{ let _items = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); let _f = ");
                self.emit_expr(&args[1]);
                self.push("; let _handles: Vec<_> = _items.into_iter().map(|_item| { let _f = _f.clone(); std::thread::spawn(move || _f(_item)) }).collect(); _handles.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>() }");
                true
            }

            // ── List operations (first arg is the list) ─────────────────
            "map" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().map(|_item| (");
                self.emit_expr(&args[1]);
                self.push(")(_item)).collect::<Vec<_>>()");
                true
            }
            "filter" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().filter(|_item| (");
                self.emit_expr(&args[1]);
                self.push(")(_item.clone())).collect::<Vec<_>>()");
                true
            }
            "fold" if args.len() == 3 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().fold(");
                self.emit_expr(&args[1]);
                self.push(", |_acc, _item| (");
                self.emit_expr(&args[2]);
                self.push(")(_acc, _item))");
                true
            }
            "each" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().for_each(|_item| { (");
                self.emit_expr(&args[1]);
                self.push(")(_item); })");
                true
            }
            "flat_map" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().flat_map(|_item| (");
                self.emit_expr(&args[1]);
                self.push(")(_item)).collect::<Vec<_>>()");
                true
            }
            "any" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().any(|_item| (");
                self.emit_expr(&args[1]);
                self.push(")(_item))");
                true
            }
            "all" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().all(|_item| (");
                self.emit_expr(&args[1]);
                self.push(")(_item))");
                true
            }
            "find" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().find(|_item| (");
                self.emit_expr(&args[1]);
                self.push(")(_item.clone()))");
                true
            }
            "enumerate" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().enumerate().map(|(i, v)| (i as i64, v)).collect::<Vec<_>>()");
                true
            }
            "take" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().take(");
                self.emit_expr(&args[1]);
                self.push(" as usize).collect::<Vec<_>>()");
                true
            }
            "drop" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().skip(");
                self.emit_expr(&args[1]);
                self.push(" as usize).collect::<Vec<_>>()");
                true
            }
            "zip" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().zip(");
                self.emit_expr(&args[1]);
                self.push(".clone().into_iter()).collect::<Vec<_>>()");
                true
            }
            "flatten" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().flatten().collect::<Vec<_>>()");
                true
            }
            "reverse" if args.len() == 1 => {
                self.push("{ let mut _v = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); _v.reverse(); _v }");
                true
            }
            "sort" if args.len() == 1 => {
                self.push("{ let mut _v = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); _v.sort(); _v }");
                true
            }
            "sort_by" if args.len() == 2 => {
                self.push("{ let mut _v = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); _v.sort_by(|a, b| (");
                self.emit_expr(&args[1]);
                self.push(")(a, b)); _v }");
                true
            }
            "head" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".first().cloned()");
                true
            }
            "tail" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".get(1..).map(|s| s.to_vec()).unwrap_or_default()");
                true
            }
            "last" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".last().cloned()");
                true
            }
            "init" if args.len() == 1 => {
                self.push("{ let _v = &");
                self.emit_expr(&args[0]);
                self.push("; if _v.is_empty() { vec![] } else { _v[.._v.len()-1].to_vec() } }");
                true
            }
            "push" if args.len() == 2 => {
                self.push("{ let mut _v = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); _v.push(");
                self.emit_expr(&args[1]);
                self.push("); _v }");
                true
            }
            "concat" if args.len() == 2 => {
                self.push("{ let mut _v = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); _v.extend(");
                self.emit_expr(&args[1]);
                self.push(".iter().cloned()); _v }");
                true
            }
            "dedup" if args.len() == 1 => {
                self.push("{ let mut _v = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); _v.dedup(); _v }");
                true
            }
            "sum" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().sum::<i64>()");
                true
            }
            "product" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().product::<i64>()");
                true
            }
            "count" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".len() as i64");
                true
            }
            "min_by" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().min_by_key(|_item| (");
                self.emit_expr(&args[1]);
                self.push(")(_item.clone()))");
                true
            }
            "max_by" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().max_by_key(|_item| (");
                self.emit_expr(&args[1]);
                self.push(")(_item.clone()))");
                true
            }

            // ── Collection algorithms & utilities ────────────────────────

            // Searching
            "binary_search" if args.len() == 2 => {
                // binary_search(sorted_list, value) -> Option<Int> (index)
                self.push("{ let _v = &");
                self.emit_expr(&args[0]);
                self.push("; match _v.binary_search(&(");
                self.emit_expr(&args[1]);
                self.push(")) { Ok(i) => Some(i as i64), Err(_) => None } }");
                true
            }
            "position" if args.len() == 2 => {
                // position(list, predicate) -> Option<Int>
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().position(|_item| (");
                self.emit_expr(&args[1]);
                self.push(")(_item)).map(|i| i as i64)");
                true
            }
            "contains_element" if args.len() == 2 => {
                // contains_element(list, value) -> Bool
                self.emit_expr(&args[0]);
                self.push(".contains(&(");
                self.emit_expr(&args[1]);
                self.push("))");
                true
            }

            // Sorting
            "sort_desc" if args.len() == 1 => {
                // sort_desc(list) -> sorted list in descending order
                self.push("{ let mut _v = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); _v.sort(); _v.reverse(); _v }");
                true
            }
            "sort_by_key" if args.len() == 2 => {
                // sort_by_key(list, key_fn) -> sorted list
                self.push("{ let mut _v = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); _v.sort_by_key(|_item| (");
                self.emit_expr(&args[1]);
                self.push(")(_item.clone())); _v }");
                true
            }
            "is_sorted" if args.len() == 1 => {
                // is_sorted(list) -> Bool
                self.emit_expr(&args[0]);
                self.push(".windows(2).all(|w| w[0] <= w[1])");
                true
            }

            // Slicing & windowing
            "chunks" if args.len() == 2 => {
                // chunks(list, n) -> List<List<T>>
                self.emit_expr(&args[0]);
                self.push(".chunks(");
                self.emit_expr(&args[1]);
                self.push(" as usize).map(|c| c.to_vec()).collect::<Vec<_>>()");
                true
            }
            "windows" if args.len() == 2 => {
                // windows(list, n) -> List<List<T>>
                self.emit_expr(&args[0]);
                self.push(".windows(");
                self.emit_expr(&args[1]);
                self.push(" as usize).map(|w| w.to_vec()).collect::<Vec<_>>()");
                true
            }
            "nth" if args.len() == 2 => {
                // nth(list, n) -> Option<T>
                self.emit_expr(&args[0]);
                self.push(".get(");
                self.emit_expr(&args[1]);
                self.push(" as usize).cloned()");
                true
            }
            "take_while" if args.len() == 2 => {
                // take_while(list, predicate) -> List<T>
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().take_while(|_item| (");
                self.emit_expr(&args[1]);
                self.push(")(_item.clone())).collect::<Vec<_>>()");
                true
            }
            "drop_while" if args.len() == 2 => {
                // drop_while(list, predicate) -> List<T>
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().skip_while(|_item| (");
                self.emit_expr(&args[1]);
                self.push(")(_item.clone())).collect::<Vec<_>>()");
                true
            }
            "split_at" if args.len() == 2 => {
                // split_at(list, n) -> (List<T>, List<T>)
                self.push("{ let _v = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); let _n = ");
                self.emit_expr(&args[1]);
                self.push(" as usize; let (_a, _b) = _v.split_at(std::cmp::min(_n, _v.len())); (_a.to_vec(), _b.to_vec()) }");
                true
            }

            // Transformations
            "scan" if args.len() == 3 => {
                // scan(list, init, f) -> List<T> — like fold but collects intermediates
                self.push("{ let mut _acc = ");
                self.emit_expr(&args[1]);
                self.push("; ");
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().map(|_item| { _acc = (");
                self.emit_expr(&args[2]);
                self.push(")(_acc.clone(), _item); _acc.clone() }).collect::<Vec<_>>() }");
                true
            }
            "reduce" if args.len() == 2 => {
                // reduce(list, f) -> Option<T> — fold without initial value
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().reduce(|_acc, _item| (");
                self.emit_expr(&args[1]);
                self.push(")(_acc, _item))");
                true
            }
            "partition" if args.len() == 2 => {
                // partition(list, predicate) -> (List<T>, List<T>)
                self.push("{ let (_t, _f): (Vec<_>, Vec<_>) = ");
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().partition(|_item| (");
                self.emit_expr(&args[1]);
                self.push(")(_item.clone())); (_t, _f) }");
                true
            }
            "group_by" if args.len() == 2 => {
                // group_by(list, key_fn) -> List<(K, List<V>)>
                self.push("{ let mut _map: std::collections::HashMap<_, Vec<_>> = std::collections::HashMap::new(); for _item in ");
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter() { let _k = (");
                self.emit_expr(&args[1]);
                self.push(")(_item.clone()); _map.entry(_k).or_default().push(_item); } let mut _pairs: Vec<_> = _map.into_iter().collect(); _pairs.sort_by(|a, b| format!(\"{:?}\", a.0).cmp(&format!(\"{:?}\", b.0))); _pairs }");
                true
            }
            "unique" if args.len() == 1 => {
                // unique(list) -> List<T> (preserves order, removes duplicates)
                self.push("{ let mut _seen = std::collections::HashSet::new(); ");
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().filter(|_item| { let _key = format!(\"{:?}\", _item); _seen.insert(_key) }).collect::<Vec<_>>() }");
                true
            }
            "intersperse" if args.len() == 2 => {
                // intersperse(list, separator) -> List<T>
                self.push("{ let _v = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); let _sep = ");
                self.emit_expr(&args[1]);
                self.push("; let mut _out = Vec::new(); for (i, _item) in _v.into_iter().enumerate() { if i > 0 { _out.push(_sep.clone()); } _out.push(_item); } _out }");
                true
            }

            // Aggregations
            "min_of" if args.len() == 1 => {
                // min_of(list) -> Option<T>
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().min()");
                true
            }
            "max_of" if args.len() == 1 => {
                // max_of(list) -> Option<T>
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().max()");
                true
            }
            "sum_float" if args.len() == 1 => {
                // sum_float(list) -> Float
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().sum::<f64>()");
                true
            }
            "product_float" if args.len() == 1 => {
                // product_float(list) -> Float
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().product::<f64>()");
                true
            }

            // Zip utilities
            "unzip" if args.len() == 1 => {
                // unzip(list_of_pairs) -> (List<A>, List<B>)
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().unzip::<_, _, Vec<_>, Vec<_>>()");
                true
            }
            "zip_with" if args.len() == 3 => {
                // zip_with(list1, list2, f) -> List<C>
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().zip(");
                self.emit_expr(&args[1]);
                self.push(".clone().into_iter()).map(|(_a, _b)| (");
                self.emit_expr(&args[2]);
                self.push(")(_a, _b)).collect::<Vec<_>>()");
                true
            }

            // ── String operations ───────────────────────────────────────
            "to_string" if args.len() == 1 => {
                self.push("format!(\"{}\", star_display(&(");
                self.emit_expr(&args[0]);
                self.push(")))");
                true
            }
            "trim" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".trim().to_string()");
                true
            }
            "split" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".split(&*");
                self.emit_expr(&args[1]);
                self.push(").map(|s| s.to_string()).collect::<Vec<_>>()");
                true
            }
            "join" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".join(&*");
                self.emit_expr(&args[1]);
                self.push(")");
                true
            }
            "contains" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".contains(&*");
                self.emit_expr(&args[1]);
                self.push(")");
                true
            }
            "replace" if args.len() == 3 => {
                self.emit_expr(&args[0]);
                self.push(".replace(&*");
                self.emit_expr(&args[1]);
                self.push(", &*");
                self.emit_expr(&args[2]);
                self.push(")");
                true
            }
            "uppercase" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".to_uppercase()");
                true
            }
            "lowercase" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".to_lowercase()");
                true
            }
            "starts_with" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".starts_with(&*");
                self.emit_expr(&args[1]);
                self.push(")");
                true
            }
            "ends_with" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".ends_with(&*");
                self.emit_expr(&args[1]);
                self.push(")");
                true
            }
            "chars" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".chars().map(|c| c.to_string()).collect::<Vec<_>>()");
                true
            }
            "char_at" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".chars().nth(");
                self.emit_expr(&args[1]);
                self.push(" as usize)");
                true
            }
            "trim_start" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".trim_start().to_string()");
                true
            }
            "trim_end" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".trim_end().to_string()");
                true
            }
            "substring" if args.len() == 3 => {
                // substring(s, start, end) — character-based slice
                self.push("{ let _s: Vec<char> = ");
                self.emit_expr(&args[0]);
                self.push(".chars().collect(); _s[");
                self.emit_expr(&args[1]);
                self.push(" as usize..");
                self.emit_expr(&args[2]);
                self.push(" as usize].iter().collect::<String>() }");
                true
            }
            "substring" if args.len() == 2 => {
                // substring(s, start) — from start to end
                self.push("{ let _s: Vec<char> = ");
                self.emit_expr(&args[0]);
                self.push(".chars().collect(); _s[");
                self.emit_expr(&args[1]);
                self.push(" as usize..].iter().collect::<String>() }");
                true
            }
            "index_of" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".find(&*");
                self.emit_expr(&args[1]);
                self.push(").map(|i| i as i64)");
                true
            }
            "last_index_of" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".rfind(&*");
                self.emit_expr(&args[1]);
                self.push(").map(|i| i as i64)");
                true
            }
            "replace_first" if args.len() == 3 => {
                self.emit_expr(&args[0]);
                self.push(".replacen(&*");
                self.emit_expr(&args[1]);
                self.push(", &*");
                self.emit_expr(&args[2]);
                self.push(", 1)");
                true
            }
            "capitalize" if args.len() == 1 => {
                self.push("{ let _s = ");
                self.emit_expr(&args[0]);
                self.push("; let mut _c = _s.chars(); match _c.next() { None => String::new(), Some(f) => f.to_uppercase().to_string() + _c.as_str() } }");
                true
            }
            "pad_left" if args.len() == 3 => {
                // pad_left(s, width, fill_char)
                self.push("{ let _s = ");
                self.emit_expr(&args[0]);
                self.push("; let _w = ");
                self.emit_expr(&args[1]);
                self.push(" as usize; let _f = ");
                self.emit_expr(&args[2]);
                self.push(".chars().next().unwrap_or(' '); if _s.len() >= _w { _s } else { format!(\"{}{}\", std::iter::repeat(_f).take(_w - _s.len()).collect::<String>(), _s) } }");
                true
            }
            "pad_left" if args.len() == 2 => {
                // pad_left(s, width) — default space
                self.push("{ let _s = ");
                self.emit_expr(&args[0]);
                self.push("; let _w = ");
                self.emit_expr(&args[1]);
                self.push(" as usize; if _s.len() >= _w { _s } else { format!(\"{}{}\", \" \".repeat(_w - _s.len()), _s) } }");
                true
            }
            "pad_right" if args.len() == 3 => {
                self.push("{ let _s = ");
                self.emit_expr(&args[0]);
                self.push("; let _w = ");
                self.emit_expr(&args[1]);
                self.push(" as usize; let _f = ");
                self.emit_expr(&args[2]);
                self.push(".chars().next().unwrap_or(' '); if _s.len() >= _w { _s } else { format!(\"{}{}\", _s, std::iter::repeat(_f).take(_w - _s.len()).collect::<String>()) } }");
                true
            }
            "pad_right" if args.len() == 2 => {
                self.push("{ let _s = ");
                self.emit_expr(&args[0]);
                self.push("; let _w = ");
                self.emit_expr(&args[1]);
                self.push(" as usize; if _s.len() >= _w { _s } else { format!(\"{}{}\", _s, \" \".repeat(_w - _s.len())) } }");
                true
            }
            "repeat" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".repeat(");
                self.emit_expr(&args[1]);
                self.push(" as usize)");
                true
            }
            "is_empty" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".is_empty()");
                true
            }
            "is_blank" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".trim().is_empty()");
                true
            }
            "reverse_string" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".chars().rev().collect::<String>()");
                true
            }
            "lines" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".lines().map(|s| s.to_string()).collect::<Vec<_>>()");
                true
            }
            "words" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".split_whitespace().map(|s| s.to_string()).collect::<Vec<_>>()");
                true
            }
            "strip_prefix" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".strip_prefix(&*");
                self.emit_expr(&args[1]);
                self.push(").map(|s| s.to_string())");
                true
            }
            "strip_suffix" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".strip_suffix(&*");
                self.emit_expr(&args[1]);
                self.push(").map(|s| s.to_string())");
                true
            }
            "is_numeric" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".chars().all(|c| c.is_ascii_digit())");
                true
            }
            "is_alphabetic" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".chars().all(|c| c.is_alphabetic())");
                true
            }
            "is_alphanumeric" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".chars().all(|c| c.is_alphanumeric())");
                true
            }

            // ── Regex operations ─────────────────────────────────────────
            // These require the `regex` crate in the generated Cargo.toml
            "regex_match" if args.len() == 2 => {
                // regex_match(string, pattern) -> bool
                self.push("regex::Regex::new(&");
                self.emit_expr(&args[1]);
                self.push(").unwrap().is_match(&");
                self.emit_expr(&args[0]);
                self.push(")");
                true
            }
            "regex_find" if args.len() == 2 => {
                // regex_find(string, pattern) -> Option<String>
                self.push("regex::Regex::new(&");
                self.emit_expr(&args[1]);
                self.push(").unwrap().find(&");
                self.emit_expr(&args[0]);
                self.push(").map(|m| m.as_str().to_string())");
                true
            }
            "regex_find_all" if args.len() == 2 => {
                // regex_find_all(string, pattern) -> List<String>
                self.push("regex::Regex::new(&");
                self.emit_expr(&args[1]);
                self.push(").unwrap().find_iter(&");
                self.emit_expr(&args[0]);
                self.push(").map(|m| m.as_str().to_string()).collect::<Vec<_>>()");
                true
            }
            "regex_replace" if args.len() == 3 => {
                // regex_replace(string, pattern, replacement) -> String
                self.push("regex::Regex::new(&");
                self.emit_expr(&args[1]);
                self.push(").unwrap().replace_all(&");
                self.emit_expr(&args[0]);
                self.push(", ");
                self.emit_expr(&args[2]);
                self.push(".as_str()).to_string()");
                true
            }

            // ── Encoding operations ──────────────────────────────────────
            "bytes" if args.len() == 1 => {
                // bytes(string) -> List<Int>  (UTF-8 bytes)
                self.emit_expr(&args[0]);
                self.push(".bytes().map(|b| b as i64).collect::<Vec<_>>()");
                true
            }
            "from_bytes" if args.len() == 1 => {
                // from_bytes(list) -> String
                self.push("String::from_utf8(");
                self.emit_expr(&args[0]);
                self.push(".iter().map(|&b| b as u8).collect()).unwrap_or_default()");
                true
            }
            "encode_base64" if args.len() == 1 => {
                // Uses the base64 crate via general_purpose engine
                self.push("{ use base64::Engine; base64::engine::general_purpose::STANDARD.encode(");
                self.emit_expr(&args[0]);
                self.push(".as_bytes()) }");
                true
            }
            "decode_base64" if args.len() == 1 => {
                self.push("{ use base64::Engine; base64::engine::general_purpose::STANDARD.decode(");
                self.emit_expr(&args[0]);
                self.push(".as_bytes()).ok().and_then(|b| String::from_utf8(b).ok()) }");
                true
            }
            "char_code" if args.len() == 1 => {
                // char_code(string) -> Int  (first char's Unicode code point)
                self.emit_expr(&args[0]);
                self.push(".chars().next().map(|c| c as i64).unwrap_or(0)");
                true
            }
            "from_char_code" if args.len() == 1 => {
                // from_char_code(int) -> String
                self.push("String::from(char::from_u32(");
                self.emit_expr(&args[0]);
                self.push(" as u32).unwrap_or('\\0'))");
                true
            }
            "format" if args.len() >= 2 => {
                // format(fmt_string, args...) — Rust format! pass-through
                self.push("format!(");
                self.emit_expr(&args[0]);
                for arg in &args[1..] {
                    self.push(", ");
                    self.emit_expr(arg);
                }
                self.push(")");
                true
            }

            // ── Cryptography & security ─────────────────────────────────
            "sha256" if args.len() == 1 => {
                // sha256(data) -> String (hex digest) — pure inline implementation
                self.push("{ fn _star_sha256(data: &[u8]) -> String { ");
                self.push("let k: [u32; 64] = [0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2]; ");
                self.push("let mut h: [u32; 8] = [0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19]; ");
                self.push("let bit_len = (data.len() as u64) * 8; let mut msg = data.to_vec(); msg.push(0x80); while (msg.len() % 64) != 56 { msg.push(0); } msg.extend_from_slice(&bit_len.to_be_bytes()); ");
                self.push("for chunk in msg.chunks(64) { let mut w = [0u32; 64]; for i in 0..16 { w[i] = u32::from_be_bytes([chunk[4*i], chunk[4*i+1], chunk[4*i+2], chunk[4*i+3]]); } ");
                self.push("for i in 16..64 { let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3); let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10); w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1); } ");
                self.push("let (mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut hh) = (h[0],h[1],h[2],h[3],h[4],h[5],h[6],h[7]); ");
                self.push("for i in 0..64 { let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25); let ch = (e & f) ^ ((!e) & g); let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(k[i]).wrapping_add(w[i]); let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22); let maj = (a & b) ^ (a & c) ^ (b & c); let t2 = s0.wrapping_add(maj); hh=g; g=f; f=e; e=d.wrapping_add(t1); d=c; c=b; b=a; a=t1.wrapping_add(t2); } ");
                self.push("h[0]=h[0].wrapping_add(a); h[1]=h[1].wrapping_add(b); h[2]=h[2].wrapping_add(c); h[3]=h[3].wrapping_add(d); h[4]=h[4].wrapping_add(e); h[5]=h[5].wrapping_add(f); h[6]=h[6].wrapping_add(g); h[7]=h[7].wrapping_add(hh); } ");
                self.push("h.iter().map(|x| format!(\"{:08x}\", x)).collect::<Vec<_>>().join(\"\") } ");
                self.push("_star_sha256(");
                self.emit_expr(&args[0]);
                self.push(".as_bytes()) }");
                true
            }
            "sha512" if args.len() == 1 => {
                // sha512(data) -> String (hex digest) — pure inline implementation
                self.push("{ fn _star_sha512(data: &[u8]) -> String { ");
                self.push("let k: [u64; 80] = [0x428a2f98d728ae22,0x7137449123ef65cd,0xb5c0fbcfec4d3b2f,0xe9b5dba58189dbbc,0x3956c25bf348b538,0x59f111f1b605d019,0x923f82a4af194f9b,0xab1c5ed5da6d8118,0xd807aa98a3030242,0x12835b0145706fbe,0x243185be4ee4b28c,0x550c7dc3d5ffb4e2,0x72be5d74f27b896f,0x80deb1fe3b1696b1,0x9bdc06a725c71235,0xc19bf174cf692694,0xe49b69c19ef14ad2,0xefbe4786384f25e3,0x0fc19dc68b8cd5b5,0x240ca1cc77ac9c65,0x2de92c6f592b0275,0x4a7484aa6ea6e483,0x5cb0a9dcbd41fbd4,0x76f988da831153b5,0x983e5152ee66dfab,0xa831c66d2db43210,0xb00327c898fb213f,0xbf597fc7beef0ee4,0xc6e00bf33da88fc2,0xd5a79147930aa725,0x06ca6351e003826f,0x142929670a0e6e70,0x27b70a8546d22ffc,0x2e1b21385c26c926,0x4d2c6dfc5ac42aed,0x53380d139d95b3df,0x650a73548baf63de,0x766a0abb3c77b2a8,0x81c2c92e47edaee6,0x92722c851482353b,0xa2bfe8a14cf10364,0xa81a664bbc423001,0xc24b8b70d0f89791,0xc76c51a30654be30,0xd192e819d6ef5218,0xd69906245565a910,0xf40e35855771202a,0x106aa07032bbd1b8,0x19a4c116b8d2d0c8,0x1e376c085141ab53,0x2748774cdf8eeb99,0x34b0bcb5e19b48a8,0x391c0cb3c5c95a63,0x4ed8aa4ae3418acb,0x5b9cca4f7763e373,0x682e6ff3d6b2b8a3,0x748f82ee5defb2fc,0x78a5636f43172f60,0x84c87814a1f0ab72,0x8cc702081a6439ec,0x90befffa23631e28,0xa4506cebde82bde9,0xbef9a3f7b2c67915,0xc67178f2e372532b,0xca273eceea26619c,0xd186b8c721c0c207,0xeada7dd6cde0eb1e,0xf57d4f7fee6ed178,0x06f067aa72176fba,0x0a637dc5a2c898a6,0x113f9804bef90dae,0x1b710b35131c471b,0x28db77f523047d84,0x32caab7b40c72493,0x3c9ebe0a15c9bebc,0x431d67c49c100d4c,0x4cc5d4becb3e42b6,0x597f299cfc657e2a,0x5fcb6fab3ad6faec,0x6c44198c4a475817]; ");
                self.push("let mut h: [u64; 8] = [0x6a09e667f3bcc908,0xbb67ae8584caa73b,0x3c6ef372fe94f82b,0xa54ff53a5f1d36f1,0x510e527fade682d1,0x9b05688c2b3e6c1f,0x1f83d9abfb41bd6b,0x5be0cd19137e2179]; ");
                self.push("let bit_len = (data.len() as u128) * 8; let mut msg = data.to_vec(); msg.push(0x80); while (msg.len() % 128) != 112 { msg.push(0); } msg.extend_from_slice(&bit_len.to_be_bytes()); ");
                self.push("for chunk in msg.chunks(128) { let mut w = [0u64; 80]; for i in 0..16 { let off = 8*i; w[i] = u64::from_be_bytes([chunk[off],chunk[off+1],chunk[off+2],chunk[off+3],chunk[off+4],chunk[off+5],chunk[off+6],chunk[off+7]]); } ");
                self.push("for i in 16..80 { let s0 = w[i-15].rotate_right(1) ^ w[i-15].rotate_right(8) ^ (w[i-15] >> 7); let s1 = w[i-2].rotate_right(19) ^ w[i-2].rotate_right(61) ^ (w[i-2] >> 6); w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1); } ");
                self.push("let (mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut hh) = (h[0],h[1],h[2],h[3],h[4],h[5],h[6],h[7]); ");
                self.push("for i in 0..80 { let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41); let ch = (e & f) ^ ((!e) & g); let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(k[i]).wrapping_add(w[i]); let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39); let maj = (a & b) ^ (a & c) ^ (b & c); let t2 = s0.wrapping_add(maj); hh=g; g=f; f=e; e=d.wrapping_add(t1); d=c; c=b; b=a; a=t1.wrapping_add(t2); } ");
                self.push("h[0]=h[0].wrapping_add(a); h[1]=h[1].wrapping_add(b); h[2]=h[2].wrapping_add(c); h[3]=h[3].wrapping_add(d); h[4]=h[4].wrapping_add(e); h[5]=h[5].wrapping_add(f); h[6]=h[6].wrapping_add(g); h[7]=h[7].wrapping_add(hh); } ");
                self.push("h.iter().map(|x| format!(\"{:016x}\", x)).collect::<Vec<_>>().join(\"\") } ");
                self.push("_star_sha512(");
                self.emit_expr(&args[0]);
                self.push(".as_bytes()) }");
                true
            }
            "md5" if args.len() == 1 => {
                // md5(data) -> String (hex digest) — pure inline implementation
                self.push("{ fn _star_md5(data: &[u8]) -> String { ");
                self.push("let s: [u32; 64] = [7,12,17,22,7,12,17,22,7,12,17,22,7,12,17,22,5,9,14,20,5,9,14,20,5,9,14,20,5,9,14,20,4,11,16,23,4,11,16,23,4,11,16,23,4,11,16,23,6,10,15,21,6,10,15,21,6,10,15,21,6,10,15,21]; ");
                self.push("let k: [u32; 64] = [0xd76aa478,0xe8c7b756,0x242070db,0xc1bdceee,0xf57c0faf,0x4787c62a,0xa8304613,0xfd469501,0x698098d8,0x8b44f7af,0xffff5bb1,0x895cd7be,0x6b901122,0xfd987193,0xa679438e,0x49b40821,0xf61e2562,0xc040b340,0x265e5a51,0xe9b6c7aa,0xd62f105d,0x02441453,0xd8a1e681,0xe7d3fbc8,0x21e1cde6,0xc33707d6,0xf4d50d87,0x455a14ed,0xa9e3e905,0xfcefa3f8,0x676f02d9,0x8d2a4c8a,0xfffa3942,0x8771f681,0x6d9d6122,0xfde5380c,0xa4beea44,0x4bdecfa9,0xf6bb4b60,0xbebfbc70,0x289b7ec6,0xeaa127fa,0xd4ef3085,0x04881d05,0xd9d4d039,0xe6db99e5,0x1fa27cf8,0xc4ac5665,0xf4292244,0x432aff97,0xab9423a7,0xfc93a039,0x655b59c3,0x8f0ccc92,0xffeff47d,0x85845dd1,0x6fa87e4f,0xfe2ce6e0,0xa3014314,0x4e0811a1,0xf7537e82,0xbd3af235,0x2ad7d2bb,0xeb86d391]; ");
                self.push("let (mut a0,mut b0,mut c0,mut d0): (u32,u32,u32,u32) = (0x67452301,0xefcdab89,0x98badcfe,0x10325476); ");
                self.push("let bit_len = (data.len() as u64) * 8; let mut msg = data.to_vec(); msg.push(0x80); while (msg.len() % 64) != 56 { msg.push(0); } msg.extend_from_slice(&bit_len.to_le_bytes()); ");
                self.push("for chunk in msg.chunks(64) { let mut m = [0u32; 16]; for i in 0..16 { m[i] = u32::from_le_bytes([chunk[4*i],chunk[4*i+1],chunk[4*i+2],chunk[4*i+3]]); } ");
                self.push("let (mut a,mut b,mut c,mut d) = (a0,b0,c0,d0); ");
                self.push("for i in 0..64 { let (f, g) = match i { 0..=15 => ((b & c) | ((!b) & d), i), 16..=31 => ((d & b) | ((!d) & c), (5*i+1)%16), 32..=47 => (b ^ c ^ d, (3*i+5)%16), _ => (c ^ (b | (!d)), (7*i)%16) }; ");
                self.push("let f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]); a=d; d=c; c=b; b=b.wrapping_add(f.rotate_left(s[i])); } ");
                self.push("a0=a0.wrapping_add(a); b0=b0.wrapping_add(b); c0=c0.wrapping_add(c); d0=d0.wrapping_add(d); } ");
                self.push("[a0,b0,c0,d0].iter().map(|x| { let b = x.to_le_bytes(); format!(\"{:02x}{:02x}{:02x}{:02x}\", b[0], b[1], b[2], b[3]) }).collect::<Vec<_>>().join(\"\") } ");
                self.push("_star_md5(");
                self.emit_expr(&args[0]);
                self.push(".as_bytes()) }");
                true
            }
            "hash_bytes" if args.len() == 1 => {
                // hash_bytes(data) -> String (hex string using DefaultHasher, no crate needed)
                self.push("{ use std::hash::{Hash, Hasher}; let mut _h = std::collections::hash_map::DefaultHasher::new(); ");
                self.emit_expr(&args[0]);
                self.push(".hash(&mut _h); format!(\"{:016x}\", _h.finish()) }");
                true
            }
            "secure_random_bytes" if args.len() == 1 => {
                // secure_random_bytes(n) -> List<Int> (cryptographically secure)
                self.push("{ let _n = ");
                self.emit_expr(&args[0]);
                self.push(" as usize; let mut _buf = vec![0u8; _n]; ");
                self.push("std::io::Read::read_exact(&mut std::fs::File::open(\"/dev/urandom\").expect(\"cannot open /dev/urandom\"), &mut _buf).expect(\"cannot read /dev/urandom\"); ");
                self.push("_buf.into_iter().map(|b| b as i64).collect::<Vec<i64>>() }");
                true
            }
            "secure_random_hex" if args.len() == 1 => {
                // secure_random_hex(n) -> String (n random bytes as hex)
                self.push("{ let _n = ");
                self.emit_expr(&args[0]);
                self.push(" as usize; let mut _buf = vec![0u8; _n]; ");
                self.push("std::io::Read::read_exact(&mut std::fs::File::open(\"/dev/urandom\").expect(\"cannot open /dev/urandom\"), &mut _buf).expect(\"cannot read /dev/urandom\"); ");
                self.push("_buf.iter().map(|b| format!(\"{:02x}\", b)).collect::<Vec<_>>().join(\"\") }");
                true
            }
            "uuid_v4" if args.is_empty() => {
                // uuid_v4() -> String (random UUID v4)
                self.push("{ let mut _buf = [0u8; 16]; ");
                self.push("std::io::Read::read_exact(&mut std::fs::File::open(\"/dev/urandom\").expect(\"cannot open /dev/urandom\"), &mut _buf).expect(\"cannot read /dev/urandom\"); ");
                self.push("_buf[6] = (_buf[6] & 0x0f) | 0x40; _buf[8] = (_buf[8] & 0x3f) | 0x80; ");
                self.push("format!(\"{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}\", ");
                self.push("_buf[0], _buf[1], _buf[2], _buf[3], _buf[4], _buf[5], _buf[6], _buf[7], ");
                self.push("_buf[8], _buf[9], _buf[10], _buf[11], _buf[12], _buf[13], _buf[14], _buf[15]) }");
                true
            }

            // ── Constructors / ranges ───────────────────────────────────
            "range" if args.len() == 2 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push("..");
                self.emit_expr(&args[1]);
                self.push(").collect::<Vec<_>>()");
                true
            }
            "range_inclusive" if args.len() == 2 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push("..=");
                self.emit_expr(&args[1]);
                self.push(").collect::<Vec<_>>()");
                true
            }

            // ── Math ────────────────────────────────────────────────────
            "abs" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as i64).abs()");
                true
            }
            "min" if args.len() == 2 => {
                self.push("std::cmp::min(");
                self.emit_expr(&args[0]);
                self.push(", ");
                self.emit_expr(&args[1]);
                self.push(")");
                true
            }
            "max" if args.len() == 2 => {
                self.push("std::cmp::max(");
                self.emit_expr(&args[0]);
                self.push(", ");
                self.emit_expr(&args[1]);
                self.push(")");
                true
            }
            "pow" if args.len() == 2 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).powf(");
                self.emit_expr(&args[1]);
                self.push(" as f64)");
                true
            }
            "sqrt" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).sqrt()");
                true
            }
            "clamp" if args.len() == 3 => {
                self.emit_expr(&args[0]);
                self.push(".clamp(");
                self.emit_expr(&args[1]);
                self.push(", ");
                self.emit_expr(&args[2]);
                self.push(")");
                true
            }

            // ── Trigonometry ─────────────────────────────────────────────
            "sin" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).sin()");
                true
            }
            "cos" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).cos()");
                true
            }
            "tan" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).tan()");
                true
            }
            "asin" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).asin()");
                true
            }
            "acos" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).acos()");
                true
            }
            "atan" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).atan()");
                true
            }
            "atan2" if args.len() == 2 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).atan2(");
                self.emit_expr(&args[1]);
                self.push(" as f64)");
                true
            }

            // ── Rounding ─────────────────────────────────────────────────
            "floor" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).floor()");
                true
            }
            "ceil" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).ceil()");
                true
            }
            "round" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).round()");
                true
            }
            "truncate" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).trunc()");
                true
            }

            // ── Logarithms & Exponentials ────────────────────────────────
            "log" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).ln()");
                true
            }
            "log2" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).log2()");
                true
            }
            "log10" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).log10()");
                true
            }
            "exp" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).exp()");
                true
            }
            "exp2" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).exp2()");
                true
            }

            // ── Misc math ────────────────────────────────────────────────
            "signum" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).signum()");
                true
            }
            "hypot" if args.len() == 2 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).hypot(");
                self.emit_expr(&args[1]);
                self.push(" as f64)");
                true
            }
            "cbrt" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).cbrt()");
                true
            }

            // ── Constants ────────────────────────────────────────────────
            "pi" if args.is_empty() => {
                self.push("std::f64::consts::PI");
                true
            }
            "e_const" if args.is_empty() => {
                self.push("std::f64::consts::E");
                true
            }
            "infinity" if args.is_empty() => {
                self.push("f64::INFINITY");
                true
            }
            "neg_infinity" if args.is_empty() => {
                self.push("f64::NEG_INFINITY");
                true
            }
            "nan" if args.is_empty() => {
                self.push("f64::NAN");
                true
            }

            // ── Float predicates ─────────────────────────────────────────
            "is_nan" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).is_nan()");
                true
            }
            "is_infinite" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).is_infinite()");
                true
            }
            "is_finite" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).is_finite()");
                true
            }

            // ── Angle conversion ─────────────────────────────────────────
            "to_radians" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).to_radians()");
                true
            }
            "to_degrees" if args.len() == 1 => {
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(" as f64).to_degrees()");
                true
            }

            // ── Random ───────────────────────────────────────────────────
            "random" if args.is_empty() => {
                // random() -> Int — random i64 using simple LCG (no external crate)
                self.push("{ use std::time::SystemTime; let _seed = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().subsec_nanos() as i64; ((_seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)) % 1000000).abs() }");
                true
            }
            "random_range" if args.len() == 2 => {
                // random_range(min, max) -> Int — random integer in [min, max)
                self.push("{ use std::time::SystemTime; let _seed = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().subsec_nanos() as i64; let _min = ");
                self.emit_expr(&args[0]);
                self.push("; let _max = ");
                self.emit_expr(&args[1]);
                self.push("; _min + ((_seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)) % (_max - _min)).abs() }");
                true
            }
            "random_float" if args.is_empty() => {
                // random_float() -> Float — random f64 in [0.0, 1.0)
                self.push("{ use std::time::SystemTime; let _seed = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().subsec_nanos() as f64; ((_seed * 6364136223846793005.0) % 1.0).abs() }");
                true
            }

            // ── Integer math ─────────────────────────────────────────────
            "gcd" if args.len() == 2 => {
                // gcd(a, b) -> Int — greatest common divisor
                self.push("{ let mut _a = (");
                self.emit_expr(&args[0]);
                self.push(" as i64).abs(); let mut _b = (");
                self.emit_expr(&args[1]);
                self.push(" as i64).abs(); while _b != 0 { let _t = _b; _b = _a % _b; _a = _t; } _a }");
                true
            }
            "lcm" if args.len() == 2 => {
                // lcm(a, b) -> Int — least common multiple
                self.push("{ let _a = (");
                self.emit_expr(&args[0]);
                self.push(" as i64).abs(); let _b = (");
                self.emit_expr(&args[1]);
                self.push(" as i64).abs(); if _a == 0 || _b == 0 { 0 } else { (_a / { let mut _x = _a; let mut _y = _b; while _y != 0 { let _t = _y; _y = _x % _y; _x = _t; } _x }) * _b } }");
                true
            }

            // ── Date & Time ──────────────────────────────────────────────
            "now" if args.is_empty() => {
                // now() -> Int — current Unix timestamp in seconds
                self.push("std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64");
                true
            }
            "now_ms" if args.is_empty() => {
                // now_ms() -> Int — current Unix timestamp in milliseconds
                self.push("std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64");
                true
            }
            "now_ns" if args.is_empty() => {
                // now_ns() -> Int — current Unix timestamp in nanoseconds
                self.push("std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as i64");
                true
            }
            "elapsed" if args.len() == 1 => {
                // elapsed(start_instant) -> Float — seconds elapsed since Instant
                self.emit_expr(&args[0]);
                self.push(".elapsed().as_secs_f64()");
                true
            }
            "elapsed_ms" if args.len() == 1 => {
                // elapsed_ms(start_instant) -> Int — milliseconds elapsed since Instant
                self.emit_expr(&args[0]);
                self.push(".elapsed().as_millis() as i64");
                true
            }
            "monotonic" if args.is_empty() => {
                // monotonic() -> Instant — monotonic clock for measuring durations
                self.push("std::time::Instant::now()");
                true
            }
            "monotonic_elapsed_ms" if args.len() == 2 => {
                // monotonic_elapsed_ms(start, end) -> Int — duration between two instants in ms
                self.emit_expr(&args[1]);
                self.push(".duration_since(");
                self.emit_expr(&args[0]);
                self.push(").as_millis() as i64");
                true
            }
            "timestamp_secs" if args.len() == 1 => {
                // timestamp_secs(secs) -> SystemTime — create SystemTime from Unix seconds
                self.push("(std::time::UNIX_EPOCH + std::time::Duration::from_secs(");
                self.emit_expr(&args[0]);
                self.push(" as u64))");
                true
            }
            "timestamp_millis" if args.len() == 1 => {
                // timestamp_millis(ms) -> SystemTime — create SystemTime from Unix millis
                self.push("(std::time::UNIX_EPOCH + std::time::Duration::from_millis(");
                self.emit_expr(&args[0]);
                self.push(" as u64))");
                true
            }
            "format_timestamp" if args.len() == 1 => {
                // format_timestamp(unix_secs) -> String — basic UTC date/time string
                self.push("{ let _ts = ");
                self.emit_expr(&args[0]);
                self.push(" as i64; let _s = _ts % 60; let _m = (_ts / 60) % 60; let _h = (_ts / 3600) % 24; let mut _days = _ts / 86400; let mut _y = 1970i64; loop { let _leap = _y % 4 == 0 && (_y % 100 != 0 || _y % 400 == 0); let _diy = if _leap { 366 } else { 365 }; if _days < _diy { break; } _days -= _diy; _y += 1; } let _leap = _y % 4 == 0 && (_y % 100 != 0 || _y % 400 == 0); let _mdays = [31, if _leap {29} else {28}, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]; let mut _mo = 0usize; while _mo < 12 && _days >= _mdays[_mo] { _days -= _mdays[_mo]; _mo += 1; } format!(\"{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z\", _y, _mo + 1, _days + 1, _h, _m, _s) }");
                true
            }
            "parse_timestamp" if args.len() == 1 => {
                // parse_timestamp(iso_string) -> Result<Int, String> — parse ISO 8601 to Unix secs
                self.push("{ let _s: &str = &");
                self.emit_expr(&args[0]);
                self.push("; (|| -> Result<i64, String> { let _s = _s.trim_end_matches('Z'); let _parts: Vec<&str> = _s.split('T').collect(); if _parts.len() != 2 { return Err(\"invalid format\".to_string()); } let _dp: Vec<i64> = _parts[0].split('-').map(|p| p.parse::<i64>().map_err(|e| e.to_string())).collect::<Result<Vec<_>,_>>()?; let _tp: Vec<i64> = _parts[1].split(':').map(|p| p.parse::<i64>().map_err(|e| e.to_string())).collect::<Result<Vec<_>,_>>()?; if _dp.len() != 3 || _tp.len() != 3 { return Err(\"invalid format\".to_string()); } let (_y, _mo, _d) = (_dp[0], _dp[1], _dp[2]); let (_h, _mi, _sec) = (_tp[0], _tp[1], _tp[2]); let mut _days: i64 = 0; for _yr in 1970.._y { let _leap = _yr % 4 == 0 && (_yr % 100 != 0 || _yr % 400 == 0); _days += if _leap { 366 } else { 365 }; } let _leap = _y % 4 == 0 && (_y % 100 != 0 || _y % 400 == 0); let _mdays = [31, if _leap {29} else {28}, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]; for _m in 0..(_mo - 1) as usize { _days += _mdays[_m]; } _days += _d - 1; Ok(_days * 86400 + _h * 3600 + _mi * 60 + _sec) })() }");
                true
            }
            "duration_secs" if args.len() == 1 => {
                // duration_secs(secs) -> Duration
                self.push("std::time::Duration::from_secs(");
                self.emit_expr(&args[0]);
                self.push(" as u64)");
                true
            }
            "duration_ms" if args.len() == 1 => {
                // duration_ms(ms) -> Duration
                self.push("std::time::Duration::from_millis(");
                self.emit_expr(&args[0]);
                self.push(" as u64)");
                true
            }
            "sleep_secs" if args.len() == 1 => {
                // sleep_secs(secs) — blocking thread sleep
                self.push("std::thread::sleep(std::time::Duration::from_secs(");
                self.emit_expr(&args[0]);
                self.push(" as u64))");
                true
            }
            "sleep_millis" if args.len() == 1 => {
                // sleep_millis(ms) — blocking thread sleep
                self.push("std::thread::sleep(std::time::Duration::from_millis(");
                self.emit_expr(&args[0]);
                self.push(" as u64))");
                true
            }

            // ── Networking — TCP ─────────────────────────────────────────
            "tcp_connect" if args.len() == 1 => {
                // tcp_connect(addr) -> Result<TcpStream, String>
                self.push("std::net::TcpStream::connect(&*");
                self.emit_expr(&args[0]);
                self.push(").map_err(|e| e.to_string())");
                true
            }
            "tcp_listen" if args.len() == 1 => {
                // tcp_listen(addr) -> Result<TcpListener, String>
                self.push("std::net::TcpListener::bind(&*");
                self.emit_expr(&args[0]);
                self.push(").map_err(|e| e.to_string())");
                true
            }
            "tcp_accept" if args.len() == 1 => {
                // tcp_accept(listener) -> Result<(TcpStream, String), String>
                self.emit_expr(&args[0]);
                self.push(".accept().map(|(s, a)| (s, a.to_string())).map_err(|e| e.to_string())");
                true
            }
            "tcp_read" if args.len() == 2 => {
                // tcp_read(stream, max_bytes) -> Result<String, String>
                self.push("{ use std::io::Read; let mut _buf = vec![0u8; ");
                self.emit_expr(&args[1]);
                self.push(" as usize]; let mut _s = ");
                self.emit_expr(&args[0]);
                self.push(".try_clone().map_err(|e| e.to_string())?; match _s.read(&mut _buf) { Ok(n) => Ok(String::from_utf8_lossy(&_buf[..n]).to_string()), Err(e) => Err(e.to_string()) } }");
                true
            }
            "tcp_write" if args.len() == 2 => {
                // tcp_write(stream, data) -> Result<Int, String>
                self.push("{ use std::io::Write; let mut _s = ");
                self.emit_expr(&args[0]);
                self.push(".try_clone().map_err(|e| e.to_string())?; _s.write(");
                self.emit_expr(&args[1]);
                self.push(".as_bytes()).map(|n| n as i64).map_err(|e| e.to_string()) }");
                true
            }
            "tcp_close" if args.len() == 1 => {
                // tcp_close(stream) — shutdown the connection
                self.emit_expr(&args[0]);
                self.push(".shutdown(std::net::Shutdown::Both).map_err(|e| e.to_string())");
                true
            }
            "tcp_read_line" if args.len() == 1 => {
                // tcp_read_line(stream) -> Result<String, String>
                self.push("{ use std::io::{BufRead, BufReader}; let _s = ");
                self.emit_expr(&args[0]);
                self.push(".try_clone().map_err(|e| e.to_string())?; let mut _r = BufReader::new(_s); let mut _line = String::new(); _r.read_line(&mut _line).map_err(|e| e.to_string())?; Ok(_line.trim_end().to_string()) }");
                true
            }
            "tcp_write_line" if args.len() == 2 => {
                // tcp_write_line(stream, data) -> Result<(), String>
                self.push("{ use std::io::Write; let mut _s = ");
                self.emit_expr(&args[0]);
                self.push(".try_clone().map_err(|e| e.to_string())?; write!(_s, \"{}\\n\", ");
                self.emit_expr(&args[1]);
                self.push(").map_err(|e| e.to_string()) }");
                true
            }
            "tcp_set_timeout" if args.len() == 2 => {
                // tcp_set_timeout(stream, ms) — set read/write timeout
                self.push("{ let _s = &");
                self.emit_expr(&args[0]);
                self.push("; let _d = Some(std::time::Duration::from_millis(");
                self.emit_expr(&args[1]);
                self.push(" as u64)); _s.set_read_timeout(_d).map_err(|e| e.to_string())?; _s.set_write_timeout(_d).map_err(|e| e.to_string()) }");
                true
            }

            // ── Networking — UDP ─────────────────────────────────────────
            "udp_bind" if args.len() == 1 => {
                // udp_bind(addr) -> Result<UdpSocket, String>
                self.push("std::net::UdpSocket::bind(&*");
                self.emit_expr(&args[0]);
                self.push(").map_err(|e| e.to_string())");
                true
            }
            "udp_send_to" if args.len() == 3 => {
                // udp_send_to(socket, data, addr) -> Result<Int, String>
                self.emit_expr(&args[0]);
                self.push(".send_to(");
                self.emit_expr(&args[1]);
                self.push(".as_bytes(), &*");
                self.emit_expr(&args[2]);
                self.push(").map(|n| n as i64).map_err(|e| e.to_string())");
                true
            }
            "udp_recv_from" if args.len() == 2 => {
                // udp_recv_from(socket, max_bytes) -> Result<(String, String), String>
                self.push("{ let mut _buf = vec![0u8; ");
                self.emit_expr(&args[1]);
                self.push(" as usize]; ");
                self.emit_expr(&args[0]);
                self.push(".recv_from(&mut _buf).map(|(n, a)| (String::from_utf8_lossy(&_buf[..n]).to_string(), a.to_string())).map_err(|e| e.to_string()) }");
                true
            }

            // ── Networking — DNS & URL ───────────────────────────────────
            "dns_lookup" if args.len() == 1 => {
                // dns_lookup(host) -> Result<List<String>, String>
                self.push("std::net::ToSocketAddrs::to_socket_addrs(&(&*");
                self.emit_expr(&args[0]);
                self.push(", 0u16)).map(|addrs| addrs.map(|a| a.ip().to_string()).collect::<Vec<_>>()).map_err(|e| e.to_string())");
                true
            }
            "url_parse" if args.len() == 1 => {
                // url_parse(url_string) -> Result<Map, String> — parse URL into components
                self.push("{ let _url: &str = &");
                self.emit_expr(&args[0]);
                self.push("; (|| -> Result<std::collections::HashMap<String, String>, String> { let mut _m = std::collections::HashMap::new(); let (_scheme, _rest) = _url.split_once(\"://\").ok_or(\"missing scheme\")?; _m.insert(\"scheme\".to_string(), _scheme.to_string()); let (_authority, _path) = _rest.split_once('/').unwrap_or((_rest, \"\")); let _path = format!(\"/{}\", _path); let (_path_only, _query) = _path.split_once('?').unwrap_or((&_path, \"\")); _m.insert(\"path\".to_string(), _path_only.to_string()); if !_query.is_empty() { let (_q, _frag) = _query.split_once('#').unwrap_or((_query, \"\")); _m.insert(\"query\".to_string(), _q.to_string()); if !_frag.is_empty() { _m.insert(\"fragment\".to_string(), _frag.to_string()); } } else { let (_p, _frag) = _path_only.split_once('#').unwrap_or((_path_only, \"\")); if !_frag.is_empty() { _m.insert(\"path\".to_string(), _p.to_string()); _m.insert(\"fragment\".to_string(), _frag.to_string()); } } let (_host, _port) = if _authority.contains(':') { let (h, p) = _authority.rsplit_once(':').unwrap(); (h.to_string(), p.to_string()) } else { (_authority.to_string(), String::new()) }; _m.insert(\"host\".to_string(), _host); if !_port.is_empty() { _m.insert(\"port\".to_string(), _port); } Ok(_m) })() }");
                true
            }

            // ── Networking — HTTP (simple, std-only) ─────────────────────
            "http_get" if args.len() == 1 => {
                // http_get(url) -> Result<String, String> — convenience GET
                self.push("{ let _url: &str = &");
                self.emit_expr(&args[0]);
                self.push("; ");
                self.push(Self::HTTP_HELPER);
                self.push(" _star_http(\"GET\", _url, &[], \"\") }");
                true
            }
            "http" if args.len() == 2 => {
                // http(method, url) -> Result<String, String>
                self.push("{ let _method: &str = &");
                self.emit_expr(&args[0]);
                self.push("; let _url: &str = &");
                self.emit_expr(&args[1]);
                self.push("; ");
                self.push(Self::HTTP_HELPER);
                self.push(" _star_http(_method, _url, &[], \"\") }");
                true
            }
            "http" if args.len() == 3 => {
                // http(method, url, body) -> Result<String, String>
                self.push("{ let _method: &str = &");
                self.emit_expr(&args[0]);
                self.push("; let _url: &str = &");
                self.emit_expr(&args[1]);
                self.push("; let _body: &str = &");
                self.emit_expr(&args[2]);
                self.push("; ");
                self.push(Self::HTTP_HELPER);
                self.push(" _star_http(_method, _url, &[], _body) }");
                true
            }
            "http_with_headers" if args.len() == 4 => {
                // http_with_headers(method, url, headers, body) -> Result<String, String>
                // headers is a List of (key, value) strings like ["Content-Type: application/json"]
                self.push("{ let _method: &str = &");
                self.emit_expr(&args[0]);
                self.push("; let _url: &str = &");
                self.emit_expr(&args[1]);
                self.push("; let _hdrs: Vec<String> = ");
                self.emit_expr(&args[2]);
                self.push(".clone(); let _body: &str = &");
                self.emit_expr(&args[3]);
                self.push("; ");
                self.push(Self::HTTP_HELPER);
                self.push(" let _hdr_refs: Vec<&str> = _hdrs.iter().map(|s| s.as_str()).collect(); _star_http(_method, _url, &_hdr_refs, _body) }");
                true
            }

            // ── Conversions ─────────────────────────────────────────────
            "to_int" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".parse::<i64>().unwrap_or(0)");
                true
            }
            "to_float" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".parse::<f64>().unwrap_or(0.0)");
                true
            }

            // ── Utility ─────────────────────────────────────────────────
            "length" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".len() as i64");
                true
            }

            // ── Process ─────────────────────────────────────────────────
            "exit" if args.len() == 1 => {
                self.push("std::process::exit(");
                self.emit_expr(&args[0]);
                self.push(" as i32)");
                true
            }
            "panic" if args.len() == 1 => {
                self.push("panic!(\"{}\", ");
                self.emit_expr(&args[0]);
                self.push(")");
                true
            }

            // ── Testing & debugging — assertions ────────────────────────
            "assert" if args.len() == 1 => {
                self.push("assert!(");
                self.emit_expr(&args[0]);
                self.push(")");
                true
            }
            "assert_msg" if args.len() == 2 => {
                self.push("assert!(");
                self.emit_expr(&args[0]);
                self.push(", \"{}\", ");
                self.emit_expr(&args[1]);
                self.push(")");
                true
            }
            "assert_eq" if args.len() == 2 => {
                self.push("assert_eq!(");
                self.emit_expr(&args[0]);
                self.push(", ");
                self.emit_expr(&args[1]);
                self.push(")");
                true
            }
            "assert_ne" if args.len() == 2 => {
                self.push("assert_ne!(");
                self.emit_expr(&args[0]);
                self.push(", ");
                self.emit_expr(&args[1]);
                self.push(")");
                true
            }

            // ── Testing & debugging — logging ───────────────────────────
            "log_debug" if args.len() == 1 => {
                self.push("eprintln!(\"[DEBUG] {}\", star_display(&(");
                self.emit_expr(&args[0]);
                self.push(")))");
                true
            }
            "log_info" if args.len() == 1 => {
                self.push("eprintln!(\"[INFO] {}\", star_display(&(");
                self.emit_expr(&args[0]);
                self.push(")))");
                true
            }
            "log_warn" if args.len() == 1 => {
                self.push("eprintln!(\"[WARN] {}\", star_display(&(");
                self.emit_expr(&args[0]);
                self.push(")))");
                true
            }
            "log_error" if args.len() == 1 => {
                self.push("eprintln!(\"[ERROR] {}\", star_display(&(");
                self.emit_expr(&args[0]);
                self.push(")))");
                true
            }

            // ── Testing & debugging — profiling ─────────────────────────
            "time_fn" if args.len() == 1 => {
                // time_fn(f) -> (result, elapsed_ms: Int)
                self.push("{ let _t = std::time::Instant::now(); let _r = (");
                self.emit_expr(&args[0]);
                self.push(")(); let _ms = _t.elapsed().as_millis() as i64; (_r, _ms) }");
                true
            }
            "bench" if args.len() == 2 => {
                // bench(n, f) -> Float (average ms per iteration)
                self.push("{ let _n = ");
                self.emit_expr(&args[0]);
                self.push(" as u64; let _f = ");
                self.emit_expr(&args[1]);
                self.push("; let _t = std::time::Instant::now(); for _ in 0.._n { let _ = _f(); } let _elapsed = _t.elapsed().as_secs_f64() * 1000.0; _elapsed / (_n as f64) }");
                true
            }

            // ── Testing & debugging — debug utilities ───────────────────
            "dbg" if args.len() == 1 => {
                // dbg(x) -> x — prints value to stderr and returns it
                self.push("{ let _val = ");
                self.emit_expr(&args[0]);
                self.push("; eprintln!(\"[dbg] {:?}\", _val); _val }");
                true
            }
            "type_name_of" if args.len() == 1 => {
                // type_name_of(x) -> String
                self.push("{ let ref _val = ");
                self.emit_expr(&args[0]);
                self.push("; fn _type_name<T: ?Sized>(_: &T) -> String { std::any::type_name::<T>().to_string() } _type_name(_val) }");
                true
            }
            "todo" if args.is_empty() => {
                self.push("todo!()");
                true
            }
            "todo_msg" if args.len() == 1 => {
                self.push("todo!(\"{}\", ");
                self.emit_expr(&args[0]);
                self.push(")");
                true
            }
            "unreachable_msg" if args.len() == 1 => {
                self.push("unreachable!(\"{}\", ");
                self.emit_expr(&args[0]);
                self.push(")");
                true
            }

            // ── Configuration & CLI — argument parsing ──────────────────
            "arg_get" if args.len() == 1 => {
                // arg_get(n) -> Option<String> — get nth arg (0 = first after program)
                self.push("std::env::args().skip(1).nth(");
                self.emit_expr(&args[0]);
                self.push(" as usize)");
                true
            }
            "arg_count" if args.is_empty() => {
                // arg_count() -> Int — number of args (excluding program name)
                self.push("(std::env::args().count() as i64 - 1)");
                true
            }
            "arg_has" if args.len() == 1 => {
                // arg_has(flag) -> Bool — check if flag like "--verbose" exists
                self.push("std::env::args().any(|a| a == ");
                self.emit_expr(&args[0]);
                self.push(")");
                true
            }
            "arg_value" if args.len() == 1 => {
                // arg_value(flag) -> Option<String> — get value after flag (--key val or --key=val)
                self.push("{ let _flag = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); let _args: Vec<String> = std::env::args().collect(); ");
                self.push("(|| -> Option<String> { ");
                self.push("for (i, a) in _args.iter().enumerate() { ");
                self.push("if a == &_flag { return _args.get(i + 1).cloned(); } ");
                self.push("if a.starts_with(&format!(\"{}=\", _flag)) { return Some(a[_flag.len()+1..].to_string()); } ");
                self.push("} None })() }");
                true
            }
            "arg_pairs" if args.is_empty() => {
                // arg_pairs() -> List<(String, String)> — parse --key=value and --key value pairs
                self.push("{ let _args: Vec<String> = std::env::args().skip(1).collect(); ");
                self.push("let mut _pairs: Vec<(String, String)> = Vec::new(); let mut _i = 0; ");
                self.push("while _i < _args.len() { ");
                self.push("if _args[_i].starts_with(\"--\") { ");
                self.push("if let Some(_eq) = _args[_i].find('=') { _pairs.push((_args[_i][2.._eq].to_string(), _args[_i][_eq+1..].to_string())); } ");
                self.push("else if _i + 1 < _args.len() && !_args[_i+1].starts_with('-') { _pairs.push((_args[_i][2..].to_string(), _args[_i+1].clone())); _i += 1; } ");
                self.push("else { _pairs.push((_args[_i][2..].to_string(), String::new())); } ");
                self.push("} _i += 1; } _pairs }");
                true
            }

            // ── Configuration & CLI — JSON ──────────────────────────────
            "json_get" if args.len() == 2 => {
                // json_get(json_str, key) -> Option<String> — extract value for key from JSON object
                self.push("{ fn _star_json_get(json: &str, key: &str) -> Option<String> { ");
                self.push("let json = json.trim(); if !json.starts_with('{') { return None; } ");
                self.push("let needle = format!(\"\\\"{}\\\":\", key); ");
                // Simple state machine to find key, handling nested structures
                self.push("let mut i = match json.find(&needle) { Some(p) => p + needle.len(), None => { let needle2 = format!(\"\\\"{}\\\" :\", key); match json.find(&needle2) { Some(p) => p + needle2.len(), None => return None } } }; ");
                self.push("let bytes = json.as_bytes(); while i < bytes.len() && bytes[i] == b' ' { i += 1; } ");
                self.push("if i >= bytes.len() { return None; } ");
                self.push("match bytes[i] { ");
                // String value
                self.push("b'\"' => { let start = i + 1; let mut end = start; while end < bytes.len() && bytes[end] != b'\"' { if bytes[end] == b'\\\\' { end += 1; } end += 1; } Some(json[start..end].to_string()) } ");
                // Nested object or array - find matching close bracket
                self.push("b'{' | b'[' => { let open = bytes[i]; let close = if open == b'{' { b'}' } else { b']' }; let start = i; let mut depth = 1; i += 1; while i < bytes.len() && depth > 0 { if bytes[i] == open { depth += 1; } else if bytes[i] == close { depth -= 1; } else if bytes[i] == b'\"' { i += 1; while i < bytes.len() && bytes[i] != b'\"' { if bytes[i] == b'\\\\' { i += 1; } i += 1; } } i += 1; } Some(json[start..i].to_string()) } ");
                // Number, bool, null
                self.push("_ => { let start = i; while i < bytes.len() && bytes[i] != b',' && bytes[i] != b'}' && bytes[i] != b']' && bytes[i] != b' ' && bytes[i] != b'\\n' { i += 1; } Some(json[start..i].trim().to_string()) } ");
                self.push("} } ");
                self.push("_star_json_get(&");
                self.emit_expr(&args[0]);
                self.push(", &");
                self.emit_expr(&args[1]);
                self.push(") }");
                true
            }
            "json_object" if args.len() == 1 => {
                // json_object(pairs: List<(String, String)>) -> String — build JSON object
                self.push("{ let _pairs: &Vec<(String, String)> = &");
                self.emit_expr(&args[0]);
                self.push("; let _inner: Vec<String> = _pairs.iter().map(|(k, v)| { ");
                self.push("let _ek = k.replace('\\\\', \"\\\\\\\\\").replace('\"', \"\\\\\\\"\"); ");
                self.push("let _ev = v.replace('\\\\', \"\\\\\\\\\").replace('\"', \"\\\\\\\"\"); ");
                self.push("format!(\"\\\"{}\\\": \\\"{}\\\"\", _ek, _ev) }).collect(); ");
                self.push("format!(\"{{{}}}\" , _inner.join(\", \")) }");
                true
            }
            "json_array" if args.len() == 1 => {
                // json_array(items: List<String>) -> String — build JSON array of strings
                self.push("{ let _items: &Vec<String> = &");
                self.emit_expr(&args[0]);
                self.push("; let _inner: Vec<String> = _items.iter().map(|v| { ");
                self.push("let _ev = v.replace('\\\\', \"\\\\\\\\\").replace('\"', \"\\\\\\\"\"); ");
                self.push("format!(\"\\\"{}\\\"\", _ev) }).collect(); ");
                self.push("format!(\"[{}]\", _inner.join(\", \")) }");
                true
            }
            "json_escape" if args.len() == 1 => {
                // json_escape(s) -> String — JSON-escape a string value
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(").replace('\\\\', \"\\\\\\\\\").replace('\"', \"\\\\\\\"\").replace('\\n', \"\\\\n\").replace('\\r', \"\\\\r\").replace('\\t', \"\\\\t\")");
                true
            }
            "json_parse" if args.len() == 1 => {
                // json_parse(s) -> Result<String, String> — parse JSON and return string representation
                self.push("{ ");
                self.push(Self::JSON_HELPER);
                self.push("_star_json_parse(&");
                self.emit_expr(&args[0]);
                self.push(") }");
                true
            }
            "json_encode" if args.len() == 1 => {
                // json_encode(val) -> String — encode value to JSON-like string via Debug
                self.push("{ ");
                self.push(Self::JSON_HELPER);
                self.push("_star_json_encode(&(");
                self.emit_expr(&args[0]);
                self.push(")) }");
                true
            }

            // ── Configuration & CLI — ENV file parsing ──────────────────
            "parse_env_string" if args.len() == 1 => {
                // parse_env_string(content) -> List<(String, String)>
                self.push("(");
                self.emit_expr(&args[0]);
                self.push(").lines().filter_map(|line| { let line = line.trim(); ");
                self.push("if line.is_empty() || line.starts_with('#') { return None; } ");
                self.push("let mut parts = line.splitn(2, '='); ");
                self.push("let key = parts.next()?.trim().to_string(); ");
                self.push("let val = parts.next().unwrap_or(\"\").trim().trim_matches('\"').trim_matches('\\'').to_string(); ");
                self.push("Some((key, val)) }).collect::<Vec<(String, String)>>()");
                true
            }
            "load_env_file" if args.len() == 1 => {
                // load_env_file(path) -> Result<List<(String, String)>, String>
                self.push("std::fs::read_to_string(&*");
                self.emit_expr(&args[0]);
                self.push(").map(|content| { content.lines().filter_map(|line| { let line = line.trim(); ");
                self.push("if line.is_empty() || line.starts_with('#') { return None; } ");
                self.push("let mut parts = line.splitn(2, '='); ");
                self.push("let key = parts.next()?.trim().to_string(); ");
                self.push("let val = parts.next().unwrap_or(\"\").trim().trim_matches('\"').trim_matches('\\'').to_string(); ");
                self.push("Some((key, val)) }).collect::<Vec<(String, String)>>() }).map_err(|e| e.to_string())");
                true
            }

            // ── Configuration & CLI — terminal colors ───────────────────
            "color_red" if args.len() == 1 => {
                self.push("format!(\"\\x1b[31m{}\\x1b[0m\", ");
                self.emit_expr(&args[0]);
                self.push(")");
                true
            }
            "color_green" if args.len() == 1 => {
                self.push("format!(\"\\x1b[32m{}\\x1b[0m\", ");
                self.emit_expr(&args[0]);
                self.push(")");
                true
            }
            "color_blue" if args.len() == 1 => {
                self.push("format!(\"\\x1b[34m{}\\x1b[0m\", ");
                self.emit_expr(&args[0]);
                self.push(")");
                true
            }
            "color_yellow" if args.len() == 1 => {
                self.push("format!(\"\\x1b[33m{}\\x1b[0m\", ");
                self.emit_expr(&args[0]);
                self.push(")");
                true
            }
            "color_cyan" if args.len() == 1 => {
                self.push("format!(\"\\x1b[36m{}\\x1b[0m\", ");
                self.emit_expr(&args[0]);
                self.push(")");
                true
            }
            "color_magenta" if args.len() == 1 => {
                self.push("format!(\"\\x1b[35m{}\\x1b[0m\", ");
                self.emit_expr(&args[0]);
                self.push(")");
                true
            }
            "bold" if args.len() == 1 => {
                self.push("format!(\"\\x1b[1m{}\\x1b[0m\", ");
                self.emit_expr(&args[0]);
                self.push(")");
                true
            }
            "dim" if args.len() == 1 => {
                self.push("format!(\"\\x1b[2m{}\\x1b[0m\", ");
                self.emit_expr(&args[0]);
                self.push(")");
                true
            }
            "underline" if args.len() == 1 => {
                self.push("format!(\"\\x1b[4m{}\\x1b[0m\", ");
                self.emit_expr(&args[0]);
                self.push(")");
                true
            }
            "strip_ansi" if args.len() == 1 => {
                // strip_ansi(s) -> String — remove ANSI escape sequences
                self.push("{ let _s = (");
                self.emit_expr(&args[0]);
                self.push(").clone(); let mut _out = String::new(); let mut _in_esc = false; ");
                self.push("for _c in _s.chars() { if _c == '\\x1b' { _in_esc = true; } else if _in_esc { if _c.is_ascii_alphabetic() { _in_esc = false; } } else { _out.push(_c); } } _out }");
                true
            }
            "prompt" if args.len() == 1 => {
                // prompt(message) -> String — print message, read line
                self.push("{ print!(\"{}\", ");
                self.emit_expr(&args[0]);
                self.push("); use std::io::Write; std::io::stdout().flush().unwrap(); ");
                self.push("let mut _buf = String::new(); std::io::stdin().read_line(&mut _buf).unwrap(); _buf.trim_end_matches('\\n').trim_end_matches('\\r').to_string() }");
                true
            }
            "confirm" if args.len() == 1 => {
                // confirm(message) -> Bool — print message with [y/n], return true for y/Y
                self.push("{ print!(\"{} [y/n] \", ");
                self.emit_expr(&args[0]);
                self.push("); use std::io::Write; std::io::stdout().flush().unwrap(); ");
                self.push("let mut _buf = String::new(); std::io::stdin().read_line(&mut _buf).unwrap(); ");
                self.push("matches!(_buf.trim().to_lowercase().as_str(), \"y\" | \"yes\") }");
                true
            }
            "clear_screen" if args.is_empty() => {
                self.push("print!(\"\\x1b[2J\\x1b[H\")");
                true
            }
            "cursor_up" if args.len() == 1 => {
                self.push("print!(\"\\x1b[{}A\", ");
                self.emit_expr(&args[0]);
                self.push(")");
                true
            }
            "cursor_down" if args.len() == 1 => {
                self.push("print!(\"\\x1b[{}B\", ");
                self.emit_expr(&args[0]);
                self.push(")");
                true
            }

            // ── Option/Result helpers ───────────────────────────────────
            "unwrap" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".clone().unwrap()");
                true
            }
            "unwrap_or" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".clone().unwrap_or(");
                self.emit_expr(&args[1]);
                self.push(")");
                true
            }
            "or_else" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".clone().or_else(|_e| (");
                self.emit_expr(&args[1]);
                self.push(")(_e))");
                true
            }
            "map_or" if args.len() == 3 => {
                self.emit_expr(&args[0]);
                self.push(".clone().map_or(");
                self.emit_expr(&args[1]);
                self.push(", |_v| (");
                self.emit_expr(&args[2]);
                self.push(")(_v))");
                true
            }
            "is_some" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".is_some()");
                true
            }
            "is_none" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".is_none()");
                true
            }
            "is_ok" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".is_ok()");
                true
            }
            "is_err" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".is_err()");
                true
            }
            "ok" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".clone().ok()");
                true
            }
            "err" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".clone().err()");
                true
            }
            "expect" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".clone().expect(&");
                self.emit_expr(&args[1]);
                self.push(")");
                true
            }
            "unwrap_err" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".clone().unwrap_err()");
                true
            }
            "unwrap_or_else" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".clone().unwrap_or_else(|_e| (");
                self.emit_expr(&args[1]);
                self.push(")(_e))");
                true
            }
            "map_result" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".clone().map(|_v| (");
                self.emit_expr(&args[1]);
                self.push(")(_v))");
                true
            }
            "map_option" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".clone().map(|_v| (");
                self.emit_expr(&args[1]);
                self.push(")(_v))");
                true
            }
            "map_err" if args.len() == 2 => {
                // map_err(result, fn) — transform Err value
                self.emit_expr(&args[0]);
                self.push(".clone().map_err(|_e| (");
                self.emit_expr(&args[1]);
                self.push(")(_e))");
                true
            }
            "and_then" if args.len() == 2 => {
                // and_then(result_or_option, fn) — monadic bind
                self.emit_expr(&args[0]);
                self.push(".clone().and_then(|_v| (");
                self.emit_expr(&args[1]);
                self.push(")(_v))");
                true
            }
            "or_default" if args.len() == 1 => {
                // or_default(option_or_result) — unwrap_or_default
                self.emit_expr(&args[0]);
                self.push(".clone().unwrap_or_default()");
                true
            }
            "flatten_result" if args.len() == 1 => {
                // flatten_result(Result<Result<T,E>,E>) -> Result<T,E>
                self.emit_expr(&args[0]);
                self.push(".clone().and_then(|_v| _v)");
                true
            }
            "flatten_option" if args.len() == 1 => {
                // flatten_option(Option<Option<T>>) -> Option<T>
                self.emit_expr(&args[0]);
                self.push(".clone().flatten()");
                true
            }
            "ok_or" if args.len() == 2 => {
                // ok_or(option, error) — Option -> Result
                self.emit_expr(&args[0]);
                self.push(".clone().ok_or(");
                self.emit_expr(&args[1]);
                self.push(")");
                true
            }
            "ok_or_else" if args.len() == 2 => {
                // ok_or_else(option, fn) — lazy Option -> Result
                self.emit_expr(&args[0]);
                self.push(".clone().ok_or_else(|| (");
                self.emit_expr(&args[1]);
                self.push(")())");
                true
            }
            "some" if args.len() == 1 => {
                // some(value) — wrap in Some
                self.push("Some(");
                self.emit_expr(&args[0]);
                self.push(")");
                true
            }
            "none" if args.is_empty() => {
                // none() — return None
                self.push("None");
                true
            }
            "transpose" if args.len() == 1 => {
                // transpose(Option<Result<T,E>>) -> Result<Option<T>,E>
                self.emit_expr(&args[0]);
                self.push(".clone().transpose()");
                true
            }

            // ── HashMap operations ───────────────────────────────────────
            "map_new" if args.is_empty() => {
                self.push("std::collections::HashMap::new()");
                true
            }
            "map_from_list" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().collect::<std::collections::HashMap<_, _>>()");
                true
            }
            "map_insert" if args.len() == 3 => {
                self.push("{ let mut _m = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); _m.insert(");
                self.emit_expr(&args[1]);
                self.push(", ");
                self.emit_expr(&args[2]);
                self.push("); _m }");
                true
            }
            "map_remove" if args.len() == 2 => {
                self.push("{ let mut _m = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); _m.remove(&");
                self.emit_expr(&args[1]);
                self.push("); _m }");
                true
            }
            "map_get" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".get(&");
                self.emit_expr(&args[1]);
                self.push(").cloned()");
                true
            }
            "map_contains_key" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".contains_key(&");
                self.emit_expr(&args[1]);
                self.push(")");
                true
            }
            "map_keys" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".keys().cloned().collect::<Vec<_>>()");
                true
            }
            "map_values" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".values().cloned().collect::<Vec<_>>()");
                true
            }
            "map_entries" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>()");
                true
            }
            "map_size" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".len() as i64");
                true
            }
            "map_merge" if args.len() == 2 => {
                self.push("{ let mut _m = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); _m.extend(");
                self.emit_expr(&args[1]);
                self.push(".iter().map(|(k, v)| (k.clone(), v.clone()))); _m }");
                true
            }

            // ── HashSet operations ───────────────────────────────────────
            "set_new" if args.is_empty() => {
                self.push("std::collections::HashSet::new()");
                true
            }
            "set_from_list" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().collect::<std::collections::HashSet<_>>()");
                true
            }
            "set_insert" if args.len() == 2 => {
                self.push("{ let mut _s = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); _s.insert(");
                self.emit_expr(&args[1]);
                self.push("); _s }");
                true
            }
            "set_remove" if args.len() == 2 => {
                self.push("{ let mut _s = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); _s.remove(&");
                self.emit_expr(&args[1]);
                self.push("); _s }");
                true
            }
            "set_contains" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".contains(&");
                self.emit_expr(&args[1]);
                self.push(")");
                true
            }
            "set_union" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".union(&");
                self.emit_expr(&args[1]);
                self.push(").cloned().collect::<std::collections::HashSet<_>>()");
                true
            }
            "set_intersection" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".intersection(&");
                self.emit_expr(&args[1]);
                self.push(").cloned().collect::<std::collections::HashSet<_>>()");
                true
            }
            "set_difference" if args.len() == 2 => {
                self.emit_expr(&args[0]);
                self.push(".difference(&");
                self.emit_expr(&args[1]);
                self.push(").cloned().collect::<std::collections::HashSet<_>>()");
                true
            }
            "set_size" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".len() as i64");
                true
            }
            "set_to_list" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".iter().cloned().collect::<Vec<_>>()");
                true
            }

            // ── Deque operations ─────────────────────────────────────────
            "deque_new" if args.is_empty() => {
                self.push("std::collections::VecDeque::new()");
                true
            }
            "deque_from_list" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().collect::<std::collections::VecDeque<_>>()");
                true
            }
            "deque_push_back" if args.len() == 2 => {
                self.push("{ let mut _d = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); _d.push_back(");
                self.emit_expr(&args[1]);
                self.push("); _d }");
                true
            }
            "deque_push_front" if args.len() == 2 => {
                self.push("{ let mut _d = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); _d.push_front(");
                self.emit_expr(&args[1]);
                self.push("); _d }");
                true
            }
            "deque_pop_back" if args.len() == 1 => {
                self.push("{ let mut _d = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); (_d.pop_back(), _d) }");
                true
            }
            "deque_pop_front" if args.len() == 1 => {
                self.push("{ let mut _d = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); (_d.pop_front(), _d) }");
                true
            }
            "deque_size" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".len() as i64");
                true
            }
            "deque_to_list" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".iter().cloned().collect::<Vec<_>>()");
                true
            }

            // ── Heap operations ──────────────────────────────────────────
            "heap_new" if args.is_empty() => {
                self.push("std::collections::BinaryHeap::new()");
                true
            }
            "heap_from_list" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_iter().collect::<std::collections::BinaryHeap<_>>()");
                true
            }
            "heap_push" if args.len() == 2 => {
                self.push("{ let mut _h = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); _h.push(");
                self.emit_expr(&args[1]);
                self.push("); _h }");
                true
            }
            "heap_pop" if args.len() == 1 => {
                self.push("{ let mut _h = ");
                self.emit_expr(&args[0]);
                self.push(".clone(); (_h.pop(), _h) }");
                true
            }
            "heap_peek" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".peek().cloned()");
                true
            }
            "heap_size" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".len() as i64");
                true
            }
            "heap_to_list" if args.len() == 1 => {
                self.emit_expr(&args[0]);
                self.push(".clone().into_sorted_vec()");
                true
            }

            _ => false,
        }
    }

    /// Try to emit a builtin pipe. `left |> builtin(args...)` was desugared to
    /// `builtin(left, args...)` already by the Pipe handler, so this is only
    /// needed for the case where the pipe RHS is a bare builtin name with no
    /// extra args.
    fn try_emit_builtin_pipe(&mut self, left: &Expr, name: &str) -> bool {
        // Single-arg builtins piped: `x |> println`
        let args = [left.clone()];
        self.try_emit_builtin_call(name, &args)
    }

    // ── Expressions ─────────────────────────────────────────────────────────

    /// Emit an expression in return position (avoids unnecessary parens around BinOp).
    fn emit_expr_as_return(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::BinOp(left, op, right) => {
                // Emit without outer parens to avoid Rust's "unnecessary parentheses" warning
                self.emit_expr(left);
                let op_str = match op {
                    BinOp::Add => " + ",
                    BinOp::Sub => " - ",
                    BinOp::Mul => " * ",
                    BinOp::Div => " / ",
                    BinOp::Mod => " % ",
                    BinOp::Eq => " == ",
                    BinOp::Ne => " != ",
                    BinOp::Lt => " < ",
                    BinOp::Gt => " > ",
                    BinOp::Le => " <= ",
                    BinOp::Ge => " >= ",
                    BinOp::And => " && ",
                    BinOp::Or => " || ",
                    BinOp::Band => " & ",
                    BinOp::Bor => " | ",
                    BinOp::Bxor => " ^ ",
                    BinOp::Shl => " << ",
                    BinOp::Shr => " >> ",
                };
                self.push(op_str);
                self.emit_expr(right);
            }
            _ => self.emit_expr(expr),
        }
    }

    fn emit_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::IntLit(n) => self.push(&format!("{n}i64")),
            ExprKind::FloatLit(f) => {
                let s = f.to_string();
                self.push(&s);
                if !s.contains('.') {
                    self.push(".0");
                }
                self.push("f64");
            }
            ExprKind::StringLit(s) => {
                let escaped = escape_rust_string(s);
                self.push(&format!("\"{escaped}\".to_string()"));
            }
            ExprKind::StringInterp(parts) => {
                self.emit_string_interp(parts);
            }
            ExprKind::BoolLit(b) => self.push(if *b { "true" } else { "false" }),

            ExprKind::Ident(name) => {
                let qualified = self.qualify_variant(name);
                self.push(&qualified);
            }

            ExprKind::ListLit(elems) => {
                self.push("vec![");
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.emit_expr(elem);
                }
                self.push("]");
            }

            ExprKind::Tuple(elems) => {
                self.push("(");
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.emit_expr(elem);
                }
                self.push(")");
            }

            ExprKind::BinOp(left, op, right) => {
                self.push("(");
                self.emit_expr(left);
                let op_str = match op {
                    BinOp::Add => " + ",
                    BinOp::Sub => " - ",
                    BinOp::Mul => " * ",
                    BinOp::Div => " / ",
                    BinOp::Mod => " % ",
                    BinOp::Eq => " == ",
                    BinOp::Ne => " != ",
                    BinOp::Lt => " < ",
                    BinOp::Gt => " > ",
                    BinOp::Le => " <= ",
                    BinOp::Ge => " >= ",
                    BinOp::And => " && ",
                    BinOp::Or => " || ",
                    BinOp::Band => " & ",
                    BinOp::Bor => " | ",
                    BinOp::Bxor => " ^ ",
                    BinOp::Shl => " << ",
                    BinOp::Shr => " >> ",
                };
                self.push(op_str);
                self.emit_expr(right);
                self.push(")");
            }

            ExprKind::UnaryOp(op, inner) => {
                let op_str = match op {
                    UnaryOp::Neg => "-",
                    UnaryOp::Not => "!",
                };
                self.push(op_str);
                self.emit_expr(inner);
            }

            ExprKind::Pipe(left, right) => {
                self.emit_pipe(left, right);
            }

            ExprKind::Call(func, args) => {
                self.emit_call(func, args);
            }

            ExprKind::Lambda(params, return_type, body, is_move) => {
                if *is_move {
                    self.push("move ");
                }
                self.push("|");
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.push(&param.name);
                    if let Some(ref ty) = param.ty {
                        self.push(": ");
                        self.push(&self.type_to_rust(ty, ""));
                    }
                }
                self.push("|");
                if let Some(ret_ty) = return_type {
                    self.push(" -> ");
                    self.push(&self.type_to_rust(ret_ty, ""));
                    self.push(" { ");
                    self.emit_expr(body);
                    self.push(" }");
                } else {
                    self.push(" ");
                    self.emit_expr_as_return(body);
                }
            }

            ExprKind::If(cond, then_branch, else_branch) => {
                self.push("if ");
                self.emit_expr(cond);
                self.push(" { ");
                self.emit_expr(then_branch);
                self.push(" }");
                if let Some(else_branch) = else_branch {
                    self.push(" else { ");
                    self.emit_expr(else_branch);
                    self.push(" }");
                }
            }

            ExprKind::Match(scrutinee, arms) => {
                // If any arm has a string literal pattern, match on .as_str()
                let has_string_pat = arms.iter().any(|arm| self.pattern_has_string_lit(&arm.pattern));
                self.push("match ");
                self.emit_expr(scrutinee);
                if has_string_pat {
                    self.push(".as_ref()");
                }
                self.push(" {\n");
                self.indent();
                for arm in arms {
                    self.emit_indent();
                    self.emit_pattern(&arm.pattern);
                    if let Some(guard) = &arm.guard {
                        self.push(" if ");
                        self.emit_expr(guard);
                    }
                    // Collect boxed bindings that need auto-deref
                    let boxed_bindings = self.collect_boxed_bindings(&arm.pattern);
                    if boxed_bindings.is_empty() {
                        self.push(" => ");
                        self.emit_expr(&arm.body);
                    } else {
                        self.push(" => {\n");
                        self.indent();
                        for binding in &boxed_bindings {
                            self.emit_indent();
                            self.push(&format!("let {binding} = *{binding};\n"));
                        }
                        self.emit_indent();
                        self.emit_expr(&arm.body);
                        self.push("\n");
                        self.dedent();
                        self.emit_indent();
                        self.push("}");
                    }
                    self.push(",\n");
                }
                self.dedent();
                self.emit_indent();
                self.push("}");
            }

            ExprKind::Block(stmts, final_expr) => {
                self.push("{\n");
                self.indent();
                for stmt in stmts {
                    self.emit_indent();
                    match stmt {
                        Stmt::Let(is_mut, pat, _ty, val) => {
                            if *is_mut {
                                self.push("let mut ");
                            } else {
                                self.push("let ");
                            }
                            self.emit_pattern(pat);
                            self.push(" = ");
                            self.emit_expr(val);
                            self.push(";\n");
                        }
                        Stmt::Expr(e) => {
                            self.emit_expr(e);
                            self.push(";\n");
                        }
                        Stmt::Assign(name, val) => {
                            self.push(name);
                            self.push(" = ");
                            self.emit_expr(val);
                            self.push(";\n");
                        }
                        Stmt::CompoundAssign(name, op, val) => {
                            self.push(name);
                            self.push(match op {
                                BinOp::Add => " += ",
                                BinOp::Sub => " -= ",
                                BinOp::Mul => " *= ",
                                BinOp::Div => " /= ",
                                BinOp::Mod => " %= ",
                                _ => " = ",
                            });
                            self.emit_expr(val);
                            self.push(";\n");
                        }
                        Stmt::IndexAssign(obj, index, val) => {
                            self.emit_expr(obj);
                            self.push("[(");
                            self.emit_expr(index);
                            self.push(") as usize] = ");
                            self.emit_expr(val);
                            self.push(";\n");
                        }
                    }
                }
                self.emit_indent();
                self.emit_expr(final_expr);
                self.push("\n");
                self.dedent();
                self.emit_indent();
                self.push("}");
            }

            ExprKind::Let(pat, _ty, val) => {
                self.push("let ");
                self.emit_pattern(pat);
                self.push(" = ");
                self.emit_expr(val);
            }

            ExprKind::FieldAccess(obj, field) => {
                self.emit_expr(obj);
                self.push(".");
                self.push(field);
            }

            ExprKind::MethodCall(obj, method, args) => {
                if method == "__index" && args.len() == 1 {
                    self.emit_expr(obj);
                    self.push("[(");
                    self.emit_expr(&args[0]);
                    self.push(") as usize]");
                } else if self.is_builtin(method) {
                    // Treat as a builtin call with obj prepended to args
                    let mut builtin_args = vec![(**obj).clone()];
                    builtin_args.extend(args.iter().cloned());
                    if !self.try_emit_builtin_call(method, &builtin_args) {
                        // Arity mismatch — fall back to Rust method call
                        self.emit_expr(obj);
                        self.push(".");
                        self.push(method);
                        self.push("(");
                        for (i, arg) in args.iter().enumerate() {
                            if i > 0 {
                                self.push(", ");
                            }
                            self.emit_expr(arg);
                        }
                        self.push(")");
                    }
                } else {
                    self.emit_expr(obj);
                    self.push(".");
                    self.push(method);
                    self.push("(");
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            self.push(", ");
                        }
                        self.emit_expr(arg);
                    }
                    self.push(")");
                }
            }

            ExprKind::StructLit(name, fields, spread) => {
                self.push(name);
                self.push(" { ");
                for (i, (fname, fval)) in fields.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.push(fname);
                    self.push(": ");
                    self.emit_expr(fval);
                }
                if let Some(spread) = spread {
                    if !fields.is_empty() {
                        self.push(", ");
                    }
                    self.push("..");
                    self.emit_expr(spread);
                }
                self.push(" }");
            }

            ExprKind::RustBlock(code) => {
                self.push(code);
            }

            ExprKind::Try(inner) => {
                self.emit_expr(inner);
                self.push("?");
            }

            ExprKind::Await(inner) => {
                self.emit_expr(inner);
                self.push(".await");
            }

            ExprKind::For(pattern, iter, body) => {
                self.push("for ");
                self.emit_pattern(pattern);
                self.push(" in ");
                self.emit_expr(iter);
                self.push(" {\n");
                self.indent();
                self.emit_indent();
                self.emit_expr(body);
                self.push(";\n");
                self.dedent();
                self.emit_indent();
                self.push("}");
            }

            ExprKind::While(cond, body) => {
                self.push("while ");
                self.emit_expr(cond);
                self.push(" {\n");
                self.indent();
                self.emit_indent();
                self.emit_expr(body);
                self.push(";\n");
                self.dedent();
                self.emit_indent();
                self.push("}");
            }

            ExprKind::Break => {
                self.push("break");
            }

            ExprKind::Continue => {
                self.push("continue");
            }
        }
    }

    // ── Pipe emission ───────────────────────────────────────────────────────

    fn emit_pipe(&mut self, left: &Expr, right: &Expr) {
        match &right.kind {
            ExprKind::Call(func, extra_args) => {
                // `left |> func(extra_args...)` => builtin(left, extra_args...) or func(left, extra_args...)
                if let ExprKind::Ident(name) = &func.kind {
                    if self.is_builtin(name) {
                        let mut all_args = vec![left.clone()];
                        all_args.extend(extra_args.iter().cloned());
                        if self.try_emit_builtin_call(name, &all_args) {
                            return;
                        }
                    }
                }
                // Regular pipe with args
                self.emit_expr(func);
                self.push("(");
                self.emit_expr(left);
                for arg in extra_args {
                    self.push(", ");
                    self.emit_expr(arg);
                }
                self.push(")");
            }
            ExprKind::Ident(name) => {
                if self.is_builtin(name) {
                    if self.try_emit_builtin_pipe(left, name) {
                        return;
                    }
                }
                // Regular pipe to function name
                self.push(name);
                self.push("(");
                self.emit_expr(left);
                self.push(")");
            }
            _ => {
                self.emit_expr(right);
                self.push("(");
                self.emit_expr(left);
                self.push(")");
            }
        }
    }

    // ── Call emission ───────────────────────────────────────────────────────

    fn emit_call(&mut self, func: &Expr, args: &[Expr]) {
        // Check if this is a builtin call
        if let ExprKind::Ident(name) = &func.kind {
            if self.is_builtin(name) {
                if self.try_emit_builtin_call(name, args) {
                    return;
                }
            }
        }

        // Check if this is a recursive variant constructor call that needs Box::new wrapping
        if let ExprKind::Ident(name) = &func.kind {
            if let Some(enum_name) = self.variant_to_enum.get(name).cloned() {
                if self.recursive_types.contains_key(&enum_name) {
                    let variant_name = name.clone();
                    let qualified = self.qualify_variant(&variant_name);
                    self.push(&qualified);
                    self.push("(");
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            self.push(", ");
                        }
                        if self.is_boxed_field(&enum_name, &variant_name, i) {
                            self.push("Box::new(");
                            self.emit_expr(arg);
                            self.push(")");
                        } else {
                            self.emit_expr(arg);
                        }
                    }
                    self.push(")");
                    return;
                }
            }
        }

        // Regular call
        self.emit_expr(func);
        self.push("(");
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.push(", ");
            }
            self.emit_expr(arg);
        }
        self.push(")");
    }

    // ── String interpolation ────────────────────────────────────────────────

    fn emit_string_interp(&mut self, parts: &[StringPart]) {
        self.push("format!(\"");
        let mut expr_list: Vec<&Expr> = Vec::new();
        for part in parts {
            match part {
                StringPart::Lit(s) => {
                    let escaped = escape_rust_string(s);
                    // Also escape { and } for format!
                    let escaped = escaped.replace('{', "{{").replace('}', "}}");
                    self.push(&escaped);
                }
                StringPart::Expr(e) => {
                    self.push("{}");
                    expr_list.push(e);
                }
            }
        }
        self.push("\"");
        for e in expr_list {
            self.push(", star_display(&(");
            self.emit_expr(e);
            self.push("))");
        }
        self.push(")");
    }

    // ── Patterns ────────────────────────────────────────────────────────────

    /// Collect the names of pattern variables that are bound to boxed fields,
    /// so we can emit `let name = *name;` deref statements in match arm bodies.
    fn collect_boxed_bindings(&self, pattern: &Pattern) -> Vec<String> {
        match pattern {
            Pattern::Constructor(name, pats) => {
                if let Some(enum_name) = self.variant_to_enum.get(name) {
                    let mut bindings = Vec::new();
                    for (fi, pat) in pats.iter().enumerate() {
                        if self.is_boxed_field(enum_name, name, fi) {
                            // Collect all simple Ident bindings at this position
                            self.collect_ident_bindings(pat, &mut bindings);
                        }
                    }
                    return bindings;
                }
                Vec::new()
            }
            Pattern::Or(pats) => {
                // Collect from first alternative (they should all bind same names)
                if let Some(first) = pats.first() {
                    self.collect_boxed_bindings(first)
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        }
    }

    fn collect_ident_bindings(&self, pattern: &Pattern, out: &mut Vec<String>) {
        match pattern {
            Pattern::Ident(name) => out.push(name.clone()),
            Pattern::Bind(name, _) => out.push(name.clone()),
            _ => {}
        }
    }

    fn pattern_has_string_lit(&self, pattern: &Pattern) -> bool {
        match pattern {
            Pattern::StringLit(_) => true,
            Pattern::Or(pats) => pats.iter().any(|p| self.pattern_has_string_lit(p)),
            Pattern::Bind(_, inner) => self.pattern_has_string_lit(inner),
            Pattern::Constructor(_, pats) => pats.iter().any(|p| self.pattern_has_string_lit(p)),
            Pattern::Tuple(pats) => pats.iter().any(|p| self.pattern_has_string_lit(p)),
            Pattern::List(pats, _) => pats.iter().any(|p| self.pattern_has_string_lit(p)),
            _ => false,
        }
    }

    fn pattern_to_rust(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Ident(name) => name.clone(),
            Pattern::Tuple(pats) => {
                let parts: Vec<String> = pats.iter().map(|p| self.pattern_to_rust(p)).collect();
                format!("({})", parts.join(", "))
            }
            _ => "_".to_string(), // Fallback for complex patterns
        }
    }

    fn emit_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Wildcard => self.push("_"),
            Pattern::Ident(name) => self.push(name),
            Pattern::IntLit(n) => self.push(&n.to_string()),
            Pattern::FloatLit(f) => self.push(&f.to_string()),
            Pattern::StringLit(s) => self.push(&format!("\"{}\"", s)),
            Pattern::BoolLit(b) => self.push(if *b { "true" } else { "false" }),
            Pattern::Constructor(name, pats) => {
                let qualified = self.qualify_variant(name);
                self.push(&qualified);
                if !pats.is_empty() {
                    self.push("(");
                    for (i, pat) in pats.iter().enumerate() {
                        if i > 0 {
                            self.push(", ");
                        }
                        self.emit_pattern(pat);
                    }
                    self.push(")");
                }
            }
            Pattern::Tuple(pats) => {
                self.push("(");
                for (i, pat) in pats.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.emit_pattern(pat);
                }
                self.push(")");
            }
            Pattern::List(pats, rest) => {
                self.push("[");
                for (i, pat) in pats.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.emit_pattern(pat);
                }
                if let Some(rest_name) = rest {
                    if !pats.is_empty() {
                        self.push(", ");
                    }
                    self.push(&format!("{rest_name} @ .."));
                }
                self.push("]");
            }
            Pattern::Bind(name, inner) => {
                self.push(&format!("{name} @ "));
                self.emit_pattern(inner);
            }
            Pattern::Or(pats) => {
                for (i, pat) in pats.iter().enumerate() {
                    if i > 0 {
                        self.push(" | ");
                    }
                    self.emit_pattern(pat);
                }
            }
            Pattern::Range(start, end) => {
                self.push(&format!("{start}i64..={end}i64"));
            }
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn escape_rust_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

/// Convert PascalCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;
    use crate::parser;
    use crate::typeck;

    fn compile(src: &str) -> String {
        let tokens = lexer::lex(src).unwrap();
        let (program, _comments) = parser::parse(tokens).unwrap();
        // Skip typeck — codegen tests focus on code generation, not type checking
        generate(&program, false)
    }

    #[test]
    fn test_simple_function() {
        let rust = compile("fn add(a: Int, b: Int): Int = a + b");
        assert!(rust.contains("fn add(a: i64, b: i64) -> i64"));
        assert!(rust.contains("a + b"));
    }

    #[test]
    fn test_pipe_operator() {
        let rust = compile("fn main() = 5 |> double");
        assert!(rust.contains("double(5i64)"));
    }

    #[test]
    fn test_if_expression() {
        let rust = compile("fn abs(x: Int): Int = if x > 0 then x else 0 - x end");
        assert!(rust.contains("if (x > 0i64) { x } else { (0i64 - x) }"));
    }

    #[test]
    fn test_type_decl_enum() {
        let rust = compile("type Color =\n  | Red\n  | Green\n  | Blue");
        assert!(rust.contains("enum Color"));
        assert!(rust.contains("Red,"));
        assert!(rust.contains("Green,"));
        assert!(rust.contains("Blue,"));
    }

    #[test]
    fn test_type_decl_struct() {
        let rust = compile("type Point = {\n  x: Float,\n  y: Float\n}");
        assert!(rust.contains("struct Point"));
        assert!(rust.contains("x: f64"));
        assert!(rust.contains("y: f64"));
    }

    #[test]
    fn test_type_alias_simple() {
        let rust = compile("type StringList = List<String>");
        assert!(rust.contains("type StringList = Vec<String>;"));
    }

    #[test]
    fn test_type_alias_tuple() {
        let rust = compile("type IntPair = (Int, Int)");
        assert!(rust.contains("type IntPair = (i64, i64);"));
    }

    #[test]
    fn test_type_alias_function() {
        let rust = compile("type Callback = fn(Int) -> String");
        assert!(rust.contains("type Callback = impl Fn(i64) -> String;"));
    }

    #[test]
    fn test_type_alias_with_type_params() {
        let rust = compile("type Pair<A, B> = (A, B)");
        assert!(rust.contains("type Pair<A, B> = (A, B);"));
    }

    #[test]
    fn test_lambda() {
        let rust = compile("fn main() = fn(x) => x + 1");
        assert!(rust.contains("|x| x + 1i64") || rust.contains("|x| (x + 1i64)"));
    }

    #[test]
    fn test_list_literal() {
        let rust = compile("fn main() = [1, 2, 3]");
        assert!(rust.contains("vec![1i64, 2i64, 3i64]"));
    }

    #[test]
    fn test_match_expression() {
        let src = r#"fn f(x: Int): Int = match x
  | 0 => 1
  | n => n * 2
  end"#;
        let rust = compile(src);
        assert!(rust.contains("match x"));
        assert!(rust.contains("0 =>"));
        assert!(rust.contains("n =>"));
    }

    #[test]
    fn test_println_builtin() {
        let rust = compile("fn main() = println(42)");
        assert!(rust.contains("println!(\"{}\", star_display(&(42i64)))"));
    }

    #[test]
    fn test_println_string() {
        let rust = compile(r#"fn main() = println("hello")"#);
        assert!(rust.contains("println!"));
    }

    #[test]
    fn test_debug_builtin() {
        let rust = compile("fn main() = debug([1, 2, 3])");
        assert!(rust.contains("println!(\"{:?}\""));
    }

    #[test]
    fn test_pipe_to_println() {
        let rust = compile(r#"fn main() = "hello" |> println"#);
        assert!(rust.contains("println!(\"{}\", star_display(&("));
    }

    #[test]
    fn test_map_builtin() {
        let rust = compile("fn main() = map([1, 2], fn(x) => x + 1)");
        assert!(rust.contains(".clone().into_iter().map("));
        assert!(rust.contains(".collect::<Vec<_>>()"));
    }

    #[test]
    fn test_filter_builtin() {
        let rust = compile("fn main() = filter([1, 2, 3], fn(x) => x > 1)");
        assert!(rust.contains(".clone().into_iter().filter("));
    }

    #[test]
    fn test_pipe_map_filter() {
        let rust = compile(
            "fn main() = [1, 2, 3] |> filter(fn(x) => x > 1) |> map(fn(x) => x * 2)",
        );
        assert!(rust.contains(".clone().into_iter().filter("));
        assert!(rust.contains(".clone().into_iter().map("));
    }

    #[test]
    fn test_range_builtin() {
        let rust = compile("fn main() = range(1, 10)");
        assert!(rust.contains("(1i64..10i64).collect::<Vec<_>>()"));
    }

    #[test]
    fn test_length_builtin() {
        let rust = compile("fn main() = length([1, 2, 3])");
        assert!(rust.contains(".len() as i64"));
    }

    #[test]
    fn test_to_string_builtin() {
        let rust = compile("fn main() = to_string(42)");
        assert!(rust.contains("format!(\"{}\", star_display(&(42i64)))"));
    }

    #[test]
    fn test_string_interpolation() {
        let rust = compile(r#"fn main() = "hello #{name}""#);
        assert!(rust.contains("format!(\"hello {}\""));
        assert!(rust.contains("star_display(&(name))"));
    }

    #[test]
    fn test_fold_builtin() {
        let rust = compile("fn main() = fold([1, 2, 3], 0, fn(a, b) => a + b)");
        assert!(rust.contains(".clone().into_iter().fold(0i64,"));
    }

    #[test]
    fn test_sum_builtin() {
        let rust = compile("fn main() = sum([1, 2, 3])");
        assert!(rust.contains(".clone().into_iter().sum::<i64>()"));
    }

    #[test]
    fn test_sort_builtin() {
        let rust = compile("fn main() = sort([3, 1, 2])");
        assert!(rust.contains(".sort()"));
    }

    #[test]
    fn test_reverse_builtin() {
        let rust = compile("fn main() = reverse([1, 2, 3])");
        assert!(rust.contains(".reverse()"));
    }

    #[test]
    fn test_join_builtin() {
        let rust = compile(r#"fn main() = join(["a", "b"], ", ")"#);
        assert!(rust.contains(".join("));
    }

    #[test]
    fn test_split_builtin() {
        let rust = compile(r#"fn main() = split("a,b,c", ",")"#);
        assert!(rust.contains(".split("));
    }

    #[test]
    fn test_abs_builtin() {
        let rust = compile("fn main() = abs(-5)");
        assert!(rust.contains("as i64).abs()"));
    }

    #[test]
    fn test_min_max_builtin() {
        let rust = compile("fn main() = min(3, 7)");
        assert!(rust.contains("std::cmp::min(3i64, 7i64)"));
    }

    #[test]
    fn test_each_pipe() {
        let rust = compile("fn main() = [1, 2, 3] |> each(fn(x) => println(x))");
        assert!(rust.contains(".clone().into_iter().for_each("));
    }

    // ── HashMap tests ──────────────────────────────────────

    #[test]
    fn test_map_new() {
        let rust = compile("fn main() = map_new()");
        assert!(rust.contains("std::collections::HashMap::new()"));
    }

    #[test]
    fn test_map_insert() {
        let rust = compile(r#"fn main() = map_insert(m, "key", 42)"#);
        assert!(rust.contains("_m.insert("));
    }

    #[test]
    fn test_map_get() {
        let rust = compile(r#"fn main() = map_get(m, "key")"#);
        assert!(rust.contains(".get(&"));
        assert!(rust.contains(").cloned()"));
    }

    #[test]
    fn test_map_keys() {
        let rust = compile("fn main() = map_keys(m)");
        assert!(rust.contains(".keys().cloned().collect::<Vec<_>>()"));
    }

    #[test]
    fn test_map_from_list() {
        let rust = compile("fn main() = map_from_list(pairs)");
        assert!(rust.contains(".collect::<std::collections::HashMap<_, _>>()"));
    }

    // ── HashSet tests ──────────────────────────────────────

    #[test]
    fn test_set_new() {
        let rust = compile("fn main() = set_new()");
        assert!(rust.contains("std::collections::HashSet::new()"));
    }

    #[test]
    fn test_set_from_list() {
        let rust = compile("fn main() = set_from_list([1, 2, 3])");
        assert!(rust.contains(".collect::<std::collections::HashSet<_>>()"));
    }

    #[test]
    fn test_set_contains() {
        let rust = compile("fn main() = set_contains(s, 42)");
        assert!(rust.contains(".contains(&"));
    }

    #[test]
    fn test_set_union() {
        let rust = compile("fn main() = set_union(a, b)");
        assert!(rust.contains(".union(&"));
    }

    // ── Deque tests ────────────────────────────────────────

    #[test]
    fn test_deque_new() {
        let rust = compile("fn main() = deque_new()");
        assert!(rust.contains("std::collections::VecDeque::new()"));
    }

    #[test]
    fn test_deque_push_back() {
        let rust = compile("fn main() = deque_push_back(d, 42)");
        assert!(rust.contains("_d.push_back("));
    }

    #[test]
    fn test_deque_push_front() {
        let rust = compile("fn main() = deque_push_front(d, 42)");
        assert!(rust.contains("_d.push_front("));
    }

    // ── Heap tests ─────────────────────────────────────────

    #[test]
    fn test_heap_new() {
        let rust = compile("fn main() = heap_new()");
        assert!(rust.contains("std::collections::BinaryHeap::new()"));
    }

    #[test]
    fn test_heap_push() {
        let rust = compile("fn main() = heap_push(h, 42)");
        assert!(rust.contains("_h.push("));
    }

    #[test]
    fn test_heap_peek() {
        let rust = compile("fn main() = heap_peek(h)");
        assert!(rust.contains(".peek().cloned()"));
    }

    // ── Type mapping tests ─────────────────────────────────

    #[test]
    fn test_map_type() {
        let rust = compile("type Env = {\n  vars: Map<String, Int>\n}");
        assert!(rust.contains("std::collections::HashMap<String, i64>"));
    }

    #[test]
    fn test_set_type() {
        let rust = compile("type Tags = {\n  items: Set<String>\n}");
        assert!(rust.contains("std::collections::HashSet<String>"));
    }

    // ── String processing tests ────────────────────────────

    #[test]
    fn test_trim_start() {
        let rust = compile(r#"fn main() = trim_start("  hi")"#);
        assert!(rust.contains(".trim_start().to_string()"));
    }

    #[test]
    fn test_trim_end() {
        let rust = compile(r#"fn main() = trim_end("hi  ")"#);
        assert!(rust.contains(".trim_end().to_string()"));
    }

    #[test]
    fn test_substring() {
        let rust = compile(r#"fn main() = substring("hello", 1, 3)"#);
        assert!(rust.contains(".chars().collect()"));
        assert!(rust.contains("as usize.."));
    }

    #[test]
    fn test_index_of() {
        let rust = compile(r#"fn main() = index_of("hello", "ll")"#);
        assert!(rust.contains(".find("));
        assert!(rust.contains(".map(|i| i as i64)"));
    }

    #[test]
    fn test_capitalize() {
        let rust = compile(r#"fn main() = capitalize("hello")"#);
        assert!(rust.contains("f.to_uppercase()"));
    }

    #[test]
    fn test_pad_left() {
        let rust = compile(r#"fn main() = pad_left("hi", 5)"#);
        assert!(rust.contains("repeat"));
    }

    #[test]
    fn test_repeat_string() {
        let rust = compile(r#"fn main() = repeat("ab", 3)"#);
        assert!(rust.contains(".repeat("));
    }

    #[test]
    fn test_is_empty() {
        let rust = compile(r#"fn main() = is_empty("")"#);
        assert!(rust.contains(".is_empty()"));
    }

    #[test]
    fn test_is_blank() {
        let rust = compile(r#"fn main() = is_blank("  ")"#);
        assert!(rust.contains(".trim().is_empty()"));
    }

    #[test]
    fn test_reverse_string() {
        let rust = compile(r#"fn main() = reverse_string("abc")"#);
        assert!(rust.contains(".chars().rev().collect::<String>()"));
    }

    #[test]
    fn test_lines() {
        let rust = compile(r#"fn main() = lines("a\nb")"#);
        assert!(rust.contains(".lines()"));
    }

    #[test]
    fn test_words() {
        let rust = compile(r#"fn main() = words("hello world")"#);
        assert!(rust.contains(".split_whitespace()"));
    }

    #[test]
    fn test_is_numeric() {
        let rust = compile(r#"fn main() = is_numeric("123")"#);
        assert!(rust.contains("is_ascii_digit"));
    }

    #[test]
    fn test_regex_match() {
        let rust = compile(r#"fn main() = regex_match("hello", "h.*o")"#);
        assert!(rust.contains("regex::Regex::new("));
        assert!(rust.contains(".is_match("));
    }

    #[test]
    fn test_regex_find_all() {
        let rust = compile(r#"fn main() = regex_find_all("abc123def456", "[0-9]+")"#);
        assert!(rust.contains(".find_iter("));
    }

    #[test]
    fn test_bytes() {
        let rust = compile(r#"fn main() = bytes("hi")"#);
        assert!(rust.contains(".bytes().map(|b| b as i64)"));
    }

    #[test]
    fn test_char_code() {
        let rust = compile(r#"fn main() = char_code("A")"#);
        assert!(rust.contains(".chars().next().map(|c| c as i64)"));
    }

    #[test]
    fn test_encode_base64() {
        let rust = compile(r#"fn main() = encode_base64("hello")"#);
        assert!(rust.contains("base64::engine::general_purpose::STANDARD.encode("));
    }

    #[test]
    fn test_strip_prefix() {
        let rust = compile(r#"fn main() = strip_prefix("hello world", "hello ")"#);
        assert!(rust.contains(".strip_prefix("));
    }

    // ── I/O & File system tests ────────────────────────────

    #[test]
    fn test_read_line() {
        let rust = compile("fn main() = read_line()");
        assert!(rust.contains("std::io::stdin().read_line("));
    }

    #[test]
    fn test_read_file() {
        let rust = compile(r#"fn main() = read_file("test.txt")"#);
        assert!(rust.contains("std::fs::read_to_string("));
        assert!(rust.contains(".map_err("));
    }

    #[test]
    fn test_write_file() {
        let rust = compile(r#"fn main() = write_file("out.txt", "data")"#);
        assert!(rust.contains("std::fs::write("));
    }

    #[test]
    fn test_append_file() {
        let rust = compile(r#"fn main() = append_file("log.txt", "entry")"#);
        assert!(rust.contains("OpenOptions::new().append(true)"));
    }

    #[test]
    fn test_file_exists() {
        let rust = compile(r#"fn main() = file_exists("test.txt")"#);
        assert!(rust.contains("Path::new("));
        assert!(rust.contains(".exists()"));
    }

    #[test]
    fn test_delete_file() {
        let rust = compile(r#"fn main() = delete_file("tmp.txt")"#);
        assert!(rust.contains("std::fs::remove_file("));
    }

    #[test]
    fn test_read_lines() {
        let rust = compile(r#"fn main() = read_lines("data.txt")"#);
        assert!(rust.contains(".lines()"));
    }

    #[test]
    fn test_list_dir() {
        let rust = compile(r#"fn main() = list_dir(".")"#);
        assert!(rust.contains("std::fs::read_dir("));
    }

    #[test]
    fn test_create_dir_all() {
        let rust = compile(r#"fn main() = create_dir_all("a/b/c")"#);
        assert!(rust.contains("std::fs::create_dir_all("));
    }

    #[test]
    fn test_dir_exists() {
        let rust = compile(r#"fn main() = dir_exists("src")"#);
        assert!(rust.contains(".is_dir()"));
    }

    #[test]
    fn test_path_join() {
        let rust = compile(r#"fn main() = path_join("/usr", "local")"#);
        assert!(rust.contains(".join("));
        assert!(rust.contains("to_string_lossy"));
    }

    #[test]
    fn test_path_parent() {
        let rust = compile(r#"fn main() = path_parent("/usr/local/bin")"#);
        assert!(rust.contains(".parent()"));
    }

    #[test]
    fn test_path_filename() {
        let rust = compile(r#"fn main() = path_filename("/usr/local/bin")"#);
        assert!(rust.contains(".file_name()"));
    }

    #[test]
    fn test_path_extension() {
        let rust = compile(r#"fn main() = path_extension("file.txt")"#);
        assert!(rust.contains(".extension()"));
    }

    #[test]
    fn test_env_get() {
        let rust = compile(r#"fn main() = env_get("HOME")"#);
        assert!(rust.contains("std::env::var("));
    }

    #[test]
    fn test_env_vars() {
        let rust = compile("fn main() = env_vars()");
        assert!(rust.contains("std::env::vars()"));
    }

    #[test]
    fn test_current_dir() {
        let rust = compile("fn main() = current_dir()");
        assert!(rust.contains("std::env::current_dir()"));
    }

    #[test]
    fn test_args() {
        let rust = compile("fn main() = args()");
        assert!(rust.contains("std::env::args()"));
    }

    #[test]
    fn test_command() {
        let rust = compile(r#"fn main() = command("echo hi")"#);
        assert!(rust.contains("Command::new(\"sh\")"));
        assert!(rust.contains(".status()"));
    }

    #[test]
    fn test_command_output() {
        let rust = compile(r#"fn main() = command_output("echo hi")"#);
        assert!(rust.contains("Command::new(\"sh\")"));
        assert!(rust.contains(".output()"));
    }

    #[test]
    fn test_copy_file() {
        let rust = compile(r#"fn main() = copy_file("a.txt", "b.txt")"#);
        assert!(rust.contains("std::fs::copy("));
    }

    #[test]
    fn test_rename_file() {
        let rust = compile(r#"fn main() = rename_file("old.txt", "new.txt")"#);
        assert!(rust.contains("std::fs::rename("));
    }

    #[test]
    fn test_file_size() {
        let rust = compile(r#"fn main() = file_size("test.txt")"#);
        assert!(rust.contains("std::fs::metadata("));
        assert!(rust.contains(".len() as i64"));
    }

    #[test]
    fn test_path_stem() {
        let rust = compile(r#"fn main() = path_stem("file.txt")"#);
        assert!(rust.contains(".file_stem()"));
    }

    #[test]
    fn test_path_is_absolute() {
        let rust = compile(r#"fn main() = path_is_absolute("/usr")"#);
        assert!(rust.contains(".is_absolute()"));
    }

    // ── Concurrency tests ────────────────────────────────────

    #[test]
    fn test_spawn() {
        let rust = compile("fn main() = spawn(f)");
        assert!(rust.contains("std::thread::spawn(move || { ("));
    }

    #[test]
    fn test_spawn_join() {
        let rust = compile("fn main() = spawn_join(f)");
        assert!(rust.contains("std::thread::spawn(move || { ("));
        assert!(rust.contains(").join().map_err("));
    }

    #[test]
    fn test_channel() {
        let rust = compile("fn main() = channel()");
        assert!(rust.contains("std::sync::mpsc::channel()"));
    }

    #[test]
    fn test_send() {
        let rust = compile("fn main() = send(tx, 42)");
        assert!(rust.contains(".send("));
    }

    #[test]
    fn test_recv() {
        let rust = compile("fn main() = recv(rx)");
        assert!(rust.contains(".recv().map_err("));
    }

    #[test]
    fn test_try_recv() {
        let rust = compile("fn main() = try_recv(rx)");
        assert!(rust.contains(".try_recv().ok()"));
    }

    #[test]
    fn test_mutex_new() {
        let rust = compile("fn main() = mutex_new(0)");
        assert!(rust.contains("Arc::new(std::sync::Mutex::new("));
    }

    #[test]
    fn test_mutex_lock() {
        let rust = compile("fn main() = mutex_lock(m)");
        assert!(rust.contains(".lock().unwrap().clone()"));
    }

    #[test]
    fn test_rwlock_new() {
        let rust = compile("fn main() = rwlock_new(0)");
        assert!(rust.contains("Arc::new(std::sync::RwLock::new("));
    }

    #[test]
    fn test_atomic_new() {
        let rust = compile("fn main() = atomic_new(0)");
        assert!(rust.contains("AtomicI64::new("));
    }

    #[test]
    fn test_atomic_get() {
        let rust = compile("fn main() = atomic_get(a)");
        assert!(rust.contains(".load(std::sync::atomic::Ordering::SeqCst)"));
    }

    #[test]
    fn test_atomic_add() {
        let rust = compile("fn main() = atomic_add(a, 1)");
        assert!(rust.contains(".fetch_add("));
    }

    #[test]
    fn test_sleep() {
        let rust = compile("fn main() = sleep(1)");
        assert!(rust.contains("tokio::time::sleep("));
        assert!(rust.contains(".await"));
    }

    #[test]
    fn test_sleep_ms() {
        let rust = compile("fn main() = sleep_ms(100)");
        assert!(rust.contains("Duration::from_millis("));
        assert!(rust.contains(".await"));
    }

    #[test]
    fn test_timeout() {
        let rust = compile("fn main() = timeout(5, fut)");
        assert!(rust.contains("tokio::time::timeout("));
    }

    #[test]
    fn test_spawn_async() {
        let rust = compile("fn main() = spawn_async(f)");
        assert!(rust.contains("tokio::spawn(async move { ("));
    }

    #[test]
    fn test_spawn_blocking() {
        let rust = compile("fn main() = spawn_blocking(f)");
        assert!(rust.contains("tokio::task::spawn_blocking("));
    }

    #[test]
    fn test_parallel_map() {
        let rust = compile("fn main() = parallel_map(items, f)");
        assert!(rust.contains("std::thread::spawn("));
        assert!(rust.contains(".join().unwrap()"));
    }

    #[test]
    fn test_async_function() {
        let rust = compile("async fn fetch() = 42");
        assert!(rust.contains("async fn fetch()"));
    }

    #[test]
    fn test_async_main() {
        let rust = compile("async fn main() = println(42)");
        assert!(rust.contains("#[tokio::main]"));
        assert!(rust.contains("async fn main()"));
    }

    #[test]
    fn test_await_expr() {
        let rust = compile("fn main() = f().await");
        assert!(rust.contains(".await"));
    }

    // ── Error handling tests ─────────────────────────────────

    #[test]
    fn test_unwrap() {
        let rust = compile("fn main() = unwrap(x)");
        assert!(rust.contains(".unwrap()"));
    }

    #[test]
    fn test_unwrap_or() {
        let rust = compile("fn main() = unwrap_or(x, 0)");
        assert!(rust.contains(".unwrap_or("));
    }

    #[test]
    fn test_unwrap_or_else() {
        let rust = compile("fn main() = unwrap_or_else(x, fn(e) => 0)");
        assert!(rust.contains(".unwrap_or_else("));
    }

    #[test]
    fn test_expect() {
        let rust = compile(r#"fn main() = expect(x, "failed")"#);
        assert!(rust.contains(".expect("));
    }

    #[test]
    fn test_unwrap_err() {
        let rust = compile("fn main() = unwrap_err(x)");
        assert!(rust.contains(".unwrap_err()"));
    }

    #[test]
    fn test_map_result() {
        let rust = compile("fn main() = map_result(x, fn(v) => v + 1)");
        assert!(rust.contains(".map(|_v| ("));
    }

    #[test]
    fn test_map_option() {
        let rust = compile("fn main() = map_option(x, fn(v) => v + 1)");
        assert!(rust.contains(".map(|_v| ("));
    }

    #[test]
    fn test_map_err() {
        let rust = compile(r#"fn main() = map_err(x, fn(e) => "wrapped")"#);
        assert!(rust.contains(".map_err(|_e| ("));
    }

    #[test]
    fn test_and_then() {
        let rust = compile("fn main() = and_then(x, fn(v) => v)");
        assert!(rust.contains(".and_then(|_v| ("));
    }

    #[test]
    fn test_or_default() {
        let rust = compile("fn main() = or_default(x)");
        assert!(rust.contains(".unwrap_or_default()"));
    }

    #[test]
    fn test_flatten_result() {
        let rust = compile("fn main() = flatten_result(x)");
        assert!(rust.contains(".and_then(|_v| _v)"));
    }

    #[test]
    fn test_flatten_option() {
        let rust = compile("fn main() = flatten_option(x)");
        assert!(rust.contains(".flatten()"));
    }

    #[test]
    fn test_ok_or() {
        let rust = compile(r#"fn main() = ok_or(x, "missing")"#);
        assert!(rust.contains(".ok_or("));
    }

    #[test]
    fn test_ok_or_else() {
        let rust = compile(r#"fn main() = ok_or_else(x, fn() => "missing")"#);
        assert!(rust.contains(".ok_or_else("));
    }

    #[test]
    fn test_some() {
        let rust = compile("fn main() = some(42)");
        assert!(rust.contains("Some("));
    }

    #[test]
    fn test_none() {
        let rust = compile("fn main() = none()");
        assert!(rust.contains("None"));
    }

    #[test]
    fn test_transpose() {
        let rust = compile("fn main() = transpose(x)");
        assert!(rust.contains(".transpose()"));
    }

    #[test]
    fn test_try_operator() {
        let rust = compile("fn main() = read_file(\"test.txt\")?");
        assert!(rust.contains("?"));
    }

    #[test]
    fn test_is_ok() {
        let rust = compile("fn main() = is_ok(x)");
        assert!(rust.contains(".is_ok()"));
    }

    #[test]
    fn test_is_err() {
        let rust = compile("fn main() = is_err(x)");
        assert!(rust.contains(".is_err()"));
    }

    #[test]
    fn test_is_some() {
        let rust = compile("fn main() = is_some(x)");
        assert!(rust.contains(".is_some()"));
    }

    #[test]
    fn test_is_none() {
        let rust = compile("fn main() = is_none(x)");
        assert!(rust.contains(".is_none()"));
    }

    #[test]
    fn test_ok_to_option() {
        let rust = compile("fn main() = ok(x)");
        assert!(rust.contains(".ok()"));
    }

    #[test]
    fn test_err_to_option() {
        let rust = compile("fn main() = err(x)");
        assert!(rust.contains(".err()"));
    }

    #[test]
    fn test_map_or() {
        let rust = compile("fn main() = map_or(x, 0, fn(v) => v + 1)");
        assert!(rust.contains(".map_or("));
    }

    #[test]
    fn test_or_else_builtin() {
        let rust = compile("fn main() = or_else(x, fn(e) => e)");
        assert!(rust.contains(".or_else("));
    }

    // ── Math & numeric tests ─────────────────────────────────

    #[test]
    fn test_sin() {
        let rust = compile("fn main() = sin(1.0)");
        assert!(rust.contains("as f64).sin()"));
    }

    #[test]
    fn test_cos() {
        let rust = compile("fn main() = cos(0.0)");
        assert!(rust.contains("as f64).cos()"));
    }

    #[test]
    fn test_tan() {
        let rust = compile("fn main() = tan(1.0)");
        assert!(rust.contains("as f64).tan()"));
    }

    #[test]
    fn test_asin() {
        let rust = compile("fn main() = asin(0.5)");
        assert!(rust.contains("as f64).asin()"));
    }

    #[test]
    fn test_atan2() {
        let rust = compile("fn main() = atan2(1.0, 2.0)");
        assert!(rust.contains("as f64).atan2("));
    }

    #[test]
    fn test_floor() {
        let rust = compile("fn main() = floor(3.7)");
        assert!(rust.contains("as f64).floor()"));
    }

    #[test]
    fn test_ceil() {
        let rust = compile("fn main() = ceil(3.2)");
        assert!(rust.contains("as f64).ceil()"));
    }

    #[test]
    fn test_round() {
        let rust = compile("fn main() = round(3.5)");
        assert!(rust.contains("as f64).round()"));
    }

    #[test]
    fn test_truncate() {
        let rust = compile("fn main() = truncate(3.9)");
        assert!(rust.contains("as f64).trunc()"));
    }

    #[test]
    fn test_log() {
        let rust = compile("fn main() = log(2.718)");
        assert!(rust.contains("as f64).ln()"));
    }

    #[test]
    fn test_log2() {
        let rust = compile("fn main() = log2(8.0)");
        assert!(rust.contains("as f64).log2()"));
    }

    #[test]
    fn test_log10() {
        let rust = compile("fn main() = log10(100.0)");
        assert!(rust.contains("as f64).log10()"));
    }

    #[test]
    fn test_exp() {
        let rust = compile("fn main() = exp(1.0)");
        assert!(rust.contains("as f64).exp()"));
    }

    #[test]
    fn test_signum() {
        let rust = compile("fn main() = signum(5.0)");
        assert!(rust.contains("as f64).signum()"));
    }

    #[test]
    fn test_hypot() {
        let rust = compile("fn main() = hypot(3.0, 4.0)");
        assert!(rust.contains("as f64).hypot("));
    }

    #[test]
    fn test_cbrt() {
        let rust = compile("fn main() = cbrt(27.0)");
        assert!(rust.contains("as f64).cbrt()"));
    }

    #[test]
    fn test_pi() {
        let rust = compile("fn main() = pi()");
        assert!(rust.contains("std::f64::consts::PI"));
    }

    #[test]
    fn test_e_const() {
        let rust = compile("fn main() = e_const()");
        assert!(rust.contains("std::f64::consts::E"));
    }

    #[test]
    fn test_infinity() {
        let rust = compile("fn main() = infinity()");
        assert!(rust.contains("f64::INFINITY"));
    }

    #[test]
    fn test_is_nan() {
        let rust = compile("fn main() = is_nan(x)");
        assert!(rust.contains("as f64).is_nan()"));
    }

    #[test]
    fn test_is_finite() {
        let rust = compile("fn main() = is_finite(x)");
        assert!(rust.contains("as f64).is_finite()"));
    }

    #[test]
    fn test_to_radians() {
        let rust = compile("fn main() = to_radians(180.0)");
        assert!(rust.contains("as f64).to_radians()"));
    }

    #[test]
    fn test_to_degrees() {
        let rust = compile("fn main() = to_degrees(3.14)");
        assert!(rust.contains("as f64).to_degrees()"));
    }

    #[test]
    fn test_random() {
        let rust = compile("fn main() = random()");
        assert!(rust.contains("SystemTime::now()"));
    }

    #[test]
    fn test_random_range() {
        let rust = compile("fn main() = random_range(1, 10)");
        assert!(rust.contains("_min"));
        assert!(rust.contains("_max"));
    }

    #[test]
    fn test_gcd() {
        let rust = compile("fn main() = gcd(12, 8)");
        assert!(rust.contains("while _b != 0"));
    }

    #[test]
    fn test_lcm() {
        let rust = compile("fn main() = lcm(4, 6)");
        assert!(rust.contains("while _y != 0"));
    }

    // ── Date & Time tests ────────────────────────────────────

    #[test]
    fn test_now() {
        let rust = compile("fn main() = now()");
        assert!(rust.contains("SystemTime::now()"));
        assert!(rust.contains("as_secs()"));
    }

    #[test]
    fn test_now_ms() {
        let rust = compile("fn main() = now_ms()");
        assert!(rust.contains("as_millis()"));
    }

    #[test]
    fn test_now_ns() {
        let rust = compile("fn main() = now_ns()");
        assert!(rust.contains("as_nanos()"));
    }

    #[test]
    fn test_monotonic() {
        let rust = compile("fn main() = monotonic()");
        assert!(rust.contains("Instant::now()"));
    }

    #[test]
    fn test_elapsed() {
        let rust = compile("fn main() = elapsed(t)");
        assert!(rust.contains(".elapsed().as_secs_f64()"));
    }

    #[test]
    fn test_elapsed_ms() {
        let rust = compile("fn main() = elapsed_ms(t)");
        assert!(rust.contains(".elapsed().as_millis()"));
    }

    #[test]
    fn test_format_timestamp() {
        let rust = compile("fn main() = format_timestamp(0)");
        assert!(rust.contains("format!(\"{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z\""));
    }

    #[test]
    fn test_parse_timestamp() {
        let rust = compile(r#"fn main() = parse_timestamp("2024-01-01T00:00:00Z")"#);
        assert!(rust.contains("split('T')"));
    }

    #[test]
    fn test_timestamp_secs() {
        let rust = compile("fn main() = timestamp_secs(1000)");
        assert!(rust.contains("UNIX_EPOCH"));
        assert!(rust.contains("Duration::from_secs("));
    }

    #[test]
    fn test_duration_secs() {
        let rust = compile("fn main() = duration_secs(5)");
        assert!(rust.contains("Duration::from_secs("));
    }

    #[test]
    fn test_duration_ms() {
        let rust = compile("fn main() = duration_ms(100)");
        assert!(rust.contains("Duration::from_millis("));
    }

    #[test]
    fn test_sleep_secs() {
        let rust = compile("fn main() = sleep_secs(1)");
        assert!(rust.contains("std::thread::sleep("));
        assert!(rust.contains("Duration::from_secs("));
    }

    #[test]
    fn test_sleep_millis() {
        let rust = compile("fn main() = sleep_millis(100)");
        assert!(rust.contains("std::thread::sleep("));
        assert!(rust.contains("Duration::from_millis("));
    }

    // ── Networking tests ─────────────────────────────────────

    #[test]
    fn test_tcp_connect() {
        let rust = compile(r#"fn main() = tcp_connect("127.0.0.1:8080")"#);
        assert!(rust.contains("TcpStream::connect("));
    }

    #[test]
    fn test_tcp_listen() {
        let rust = compile(r#"fn main() = tcp_listen("127.0.0.1:8080")"#);
        assert!(rust.contains("TcpListener::bind("));
    }

    #[test]
    fn test_tcp_accept() {
        let rust = compile("fn main() = tcp_accept(listener)");
        assert!(rust.contains(".accept()"));
    }

    #[test]
    fn test_tcp_read() {
        let rust = compile("fn main() = tcp_read(stream, 1024)");
        assert!(rust.contains("_s.read(&mut _buf)"));
    }

    #[test]
    fn test_tcp_write() {
        let rust = compile(r#"fn main() = tcp_write(stream, "hello")"#);
        assert!(rust.contains("_s.write("));
    }

    #[test]
    fn test_tcp_close() {
        let rust = compile("fn main() = tcp_close(stream)");
        assert!(rust.contains(".shutdown("));
    }

    #[test]
    fn test_tcp_read_line() {
        let rust = compile("fn main() = tcp_read_line(stream)");
        assert!(rust.contains("BufReader"));
        assert!(rust.contains("read_line"));
    }

    #[test]
    fn test_tcp_write_line() {
        let rust = compile(r#"fn main() = tcp_write_line(stream, "hello")"#);
        assert!(rust.contains("write!(_s"));
    }

    #[test]
    fn test_tcp_set_timeout() {
        let rust = compile("fn main() = tcp_set_timeout(stream, 5000)");
        assert!(rust.contains("set_read_timeout"));
        assert!(rust.contains("set_write_timeout"));
    }

    #[test]
    fn test_udp_bind() {
        let rust = compile(r#"fn main() = udp_bind("0.0.0.0:9000")"#);
        assert!(rust.contains("UdpSocket::bind("));
    }

    #[test]
    fn test_udp_send_to() {
        let rust = compile(r#"fn main() = udp_send_to(sock, "hi", "127.0.0.1:9000")"#);
        assert!(rust.contains(".send_to("));
    }

    #[test]
    fn test_udp_recv_from() {
        let rust = compile("fn main() = udp_recv_from(sock, 1024)");
        assert!(rust.contains(".recv_from("));
    }

    #[test]
    fn test_dns_lookup() {
        let rust = compile(r#"fn main() = dns_lookup("localhost:0")"#);
        assert!(rust.contains("to_socket_addrs"));
    }

    #[test]
    fn test_url_parse() {
        let rust = compile(r#"fn main() = url_parse("http://example.com/path")"#);
        assert!(rust.contains("split_once(\"://\")"));
    }

    #[test]
    fn test_http_get() {
        let rust = compile(r#"fn main() = http_get("http://example.com")"#);
        assert!(rust.contains("_star_http(\"GET\""));
        assert!(rust.contains("TcpStream::connect"));
    }

    #[test]
    fn test_http_method() {
        let rust = compile(r#"fn main() = http("POST", "http://example.com/api")"#);
        assert!(rust.contains("_star_http(_method"));
    }

    #[test]
    fn test_http_with_body() {
        let rust = compile(r#"fn main() = http("PUT", "http://example.com/api", "data")"#);
        assert!(rust.contains("_star_http(_method"));
        assert!(rust.contains("_body"));
    }

    #[test]
    fn test_http_with_headers() {
        let rust = compile(r#"fn main() = http_with_headers("POST", "http://example.com", ["Content-Type: application/json"], "{}")"#);
        assert!(rust.contains("_star_http(_method"));
        assert!(rust.contains("_hdr_refs"));
    }

    // ── OS & Environment Interaction tests ───────────────────────────

    #[test]
    fn test_process_id() {
        let rust = compile("fn main() = process_id()");
        assert!(rust.contains("std::process::id() as i64"));
    }

    #[test]
    fn test_command_with_stdin() {
        let rust = compile(r#"fn main() = command_with_stdin("cat", "hello")"#);
        assert!(rust.contains("Stdio::piped()"));
        assert!(rust.contains("write_all"));
    }

    #[test]
    fn test_command_with_args() {
        let rust = compile(r#"fn main() = command_with_args("echo", ["hello", "world"])"#);
        assert!(rust.contains("Command::new"));
        assert!(rust.contains(".args("));
    }

    #[test]
    fn test_command_with_args_output() {
        let rust = compile(r#"fn main() = command_with_args_output("echo", ["hello"])"#);
        assert!(rust.contains("Command::new"));
        assert!(rust.contains(".args("));
        assert!(rust.contains("from_utf8_lossy"));
    }

    #[test]
    fn test_kill_process() {
        let rust = compile("fn main() = kill_process(1234)");
        assert!(rust.contains("kill"));
    }

    #[test]
    fn test_is_file() {
        let rust = compile(r#"fn main() = is_file("/tmp/test.txt")"#);
        assert!(rust.contains("Path::new"));
        assert!(rust.contains(".is_file()"));
    }

    #[test]
    fn test_is_dir() {
        let rust = compile(r#"fn main() = is_dir("/tmp")"#);
        assert!(rust.contains("Path::new"));
        assert!(rust.contains(".is_dir()"));
    }

    #[test]
    fn test_is_symlink() {
        let rust = compile(r#"fn main() = is_symlink("/tmp/link")"#);
        assert!(rust.contains("Path::new"));
        assert!(rust.contains(".is_symlink()"));
    }

    #[test]
    fn test_file_modified() {
        let rust = compile(r#"fn main() = file_modified("/tmp/test.txt")"#);
        assert!(rust.contains("metadata"));
        assert!(rust.contains(".modified()"));
    }

    #[test]
    fn test_file_created() {
        let rust = compile(r#"fn main() = file_created("/tmp/test.txt")"#);
        assert!(rust.contains("metadata"));
        assert!(rust.contains(".created()"));
    }

    #[test]
    fn test_file_readonly() {
        let rust = compile(r#"fn main() = file_readonly("/tmp/test.txt")"#);
        assert!(rust.contains("metadata"));
        assert!(rust.contains(".readonly()"));
    }

    #[test]
    fn test_set_readonly() {
        let rust = compile(r#"fn main() = set_readonly("/tmp/test.txt", true)"#);
        assert!(rust.contains("set_readonly"));
        assert!(rust.contains("set_permissions"));
    }

    #[test]
    fn test_symlink() {
        let rust = compile(r#"fn main() = symlink("/tmp/src", "/tmp/dst")"#);
        assert!(rust.contains("symlink"));
    }

    #[test]
    fn test_read_link() {
        let rust = compile(r#"fn main() = read_link("/tmp/link")"#);
        assert!(rust.contains("read_link"));
    }

    #[test]
    fn test_canonicalize() {
        let rust = compile(r#"fn main() = canonicalize("./src")"#);
        assert!(rust.contains("canonicalize"));
    }

    #[test]
    fn test_temp_dir() {
        let rust = compile("fn main() = temp_dir()");
        assert!(rust.contains("temp_dir()"));
    }

    #[test]
    fn test_exe_path() {
        let rust = compile("fn main() = exe_path()");
        assert!(rust.contains("current_exe()"));
    }

    // ── Testing & debugging tests ───────────────────────────────────

    #[test]
    fn test_assert() {
        let rust = compile("fn main() = assert(true)");
        assert!(rust.contains("assert!(true"));
    }

    #[test]
    fn test_assert_msg() {
        let rust = compile(r#"fn main() = assert_msg(true, "should be true")"#);
        assert!(rust.contains("assert!(true"));
        assert!(rust.contains("should be true"));
    }

    #[test]
    fn test_assert_eq() {
        let rust = compile("fn main() = assert_eq(1, 1)");
        assert!(rust.contains("assert_eq!(1i64, 1i64)"));
    }

    #[test]
    fn test_assert_ne() {
        let rust = compile("fn main() = assert_ne(1, 2)");
        assert!(rust.contains("assert_ne!(1i64, 2i64)"));
    }

    #[test]
    fn test_log_debug() {
        let rust = compile(r#"fn main() = log_debug("test")"#);
        assert!(rust.contains("[DEBUG]"));
        assert!(rust.contains("eprintln!"));
    }

    #[test]
    fn test_log_info() {
        let rust = compile(r#"fn main() = log_info("test")"#);
        assert!(rust.contains("[INFO]"));
        assert!(rust.contains("eprintln!"));
    }

    #[test]
    fn test_log_warn() {
        let rust = compile(r#"fn main() = log_warn("test")"#);
        assert!(rust.contains("[WARN]"));
        assert!(rust.contains("eprintln!"));
    }

    #[test]
    fn test_log_error() {
        let rust = compile(r#"fn main() = log_error("test")"#);
        assert!(rust.contains("[ERROR]"));
        assert!(rust.contains("eprintln!"));
    }

    #[test]
    fn test_time_fn() {
        let rust = compile("fn main() = time_fn(fn() => 42)");
        assert!(rust.contains("Instant::now()"));
        assert!(rust.contains("as_millis"));
    }

    #[test]
    fn test_bench() {
        let rust = compile("fn main() = bench(1000, fn() => 42)");
        assert!(rust.contains("Instant::now()"));
        assert!(rust.contains("as_secs_f64"));
    }

    #[test]
    fn test_dbg() {
        let rust = compile("fn main() = dbg(42)");
        assert!(rust.contains("[dbg]"));
        assert!(rust.contains("eprintln!"));
    }

    #[test]
    fn test_type_name_of() {
        let rust = compile(r#"fn main() = type_name_of("hello")"#);
        assert!(rust.contains("type_name"));
    }

    #[test]
    fn test_todo() {
        let rust = compile("fn main() = todo()");
        assert!(rust.contains("todo!()"));
    }

    #[test]
    fn test_todo_msg() {
        let rust = compile(r#"fn main() = todo_msg("not done")"#);
        assert!(rust.contains("todo!"));
        assert!(rust.contains("not done"));
    }

    #[test]
    fn test_unreachable_msg() {
        let rust = compile(r#"fn main() = unreachable_msg("impossible")"#);
        assert!(rust.contains("unreachable!"));
        assert!(rust.contains("impossible"));
    }

    // ── Collection algorithms & utilities tests ─────────────────────

    #[test]
    fn test_binary_search() {
        let rust = compile("fn main() = binary_search([1, 2, 3], 2)");
        assert!(rust.contains("binary_search"));
    }

    #[test]
    fn test_position() {
        let rust = compile("fn main() = position([1, 2, 3], fn(x) => x == 2)");
        assert!(rust.contains(".position("));
    }

    #[test]
    fn test_contains_element() {
        let rust = compile("fn main() = contains_element([1, 2, 3], 2)");
        assert!(rust.contains(".contains("));
    }

    #[test]
    fn test_sort_desc() {
        let rust = compile("fn main() = sort_desc([3, 1, 2])");
        assert!(rust.contains(".sort()"));
        assert!(rust.contains(".reverse()"));
    }

    #[test]
    fn test_sort_by_key() {
        let rust = compile("fn main() = sort_by_key([3, 1, 2], fn(x) => x)");
        assert!(rust.contains("sort_by_key"));
    }

    #[test]
    fn test_is_sorted() {
        let rust = compile("fn main() = is_sorted([1, 2, 3])");
        assert!(rust.contains(".windows(2)"));
    }

    #[test]
    fn test_chunks() {
        let rust = compile("fn main() = chunks([1, 2, 3, 4], 2)");
        assert!(rust.contains(".chunks("));
    }

    #[test]
    fn test_windows() {
        let rust = compile("fn main() = windows([1, 2, 3, 4], 2)");
        assert!(rust.contains(".windows("));
    }

    #[test]
    fn test_nth() {
        let rust = compile("fn main() = nth([1, 2, 3], 1)");
        assert!(rust.contains(".get("));
    }

    #[test]
    fn test_take_while() {
        let rust = compile("fn main() = take_while([1, 2, 3], fn(x) => x < 3)");
        assert!(rust.contains(".take_while("));
    }

    #[test]
    fn test_drop_while() {
        let rust = compile("fn main() = drop_while([1, 2, 3], fn(x) => x < 3)");
        assert!(rust.contains(".skip_while("));
    }

    #[test]
    fn test_split_at() {
        let rust = compile("fn main() = split_at([1, 2, 3, 4], 2)");
        assert!(rust.contains(".split_at("));
    }

    #[test]
    fn test_scan() {
        let rust = compile("fn main() = scan([1, 2, 3], 0, fn(acc, x) => acc + x)");
        assert!(rust.contains("_acc ="));
        assert!(rust.contains(".map("));
    }

    #[test]
    fn test_reduce() {
        let rust = compile("fn main() = reduce([1, 2, 3], fn(a, b) => a + b)");
        assert!(rust.contains(".reduce("));
    }

    #[test]
    fn test_partition() {
        let rust = compile("fn main() = partition([1, 2, 3, 4], fn(x) => x > 2)");
        assert!(rust.contains(".partition("));
    }

    #[test]
    fn test_group_by() {
        let rust = compile("fn main() = group_by([1, 2, 3, 4], fn(x) => x % 2)");
        assert!(rust.contains("HashMap"));
        assert!(rust.contains(".entry("));
    }

    #[test]
    fn test_unique() {
        let rust = compile("fn main() = unique([1, 2, 2, 3])");
        assert!(rust.contains("HashSet"));
    }

    #[test]
    fn test_intersperse() {
        let rust = compile("fn main() = intersperse([1, 2, 3], 0)");
        assert!(rust.contains("_sep"));
        assert!(rust.contains("_out.push"));
    }

    #[test]
    fn test_min_of() {
        let rust = compile("fn main() = min_of([3, 1, 2])");
        assert!(rust.contains(".min()"));
    }

    #[test]
    fn test_max_of() {
        let rust = compile("fn main() = max_of([3, 1, 2])");
        assert!(rust.contains(".max()"));
    }

    #[test]
    fn test_sum_float() {
        let rust = compile("fn main() = sum_float([1.0, 2.0, 3.0])");
        assert!(rust.contains("sum::<f64>()"));
    }

    #[test]
    fn test_product_float() {
        let rust = compile("fn main() = product_float([1.0, 2.0, 3.0])");
        assert!(rust.contains("product::<f64>()"));
    }

    #[test]
    fn test_unzip() {
        let rust = compile("fn main() = unzip([(1, 2), (3, 4)])");
        assert!(rust.contains(".unzip::<"));
    }

    #[test]
    fn test_zip_with() {
        let rust = compile("fn main() = zip_with([1, 2], [3, 4], fn(a, b) => a + b)");
        assert!(rust.contains(".zip("));
        assert!(rust.contains(".map("));
    }

    // ── Cryptography & security tests ───────────────────────────────

    #[test]
    fn test_sha256() {
        let rust = compile(r#"fn main() = sha256("hello")"#);
        assert!(rust.contains("_star_sha256"));
        assert!(rust.contains("0x6a09e667"));
    }

    #[test]
    fn test_sha512() {
        let rust = compile(r#"fn main() = sha512("hello")"#);
        assert!(rust.contains("_star_sha512"));
        assert!(rust.contains("0x6a09e667f3bcc908"));
    }

    #[test]
    fn test_md5() {
        let rust = compile(r#"fn main() = md5("hello")"#);
        assert!(rust.contains("_star_md5"));
        assert!(rust.contains("0x67452301"));
    }

    #[test]
    fn test_hash_bytes() {
        let rust = compile(r#"fn main() = hash_bytes("hello")"#);
        assert!(rust.contains("DefaultHasher::new()"));
        assert!(rust.contains(".hash("));
    }

    #[test]
    fn test_secure_random_bytes() {
        let rust = compile("fn main() = secure_random_bytes(16)");
        assert!(rust.contains("/dev/urandom"));
        assert!(rust.contains("read_exact"));
    }

    #[test]
    fn test_secure_random_hex() {
        let rust = compile("fn main() = secure_random_hex(16)");
        assert!(rust.contains("/dev/urandom"));
        assert!(rust.contains(":02x"));
    }

    #[test]
    fn test_uuid_v4() {
        let rust = compile("fn main() = uuid_v4()");
        assert!(rust.contains("/dev/urandom"));
        assert!(rust.contains("0x40"));
        assert!(rust.contains("0x80"));
    }

    // ── Configuration & CLI tests ───────────────────────────────────

    #[test]
    fn test_arg_get() {
        let rust = compile("fn main() = arg_get(0)");
        assert!(rust.contains("args().skip(1).nth("));
    }

    #[test]
    fn test_arg_count() {
        let rust = compile("fn main() = arg_count()");
        assert!(rust.contains("args().count()"));
    }

    #[test]
    fn test_arg_has() {
        let rust = compile(r#"fn main() = arg_has("--verbose")"#);
        assert!(rust.contains("args().any("));
    }

    #[test]
    fn test_arg_value() {
        let rust = compile(r#"fn main() = arg_value("--name")"#);
        assert!(rust.contains("_flag"));
        assert!(rust.contains("starts_with"));
    }

    #[test]
    fn test_arg_pairs() {
        let rust = compile("fn main() = arg_pairs()");
        assert!(rust.contains("args().skip(1)"));
        assert!(rust.contains("starts_with(\"--\")"));
    }

    #[test]
    fn test_json_get() {
        let rust = compile(r#"fn main() = json_get("{}", "key")"#);
        assert!(rust.contains("_star_json_get"));
    }

    #[test]
    fn test_json_object() {
        let rust = compile(r#"fn main() = json_object([("a", "b")])"#);
        assert!(rust.contains("format!"));
        assert!(rust.contains("join(\", \")"));
    }

    #[test]
    fn test_json_array() {
        let rust = compile(r#"fn main() = json_array(["a", "b"])"#);
        assert!(rust.contains("format!"));
        assert!(rust.contains("join(\", \")"));
    }

    #[test]
    fn test_json_escape() {
        let rust = compile(r#"fn main() = json_escape("hello")"#);
        assert!(rust.contains("replace"));
    }

    #[test]
    fn test_json_parse() {
        let rust = compile(r#"fn main() = json_parse("{\"a\": 1}")"#);
        assert!(rust.contains("_star_json_parse"));
        assert!(rust.contains("_StarJson"));
    }

    #[test]
    fn test_json_encode() {
        let rust = compile(r#"fn main() = json_encode("hello")"#);
        assert!(rust.contains("_star_json_encode"));
    }

    #[test]
    fn test_parse_env_string() {
        let rust = compile(r#"fn main() = parse_env_string("KEY=val")"#);
        assert!(rust.contains("splitn(2, '=')"));
    }

    #[test]
    fn test_load_env_file() {
        let rust = compile(r#"fn main() = load_env_file(".env")"#);
        assert!(rust.contains("read_to_string"));
        assert!(rust.contains("splitn(2, '=')"));
    }

    #[test]
    fn test_color_red() {
        let rust = compile(r#"fn main() = color_red("error")"#);
        assert!(rust.contains("31m"));
        assert!(rust.contains("0m"));
    }

    #[test]
    fn test_color_green() {
        let rust = compile(r#"fn main() = color_green("ok")"#);
        assert!(rust.contains("32m"));
    }

    #[test]
    fn test_color_blue() {
        let rust = compile(r#"fn main() = color_blue("info")"#);
        assert!(rust.contains("34m"));
    }

    #[test]
    fn test_color_yellow() {
        let rust = compile(r#"fn main() = color_yellow("warn")"#);
        assert!(rust.contains("33m"));
    }

    #[test]
    fn test_color_cyan() {
        let rust = compile(r#"fn main() = color_cyan("note")"#);
        assert!(rust.contains("36m"));
    }

    #[test]
    fn test_color_magenta() {
        let rust = compile(r#"fn main() = color_magenta("special")"#);
        assert!(rust.contains("35m"));
    }

    #[test]
    fn test_bold() {
        let rust = compile(r#"fn main() = bold("text")"#);
        assert!(rust.contains("1m"));
        assert!(rust.contains("0m"));
    }

    #[test]
    fn test_dim() {
        let rust = compile(r#"fn main() = dim("text")"#);
        assert!(rust.contains("2m"));
    }

    #[test]
    fn test_underline() {
        let rust = compile(r#"fn main() = underline("text")"#);
        assert!(rust.contains("4m"));
    }

    #[test]
    fn test_strip_ansi() {
        let rust = compile(r#"fn main() = strip_ansi("text")"#);
        assert!(rust.contains("_in_esc"));
    }

    #[test]
    fn test_prompt() {
        let rust = compile(r#"fn main() = prompt("name: ")"#);
        assert!(rust.contains("print!"));
        assert!(rust.contains("flush()"));
        assert!(rust.contains("read_line"));
    }

    #[test]
    fn test_confirm() {
        let rust = compile(r#"fn main() = confirm("sure?")"#);
        assert!(rust.contains("[y/n]"));
        assert!(rust.contains("read_line"));
    }

    #[test]
    fn test_clear_screen() {
        let rust = compile("fn main() = clear_screen()");
        assert!(rust.contains("2J"));
    }

    #[test]
    fn test_cursor_up() {
        let rust = compile("fn main() = cursor_up(3)");
        assert!(rust.contains("A\""));
    }

    #[test]
    fn test_cursor_down() {
        let rust = compile("fn main() = cursor_down(3)");
        assert!(rust.contains("B\""));
    }

    #[test]
    fn test_numeric_type_int8() {
        let rust = compile("fn to_byte(x: Int8): Int8 = x");
        assert!(rust.contains("fn to_byte(x: i8) -> i8"));
    }

    #[test]
    fn test_numeric_type_int16() {
        let rust = compile("fn short(x: Int16): Int16 = x");
        assert!(rust.contains("fn short(x: i16) -> i16"));
    }

    #[test]
    fn test_numeric_type_int32() {
        let rust = compile("fn mid(x: Int32): Int32 = x");
        assert!(rust.contains("fn mid(x: i32) -> i32"));
    }

    #[test]
    fn test_numeric_type_uint() {
        let rust = compile("fn wide(x: UInt): UInt = x");
        assert!(rust.contains("fn wide(x: u64) -> u64"));
    }

    #[test]
    fn test_numeric_type_uint8() {
        let rust = compile("fn byte(x: UInt8): UInt8 = x");
        assert!(rust.contains("fn byte(x: u8) -> u8"));
    }

    #[test]
    fn test_numeric_type_uint16() {
        let rust = compile("fn ushort(x: UInt16): UInt16 = x");
        assert!(rust.contains("fn ushort(x: u16) -> u16"));
    }

    #[test]
    fn test_numeric_type_uint32() {
        let rust = compile("fn umid(x: UInt32): UInt32 = x");
        assert!(rust.contains("fn umid(x: u32) -> u32"));
    }

    #[test]
    fn test_numeric_type_float32() {
        let rust = compile("fn half(x: Float32): Float32 = x");
        assert!(rust.contains("fn half(x: f32) -> f32"));
    }

    #[test]
    fn test_or_pattern() {
        let rust = compile(r#"fn f(x: Int): String = match x
  | 0 | 1 => "small"
  | _ => "big"
  end"#);
        assert!(rust.contains("0 | 1"));
    }

    #[test]
    fn test_range_pattern() {
        let rust = compile(r#"fn f(x: Int): String = match x
  | 1..10 => "range"
  | _ => "other"
  end"#);
        assert!(rust.contains("1i64..=10i64"));
    }

    #[test]
    fn test_string_pattern() {
        let rust = compile(r#"fn f(x: String): Int = match x
  | "hello" => 1
  | _ => 0
  end"#);
        assert!(rust.contains("\"hello\""));
    }

    #[test]
    fn test_or_pattern_with_constructors() {
        let rust = compile("type Color =\n  | Red\n  | Green\n  | Blue\n\nfn is_warm(c: Color): Bool = match c\n  | Red | Green => true\n  | Blue => false\n  end");
        assert!(rust.contains("Color::Red | Color::Green"));
    }

    // ── Edge case tests ─────────────────────────────────────────

    #[test]
    fn test_empty_program() {
        let rust = compile("");
        // Should at minimum produce the star_display helper
        assert!(rust.contains("fn star_display"));
    }

    #[test]
    fn test_unicode_in_string() {
        let rust = compile(r#"fn main() = println("Hello!")"#);
        assert!(rust.contains("Hello!"));
        assert!(rust.contains("println!"));
    }

    #[test]
    fn test_deeply_nested_arithmetic() {
        let rust = compile("fn main() = ((1 + 2) + (3 + 4)) + ((5 + 6) + (7 + 8))");
        assert!(rust.contains("fn main()"));
    }

    #[test]
    fn test_long_identifier_codegen() {
        let rust = compile("fn my_really_long_function_name_here(x: Int): Int = x");
        assert!(rust.contains("fn my_really_long_function_name_here"));
    }

    #[test]
    fn test_multiple_enum_variants_with_fields() {
        let rust = compile("type Tree =\n  | Leaf(Int)\n  | Node(Tree, Tree)");
        assert!(rust.contains("enum Tree"));
        assert!(rust.contains("Leaf(i64)"));
        assert!(rust.contains("Node("));
    }

    #[test]
    fn test_empty_list() {
        let rust = compile("fn main() = []");
        assert!(rust.contains("vec![]"));
    }

    #[test]
    fn test_single_element_list() {
        let rust = compile("fn main() = [42]");
        assert!(rust.contains("vec![42i64]"));
    }

    #[test]
    fn test_nested_function_calls() {
        let rust = compile("fn main() = f(g(h(1)))");
        assert!(rust.contains("f(g(h(1i64)))"));
    }

    #[test]
    fn test_chained_pipe() {
        let rust = compile("fn main() = 1 |> f |> g |> h");
        assert!(rust.contains("h(g(f(1i64)))"));
    }

    #[test]
    fn test_for_loop_codegen() {
        let rust = compile("fn main() = for x in [1, 2, 3] do\n  println(x)\nend");
        assert!(rust.contains("for x in"));
    }

    #[test]
    fn test_while_loop_codegen() {
        let rust = compile("fn main() = while true do\n  println(1)\nend");
        assert!(rust.contains("while true"));
    }

    #[test]
    fn test_break_codegen() {
        let rust = compile("fn main() = for x in [1, 2, 3] do\n  break\nend");
        assert!(rust.contains("break"));
    }

    #[test]
    fn test_continue_codegen() {
        let rust = compile("fn main() = for x in [1, 2, 3] do\n  continue\nend");
        assert!(rust.contains("continue"));
    }

    #[test]
    fn test_compound_assignment_plus_eq() {
        let rust = compile("fn main() = do\n  let mut x = 0\n  x += 5\n  x\nend");
        assert!(rust.contains("+= 5i64") || rust.contains("x = (x + 5i64)"));
    }

    #[test]
    fn test_tuple_codegen() {
        let rust = compile("fn main() = (1, 2, 3)");
        assert!(rust.contains("(1i64, 2i64, 3i64)"));
    }

    #[test]
    fn test_let_binding_codegen() {
        let rust = compile("fn main() = do\n  let x = 42\n  x\nend");
        assert!(rust.contains("let x = 42i64"));
    }

    #[test]
    fn test_mut_binding_codegen() {
        let rust = compile("fn main() = do\n  let mut x = 0\n  x = 1\n  x\nend");
        assert!(rust.contains("let mut x = 0i64"));
    }

    #[test]
    fn test_if_without_else_codegen() {
        let rust = compile("fn main() = if true then println(1) end");
        assert!(rust.contains("if true"));
    }

    #[test]
    fn test_match_wildcard_pattern() {
        let src = r#"fn f(x: Int): Int = match x
  | _ => 42
  end"#;
        let rust = compile(src);
        assert!(rust.contains("_ =>"));
    }

    #[test]
    fn test_match_bool_pattern() {
        let src = r#"fn f(x: Bool): String = match x
  | true => "yes"
  | false => "no"
  end"#;
        let rust = compile(src);
        assert!(rust.contains("true =>"));
        assert!(rust.contains("false =>"));
    }

    #[test]
    fn test_pub_function_codegen() {
        let rust = compile("pub fn hello(): String = \"world\"");
        assert!(rust.contains("pub fn hello() -> String"));
    }

    #[test]
    fn test_generic_function_codegen() {
        let rust = compile("fn identity<T>(x: T): T = x");
        assert!(rust.contains("fn identity<T>"));
        assert!(rust.contains("-> T"));
    }

    #[test]
    fn test_field_access_codegen() {
        let rust = compile("type Point = {\n  x: Float,\n  y: Float\n}\nfn get_x(p: Point): Float = p.x");
        assert!(rust.contains("p.x"));
    }

    #[test]
    fn test_struct_literal_codegen() {
        let rust = compile("type Point = {\n  x: Float,\n  y: Float\n}\nfn origin(): Point = Point { x: 0.0, y: 0.0 }");
        assert!(rust.contains("Point { x: 0.0f64, y: 0.0f64 }"));
    }

    #[test]
    fn test_index_access_codegen() {
        let rust = compile("fn main() = do\n  let xs = [10, 20, 30]\n  xs[1]\nend");
        assert!(rust.contains("as usize]"));
    }

    #[test]
    fn test_eprintln_builtin() {
        let rust = compile(r#"fn main() = eprintln("error")"#);
        assert!(rust.contains("eprintln!"));
    }

    #[test]
    fn test_print_no_newline() {
        let rust = compile(r#"fn main() = print("no newline")"#);
        assert!(rust.contains("print!"));
    }

    #[test]
    fn test_exit_builtin() {
        let rust = compile("fn main() = exit(0)");
        assert!(rust.contains("std::process::exit("));
    }

    #[test]
    fn test_panic_builtin() {
        let rust = compile(r#"fn main() = panic("oh no")"#);
        assert!(rust.contains("panic!"));
    }

    #[test]
    fn test_any_builtin() {
        let rust = compile("fn main() = any([1, 2, 3], fn(x) => x > 2)");
        assert!(rust.contains(".any("));
    }

    #[test]
    fn test_all_builtin() {
        let rust = compile("fn main() = all([1, 2, 3], fn(x) => x > 0)");
        assert!(rust.contains(".all("));
    }

    #[test]
    fn test_flat_map_builtin() {
        let rust = compile("fn main() = flat_map([1, 2], fn(x) => [x, x])");
        assert!(rust.contains(".flat_map("));
    }

    #[test]
    fn test_find_builtin() {
        let rust = compile("fn main() = find([1, 2, 3], fn(x) => x == 2)");
        assert!(rust.contains(".find("));
    }

    #[test]
    fn test_enumerate_builtin() {
        let rust = compile("fn main() = enumerate([10, 20, 30])");
        assert!(rust.contains(".enumerate()"));
    }

    #[test]
    fn test_take_builtin() {
        let rust = compile("fn main() = take([1, 2, 3, 4], 2)");
        assert!(rust.contains(".take("));
    }

    #[test]
    fn test_drop_builtin() {
        let rust = compile("fn main() = drop([1, 2, 3, 4], 2)");
        assert!(rust.contains(".skip("));
    }

    #[test]
    fn test_zip_builtin() {
        let rust = compile("fn main() = zip([1, 2], [3, 4])");
        assert!(rust.contains(".zip("));
    }

    #[test]
    fn test_head_builtin() {
        let rust = compile("fn main() = head([1, 2, 3])");
        assert!(rust.contains(".first()"));
    }

    #[test]
    fn test_tail_builtin() {
        let rust = compile("fn main() = tail([1, 2, 3])");
        assert!(rust.contains(".get(1..)"));
    }

    #[test]
    fn test_last_builtin() {
        let rust = compile("fn main() = last([1, 2, 3])");
        assert!(rust.contains(".last()"));
    }

    #[test]
    fn test_push_builtin() {
        let rust = compile("fn main() = push([1, 2], 3)");
        assert!(rust.contains(".push("));
    }

    #[test]
    fn test_concat_builtin() {
        let rust = compile("fn main() = concat([1, 2], [3, 4])");
        assert!(rust.contains(".extend("));
    }

    #[test]
    fn test_dedup_builtin() {
        let rust = compile("fn main() = dedup([1, 1, 2, 2])");
        assert!(rust.contains(".dedup()"));
    }

    #[test]
    fn test_product_builtin() {
        let rust = compile("fn main() = product([1, 2, 3])");
        assert!(rust.contains("product::<i64>()"));
    }

    #[test]
    fn test_contains_builtin() {
        let rust = compile(r#"fn main() = contains("hello world", "world")"#);
        assert!(rust.contains(".contains("));
    }

    #[test]
    fn test_replace_builtin() {
        let rust = compile(r#"fn main() = replace("hello", "l", "r")"#);
        assert!(rust.contains(".replace("));
    }

    #[test]
    fn test_uppercase_builtin() {
        let rust = compile(r#"fn main() = uppercase("hello")"#);
        assert!(rust.contains(".to_uppercase()"));
    }

    #[test]
    fn test_lowercase_builtin() {
        let rust = compile(r#"fn main() = lowercase("HELLO")"#);
        assert!(rust.contains(".to_lowercase()"));
    }

    #[test]
    fn test_starts_with_builtin() {
        let rust = compile(r#"fn main() = starts_with("hello", "he")"#);
        assert!(rust.contains(".starts_with("));
    }

    #[test]
    fn test_ends_with_builtin() {
        let rust = compile(r#"fn main() = ends_with("hello", "lo")"#);
        assert!(rust.contains(".ends_with("));
    }

    #[test]
    fn test_chars_builtin() {
        let rust = compile(r#"fn main() = chars("hello")"#);
        assert!(rust.contains(".chars()"));
    }

    #[test]
    fn test_to_int_builtin() {
        let rust = compile(r#"fn main() = to_int("42")"#);
        assert!(rust.contains(".parse::<i64>()"));
    }

    #[test]
    fn test_to_float_builtin() {
        let rust = compile(r#"fn main() = to_float("3.14")"#);
        assert!(rust.contains(".parse::<f64>()"));
    }

    #[test]
    fn test_clamp_builtin() {
        let rust = compile("fn main() = clamp(5, 1, 10)");
        assert!(rust.contains(".clamp("));
    }

    #[test]
    fn test_pow_builtin() {
        let rust = compile("fn main() = pow(2, 10)");
        assert!(rust.contains("powf("));
    }

    #[test]
    fn test_sqrt_builtin() {
        let rust = compile("fn main() = sqrt(4.0)");
        assert!(rust.contains("sqrt()"));
    }

    // ── Additional codegen tests ───────────────────────────────────

    #[test]
    fn test_string_interpolation_codegen() {
        let rust = compile(r#"fn main() = "hello #{name}""#);
        assert!(rust.contains("format!"));
    }

    #[test]
    fn test_list_map_codegen() {
        let rust = compile("fn main() = map([1, 2, 3], fn(x) => x * 2)");
        assert!(rust.contains(".map("));
        assert!(rust.contains("collect"));
    }

    #[test]
    fn test_list_filter_codegen() {
        let rust = compile("fn main() = filter([1, 2, 3], fn(x) => x > 1)");
        assert!(rust.contains(".filter("));
    }

    #[test]
    fn test_list_fold_codegen() {
        let rust = compile("fn main() = fold([1, 2, 3], 0, fn(acc, x) => acc + x)");
        assert!(rust.contains(".fold("));
    }

    #[test]
    fn test_each_codegen() {
        let rust = compile("fn main() = each([1, 2, 3], fn(x) => println(x))");
        assert!(rust.contains(".for_each(") || rust.contains("for "));
    }

    #[test]
    fn test_head_tail_codegen() {
        let rust = compile("fn main() = do\n  let xs = [1, 2, 3]\n  head(xs)\nend");
        assert!(rust.contains("first()") || rust.contains("[0]"));
    }

    #[test]
    fn test_push_codegen() {
        let rust = compile("fn main() = push([1, 2], 3)");
        assert!(rust.contains("push("));
    }

    #[test]
    fn test_concat_codegen() {
        let rust = compile("fn main() = concat([1, 2], [3, 4])");
        assert!(rust.contains("extend") || rust.contains("concat") || rust.contains("into_iter"));
    }

    #[test]
    fn test_reverse_codegen() {
        let rust = compile("fn main() = reverse([1, 2, 3])");
        assert!(rust.contains("reverse") || rust.contains("rev"));
    }

    #[test]
    fn test_sort_codegen() {
        let rust = compile("fn main() = sort([3, 1, 2])");
        assert!(rust.contains("sort"));
    }

    #[test]
    fn test_length_codegen() {
        let rust = compile("fn main() = length([1, 2, 3])");
        assert!(rust.contains(".len()"));
    }

    #[test]
    fn test_contains_element_codegen() {
        let rust = compile("fn main() = contains_element([1, 2, 3], 2)");
        assert!(rust.contains("contains"));
    }

    #[test]
    fn test_zip_codegen() {
        let rust = compile("fn main() = zip([1, 2], [3, 4])");
        assert!(rust.contains(".zip("));
    }

    #[test]
    fn test_enumerate_codegen() {
        let rust = compile("fn main() = enumerate([10, 20, 30])");
        assert!(rust.contains(".enumerate()"));
    }

    #[test]
    fn test_flatten_codegen() {
        let rust = compile("fn main() = flatten([[1, 2], [3, 4]])");
        assert!(rust.contains("flatten") || rust.contains("into_iter"));
    }

    #[test]
    fn test_take_drop_codegen() {
        let rust_take = compile("fn main() = take([1, 2, 3, 4], 2)");
        assert!(rust_take.contains("take(") || rust_take.contains("[.."));
        let rust_drop = compile("fn main() = drop([1, 2, 3, 4], 2)");
        assert!(rust_drop.contains("skip(") || rust_drop.contains("["));
    }

    #[test]
    fn test_any_all_codegen() {
        let rust_any = compile("fn main() = any([1, 2, 3], fn(x) => x > 2)");
        assert!(rust_any.contains(".any("));
        let rust_all = compile("fn main() = all([1, 2, 3], fn(x) => x > 0)");
        assert!(rust_all.contains(".all("));
    }

    #[test]
    fn test_find_codegen() {
        let rust = compile("fn main() = find([1, 2, 3], fn(x) => x == 2)");
        assert!(rust.contains(".find(") || rust.contains("find"));
    }

    #[test]
    fn test_string_trim_codegen() {
        let rust = compile(r#"fn main() = trim("  hello  ")"#);
        assert!(rust.contains(".trim()"));
    }

    #[test]
    fn test_string_split_codegen() {
        let rust = compile(r#"fn main() = split("a,b,c", ",")"#);
        assert!(rust.contains(".split("));
    }

    #[test]
    fn test_string_join_codegen() {
        let rust = compile(r#"fn main() = join(["a", "b", "c"], ", ")"#);
        assert!(rust.contains(".join("));
    }

    #[test]
    fn test_string_contains_codegen() {
        let rust = compile(r#"fn main() = contains("hello", "ell")"#);
        assert!(rust.contains(".contains("));
    }

    #[test]
    fn test_string_replace_codegen() {
        let rust = compile(r#"fn main() = replace("hello", "l", "r")"#);
        assert!(rust.contains(".replace("));
    }

    #[test]
    fn test_string_uppercase_lowercase() {
        let rust_up = compile(r#"fn main() = uppercase("hello")"#);
        assert!(rust_up.contains(".to_uppercase()") || rust_up.contains("to_uppercase"));
        let rust_low = compile(r#"fn main() = lowercase("HELLO")"#);
        assert!(rust_low.contains(".to_lowercase()") || rust_low.contains("to_lowercase"));
    }

    #[test]
    fn test_string_starts_ends_with() {
        let rust = compile(r#"fn main() = starts_with("hello", "he")"#);
        assert!(rust.contains(".starts_with("));
        let rust2 = compile(r#"fn main() = ends_with("hello", "lo")"#);
        assert!(rust2.contains(".ends_with("));
    }

    #[test]
    fn test_string_chars_codegen() {
        let rust = compile(r#"fn main() = chars("hello")"#);
        assert!(rust.contains(".chars()"));
    }

    #[test]
    fn test_string_length_codegen() {
        let rust = compile(r#"fn main() = string_length("hello")"#);
        assert!(rust.contains("len()") || rust.contains("string_length"));
    }

    #[test]
    fn test_to_string_codegen() {
        let rust = compile("fn main() = to_string(42)");
        assert!(rust.contains("to_string()") || rust.contains("star_display"));
    }

    #[test]
    fn test_unwrap_codegen() {
        let rust = compile("fn main() = unwrap(some(42))");
        assert!(rust.contains(".unwrap()"));
    }

    #[test]
    fn test_unwrap_or_codegen() {
        let rust = compile("fn main() = unwrap_or(none(), 0)");
        assert!(rust.contains(".unwrap_or("));
    }

    #[test]
    fn test_is_some_is_none_codegen() {
        let rust = compile("fn main() = is_some(some(1))");
        assert!(rust.contains(".is_some()"));
        let rust2 = compile("fn main() = is_none(none())");
        assert!(rust2.contains(".is_none()"));
    }

    #[test]
    fn test_ok_err_codegen() {
        let rust_ok = compile("fn main() = ok(42)");
        assert!(rust_ok.contains("Ok(") || rust_ok.contains("ok("));
        let rust_err = compile(r#"fn main() = err("fail")"#);
        assert!(rust_err.contains("Err(") || rust_err.contains("err("));
    }

    #[test]
    fn test_is_ok_is_err_codegen() {
        let rust = compile("fn main() = is_ok(ok(1))");
        assert!(rust.contains(".is_ok()"));
        let rust2 = compile("fn main() = is_err(ok(1))");
        assert!(rust2.contains(".is_err()"));
    }

    #[test]
    fn test_expect_codegen() {
        let rust = compile(r#"fn main() = expect(some(42), "should be Some")"#);
        assert!(rust.contains(".expect("));
        assert!(rust.contains("&"), "expect should borrow the message");
    }

    #[test]
    fn test_map_result_codegen() {
        let rust = compile("fn main() = map_result(ok(1), fn(x) => x + 1)");
        assert!(rust.contains(".map("));
    }

    #[test]
    fn test_map_option_codegen() {
        let rust = compile("fn main() = map_option(some(1), fn(x) => x + 1)");
        assert!(rust.contains(".map("));
    }

    #[test]
    fn test_read_file_codegen() {
        let rust = compile(r#"fn main() = read_file("test.txt")"#);
        assert!(rust.contains("std::fs::read_to_string") || rust.contains("read_to_string"));
    }

    #[test]
    fn test_write_file_codegen() {
        let rust = compile(r#"fn main() = write_file("test.txt", "content")"#);
        assert!(rust.contains("std::fs::write") || rust.contains("write"));
    }

    #[test]
    fn test_println_with_int() {
        let rust = compile("fn main() = println(42)");
        assert!(rust.contains("println!"));
    }

    #[test]
    fn test_println_with_string() {
        let rust = compile(r#"fn main() = println("hello")"#);
        assert!(rust.contains("println!"));
    }

    #[test]
    fn test_debug_codegen() {
        let rust = compile("fn main() = debug(42)");
        assert!(rust.contains("eprintln!") || rust.contains("dbg!") || rust.contains("star_display"));
    }

    #[test]
    fn test_math_abs_codegen() {
        let rust = compile("fn main() = abs(-5)");
        assert!(rust.contains(".abs()") || rust.contains("abs"));
    }

    #[test]
    fn test_math_min_max_codegen() {
        let rust = compile("fn main() = min(3, 5)");
        assert!(rust.contains(".min(") || rust.contains("std::cmp::min"));
        let rust2 = compile("fn main() = max(3, 5)");
        assert!(rust2.contains(".max(") || rust2.contains("std::cmp::max"));
    }

    #[test]
    fn test_exit_codegen() {
        let rust = compile("fn main() = exit(0)");
        assert!(rust.contains("std::process::exit"));
    }

    #[test]
    fn test_async_fn_codegen() {
        let rust = compile("async fn fetch(): Int = 42");
        assert!(rust.contains("async fn fetch"));
    }

    #[test]
    fn test_for_loop_with_body() {
        let rust = compile("fn main() = for x in [1, 2, 3] do\n  println(x)\nend");
        assert!(rust.contains("for"));
        assert!(rust.contains("in"));
    }

    #[test]
    fn test_while_loop_with_mutation() {
        let rust = compile("fn main() = do\n  let mut i = 0\n  while i < 10 do\n    i += 1\n  end\nend");
        assert!(rust.contains("while"));
    }

    #[test]
    fn test_break_continue_codegen() {
        let rust = compile("fn main() = for x in [1, 2, 3] do\n  if x == 2 then break end\n  if x == 1 then continue end\nend");
        assert!(rust.contains("break"));
        assert!(rust.contains("continue"));
    }

    #[test]
    fn test_compound_assign_codegen() {
        let rust = compile("fn main() = do\n  let mut x = 0\n  x += 5\n  x -= 1\n  x *= 2\n  x /= 3\n  x %= 7\nend");
        assert!(rust.contains("+="));
        assert!(rust.contains("-="));
        assert!(rust.contains("*="));
        assert!(rust.contains("/="));
        assert!(rust.contains("%="));
    }

    #[test]
    fn test_bitwise_and_codegen() {
        let rust = compile("fn main() = 15 band 7");
        assert!(rust.contains(" & "));
    }

    #[test]
    fn test_bitwise_or_codegen() {
        let rust = compile("fn main() = 15 bor 7");
        assert!(rust.contains(" | "));
    }

    #[test]
    fn test_bitwise_xor_codegen() {
        let rust = compile("fn main() = 15 bxor 7");
        assert!(rust.contains(" ^ "));
    }

    #[test]
    fn test_shift_left_codegen() {
        let rust = compile("fn main() = 1 << 4");
        assert!(rust.contains(" << "));
    }

    #[test]
    fn test_shift_right_codegen() {
        let rust = compile("fn main() = 16 >> 2");
        assert!(rust.contains(" >> "));
    }

    #[test]
    fn test_map_new_codegen() {
        let rust = compile("fn main() = map_new()");
        assert!(rust.contains("HashMap::new()") || rust.contains("HashMap"));
    }

    #[test]
    fn test_map_insert_codegen() {
        let rust = compile(r#"fn main() = map_insert(map_new(), "key", 42)"#);
        assert!(rust.contains("insert(") || rust.contains("HashMap"));
    }

    #[test]
    fn test_set_new_codegen() {
        let rust = compile("fn main() = set_new()");
        assert!(rust.contains("HashSet::new()") || rust.contains("HashSet"));
    }

    #[test]
    fn test_sum_product_codegen() {
        let rust_sum = compile("fn main() = sum([1, 2, 3])");
        assert!(rust_sum.contains("sum") || rust_sum.contains("fold"));
        let rust_prod = compile("fn main() = product([1, 2, 3])");
        assert!(rust_prod.contains("product") || rust_prod.contains("fold"));
    }

    #[test]
    fn test_dedup_codegen() {
        let rust = compile("fn main() = dedup([1, 1, 2, 2, 3])");
        assert!(rust.contains("dedup"));
    }

    #[test]
    fn test_flat_map_codegen() {
        let rust = compile("fn main() = flat_map([1, 2, 3], fn(x) => [x, x])");
        assert!(rust.contains("flat_map"));
    }

    #[test]
    fn test_trait_decl_codegen() {
        let rust = compile("trait Printable\n  fn display(self): String\nend");
        assert!(rust.contains("trait Printable"));
        assert!(rust.contains("fn display"));
    }

    #[test]
    fn test_impl_block_codegen() {
        let rust = compile("type Foo = { x: Int }\n\nimpl Foo\n  fn get(self): Int = self.x\nend");
        assert!(rust.contains("impl Foo"));
        assert!(rust.contains("fn get"));
    }

    #[test]
    fn test_trait_impl_codegen() {
        let rust = compile("type Bar = { val: Int }\n\ntrait Show\n  fn show(self): String\nend\n\nimpl Show for Bar\n  fn show(self): String = to_string(self.val)\nend");
        assert!(rust.contains("impl Show for Bar"));
    }

    #[test]
    fn test_module_codegen() {
        let rust = compile("module Utils\n  pub fn double(x: Int): Int = x * 2\nend");
        assert!(rust.contains("mod utils"));
        assert!(rust.contains("pub fn double"));
    }

    #[test]
    fn test_struct_update_codegen() {
        let rust = compile("type Config = { debug: Bool, level: Int }\n\nfn update(c: Config): Config = Config { debug: true, ..c }");
        assert!(rust.contains(".."));
    }

    #[test]
    fn test_move_closure_codegen() {
        let rust = compile("fn main() = do\n  let x = 42\n  move fn() => x\nend");
        assert!(rust.contains("move |"));
    }

    #[test]
    fn test_try_operator_codegen() {
        let rust = compile("fn parse(s: String): Result<Int, String> = do\n  let n = to_int(s)?\n  ok(n)\nend");
        assert!(rust.contains("?"));
    }

    #[test]
    fn test_recursive_enum_boxing() {
        let rust = compile("type List =\n  | Nil\n  | Cons(Int, List)");
        assert!(rust.contains("Box<"), "Recursive type should auto-box");
    }

    #[test]
    fn test_index_access_usize_cast() {
        let rust = compile("fn main() = do\n  let xs = [1, 2, 3]\n  xs[0]\nend");
        assert!(rust.contains("as usize]"));
    }

    #[test]
    fn test_index_assign_codegen() {
        let rust = compile("fn main() = do\n  let mut xs = [1, 2, 3]\n  xs[0] = 10\nend");
        assert!(rust.contains("as usize]"));
    }

    #[test]
    fn test_tuple_three_elements() {
        let rust = compile("fn main() = (1, 2, 3)");
        assert!(rust.contains("(1i64, 2i64, 3i64)"));
    }

    #[test]
    fn test_tuple_destructure_codegen() {
        let rust = compile("fn main() = do\n  let (a, b) = (1, 2)\n  a + b\nend");
        assert!(rust.contains("let (a, b)"));
    }

    #[test]
    fn test_match_variant_with_data() {
        let rust = compile("type Option =\n  | Some(Int)\n  | None\n\nfn get(o: Option): Int = match o\n  | Some(x) => x\n  | None => 0\n  end");
        assert!(rust.contains("Option::Some("));
        assert!(rust.contains("Option::None"));
    }

    #[test]
    fn test_const_declaration() {
        let rust = compile("let MAX: Int = 100");
        assert!(rust.contains("const MAX: i64 = 100i64"));
    }

    #[test]
    fn test_const_string_declaration() {
        let rust = compile(r#"let NAME = "hello""#);
        assert!(rust.contains("const NAME: &str = \"hello\""));
    }

    #[test]
    fn test_operator_overloading_display() {
        let rust = compile("type Wrapper = { val: Int }\n\nimpl Display for Wrapper\n  fn fmt(self): String = to_string(self.val)\nend");
        assert!(rust.contains("fmt::Display") || rust.contains("impl"));
    }

    #[test]
    fn test_sleep_ms_codegen() {
        let rust = compile("fn main() = sleep_ms(100)");
        assert!(rust.contains("thread::sleep") || rust.contains("sleep"));
    }

    #[test]
    fn test_args_codegen() {
        let rust = compile("fn main() = args()");
        assert!(rust.contains("std::env::args") || rust.contains("args"));
    }

    #[test]
    fn test_env_get_codegen() {
        let rust = compile(r#"fn main() = env_get("HOME")"#);
        assert!(rust.contains("std::env::var") || rust.contains("env"));
    }

    #[test]
    fn test_type_param_bounds_codegen() {
        let rust = compile("fn max_val<T: Ord>(a: T, b: T): T = if a > b then a else b end");
        assert!(rust.contains("T: Ord"));
    }

    #[test]
    fn test_multiple_type_params_codegen() {
        let rust = compile("fn pair<A, B>(a: A, b: B): (A, B) = (a, b)");
        assert!(rust.contains("<A, B>"));
    }

    #[test]
    fn test_dyn_trait_codegen() {
        let rust = compile("trait Drawable\n  fn draw(self): String\nend\n\nfn render(obj: dyn Drawable): String = obj.draw()");
        assert!(rust.contains("Box<dyn Drawable>"));
    }

    #[test]
    fn test_associated_type_codegen() {
        let rust = compile("trait Container\n  type Item\n  fn get(self): Item\nend");
        assert!(rust.contains("type Item;"));
    }

    #[test]
    fn test_annotation_cfg_codegen() {
        let rust = compile("@[cfg(test)]\nfn test_fn() = 42");
        assert!(rust.contains("#[cfg(test)]"));
    }

    #[test]
    fn test_annotation_derive_codegen() {
        let rust = compile("@[derive(Debug, Clone)]\ntype Foo = { x: Int }");
        assert!(rust.contains("#[derive(") || rust.contains("derive"));
    }

    #[test]
    fn test_extern_fn_codegen() {
        let rust = compile("extern fn libc_exit(code: Int)");
        assert!(rust.contains("extern") || rust.contains("fn libc_exit"));
    }

    #[test]
    fn test_star_display_prelude() {
        let rust = compile("fn main() = 42");
        assert!(rust.contains("fn star_display"), "Should always include star_display helper");
    }

    #[test]
    fn test_nested_pipe_with_lambda() {
        let rust = compile("fn main() = [1, 2, 3] |> map(fn(x) => x * 2) |> filter(fn(x) => x > 2)");
        assert!(rust.contains(".map("));
        assert!(rust.contains(".filter("));
    }

    // ── Type mapping correctness ────────────────────────────────

    #[test]
    fn test_type_map_int() {
        let rust = compile("fn f(x: Int): Int = x");
        assert!(rust.contains("i64"));
    }

    #[test]
    fn test_type_map_float() {
        let rust = compile("fn f(x: Float): Float = x");
        assert!(rust.contains("f64"));
    }

    #[test]
    fn test_type_map_bool() {
        let rust = compile("fn f(x: Bool): Bool = x");
        assert!(rust.contains("bool"));
    }

    #[test]
    fn test_type_map_string() {
        let rust = compile("fn f(x: String): String = x");
        assert!(rust.contains("String"));
    }

    #[test]
    fn test_type_map_list() {
        let rust = compile("fn f(x: List<Int>): List<Int> = x");
        assert!(rust.contains("Vec<i64>"));
    }

    #[test]
    fn test_type_map_hashmap() {
        let rust = compile("fn f(x: Map<String, Int>): Map<String, Int> = x");
        assert!(rust.contains("HashMap<String, i64>"));
    }

    #[test]
    fn test_type_map_hashset() {
        let rust = compile("fn f(x: Set<Int>): Set<Int> = x");
        assert!(rust.contains("HashSet<i64>"));
    }

    #[test]
    fn test_type_map_deque() {
        let rust = compile("fn f(x: Deque<Int>): Deque<Int> = x");
        assert!(rust.contains("VecDeque<i64>"));
    }

    #[test]
    fn test_type_map_heap() {
        let rust = compile("fn f(x: Heap<Int>): Heap<Int> = x");
        assert!(rust.contains("BinaryHeap<i64>"));
    }

    // ── Integer/Float literal suffixes ──────────────────────────

    #[test]
    fn test_int_literal_suffix() {
        let rust = compile("fn main() = 42");
        assert!(rust.contains("42i64"));
    }

    #[test]
    fn test_float_literal_suffix() {
        let rust = compile("fn main() = 3.14");
        assert!(rust.contains("3.14f64"));
    }

    #[test]
    fn test_negative_int_literal() {
        let rust = compile("fn main() = -1");
        assert!(rust.contains("-1i64") || rust.contains("-(1i64)"));
    }

    // ── String interpolation codegen ────────────────────────────

    #[test]
    fn test_string_interp_format() {
        let rust = compile(r#"fn main() = "hello #{42} world""#);
        assert!(rust.contains("format!"));
    }

    #[test]
    fn test_string_interp_with_arithmetic() {
        let rust = compile(r#"fn f(x: Int): String = "value: #{x + 1}""#);
        assert!(rust.contains("format!"));
    }

    // ── Recursive type auto-boxing ──────────────────────────────

    #[test]
    fn test_recursive_enum_boxed() {
        let rust = compile("type Tree =\n  | Leaf(Int)\n  | Node(Tree, Tree)");
        assert!(rust.contains("Box<Tree>"));
    }

    #[test]
    fn test_non_recursive_enum_no_box() {
        let rust = compile("type Color =\n  | Red\n  | Green\n  | Blue");
        assert!(!rust.contains("Box<"));
    }

    // ── Variant name qualification ──────────────────────────────

    #[test]
    fn test_variant_qualified_in_match() {
        let rust = compile("type Color =\n  | Red\n  | Blue\n\nfn f(c: Color): Int = match c\n  | Red => 1\n  | Blue => 2\n  end");
        assert!(rust.contains("Color::Red"));
        assert!(rust.contains("Color::Blue"));
    }

    #[test]
    fn test_variant_qualified_in_constructor() {
        let rust = compile("type Opt =\n  | Some(Int)\n  | None\n\nfn main() = Some(42)");
        assert!(rust.contains("Opt::Some("));
    }

    // ── For loop codegen ────────────────────────────────────────

    #[test]
    fn test_for_loop_emits_for() {
        let rust = compile("fn main() = for x in [1, 2, 3] do\n  println(x)\nend");
        assert!(rust.contains("for x in"));
    }

    #[test]
    fn test_while_loop_emits_while() {
        let rust = compile("fn main() = do\n  let mut i = 0\n  while i < 10 do\n    i += 1\n  end\nend");
        assert!(rust.contains("while"));
        assert!(rust.contains("i += 1"));
    }

    #[test]
    fn test_break_in_loop() {
        let rust = compile("fn main() = for x in [1, 2, 3] do\n  if x == 2 then break end\nend");
        assert!(rust.contains("break"));
    }

    // ── Compound assignment ─────────────────────────────────────

    #[test]
    fn test_compound_assign_sub() {
        let rust = compile("fn main() = do\n  let mut x = 10\n  x -= 3\nend");
        assert!(rust.contains("-="));
    }

    #[test]
    fn test_compound_assign_mul() {
        let rust = compile("fn main() = do\n  let mut x = 2\n  x *= 5\nend");
        assert!(rust.contains("*="));
    }

    // ── Mutable let bindings ────────────────────────────────────

    #[test]
    fn test_mut_let_codegen() {
        let rust = compile("fn main() = do\n  let mut x = 0\n  x = 1\nend");
        assert!(rust.contains("let mut x"));
    }

    // ── Tuple codegen ───────────────────────────────────────────

    #[test]
    fn test_tuple_pair_codegen() {
        let rust = compile("fn first(t: (Int, String)): (Int, String) = t");
        assert!(rust.contains("(i64, String)"));
    }

    // ── Struct construction ─────────────────────────────────────

    #[test]
    fn test_struct_lit_fields() {
        let rust = compile("type Point = { x: Int, y: Int }\n\nfn origin(): Point = Point { x: 0, y: 0 }");
        assert!(rust.contains("Point {"));
        assert!(rust.contains("x: 0i64") || rust.contains("x:"));
    }

    #[test]
    fn test_struct_field_access_codegen() {
        let rust = compile("type Point = { x: Int, y: Int }\n\nfn get_x(p: Point): Int = p.x");
        assert!(rust.contains("p.x"));
    }

    // ── Trait and impl codegen ──────────────────────────────────

    #[test]
    fn test_trait_codegen() {
        let rust = compile("trait Greet\n  fn greet(self): String\nend");
        assert!(rust.contains("trait Greet"));
        assert!(rust.contains("fn greet"));
    }

    #[test]
    fn test_impl_methods() {
        let rust = compile("type Cat = { name: String }\n\nimpl Cat\n  fn speak(self): String = self.name\nend");
        assert!(rust.contains("impl Cat"));
        assert!(rust.contains("fn speak"));
    }

    // ── Module codegen ──────────────────────────────────────────
    // Note: module declarations use `module` keyword and require
    // special lexer handling, tested via integration tests instead

    // ── Async function codegen ──────────────────────────────────

    #[test]
    fn test_async_fn_emits_async() {
        let rust = compile("async fn fetch(): String = \"data\"");
        assert!(rust.contains("async fn fetch"));
    }

    #[test]
    fn test_async_main_adds_tokio() {
        let rust = compile("async fn main() = sleep_ms(100)");
        assert!(rust.contains("tokio::main") || rust.contains("#[tokio::main]"));
    }

    // ── Pub function codegen ────────────────────────────────────

    #[test]
    fn test_pub_fn_codegen() {
        let rust = compile("pub fn api_call(): Int = 42");
        assert!(rust.contains("pub fn api_call"));
    }

    // ── Do block codegen ────────────────────────────────────────

    #[test]
    fn test_do_block_codegen() {
        let rust = compile("fn main() = do\n  let x = 1\n  let y = 2\n  x + y\nend");
        assert!(rust.contains("let x"));
        assert!(rust.contains("let y"));
    }

    // ── Const codegen ───────────────────────────────────────────

    #[test]
    fn test_const_float() {
        let rust = compile("let PI: Float = 3.14159");
        assert!(rust.contains("const PI: f64 = 3.14159f64"));
    }

    #[test]
    fn test_const_bool() {
        let rust = compile("let DEBUG: Bool = true");
        assert!(rust.contains("const DEBUG: bool = true"));
    }

    // ── Unsafe env operations ───────────────────────────────────

    #[test]
    fn test_env_set_unsafe() {
        let rust = compile(r#"fn main() = env_set("KEY", "VAL")"#);
        assert!(rust.contains("unsafe"));
    }

    #[test]
    fn test_env_remove_unsafe() {
        let rust = compile(r#"fn main() = env_remove("KEY")"#);
        assert!(rust.contains("unsafe"));
    }

    // ── Clone-by-default for list ops ───────────────────────────

    #[test]
    fn test_map_clone_into_iter() {
        let rust = compile("fn main() = map([1,2,3], fn(x) => x + 1)");
        assert!(rust.contains(".clone()") || rust.contains("into_iter"));
    }

    // ── Result/Option builtins clone before consume ─────────────

    #[test]
    fn test_expect_takes_ref() {
        let rust = compile(r#"fn main() = expect(some(1), "fail")"#);
        assert!(rust.contains("&"));
    }

    // ── Index access codegen ────────────────────────────────────

    #[test]
    fn test_index_usize_cast() {
        let rust = compile("fn get(xs: List<Int>, i: Int): Int = xs[i]");
        assert!(rust.contains("as usize"));
    }

    // ── Derive attributes on types ──────────────────────────────

    #[test]
    fn test_enum_has_derive_debug_clone() {
        let rust = compile("type Dir =\n  | North\n  | South");
        assert!(rust.contains("Debug"));
        assert!(rust.contains("Clone"));
    }

    #[test]
    fn test_struct_has_derive_debug_clone() {
        let rust = compile("type Vec2 = { x: Float, y: Float }");
        assert!(rust.contains("Debug"));
        assert!(rust.contains("Clone"));
    }

    // ── Pipe desugars to nested calls ───────────────────────────

    #[test]
    fn test_pipe_chain_desugars() {
        let rust = compile("fn double(x: Int): Int = x * 2\nfn inc(x: Int): Int = x + 1\nfn main() = 5 |> double |> inc");
        assert!(rust.contains("inc(double(5i64))"));
    }

    // ── Method call codegen ─────────────────────────────────────

    #[test]
    fn test_method_call_codegen() {
        let rust = compile("type Foo = { x: Int }\n\nimpl Foo\n  fn get_x(self): Int = self.x\nend\n\nfn main() = do\n  let f = Foo { x: 42 }\n  f.get_x()\nend");
        assert!(rust.contains(".get_x()"));
    }

    // ── Lambda with type annotations ────────────────────────────

    #[test]
    fn test_lambda_typed_param_codegen() {
        let rust = compile("fn main() = fn(x: Int) => x + 1");
        assert!(rust.contains("|x: i64|"));
    }

    // ── Multi-param lambda ──────────────────────────────────────

    #[test]
    fn test_multi_param_lambda() {
        let rust = compile("fn main() = fn(a, b) => a + b");
        assert!(rust.contains("|a, b|"));
    }

    // ── Empty struct codegen ────────────────────────────────────

    #[test]
    fn test_empty_struct() {
        let rust = compile("type Unit = {}");
        assert!(rust.contains("struct Unit"));
    }

    // ── Try operator codegen ────────────────────────────────────

    #[test]
    fn test_try_question_mark_codegen() {
        let rust = compile(r#"fn read(): Result<String, String> = do
  let content = read_file("test.txt")?
  content
end"#);
        assert!(rust.contains("?"));
    }

    // ── Rust block passthrough ──────────────────────────────────
    // Note: rust blocks require special lexer handling that captures
    // raw content between braces, tested via integration tests instead

    // ── star_display wraps in parens ────────────────────────────

    #[test]
    fn test_println_wraps_star_display() {
        let rust = compile("fn main() = println(1 + 2)");
        assert!(rust.contains("star_display(&("));
    }

    // ── Move lambda codegen ─────────────────────────────────────

    #[test]
    fn test_move_lambda_codegen() {
        let rust = compile("fn main() = do\n  let x = 42\n  spawn(move fn() => println(x))\nend");
        assert!(rust.contains("move ||") || rust.contains("move |"));
    }
}
