/// Post-codegen borrow inference pass that converts `String` parameters to `&str`
/// when the parameter is only used in read-only contexts within the function body.
///
/// This is a conservative, text-based optimization applied to generated Rust code.
/// It only transforms parameters that are provably safe to borrow.
///
/// The pass works in two stages:
/// 1. Analyze each function to find String params that are read-only
/// 2. Rewrite signatures (String -> &str), fix body usages (.clone() -> .to_string(),
///    star_display(&(p)) -> star_display(&(p.to_string()))), and fix call sites
///    (func(arg) -> func(&arg) where arg is a String expression)

use std::collections::{HashMap, HashSet};

/// Apply borrow inference to generated Rust code.
pub fn infer_borrows(code: &str) -> String {
    // Phase 1: Discover which functions have which params that can be borrowed
    let functions = find_all_functions(code);
    let mut borrowed_params: HashMap<String, HashSet<String>> = HashMap::new();

    for func in &functions {
        let func_text = &code[func.start..func.end];
        let sig = &func_text[..func_text.find('{').unwrap_or(func_text.len())];

        // Skip main, star_display, and helper functions
        if sig.contains("fn main(") || sig.contains("fn main (")
            || sig.contains("fn star_display")
            || sig.contains("fn _star_")
        {
            continue;
        }

        // Extract function name
        let fn_name = match extract_fn_name(sig) {
            Some(n) => n,
            None => continue,
        };

        let params = match parse_params(func_text) {
            Some(p) => p,
            None => continue,
        };

        let open_brace = match func_text.find('{') {
            Some(p) => p,
            None => continue,
        };
        let body = &func_text[open_brace..];

        let mut param_set = HashSet::new();
        for param in &params {
            if param.ty == "String" && is_read_only_usage(&param.name, body) {
                param_set.insert(param.name.clone());
            }
            // Vec<T> -> &[T] when read-only
            if param.ty.starts_with("Vec<") && is_read_only_vec_usage(&param.name, body) {
                param_set.insert(param.name.clone());
            }
        }

        if !param_set.is_empty() {
            borrowed_params.insert(fn_name, param_set);
        }
    }

    if borrowed_params.is_empty() {
        return code.to_string();
    }

    // Phase 2: Apply transformations
    let mut result = String::with_capacity(code.len());
    let mut remaining = code;

    for func in &functions {
        let before = &remaining[..func.start - (code.len() - remaining.len())];

        // Fix call sites in the text before this function
        let fixed_before = fix_call_sites(before, &borrowed_params);
        result.push_str(&fixed_before);

        let func_offset = func.start - (code.len() - remaining.len());
        let func_len = func.end - func.start;
        let func_text = &remaining[func_offset..func_offset + func_len];

        let sig = &func_text[..func_text.find('{').unwrap_or(func_text.len())];
        let fn_name = extract_fn_name(sig).unwrap_or_default();

        if let Some(param_names) = borrowed_params.get(&fn_name) {
            let optimized = rewrite_function(func_text, param_names, &borrowed_params);
            result.push_str(&optimized);
        } else {
            // Still need to fix call sites within non-optimized functions
            let fixed = fix_call_sites(func_text, &borrowed_params);
            result.push_str(&fixed);
        }

        remaining = &remaining[func_offset + func_len..];
    }

    // Fix call sites in any remaining text
    let fixed_remaining = fix_call_sites(remaining, &borrowed_params);
    result.push_str(&fixed_remaining);

    result
}

/// Location of a function in the source text.
#[derive(Debug)]
struct FuncSpan {
    start: usize,
    end: usize,
}

/// Find all function definitions in the code.
fn find_all_functions(code: &str) -> Vec<FuncSpan> {
    let mut result = Vec::new();
    let mut search_from = 0;

    while let Some(func) = find_next_function_from(code, search_from) {
        search_from = func.end;
        result.push(func);
    }

    result
}

/// Find the next `fn name(...)` function definition starting from `from`.
fn find_next_function_from(code: &str, from: usize) -> Option<FuncSpan> {
    let mut search_from = from;
    loop {
        let fn_pos = code[search_from..].find("fn ")
            .map(|p| p + search_from)?;

        let before = if fn_pos > 0 { code.as_bytes()[fn_pos - 1] } else { b'\n' };
        if before != b'\n' && before != b' ' && before != b'\t' && before != b'{' && before != b';' {
            search_from = fn_pos + 3;
            continue;
        }

        let after_fn = &code[fn_pos..];
        let open_brace = match after_fn.find('{') {
            Some(p) => p,
            None => {
                search_from = fn_pos + 3;
                continue;
            }
        };

        let between = &after_fn[..open_brace];
        if between.contains(';') {
            search_from = fn_pos + 3;
            continue;
        }

        let body_start = fn_pos + open_brace;
        match find_matching_brace(code, body_start) {
            Some(close) => {
                return Some(FuncSpan {
                    start: fn_pos,
                    end: close + 1,
                });
            }
            None => {
                search_from = fn_pos + 3;
                continue;
            }
        }
    }
}

/// Extract the function name from a signature string like `fn greet(...)`.
fn extract_fn_name(sig: &str) -> Option<String> {
    let fn_pos = sig.find("fn ")?;
    let after = &sig[fn_pos + 3..];
    // Skip optional type params or whitespace to find the name
    let name_end = after.find(|c: char| c == '(' || c == '<' || c == ' ')?;
    let name = after[..name_end].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// Find the matching `}` for a `{` at position `open` in `code`.
fn find_matching_brace(code: &str, open: usize) -> Option<usize> {
    let bytes = code.as_bytes();
    if bytes.get(open) != Some(&b'{') {
        return None;
    }

    let mut depth = 1;
    let mut i = open + 1;
    let mut in_string = false;
    let mut in_char = false;

    while i < bytes.len() && depth > 0 {
        let b = bytes[i];

        if in_string {
            if b == b'\\' {
                i += 1;
            } else if b == b'"' {
                in_string = false;
            }
        } else if in_char {
            if b == b'\\' {
                i += 1;
            } else if b == b'\'' {
                in_char = false;
            }
        } else {
            match b {
                b'"' => in_string = true,
                b'\'' => in_char = true,
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
        }
        i += 1;
    }

    if depth == 0 { Some(i - 1) } else { None }
}

/// Information about a parsed function parameter.
#[derive(Debug)]
struct ParamInfo {
    name: String,
    ty: String,
    /// Byte range of the type in the original function text
    ty_start: usize,
    ty_end: usize,
}

/// Rewrite a function: change String params to &str, fix body usages.
fn rewrite_function(
    func_text: &str,
    param_names: &HashSet<String>,
    all_borrowed: &HashMap<String, HashSet<String>>,
) -> String {
    let params = match parse_params(func_text) {
        Some(p) => p,
        None => return func_text.to_string(),
    };

    let mut result = func_text.to_string();

    // 1. Replace types in signature (reverse order to preserve positions)
    let mut to_convert: Vec<&ParamInfo> = params.iter()
        .filter(|p| param_names.contains(&p.name))
        .collect();
    to_convert.sort_by(|a, b| b.ty_start.cmp(&a.ty_start));

    for param in &to_convert {
        let replacement = if param.ty == "String" {
            "&str".to_string()
        } else if param.ty.starts_with("Vec<") {
            // Vec<T> -> &[T]
            let inner = &param.ty[4..param.ty.len() - 1]; // extract T from Vec<T>
            format!("&[{}]", inner)
        } else {
            continue;
        };
        result = format!(
            "{}{}{}",
            &result[..param.ty_start],
            replacement,
            &result[param.ty_end..]
        );
    }

    // 2. Fix body usages
    for param in &params {
        if !param_names.contains(&param.name) {
            continue;
        }
        let name = &param.name;

        if param.ty == "String" {
            // .clone() -> .to_string() (when the result needs to be owned)
            let clone_pat = format!("{}.clone()", name);
            let to_string_pat = format!("{}.to_string()", name);
            result = result.replace(&clone_pat, &to_string_pat);

            // star_display(&(name)) -> star_display(&(name.to_string()))
            let display_pat = format!("star_display(&({}))", name);
            let display_fix = format!("star_display(&({}.to_string()))", name);
            result = result.replace(&display_pat, &display_fix);
        } else if param.ty.starts_with("Vec<") {
            // .clone().into_iter() -> .iter().cloned() (read from slice)
            let clone_iter = format!("{}.clone().into_iter()", name);
            let iter_cloned = format!("{}.iter().cloned()", name);
            result = result.replace(&clone_iter, &iter_cloned);

            // .clone() -> .to_vec() (when owned Vec is needed)
            let clone_pat = format!("{}.clone()", name);
            let to_vec_pat = format!("{}.to_vec()", name);
            result = result.replace(&clone_pat, &to_vec_pat);
        }
    }

    // 3. Fix call sites within this function's body
    result = fix_call_sites(&result, all_borrowed);

    result
}

/// Fix call sites: when calling a function that now takes &str instead of String,
/// add `&` before String arguments.
///
/// This looks for patterns like `fn_name(args)` where fn_name is in the borrowed map,
/// and wraps the corresponding String arguments with `&`.
fn fix_call_sites(code: &str, borrowed: &HashMap<String, HashSet<String>>) -> String {
    if borrowed.is_empty() {
        return code.to_string();
    }

    let mut result = code.to_string();

    for (fn_name, _param_names) in borrowed {
        // Find calls to this function: `fn_name(`
        let call_pat = format!("{}(", fn_name);
        let mut search_from = 0;

        loop {
            let pos = match result[search_from..].find(&call_pat) {
                Some(p) => p + search_from,
                None => break,
            };

            // Make sure it's a call, not the function definition
            // Check that what's before is not `fn ` (definition)
            let before_start = if pos >= 3 { pos - 3 } else { 0 };
            let before = &result[before_start..pos];
            if before.ends_with("fn ") || before.ends_with("fn\n") {
                search_from = pos + call_pat.len();
                continue;
            }

            // Also check it's a standalone identifier (not part of a longer name)
            if pos > 0 {
                let prev = result.as_bytes()[pos - 1];
                if prev.is_ascii_alphanumeric() || prev == b'_' || prev == b'.' {
                    search_from = pos + call_pat.len();
                    continue;
                }
            }

            let paren_start = pos + fn_name.len();
            let paren_close = match find_matching_paren(&result, paren_start) {
                Some(p) => p,
                None => {
                    search_from = pos + call_pat.len();
                    continue;
                }
            };

            let args_str = result[paren_start + 1..paren_close].to_string();
            let args = split_args(&args_str);

            // Get the parameter info for this function to know which positional args to wrap
            // We need to figure out which arg positions correspond to borrowed params
            // For now, use a simpler approach: find the function definition and match by position
            let fixed_args = fix_args_for_call(fn_name, &args, borrowed);

            if let Some(new_args) = fixed_args {
                let new_call = format!("{}({})", fn_name, new_args);
                result = format!(
                    "{}{}{}",
                    &result[..pos],
                    new_call,
                    &result[paren_close + 1..]
                );
                search_from = pos + new_call.len();
            } else {
                search_from = pos + call_pat.len();
            }
        }
    }

    result
}

/// Try to fix arguments for a call to a borrowed function.
/// Returns Some(new_args_string) if any changes were made, None otherwise.
fn fix_args_for_call(
    _fn_name: &str,
    args: &[String],
    _borrowed: &HashMap<String, HashSet<String>>,
) -> Option<String> {
    // For each argument that is a String expression being passed where &str is expected,
    // we need to add `&` or `&*`. However, since we don't have full type information,
    // we use heuristics:
    // - If arg ends with `.to_string()` -> strip it (string literal already is &str)
    // - If arg is `"literal".to_string()` -> change to `"literal"`
    // - If arg is a variable name -> add `&` prefix
    // - If arg is an expression -> wrap in `&(expr)` or add `.as_str()`

    let mut changed = false;
    let mut new_args = Vec::new();

    for arg in args {
        let trimmed = arg.trim();

        // If arg is `"something".to_string()`, simplify to `"something"`
        if trimmed.ends_with(".to_string()") && trimmed.contains('"') {
            let without = &trimmed[..trimmed.len() - ".to_string()".len()];
            new_args.push(without.to_string());
            changed = true;
        }
        // If arg is `format!(...)`, that returns String, need `.as_str()` or `&`
        // Actually format!() -> String, and &String coerces to &str, so `&format!()` works
        else if trimmed.starts_with("format!(") {
            new_args.push(format!("&{}", trimmed));
            changed = true;
        }
        // If arg ends with `.clone()`, it's creating an owned value — add `&` and strip clone
        else if trimmed.ends_with(".clone()") {
            let without_clone = &trimmed[..trimmed.len() - ".clone()".len()];
            new_args.push(format!("&{}", without_clone));
            changed = true;
        }
        else {
            new_args.push(trimmed.to_string());
        }
    }

    if changed {
        Some(new_args.join(", "))
    } else {
        None
    }
}

/// Split function call arguments by top-level commas.
fn split_args(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut depth_paren = 0i32;
    let mut depth_angle = 0i32;
    let mut depth_brace = 0i32;
    let mut in_string = false;
    let mut last = 0;
    let bytes = s.as_bytes();

    let mut i = 0;
    while i < bytes.len() {
        if in_string {
            if bytes[i] == b'\\' {
                i += 1;
            } else if bytes[i] == b'"' {
                in_string = false;
            }
        } else {
            match bytes[i] {
                b'"' => in_string = true,
                b'(' => depth_paren += 1,
                b')' => depth_paren -= 1,
                b'<' => depth_angle += 1,
                b'>' if depth_angle > 0 => depth_angle -= 1,
                b'{' => depth_brace += 1,
                b'}' => depth_brace -= 1,
                b',' if depth_paren == 0 && depth_angle == 0 && depth_brace == 0 => {
                    result.push(s[last..i].to_string());
                    last = i + 1;
                }
                _ => {}
            }
        }
        i += 1;
    }
    if last <= s.len() {
        result.push(s[last..].to_string());
    }
    result
}

/// Parse the parameter list from a function definition text.
fn parse_params(func_text: &str) -> Option<Vec<ParamInfo>> {
    let fn_kw = func_text.find("fn ")?;
    let after_fn = &func_text[fn_kw + 3..];

    let paren_open = after_fn.find('(')?;
    let paren_open_abs = fn_kw + 3 + paren_open;

    let paren_close = find_matching_paren(func_text, paren_open_abs)?;

    let params_str = &func_text[paren_open_abs + 1..paren_close];
    let param_parts = split_param_strs(params_str);

    let mut result = Vec::new();
    for part in &param_parts {
        let trimmed = part.trim();
        if trimmed.is_empty() || trimmed == "&self" || trimmed == "&mut self" || trimmed == "self" {
            continue;
        }

        if let Some(colon_pos) = find_top_level_colon(trimmed) {
            let name = trimmed[..colon_pos].trim().to_string();
            let ty = trimmed[colon_pos + 1..].trim().to_string();

            let part_start_in_func = find_substring_pos(func_text, paren_open_abs + 1, part)?;
            let colon_in_part = find_top_level_colon(part)?;
            let ty_start = part_start_in_func + colon_in_part + 1
                + (part[colon_in_part + 1..].len() - part[colon_in_part + 1..].trim_start().len());
            let ty_end = part_start_in_func + part.trim_end().len();

            result.push(ParamInfo {
                name,
                ty,
                ty_start,
                ty_end,
            });
        }
    }

    Some(result)
}

fn find_substring_pos(haystack: &str, search_start: usize, needle: &str) -> Option<usize> {
    haystack[search_start..].find(needle).map(|p| p + search_start)
}

fn find_matching_paren(code: &str, open: usize) -> Option<usize> {
    let bytes = code.as_bytes();
    if bytes.get(open) != Some(&b'(') {
        return None;
    }

    let mut depth = 1;
    let mut i = open + 1;
    while i < bytes.len() && depth > 0 {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' { i += 1; }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    if depth == 0 { Some(i - 1) } else { None }
}

fn split_param_strs(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth_angle = 0i32;
    let mut depth_paren = 0i32;
    let mut last = 0;

    for (i, c) in s.char_indices() {
        match c {
            '<' => depth_angle += 1,
            '>' => depth_angle -= 1,
            '(' => depth_paren += 1,
            ')' => depth_paren -= 1,
            ',' if depth_angle == 0 && depth_paren == 0 => {
                result.push(&s[last..i]);
                last = i + 1;
            }
            _ => {}
        }
    }
    result.push(&s[last..]);
    result
}

fn find_top_level_colon(s: &str) -> Option<usize> {
    let mut depth_angle = 0i32;
    let mut depth_paren = 0i32;

    for (i, c) in s.char_indices() {
        match c {
            '<' => depth_angle += 1,
            '>' => depth_angle -= 1,
            '(' => depth_paren += 1,
            ')' => depth_paren -= 1,
            ':' if depth_angle == 0 && depth_paren == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

/// Check if a parameter name is only used in read-only contexts within the function body.
/// This is conservative -- if there's any doubt, return false.
fn is_read_only_usage(param_name: &str, body: &str) -> bool {
    let consuming_patterns: Vec<String> = vec![
        format!("return {}", param_name),
        format!(".push({})", param_name),
        format!(".push({}.clone())", param_name),
        format!(".insert({}", param_name),
    ];

    for pattern in &consuming_patterns {
        if body.contains(pattern.as_str()) {
            return false;
        }
    }

    let name_bytes = param_name.as_bytes();
    let body_bytes = body.as_bytes();
    let mut search_from = 0;

    while let Some(pos) = body[search_from..].find(param_name) {
        let abs_pos = search_from + pos;

        // Ensure standalone identifier
        if abs_pos > 0 {
            let prev = body_bytes[abs_pos - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' {
                search_from = abs_pos + name_bytes.len();
                continue;
            }
        }

        let after_start = abs_pos + name_bytes.len();
        if after_start < body_bytes.len() {
            let next = body_bytes[after_start];
            if next.is_ascii_alphanumeric() || next == b'_' {
                search_from = abs_pos + name_bytes.len();
                continue;
            }
        }

        let after = &body[after_start..];

        // Preceded by `&` — always safe
        if abs_pos > 0 && body_bytes[abs_pos - 1] == b'&' {
            search_from = after_start;
            continue;
        }

        // Bare usage: followed by `)`, `,`, `;`, `\n`, ` `, `}`
        if after.is_empty() || after.starts_with(')') || after.starts_with(',')
            || after.starts_with(';') || after.starts_with('\n')
            || after.starts_with(' ') || after.starts_with('}')
        {
            let line_start = body[..abs_pos].rfind('\n').map(|p| p + 1).unwrap_or(0);
            let line = &body[line_start..];

            // Safe if inside format macro or star_display
            if line.contains("format!(") || line.contains("println!(")
                || line.contains("eprintln!(") || line.contains("print!(")
                || line.contains("write!(") || line.contains("writeln!(")
                || line.contains("star_display(&(")
            {
                search_from = after_start;
                continue;
            }

            // Safe if in comparison context
            if after.starts_with(' ') {
                let after_space = after.trim_start();
                if after_space.starts_with("==") || after_space.starts_with("!=")
                    || after_space.starts_with("<=") || after_space.starts_with(">=")
                    || (after_space.starts_with('<') && !after_space.starts_with("<<"))
                    || (after_space.starts_with('>') && !after_space.starts_with(">>"))
                {
                    search_from = after_start;
                    continue;
                }
            }

            let before = &body[..abs_pos];
            let before_trimmed = before.trim_end();
            if before_trimmed.ends_with("==") || before_trimmed.ends_with("!=")
                || before_trimmed.ends_with("<=") || before_trimmed.ends_with(">=")
            {
                search_from = after_start;
                continue;
            }

            // Unknown bare usage — conservatively reject
            return false;
        }

        if after.starts_with('.') {
            let method = &after[1..];
            let read_only_methods = [
                "len()", "is_empty()", "contains(", "starts_with(", "ends_with(",
                "as_str()", "as_bytes()", "bytes()", "chars()",
                "trim()", "trim_start()", "trim_end()",
                "split(", "splitn(", "rsplit(", "rsplitn(",
                "lines()", "find(", "rfind(",
                "to_lowercase()", "to_uppercase()",
                "to_string()", "clone()",
                "replace(", "replacen(",
                "strip_prefix(", "strip_suffix(",
                "repeat(", "matches(",
                "eq(", "ne(", "cmp(",
                "get(", "parse(",
                "as_ref(",
            ];

            let is_safe = read_only_methods.iter().any(|m| method.starts_with(m));
            if !is_safe {
                return false;
            }
            search_from = after_start;
            continue;
        }

        // Indexing with [] — read-only
        if after.starts_with('[') {
            search_from = after_start;
            continue;
        }

        search_from = after_start;
    }

    true
}

/// Check if a Vec parameter is only used in read-only contexts within the function body.
/// Similar to `is_read_only_usage` but for Vec-specific patterns.
fn is_read_only_vec_usage(param_name: &str, body: &str) -> bool {
    // Check for consuming patterns
    let consuming_patterns: Vec<String> = vec![
        format!("return {}", param_name),
        format!("{}.push(", param_name),
        format!("{}.pop(", param_name),
        format!("{}.remove(", param_name),
        format!("{}.insert(", param_name),
        format!("{}.clear(", param_name),
        format!("{}.truncate(", param_name),
        format!("{}.extend(", param_name),
        format!("{}.drain(", param_name),
        format!("{}.retain(", param_name),
        format!("{}.sort(", param_name),
        format!("{}.sort_by(", param_name),
        format!("{}.sort_unstable(", param_name),
        format!("{}.dedup(", param_name),
        format!("{}.swap(", param_name),
        format!("{}.reverse()", param_name),
        // into_iter consumes the Vec
        format!("{}.into_iter()", param_name),
    ];

    for pattern in &consuming_patterns {
        if body.contains(pattern.as_str()) {
            return false;
        }
    }

    let name_bytes = param_name.as_bytes();
    let body_bytes = body.as_bytes();
    let mut search_from = 0;

    while let Some(pos) = body[search_from..].find(param_name) {
        let abs_pos = search_from + pos;

        // Ensure standalone identifier
        if abs_pos > 0 {
            let prev = body_bytes[abs_pos - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' {
                search_from = abs_pos + name_bytes.len();
                continue;
            }
        }

        let after_start = abs_pos + name_bytes.len();
        if after_start < body_bytes.len() {
            let next = body_bytes[after_start];
            if next.is_ascii_alphanumeric() || next == b'_' {
                search_from = abs_pos + name_bytes.len();
                continue;
            }
        }

        let after = &body[after_start..];

        // Preceded by `&` — always safe
        if abs_pos > 0 && body_bytes[abs_pos - 1] == b'&' {
            search_from = after_start;
            continue;
        }

        // Method call
        if after.starts_with('.') {
            let method = &after[1..];
            let read_only_methods = [
                "len()", "is_empty()", "contains(",
                "iter()", "windows(", "chunks(",
                "first()", "last()", "get(",
                "starts_with(", "ends_with(",
                "binary_search(", "as_slice()",
                "to_vec()", "clone()",
                "join(",
                "clone().into_iter()", // clone-then-iter is read-only
            ];

            let is_safe = read_only_methods.iter().any(|m| method.starts_with(m));
            if !is_safe {
                return false;
            }
            search_from = after_start;
            continue;
        }

        // Indexing with [] — read-only
        if after.starts_with('[') {
            search_from = after_start;
            continue;
        }

        // Bare usage in argument position — could be consuming
        if after.is_empty() || after.starts_with(')') || after.starts_with(',')
            || after.starts_with(';') || after.starts_with('\n')
            || after.starts_with(' ') || after.starts_with('}')
        {
            // Conservatively reject bare usages since Vec is not Copy
            return false;
        }

        search_from = after_start;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_string_to_str() {
        let input = r#"fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}"#;
        let output = infer_borrows(input);
        assert!(output.contains("name: &str"), "Expected &str, got: {}", output);
        assert!(!output.contains("name: String"), "Should not contain String param");
    }

    #[test]
    fn test_preserves_consumed_param() {
        let input = r#"fn take_name(name: String) -> String {
    return name
}"#;
        let output = infer_borrows(input);
        assert!(output.contains("name: String"), "Should preserve String when consumed: {}", output);
    }

    #[test]
    fn test_preserves_non_string_params() {
        let input = r#"fn add(a: i64, b: i64) -> i64 {
    (a + b)
}"#;
        let output = infer_borrows(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_clone_becomes_to_string() {
        let input = r#"fn greet(name: String) -> String {
    let x = name.clone();
    format!("Hello, {}!", x)
}"#;
        let output = infer_borrows(input);
        assert!(output.contains("name: &str"), "Expected &str: {}", output);
        assert!(output.contains("name.to_string()"), "clone should become to_string: {}", output);
    }

    #[test]
    fn test_method_calls_are_read_only() {
        let input = r#"fn check(s: String) -> bool {
    s.len() > 0i64 && s.contains("x") && s.starts_with("y")
}"#;
        let output = infer_borrows(input);
        assert!(output.contains("s: &str"), "Method calls should be read-only: {}", output);
    }

    #[test]
    fn test_comparison_is_read_only() {
        let input = r#"fn eq(a: String, b: String) -> bool {
    a == b
}"#;
        let output = infer_borrows(input);
        assert!(output.contains("a: &str"), "Comparison should be read-only: {}", output);
        assert!(output.contains("b: &str"), "Comparison should be read-only: {}", output);
    }

    #[test]
    fn test_push_is_consuming() {
        let input = r#"fn store(items: Vec<String>, name: String) {
    items.push(name)
}"#;
        let output = infer_borrows(input);
        assert!(output.contains("name: String"), "push should be consuming: {}", output);
    }

    #[test]
    fn test_star_display_is_read_only() {
        let input = r#"fn show(name: String) {
    println!("{}", star_display(&(name)))
}"#;
        let output = infer_borrows(input);
        assert!(output.contains("name: &str"), "star_display should be read-only: {}", output);
        // star_display(&(name)) should become star_display(&(name.to_string()))
        assert!(output.contains("star_display(&(name.to_string()))"),
            "star_display should get .to_string(): {}", output);
    }

    #[test]
    fn test_ampersand_usage_is_read_only() {
        let input = r#"fn process(name: String) {
    do_something(&name)
}"#;
        let output = infer_borrows(input);
        assert!(output.contains("name: &str"), "&name should be read-only: {}", output);
    }

    #[test]
    fn test_main_function_skipped() {
        let input = r#"fn main() {
    let name = "world".to_string();
    println!("{}", name);
}"#;
        let output = infer_borrows(input);
        assert_eq!(output, input, "main should not be modified");
    }

    #[test]
    fn test_multiple_functions() {
        let input = r#"fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}
fn take(name: String) -> String {
    return name
}"#;
        let output = infer_borrows(input);
        assert!(output.contains("fn greet(name: &str)"), "greet should use &str: {}", output);
        assert!(output.contains("fn take(name: String)"), "take should keep String: {}", output);
    }

    #[test]
    fn test_no_functions() {
        let input = "let x = 42;\nlet y = x + 1;\n";
        let output = infer_borrows(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(infer_borrows(""), "");
    }

    #[test]
    fn test_mixed_params() {
        let input = r#"fn mixed(count: i64, name: String, flag: bool) -> String {
    format!("{}: {} ({})", count, name, flag)
}"#;
        let output = infer_borrows(input);
        assert!(output.contains("name: &str"), "name should be &str: {}", output);
        assert!(output.contains("count: i64"), "count should stay: {}", output);
        assert!(output.contains("flag: bool"), "flag should stay: {}", output);
    }

    #[test]
    fn test_find_matching_brace() {
        let code = "{ hello { world } }";
        assert_eq!(find_matching_brace(code, 0), Some(18));
    }

    #[test]
    fn test_find_matching_brace_with_string() {
        let code = r#"{ "}" }"#;
        assert_eq!(find_matching_brace(code, 0), Some(6));
    }

    #[test]
    fn test_call_site_string_literal() {
        // When greet is converted to take &str, call sites with .to_string() should simplify
        let input = r#"fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}
fn main() {
    greet("world".to_string());
}"#;
        let output = infer_borrows(input);
        assert!(output.contains("fn greet(name: &str)"), "greet should use &str: {}", output);
        assert!(output.contains(r#"greet("world")"#), "call site should simplify: {}", output);
    }

    #[test]
    fn test_call_site_format() {
        let input = r#"fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}
fn main() {
    greet(format!("world"));
}"#;
        let output = infer_borrows(input);
        assert!(output.contains("fn greet(name: &str)"), "greet should use &str: {}", output);
        assert!(output.contains(r#"greet(&format!("world"))"#), "format arg should get &: {}", output);
    }

    #[test]
    fn test_realistic_codegen() {
        let input = r#"fn star_display<T: std::fmt::Debug + std::any::Any>(val: &T) -> String {
    "helper"
}
fn greet(name: String) -> String {
    format!("Hello, {}!", star_display(&(name)))
}
fn main() {
    let result = greet("world".to_string());
    println!("{}", result);
}"#;
        let output = infer_borrows(input);
        assert!(output.contains("fn greet(name: &str)"), "greet should use &str: {}", output);
        assert!(output.contains("star_display(&(name.to_string()))"),
            "star_display should get .to_string(): {}", output);
        assert!(output.contains(r#"greet("world")"#),
            "call site should simplify: {}", output);
    }
}
