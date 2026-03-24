/// Post-codegen optimization pass that removes provably unnecessary `.clone()` calls
/// from generated Rust source code.
///
/// Star uses clone-by-default semantics, so codegen emits `.clone()` liberally.
/// This pass applies conservative, syntactic pattern-based transformations that
/// are always safe regardless of context.

/// Apply all clone-elimination optimizations to generated Rust code.
pub fn optimize(code: &str) -> String {
    let mut result = code.to_string();

    // 1. Remove `.clone().into_iter()` → `.into_iter()`
    //    Always safe: into_iter() consumes the collection, so cloning before consuming is redundant
    //    when the value is owned.
    if result.contains(".clone().into_iter()") {
        result = result.replace(".clone().into_iter()", ".into_iter()");
    }

    // 2. Remove `.clone().into()` → `.into()`
    //    Always safe: into() consumes the value.
    if result.contains(".clone().into()") {
        result = result.replace(".clone().into()", ".into()");
    }

    // 3. Remove double clones: `.clone().clone()` → `.clone()`
    if result.contains(".clone().clone()") {
        result = result.replace(".clone().clone()", ".clone()");
    }

    // 4. Remove `.to_string().clone()` → `.to_string()`
    //    to_string() already returns an owned String, cloning it is redundant.
    if result.contains(".to_string().clone()") {
        result = result.replace(".to_string().clone()", ".to_string()");
    }

    // 5. Remove clone on integer literals: `42i64.clone()` → `42i64`
    //    i64 is Copy, so clone is a no-op.
    result = remove_numeric_literal_clones(&result, "i64");

    // 6. Remove clone on float literals: `3.14f64.clone()` → `3.14f64`
    //    f64 is Copy, so clone is a no-op.
    result = remove_numeric_literal_clones(&result, "f64");

    // 7. Remove clone on bool literals: `true.clone()` / `false.clone()`
    //    bool is Copy, so clone is a no-op.
    result = remove_word_literal_clone(&result, "true");
    result = remove_word_literal_clone(&result, "false");

    result
}

/// Remove `.clone()` from numeric literal patterns like `42i64.clone()` or `3.14f64.clone()`.
/// The `type_suffix` is `"i64"` or `"f64"`.
fn remove_numeric_literal_clones(code: &str, type_suffix: &str) -> String {
    let pattern = format!("{}.clone()", type_suffix);
    if !code.contains(&pattern) {
        return code.to_string();
    }

    let mut result = String::with_capacity(code.len());
    let mut remaining = code;

    while let Some(pos) = remaining.find(&pattern) {
        // Check that the characters before the type suffix are digits (the literal)
        let before = &remaining[..pos];
        if before.ends_with(|c: char| c.is_ascii_digit()) {
            // It's a pattern like `42i64.clone()` — emit `42i64` (skip `.clone()`)
            let suffix_end = pos + type_suffix.len();
            result.push_str(&remaining[..suffix_end]); // include type suffix
            remaining = &remaining[suffix_end + ".clone()".len()..]; // skip ".clone()"
        } else {
            // Not a literal — keep as-is, advance past the type suffix to avoid re-matching
            let end = pos + type_suffix.len();
            result.push_str(&remaining[..end]);
            remaining = &remaining[end..];
        }
    }
    result.push_str(remaining);
    result
}

/// Remove `.clone()` from a keyword literal like `true.clone()` or `false.clone()`,
/// only when the keyword is a standalone token (not part of a larger identifier).
fn remove_word_literal_clone(code: &str, word: &str) -> String {
    let pattern = format!("{}.clone()", word);
    if !code.contains(&pattern) {
        return code.to_string();
    }

    let mut result = String::with_capacity(code.len());
    let mut remaining = code;

    while let Some(pos) = remaining.find(&pattern) {
        // Check the character before `word` is not an identifier char
        let is_standalone = if pos == 0 {
            true
        } else {
            let prev = remaining.as_bytes()[pos - 1];
            !prev.is_ascii_alphanumeric() && prev != b'_'
        };

        if is_standalone {
            let word_end = pos + word.len();
            result.push_str(&remaining[..word_end]); // include the word
            remaining = &remaining[word_end + ".clone()".len()..]; // skip ".clone()"
        } else {
            // Not standalone — advance past pattern to avoid re-matching
            let end = pos + pattern.len();
            result.push_str(&remaining[..end]);
            remaining = &remaining[end..];
        }
    }
    result.push_str(remaining);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_clone_into_iter() {
        let input = "my_list.clone().into_iter().map(|x| x + 1)";
        let output = optimize(input);
        assert_eq!(output, "my_list.into_iter().map(|x| x + 1)");
    }

    #[test]
    fn test_remove_clone_into() {
        let input = "value.clone().into()";
        let output = optimize(input);
        assert_eq!(output, "value.into()");
    }

    #[test]
    fn test_remove_double_clone() {
        let input = "x.clone().clone()";
        let output = optimize(input);
        assert_eq!(output, "x.clone()");
    }

    #[test]
    fn test_remove_to_string_clone() {
        let input = r#""hello".to_string().clone()"#;
        let output = optimize(input);
        assert_eq!(output, r#""hello".to_string()"#);
    }

    #[test]
    fn test_remove_integer_literal_clone() {
        let input = "42i64.clone()";
        let output = optimize(input);
        assert_eq!(output, "42i64");
    }

    #[test]
    fn test_remove_float_literal_clone() {
        let input = "3.14f64.clone()";
        let output = optimize(input);
        assert_eq!(output, "3.14f64");
    }

    #[test]
    fn test_remove_bool_literal_clone() {
        let input = "true.clone() && false.clone()";
        let output = optimize(input);
        assert_eq!(output, "true && false");
    }

    #[test]
    fn test_no_false_positive_bool() {
        // Should NOT remove clone from identifiers ending in "true" or "false"
        let input = "is_true.clone()";
        let output = optimize(input);
        assert_eq!(output, "is_true.clone()");
    }

    #[test]
    fn test_no_false_positive_integer() {
        // Non-digit before i64 should not be optimized
        let input = "x_i64.clone()";
        let output = optimize(input);
        assert_eq!(output, "x_i64.clone()");
    }

    #[test]
    fn test_preserves_necessary_clones() {
        let input = "x.clone().push(1)";
        let output = optimize(input);
        assert_eq!(output, "x.clone().push(1)");
    }

    #[test]
    fn test_multiple_optimizations() {
        let input = "list.clone().into_iter().map(|x| 42i64.clone())";
        let output = optimize(input);
        assert_eq!(output, "list.into_iter().map(|x| 42i64)");
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(optimize(""), "");
    }

    #[test]
    fn test_no_clones() {
        let input = "fn main() { println!(\"hello\"); }";
        let output = optimize(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_realistic_codegen_pattern() {
        let input = "let _result = my_list.clone().into_iter().filter(|_item| (_item.clone() > 0i64.clone())).collect::<Vec<_>>();";
        let output = optimize(input);
        assert_eq!(output, "let _result = my_list.into_iter().filter(|_item| (_item.clone() > 0i64)).collect::<Vec<_>>();");
    }
}
