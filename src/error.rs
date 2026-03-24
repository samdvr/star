use std::fmt;

/// Source location for error reporting
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// A compiler error with source context
pub struct StarError {
    pub message: String,
    pub span: Span,
    pub kind: ErrorKind,
}

#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
    Syntax,
    Type,
    Warning,
}

impl StarError {
    pub fn syntax(span: Span, message: String) -> Self {
        Self { message, span, kind: ErrorKind::Syntax }
    }

    pub fn type_error(span: Span, message: String) -> Self {
        Self { message, span, kind: ErrorKind::Type }
    }

    pub fn warning(span: Span, message: String) -> Self {
        Self { message, span, kind: ErrorKind::Warning }
    }
}

/// Format a nice error message with source context
pub fn format_error(source: &str, file: &str, err: &StarError) -> String {
    let mut out = String::new();

    let (kind_str, color) = match err.kind {
        ErrorKind::Syntax => ("syntax error", "\x1b[1;31m"),
        ErrorKind::Type => ("type error", "\x1b[1;31m"),
        ErrorKind::Warning => ("warning", "\x1b[1;33m"),
    };

    // Header: error[kind]: message  (or warning[kind]: message)
    let label = if matches!(err.kind, ErrorKind::Warning) { "warning" } else { "error" };
    out.push_str(&format!("{color}{label}\x1b[0m[{kind_str}]: \x1b[1m{}\x1b[0m\n", err.message));

    // Location: --> file:line:col
    out.push_str(&format!("  \x1b[1;34m-->\x1b[0m {file}:{}:{}\n", err.span.line, err.span.col));

    // Source context
    let lines: Vec<&str> = source.lines().collect();
    if err.span.line > 0 && err.span.line <= lines.len() {
        let line_num = err.span.line;
        let line_str = lines[line_num - 1];
        let gutter_width = format!("{}", line_num).len();

        // Blank gutter line
        out.push_str(&format!("  {:>width$} \x1b[1;34m|\x1b[0m\n", "", width = gutter_width));

        // Source line
        out.push_str(&format!("  \x1b[1;34m{line_num}\x1b[0m \x1b[1;34m|\x1b[0m {line_str}\n"));

        // Caret line
        let col = if err.span.col > 0 { err.span.col - 1 } else { 0 };
        out.push_str(&format!(
            "  {:>width$} \x1b[1;34m|\x1b[0m {}{color}^\x1b[0m\n",
            "",
            " ".repeat(col),
            width = gutter_width
        ));
    }

    out
}

/// Format an error from a raw error string (legacy format: "line:col message")
pub fn format_error_from_string(source: &str, file: &str, error_str: &str) -> String {
    // Try to parse "line:col rest..."
    if let Some((loc, msg)) = error_str.split_once(' ') {
        if let Some((line_s, col_s)) = loc.split_once(':') {
            if let (Ok(line), Ok(col)) = (line_s.parse::<usize>(), col_s.parse::<usize>()) {
                let err = StarError::syntax(Span::new(line, col), msg.to_string());
                return format_error(source, file, &err);
            }
        }
    }
    // Fallback: just print the raw error
    format!("\x1b[1;31merror\x1b[0m: {error_str}\n")
}
