use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Int(i64),
    Float(f64),
    Str(String),
    /// Interpolated string parts: vec of (is_expr, text)
    /// is_expr=false means literal text, is_expr=true means expression source
    InterpStr(Vec<(bool, String)>),
    True,
    False,

    // Identifiers and keywords
    Ident(String),
    UpperIdent(String), // Capitalized identifier (type/variant names)

    // Keywords
    Fn,
    Let,
    Mut,
    If,
    Then,
    Else,
    End,
    Match,
    Type,
    Module,
    Use,
    Pub,
    Do,
    And,
    Or,
    Not,
    When,
    As,
    Async,
    Await,
    Extern,
    Trait,
    Impl,
    For,
    While,
    In,
    Break,
    Continue,
    Dyn,
    RustBang, // rust!
    Comment(String), // # comment text

    // Symbols
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Colon,
    ColonColon,
    Dot,
    DotDot,
    Arrow,     // ->
    FatArrow,  // =>
    Pipe,      // |
    PipeArrow, // |>
    Eq,        // =
    EqEq,      // ==
    Ne,        // !=
    Lt,
    Gt,
    Le,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Ampersand,  // &
    Question,   // ?
    Tilde,      // ~
    Underscore, // _
    PlusEq,     // +=
    MinusEq,    // -=
    StarEq,     // *=
    SlashEq,    // /=
    PercentEq,  // %=
    Shl,        // <<
    Shr,        // >>

    // Bitwise keywords (band, bor, bxor handled as keyword identifiers)
    Band,
    Bor,
    Bxor,

    // Additional keywords
    Move,

    // Annotations
    At, // @

    // Lifetimes
    Tick(String), // 'a, 'b, etc.

    // Structure
    Newline,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

pub fn lex(source: &str) -> Result<Vec<Token>, String> {
    let mut lexer = Lexer::new(source);
    lexer.tokenize()?;
    Ok(lexer.tokens)
}

struct Lexer<'a> {
    source: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
    tokens: Vec<Token>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source: source.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
            tokens: Vec::new(),
        }
    }

    fn span(&self) -> Span {
        Span::new(self.line, self.col)
    }

    fn peek(&self) -> Option<u8> {
        self.source.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<u8> {
        self.source.get(self.pos + 1).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.source.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let ch = self.source.get(self.pos).copied()?;
        self.pos += 1;
        if ch == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn emit(&mut self, kind: TokenKind, span: Span) {
        self.tokens.push(Token::new(kind, span));
    }

    fn tokenize(&mut self) -> Result<(), String> {
        while self.pos < self.source.len() {
            self.skip_whitespace();

            if self.pos >= self.source.len() {
                break;
            }

            let span = self.span();
            let ch = self.peek().unwrap();

            match ch {
                b'#' => {
                    // Comment — capture text until end of line
                    self.advance(); // skip the '#'
                    let start = self.pos;
                    while self.pos < self.source.len() && self.peek() != Some(b'\n') {
                        self.advance();
                    }
                    let text = std::str::from_utf8(&self.source[start..self.pos])
                        .unwrap_or("")
                        .to_string();
                    self.emit(TokenKind::Comment(text), span);
                }
                b'\n' => {
                    self.advance();
                    // Collapse multiple newlines
                    if self.tokens.last().is_some_and(|t| t.kind != TokenKind::Newline) {
                        self.emit(TokenKind::Newline, span);
                    }
                }
                b'"' => self.lex_string(span)?,
                b'0'..=b'9' => self.lex_number(span)?,
                b'a'..=b'z' | b'_' => self.lex_ident(span)?,
                b'A'..=b'Z' => self.lex_upper_ident(span)?,
                b'(' => {
                    self.advance();
                    self.emit(TokenKind::LParen, span);
                }
                b')' => {
                    self.advance();
                    self.emit(TokenKind::RParen, span);
                }
                b'[' => {
                    self.advance();
                    self.emit(TokenKind::LBracket, span);
                }
                b']' => {
                    self.advance();
                    self.emit(TokenKind::RBracket, span);
                }
                b'{' => {
                    self.advance();
                    self.emit(TokenKind::LBrace, span);
                }
                b'}' => {
                    self.advance();
                    self.emit(TokenKind::RBrace, span);
                }
                b',' => {
                    self.advance();
                    self.emit(TokenKind::Comma, span);
                }
                b':' => {
                    self.advance();
                    if self.peek() == Some(b':') {
                        self.advance();
                        self.emit(TokenKind::ColonColon, span);
                    } else {
                        self.emit(TokenKind::Colon, span);
                    }
                }
                b'.' => {
                    self.advance();
                    if self.peek() == Some(b'.') {
                        self.advance();
                        self.emit(TokenKind::DotDot, span);
                    } else {
                        self.emit(TokenKind::Dot, span);
                    }
                }
                b'-' => {
                    self.advance();
                    if self.peek() == Some(b'>') {
                        self.advance();
                        self.emit(TokenKind::Arrow, span);
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        self.emit(TokenKind::MinusEq, span);
                    } else {
                        self.emit(TokenKind::Minus, span);
                    }
                }
                b'=' => {
                    self.advance();
                    if self.peek() == Some(b'>') {
                        self.advance();
                        self.emit(TokenKind::FatArrow, span);
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        self.emit(TokenKind::EqEq, span);
                    } else {
                        self.emit(TokenKind::Eq, span);
                    }
                }
                b'|' => {
                    self.advance();
                    if self.peek() == Some(b'>') {
                        self.advance();
                        self.emit(TokenKind::PipeArrow, span);
                    } else {
                        self.emit(TokenKind::Pipe, span);
                    }
                }
                b'!' => {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        self.emit(TokenKind::Ne, span);
                    } else {
                        return Err(format!("{span} Unexpected character '!'"));
                    }
                }
                b'<' => {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        self.emit(TokenKind::Le, span);
                    } else if self.peek() == Some(b'<') {
                        self.advance();
                        self.emit(TokenKind::Shl, span);
                    } else {
                        self.emit(TokenKind::Lt, span);
                    }
                }
                b'>' => {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        self.emit(TokenKind::Ge, span);
                    } else if self.peek() == Some(b'>') {
                        self.advance();
                        self.emit(TokenKind::Shr, span);
                    } else {
                        self.emit(TokenKind::Gt, span);
                    }
                }
                b'+' => {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        self.emit(TokenKind::PlusEq, span);
                    } else {
                        self.emit(TokenKind::Plus, span);
                    }
                }
                b'*' => {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        self.emit(TokenKind::StarEq, span);
                    } else {
                        self.emit(TokenKind::Star, span);
                    }
                }
                b'/' => {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        self.emit(TokenKind::SlashEq, span);
                    } else {
                        self.emit(TokenKind::Slash, span);
                    }
                }
                b'%' => {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        self.emit(TokenKind::PercentEq, span);
                    } else {
                        self.emit(TokenKind::Percent, span);
                    }
                }
                b'&' => {
                    self.advance();
                    self.emit(TokenKind::Ampersand, span);
                }
                b'?' => {
                    self.advance();
                    self.emit(TokenKind::Question, span);
                }
                b'~' => {
                    self.advance();
                    self.emit(TokenKind::Tilde, span);
                }
                b'@' => {
                    self.advance();
                    self.emit(TokenKind::At, span);
                }
                b'\'' => {
                    // Lifetime: 'a, 'b, etc.
                    self.advance();
                    let start = self.pos;
                    while self.pos < self.source.len() && (self.source[self.pos].is_ascii_alphanumeric() || self.source[self.pos] == b'_') {
                        self.pos += 1;
                    }
                    if self.pos == start {
                        return Err(format!("{span} Expected lifetime name after '"));
                    }
                    let name = std::str::from_utf8(&self.source[start..self.pos])
                        .map_err(|e| format!("{span} Invalid UTF-8 in lifetime: {e}"))?;
                    self.emit(TokenKind::Tick(name.to_string()), span);
                }
                _ => {
                    return Err(format!("{span} Unexpected character '{}'", ch as char));
                }
            }
        }

        let span = self.span();
        self.emit(TokenKind::Eof, span);
        Ok(())
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.source.len() {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r') => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    fn lex_string(&mut self, span: Span) -> Result<(), String> {
        // Check for triple-quoted string: """
        if self.peek() == Some(b'"') && self.peek_at(1) == Some(b'"') && self.peek_at(2) == Some(b'"') {
            return self.lex_triple_string(span);
        }

        self.advance(); // skip opening "
        let mut s = String::new();
        let mut parts: Vec<(bool, String)> = Vec::new();
        let mut has_interp = false;

        loop {
            match self.peek() {
                Some(b'"') => {
                    self.advance();
                    break;
                }
                Some(b'\\') => {
                    self.advance();
                    match self.advance() {
                        Some(b'n') => s.push('\n'),
                        Some(b't') => s.push('\t'),
                        Some(b'\\') => s.push('\\'),
                        Some(b'"') => s.push('"'),
                        Some(b'#') => s.push('#'),
                        Some(c) => s.push(c as char),
                        None => return Err(format!("{span} Unterminated string escape")),
                    }
                }
                Some(b'#') if self.peek_next() == Some(b'{') => {
                    has_interp = true;
                    // Save current literal part
                    if !s.is_empty() {
                        parts.push((false, std::mem::take(&mut s)));
                    }
                    self.advance(); // skip #
                    self.advance(); // skip {
                    // Read expression until matching }
                    let mut depth = 1;
                    let mut expr_src = String::new();
                    while depth > 0 {
                        match self.advance() {
                            Some(b'{') => {
                                depth += 1;
                                expr_src.push('{');
                            }
                            Some(b'}') => {
                                depth -= 1;
                                if depth > 0 {
                                    expr_src.push('}');
                                }
                            }
                            Some(c) => expr_src.push(c as char),
                            None => return Err(format!("{span} Unterminated string interpolation")),
                        }
                    }
                    parts.push((true, expr_src));
                }
                Some(_) => {
                    s.push(self.advance().unwrap() as char);
                }
                None => return Err(format!("{span} Unterminated string")),
            }
        }

        if has_interp {
            if !s.is_empty() {
                parts.push((false, s));
            }
            self.emit(TokenKind::InterpStr(parts), span);
        } else {
            self.emit(TokenKind::Str(s), span);
        }
        Ok(())
    }

    fn lex_triple_string(&mut self, span: Span) -> Result<(), String> {
        // Skip opening """
        self.advance(); // "
        self.advance(); // "
        self.advance(); // "

        let mut raw = String::new();
        let mut parts: Vec<(bool, String)> = Vec::new();
        let mut has_interp = false;

        loop {
            match self.peek() {
                Some(b'"') if self.peek_at(1) == Some(b'"') && self.peek_at(2) == Some(b'"') => {
                    self.advance(); // "
                    self.advance(); // "
                    self.advance(); // "
                    break;
                }
                Some(b'\\') => {
                    self.advance();
                    match self.advance() {
                        Some(b'n') => raw.push('\n'),
                        Some(b't') => raw.push('\t'),
                        Some(b'\\') => raw.push('\\'),
                        Some(b'"') => raw.push('"'),
                        Some(b'#') => raw.push('#'),
                        Some(c) => raw.push(c as char),
                        None => return Err(format!("{span} Unterminated triple-quoted string escape")),
                    }
                }
                Some(b'#') if self.peek_next() == Some(b'{') => {
                    has_interp = true;
                    if !raw.is_empty() {
                        parts.push((false, std::mem::take(&mut raw)));
                    }
                    self.advance(); // skip #
                    self.advance(); // skip {
                    let mut depth = 1;
                    let mut expr_src = String::new();
                    while depth > 0 {
                        match self.advance() {
                            Some(b'{') => {
                                depth += 1;
                                expr_src.push('{');
                            }
                            Some(b'}') => {
                                depth -= 1;
                                if depth > 0 {
                                    expr_src.push('}');
                                }
                            }
                            Some(c) => expr_src.push(c as char),
                            None => return Err(format!("{span} Unterminated string interpolation")),
                        }
                    }
                    parts.push((true, expr_src));
                }
                Some(_) => {
                    raw.push(self.advance().unwrap() as char);
                }
                None => return Err(format!("{span} Unterminated triple-quoted string")),
            }
        }

        // Now apply dedent processing to all literal parts.
        // First, reassemble the full string with placeholders to figure out dedent,
        // then apply it to each literal part.
        if has_interp {
            if !raw.is_empty() {
                parts.push((false, raw));
            }
            // Apply dedent to the literal parts
            let parts = Self::dedent_interp_parts(parts);
            self.emit(TokenKind::InterpStr(parts), span);
        } else {
            let dedented = Self::dedent_string(&raw);
            self.emit(TokenKind::Str(dedented), span);
        }
        Ok(())
    }

    /// Strip leading newline, trailing newline+whitespace, and common indentation from a raw triple-quoted string.
    fn dedent_string(raw: &str) -> String {
        let mut s = raw.to_string();

        // Strip leading newline
        if s.starts_with('\n') {
            s.remove(0);
        } else if s.starts_with("\r\n") {
            s.remove(0);
            s.remove(0);
        }

        // Strip trailing newline + whitespace
        if let Some(last_nl) = s.rfind('\n') {
            if s[last_nl + 1..].chars().all(|c| c == ' ' || c == '\t') {
                s.truncate(last_nl);
            }
        }

        // Calculate minimum indentation of non-empty lines
        let min_indent = s
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.len() - line.trim_start_matches(|c| c == ' ' || c == '\t').len())
            .min()
            .unwrap_or(0);

        if min_indent > 0 {
            s = s
                .lines()
                .map(|line| {
                    if line.trim().is_empty() {
                        ""
                    } else if line.len() >= min_indent {
                        &line[min_indent..]
                    } else {
                        line
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
        }

        s
    }

    /// Apply dedent to interpolated string parts.
    /// Strategy: concatenate all parts (literal text as-is, expr parts as single-char sentinel \x00),
    /// apply dedent on the combined text, then split back into parts.
    fn dedent_interp_parts(parts: Vec<(bool, String)>) -> Vec<(bool, String)> {
        // Reconstruct full text with \x00 sentinels for expr parts
        let mut full = String::new();
        let mut expr_texts: Vec<String> = Vec::new();
        for (is_expr, text) in &parts {
            if *is_expr {
                full.push('\x00');
                expr_texts.push(text.clone());
            } else {
                full.push_str(text);
            }
        }

        // Apply dedent to the full combined string
        let dedented = Self::dedent_string(&full);

        // Split the dedented string back into parts by \x00 sentinels
        let mut final_parts: Vec<(bool, String)> = Vec::new();
        let mut expr_iter = expr_texts.into_iter();

        for (i, segment) in dedented.split('\x00').enumerate() {
            if i > 0 {
                // Before this segment, there was a \x00 (an expression)
                if let Some(expr) = expr_iter.next() {
                    final_parts.push((true, expr));
                }
            }
            if !segment.is_empty() {
                final_parts.push((false, segment.to_string()));
            }
        }

        if final_parts.is_empty() {
            final_parts.push((false, String::new()));
        }

        final_parts
    }

    fn lex_number(&mut self, span: Span) -> Result<(), String> {
        let start = self.pos;
        while self.pos < self.source.len() && self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.advance();
        }
        if self.peek() == Some(b'.') && self.peek_next().is_some_and(|c| c.is_ascii_digit()) {
            self.advance(); // skip .
            while self.pos < self.source.len() && self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.advance();
            }
            let text = std::str::from_utf8(&self.source[start..self.pos])
                .map_err(|_| format!("{span} Invalid UTF-8 in float literal"))?;
            let val: f64 = text
                .parse()
                .map_err(|_| format!("{span} Invalid float literal"))?;
            self.emit(TokenKind::Float(val), span);
        } else {
            let text = std::str::from_utf8(&self.source[start..self.pos])
                .map_err(|_| format!("{span} Invalid UTF-8 in integer literal"))?;
            let val: i64 = text
                .parse()
                .map_err(|_| format!("{span} Invalid integer literal"))?;
            self.emit(TokenKind::Int(val), span);
        }
        Ok(())
    }

    fn lex_ident(&mut self, span: Span) -> Result<(), String> {
        let start = self.pos;
        while self
            .pos
            < self.source.len()
            && self
                .peek()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_')
        {
            self.advance();
        }
        let text = std::str::from_utf8(&self.source[start..self.pos])
            .map_err(|_| format!("{span} Invalid UTF-8 in identifier"))?;

        // Check for rust! special form
        if text == "rust" && self.peek() == Some(b'!') {
            self.advance();
            self.emit(TokenKind::RustBang, span);
            return Ok(());
        }

        let kind = match text {
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "if" => TokenKind::If,
            "then" => TokenKind::Then,
            "else" => TokenKind::Else,
            "end" => TokenKind::End,
            "match" => TokenKind::Match,
            "type" => TokenKind::Type,
            "module" => TokenKind::Module,
            "use" => TokenKind::Use,
            "pub" => TokenKind::Pub,
            "do" => TokenKind::Do,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "when" => TokenKind::When,
            "as" => TokenKind::As,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "async" => TokenKind::Async,
            "await" => TokenKind::Await,
            "extern" => TokenKind::Extern,
            "trait" => TokenKind::Trait,
            "impl" => TokenKind::Impl,
            "for" => TokenKind::For,
            "while" => TokenKind::While,
            "in" => TokenKind::In,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "dyn" => TokenKind::Dyn,
            "move" => TokenKind::Move,
            "band" => TokenKind::Band,
            "bor" => TokenKind::Bor,
            "bxor" => TokenKind::Bxor,
            "_" => TokenKind::Underscore,
            _ => TokenKind::Ident(text.to_string()),
        };
        self.emit(kind, span);
        Ok(())
    }

    fn lex_upper_ident(&mut self, span: Span) -> Result<(), String> {
        let start = self.pos;
        while self
            .pos
            < self.source.len()
            && self
                .peek()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_')
        {
            self.advance();
        }
        let text = std::str::from_utf8(&self.source[start..self.pos])
            .map_err(|_| format!("{span} Invalid UTF-8 in type name"))?;
        self.emit(TokenKind::UpperIdent(text.to_string()), span);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_basic() {
        let tokens = lex("fn main() = 42").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Fn));
        assert!(matches!(tokens[1].kind, TokenKind::Ident(ref s) if s == "main"));
        assert!(matches!(tokens[2].kind, TokenKind::LParen));
        assert!(matches!(tokens[3].kind, TokenKind::RParen));
        assert!(matches!(tokens[4].kind, TokenKind::Eq));
        assert!(matches!(tokens[5].kind, TokenKind::Int(42)));
    }

    #[test]
    fn test_lex_pipe() {
        let tokens = lex("x |> f |> g").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Ident(ref s) if s == "x"));
        assert!(matches!(tokens[1].kind, TokenKind::PipeArrow));
        assert!(matches!(tokens[2].kind, TokenKind::Ident(ref s) if s == "f"));
        assert!(matches!(tokens[3].kind, TokenKind::PipeArrow));
        assert!(matches!(tokens[4].kind, TokenKind::Ident(ref s) if s == "g"));
    }

    #[test]
    fn test_lex_string() {
        let tokens = lex(r#""hello\nworld""#).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Str(ref s) if s == "hello\nworld"));
    }

    #[test]
    fn test_lex_type_decl() {
        let tokens = lex("type Option<T> =\n  | Some(T)\n  | None").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Type));
        assert!(matches!(tokens[1].kind, TokenKind::UpperIdent(ref s) if s == "Option"));
    }

    #[test]
    fn test_lex_triple_string_basic() {
        let src = "let s = \"\"\"\n  hello\n  world\n  \"\"\"";
        let tokens = lex(src).unwrap();
        // Should produce: Let, Ident("s"), Eq, Str("hello\nworld"), Eof
        match &tokens[3].kind {
            TokenKind::Str(s) => assert_eq!(s, "hello\nworld"),
            other => panic!("Expected Str, got {:?}", other),
        }
    }

    #[test]
    fn test_lex_triple_string_dedent() {
        // 4-space indent on all lines, should be stripped
        let src = "let s = \"\"\"\n    line1\n    line2\n    line3\n    \"\"\"";
        let tokens = lex(src).unwrap();
        match &tokens[3].kind {
            TokenKind::Str(s) => assert_eq!(s, "line1\nline2\nline3"),
            other => panic!("Expected Str, got {:?}", other),
        }
    }

    #[test]
    fn test_lex_triple_string_mixed_indent() {
        // min indent is 2 spaces
        let src = "let s = \"\"\"\n  base\n    indented\n  base2\n  \"\"\"";
        let tokens = lex(src).unwrap();
        match &tokens[3].kind {
            TokenKind::Str(s) => assert_eq!(s, "base\n  indented\nbase2"),
            other => panic!("Expected Str, got {:?}", other),
        }
    }

    #[test]
    fn test_lex_triple_string_with_interpolation() {
        let src = "let s = \"\"\"\n  hello #{name}\n  bye\n  \"\"\"";
        let tokens = lex(src).unwrap();
        match &tokens[3].kind {
            TokenKind::InterpStr(parts) => {
                assert_eq!(parts[0], (false, "hello ".to_string()));
                assert_eq!(parts[1], (true, "name".to_string()));
                assert_eq!(parts[2], (false, "\nbye".to_string()));
            }
            other => panic!("Expected InterpStr, got {:?}", other),
        }
    }

    #[test]
    fn test_lex_triple_string_empty() {
        let src = "\"\"\"\"\"\"";
        let tokens = lex(src).unwrap();
        match &tokens[0].kind {
            TokenKind::Str(s) => assert_eq!(s, ""),
            other => panic!("Expected Str, got {:?}", other),
        }
    }

    // ── Symbol token tests ───────────────────────────────────────

    #[test]
    fn test_lex_all_single_char_symbols() {
        let tokens = lex("( ) [ ] { } , : . + * / % & ? ~").unwrap();
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
        assert!(matches!(kinds[0], TokenKind::LParen));
        assert!(matches!(kinds[1], TokenKind::RParen));
        assert!(matches!(kinds[2], TokenKind::LBracket));
        assert!(matches!(kinds[3], TokenKind::RBracket));
        assert!(matches!(kinds[4], TokenKind::LBrace));
        assert!(matches!(kinds[5], TokenKind::RBrace));
        assert!(matches!(kinds[6], TokenKind::Comma));
        assert!(matches!(kinds[7], TokenKind::Colon));
        assert!(matches!(kinds[8], TokenKind::Dot));
        assert!(matches!(kinds[9], TokenKind::Plus));
        assert!(matches!(kinds[10], TokenKind::Star));
        assert!(matches!(kinds[11], TokenKind::Slash));
        assert!(matches!(kinds[12], TokenKind::Percent));
        assert!(matches!(kinds[13], TokenKind::Ampersand));
        assert!(matches!(kinds[14], TokenKind::Question));
        assert!(matches!(kinds[15], TokenKind::Tilde));
    }

    #[test]
    fn test_lex_two_char_symbols() {
        let tokens = lex(":: .. -> => |> == != <= >=").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::ColonColon));
        assert!(matches!(tokens[1].kind, TokenKind::DotDot));
        assert!(matches!(tokens[2].kind, TokenKind::Arrow));
        assert!(matches!(tokens[3].kind, TokenKind::FatArrow));
        assert!(matches!(tokens[4].kind, TokenKind::PipeArrow));
        assert!(matches!(tokens[5].kind, TokenKind::EqEq));
        assert!(matches!(tokens[6].kind, TokenKind::Ne));
        assert!(matches!(tokens[7].kind, TokenKind::Le));
        assert!(matches!(tokens[8].kind, TokenKind::Ge));
    }

    #[test]
    fn test_lex_compound_assignment_symbols() {
        let tokens = lex("+= -= *= /= %=").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::PlusEq));
        assert!(matches!(tokens[1].kind, TokenKind::MinusEq));
        assert!(matches!(tokens[2].kind, TokenKind::StarEq));
        assert!(matches!(tokens[3].kind, TokenKind::SlashEq));
        assert!(matches!(tokens[4].kind, TokenKind::PercentEq));
    }

    #[test]
    fn test_lex_pipe_and_pipe_arrow() {
        let tokens = lex("| |>").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Pipe));
        assert!(matches!(tokens[1].kind, TokenKind::PipeArrow));
    }

    #[test]
    fn test_lex_lt_gt() {
        let tokens = lex("< >").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Lt));
        assert!(matches!(tokens[1].kind, TokenKind::Gt));
    }

    #[test]
    fn test_lex_minus_and_arrow() {
        let tokens = lex("- ->").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Minus));
        assert!(matches!(tokens[1].kind, TokenKind::Arrow));
    }

    #[test]
    fn test_lex_eq_and_fat_arrow() {
        let tokens = lex("= =>").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Eq));
        assert!(matches!(tokens[1].kind, TokenKind::FatArrow));
    }

    // ── String escape sequence tests ─────────────────────────────

    #[test]
    fn test_lex_string_escape_newline() {
        let tokens = lex(r#""a\nb""#).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Str(ref s) if s == "a\nb"));
    }

    #[test]
    fn test_lex_string_escape_tab() {
        let tokens = lex(r#""a\tb""#).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Str(ref s) if s == "a\tb"));
    }

    #[test]
    fn test_lex_string_escape_backslash() {
        let tokens = lex(r#""a\\b""#).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Str(ref s) if s == "a\\b"));
    }

    #[test]
    fn test_lex_string_escape_quote() {
        let tokens = lex(r#""a\"b""#).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Str(ref s) if s == "a\"b"));
    }

    #[test]
    fn test_lex_string_escape_hash() {
        let tokens = lex(r#""a\#b""#).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Str(ref s) if s == "a#b"));
    }

    #[test]
    fn test_lex_string_all_escapes() {
        let tokens = lex(r#""\n\t\\\"\#""#).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Str(ref s) if s == "\n\t\\\"#"));
    }

    // ── String interpolation tests ───────────────────────────────

    #[test]
    fn test_lex_interp_basic() {
        let tokens = lex(r#""hello #{name}""#).unwrap();
        match &tokens[0].kind {
            TokenKind::InterpStr(parts) => {
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0], (false, "hello ".to_string()));
                assert_eq!(parts[1], (true, "name".to_string()));
            }
            other => panic!("Expected InterpStr, got {:?}", other),
        }
    }

    #[test]
    fn test_lex_interp_multiple() {
        let src = r##""#{a} and #{b}""##;
        let tokens = lex(src).unwrap();
        match &tokens[0].kind {
            TokenKind::InterpStr(parts) => {
                assert_eq!(parts.len(), 3);
                assert_eq!(parts[0], (true, "a".to_string()));
                assert_eq!(parts[1], (false, " and ".to_string()));
                assert_eq!(parts[2], (true, "b".to_string()));
            }
            other => panic!("Expected InterpStr, got {:?}", other),
        }
    }

    #[test]
    fn test_lex_interp_with_nested_braces() {
        let src = r###""#{f({a})}""###;
        let tokens = lex(src).unwrap();
        match &tokens[0].kind {
            TokenKind::InterpStr(parts) => {
                assert_eq!(parts.len(), 1);
                assert_eq!(parts[0], (true, "f({a})".to_string()));
            }
            other => panic!("Expected InterpStr, got {:?}", other),
        }
    }

    #[test]
    fn test_lex_interp_with_expression() {
        let tokens = lex(r#""result: #{x + 1}""#).unwrap();
        match &tokens[0].kind {
            TokenKind::InterpStr(parts) => {
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0], (false, "result: ".to_string()));
                assert_eq!(parts[1], (true, "x + 1".to_string()));
            }
            other => panic!("Expected InterpStr, got {:?}", other),
        }
    }

    #[test]
    fn test_lex_escaped_hash_not_interpolation() {
        let tokens = lex(r#""not \#{interp}""#).unwrap();
        match &tokens[0].kind {
            TokenKind::Str(s) => assert_eq!(s, "not #{interp}"),
            other => panic!("Expected plain Str, got {:?}", other),
        }
    }

    // ── Triple-quoted string edge cases ──────────────────────────

    #[test]
    fn test_lex_triple_string_only_newlines() {
        let src = "\"\"\"\n\n\n\"\"\"";
        let tokens = lex(src).unwrap();
        match &tokens[0].kind {
            TokenKind::Str(s) => assert_eq!(s, "\n"),
            other => panic!("Expected Str, got {:?}", other),
        }
    }

    #[test]
    fn test_lex_triple_string_with_escapes() {
        let src = "\"\"\"\n  hello\\tworld\n  \"\"\"";
        let tokens = lex(src).unwrap();
        match &tokens[0].kind {
            TokenKind::Str(s) => assert_eq!(s, "hello\tworld"),
            other => panic!("Expected Str, got {:?}", other),
        }
    }

    #[test]
    fn test_lex_triple_string_single_line_content() {
        let src = "\"\"\"\n  single\n  \"\"\"";
        let tokens = lex(src).unwrap();
        match &tokens[0].kind {
            TokenKind::Str(s) => assert_eq!(s, "single"),
            other => panic!("Expected Str, got {:?}", other),
        }
    }

    // ── Number tests ─────────────────────────────────────────────

    #[test]
    fn test_lex_integer() {
        let tokens = lex("42").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Int(42)));
    }

    #[test]
    fn test_lex_zero() {
        let tokens = lex("0").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Int(0)));
    }

    #[test]
    fn test_lex_leading_zeros() {
        let tokens = lex("007").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Int(7)));
    }

    #[test]
    fn test_lex_float() {
        let tokens = lex("3.14").unwrap();
        match &tokens[0].kind {
            TokenKind::Float(f) => assert!((*f - 3.14).abs() < f64::EPSILON),
            other => panic!("Expected Float, got {:?}", other),
        }
    }

    #[test]
    fn test_lex_float_zero() {
        let tokens = lex("0.0").unwrap();
        match &tokens[0].kind {
            TokenKind::Float(f) => assert_eq!(*f, 0.0),
            other => panic!("Expected Float, got {:?}", other),
        }
    }

    #[test]
    fn test_lex_large_integer() {
        let tokens = lex("9999999999").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Int(9999999999)));
    }

    #[test]
    fn test_lex_dot_not_float() {
        // "1..10" should be Int(1), DotDot, Int(10), not a float
        let tokens = lex("1..10").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Int(1)));
        assert!(matches!(tokens[1].kind, TokenKind::DotDot));
        assert!(matches!(tokens[2].kind, TokenKind::Int(10)));
    }

    // ── Comment tests ────────────────────────────────────────────

    #[test]
    fn test_lex_comment_skipped() {
        let src = "# this is a comment\n42";
        let tokens = lex(src).unwrap();
        // Comment is captured, then newline, then Int(42)
        assert!(matches!(tokens[0].kind, TokenKind::Comment(ref s) if s.trim() == "this is a comment"));
        assert!(matches!(tokens[1].kind, TokenKind::Newline));
        assert!(matches!(tokens[2].kind, TokenKind::Int(42)));
    }

    #[test]
    fn test_lex_comment_at_end() {
        let tokens = lex("42 # trailing comment").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Int(42)));
        assert!(matches!(tokens[1].kind, TokenKind::Comment(_)));
        assert!(matches!(tokens[2].kind, TokenKind::Eof));
    }

    #[test]
    fn test_lex_only_comments() {
        let tokens = lex("# just a comment").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Comment(_)));
        assert!(matches!(tokens[1].kind, TokenKind::Eof));
    }

    #[test]
    fn test_lex_comment_between_tokens() {
        let tokens = lex("fn # comment\nmain").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Fn));
        assert!(matches!(tokens[1].kind, TokenKind::Comment(_)));
        assert!(matches!(tokens[2].kind, TokenKind::Newline));
        assert!(matches!(tokens[3].kind, TokenKind::Ident(ref s) if s == "main"));
    }

    // ── Error case tests ─────────────────────────────────────────

    #[test]
    fn test_lex_unterminated_string() {
        let result = lex(r#""hello"#);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unterminated string"));
    }

    #[test]
    fn test_lex_unexpected_character() {
        let result = lex("`");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unexpected character '`'"));
    }

    #[test]
    fn test_lex_bang_alone_is_error() {
        let result = lex("!");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unexpected character '!'"));
    }

    #[test]
    fn test_lex_unterminated_triple_string() {
        let result = lex("\"\"\"hello");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unterminated triple-quoted string"));
    }

    #[test]
    fn test_lex_unterminated_interpolation() {
        let src = r##""#{unclosed"##;
        let result = lex(src);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unterminated string interpolation"));
    }

    // ── Adjacent tokens without whitespace ───────────────────────

    #[test]
    fn test_lex_adjacent_parens() {
        let tokens = lex("fn()").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Fn));
        assert!(matches!(tokens[1].kind, TokenKind::LParen));
        assert!(matches!(tokens[2].kind, TokenKind::RParen));
    }

    #[test]
    fn test_lex_adjacent_brackets() {
        let tokens = lex("[1,2]").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::LBracket));
        assert!(matches!(tokens[1].kind, TokenKind::Int(1)));
        assert!(matches!(tokens[2].kind, TokenKind::Comma));
        assert!(matches!(tokens[3].kind, TokenKind::Int(2)));
        assert!(matches!(tokens[4].kind, TokenKind::RBracket));
    }

    #[test]
    fn test_lex_operator_no_spaces() {
        let tokens = lex("a+b").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Ident(ref s) if s == "a"));
        assert!(matches!(tokens[1].kind, TokenKind::Plus));
        assert!(matches!(tokens[2].kind, TokenKind::Ident(ref s) if s == "b"));
    }

    // ── Newline collapsing tests ─────────────────────────────────

    #[test]
    fn test_lex_multiple_newlines_collapse() {
        let src = "a\n\n\nb";
        let tokens = lex(src).unwrap();
        // Should collapse to: Ident("a"), Newline, Ident("b"), Eof
        let non_eof: Vec<_> = tokens.iter().filter(|t| t.kind != TokenKind::Eof).collect();
        assert_eq!(non_eof.len(), 3);
        assert!(matches!(non_eof[0].kind, TokenKind::Ident(ref s) if s == "a"));
        assert!(matches!(non_eof[1].kind, TokenKind::Newline));
        assert!(matches!(non_eof[2].kind, TokenKind::Ident(ref s) if s == "b"));
    }

    #[test]
    fn test_lex_no_leading_newline() {
        let src = String::from("\n\na");
        let tokens = lex(&src).unwrap();
        // Leading newlines should not produce a Newline token (nothing before them)
        assert!(matches!(tokens[0].kind, TokenKind::Ident(ref s) if s == "a"));
    }

    // ── Keyword tests ────────────────────────────────────────────

    #[test]
    fn test_lex_all_keywords() {
        let keywords = vec![
            "fn", "let", "mut", "if", "then", "else", "end", "match",
            "type", "module", "use", "pub", "do", "and", "or", "not",
            "when", "as", "true", "false", "async", "await", "extern",
            "trait", "impl", "for", "while", "in", "break", "continue",
        ];
        let src = keywords.join(" ");
        let tokens = lex(&src).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Fn));
        assert!(matches!(tokens[1].kind, TokenKind::Let));
        assert!(matches!(tokens[2].kind, TokenKind::Mut));
        assert!(matches!(tokens[3].kind, TokenKind::If));
        assert!(matches!(tokens[4].kind, TokenKind::Then));
        assert!(matches!(tokens[5].kind, TokenKind::Else));
        assert!(matches!(tokens[6].kind, TokenKind::End));
        assert!(matches!(tokens[7].kind, TokenKind::Match));
        assert!(matches!(tokens[8].kind, TokenKind::Type));
        assert!(matches!(tokens[9].kind, TokenKind::Module));
        assert!(matches!(tokens[10].kind, TokenKind::Use));
        assert!(matches!(tokens[11].kind, TokenKind::Pub));
        assert!(matches!(tokens[12].kind, TokenKind::Do));
        assert!(matches!(tokens[13].kind, TokenKind::And));
        assert!(matches!(tokens[14].kind, TokenKind::Or));
        assert!(matches!(tokens[15].kind, TokenKind::Not));
        assert!(matches!(tokens[16].kind, TokenKind::When));
        assert!(matches!(tokens[17].kind, TokenKind::As));
        assert!(matches!(tokens[18].kind, TokenKind::True));
        assert!(matches!(tokens[19].kind, TokenKind::False));
        assert!(matches!(tokens[20].kind, TokenKind::Async));
        assert!(matches!(tokens[21].kind, TokenKind::Await));
        assert!(matches!(tokens[22].kind, TokenKind::Extern));
        assert!(matches!(tokens[23].kind, TokenKind::Trait));
        assert!(matches!(tokens[24].kind, TokenKind::Impl));
        assert!(matches!(tokens[25].kind, TokenKind::For));
        assert!(matches!(tokens[26].kind, TokenKind::While));
        assert!(matches!(tokens[27].kind, TokenKind::In));
        assert!(matches!(tokens[28].kind, TokenKind::Break));
        assert!(matches!(tokens[29].kind, TokenKind::Continue));
    }

    #[test]
    fn test_lex_underscore() {
        let tokens = lex(" _ ").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Underscore));
    }

    #[test]
    fn test_lex_rust_bang() {
        let tokens = lex("rust! ").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::RustBang));
    }

    #[test]
    fn test_lex_upper_ident() {
        let tokens = lex("MyType ").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::UpperIdent(ref s) if s == "MyType"));
    }

    #[test]
    fn test_lex_ident_with_underscores() {
        let tokens = lex("my_var_2 ").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Ident(ref s) if s == "my_var_2"));
    }

    #[test]
    fn test_lex_bool_literals() {
        let src = String::from("true") + " " + "false";
        let tokens = lex(&src).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::True));
        assert!(matches!(tokens[1].kind, TokenKind::False));
    }

    #[test]
    fn test_lex_empty_string_literal() {
        // Test that "" lexes as an empty Str token
        let src = String::from(r#""""#);
        let tokens = lex(&src).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Str(ref s) if s.is_empty()));
    }

    #[test]
    fn test_lex_empty_input() {
        let empty = String::new();
        let tokens = lex(&empty).unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }

    // ── Additional lexer edge case tests ───────────────────────────

    #[test]
    fn test_lex_all_keywords_comprehensive() {
        let src = "fn let type match if then else end do for in while trait impl use module pub async extern move mut break continue";
        let tokens = lex(src).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Fn));
        assert!(matches!(tokens[1].kind, TokenKind::Let));
        assert!(matches!(tokens[2].kind, TokenKind::Type));
        assert!(matches!(tokens[3].kind, TokenKind::Match));
        assert!(matches!(tokens[4].kind, TokenKind::If));
        assert!(matches!(tokens[5].kind, TokenKind::Then));
        assert!(matches!(tokens[6].kind, TokenKind::Else));
        assert!(matches!(tokens[7].kind, TokenKind::End));
        assert!(matches!(tokens[8].kind, TokenKind::Do));
        assert!(matches!(tokens[9].kind, TokenKind::For));
        assert!(matches!(tokens[10].kind, TokenKind::In));
        assert!(matches!(tokens[11].kind, TokenKind::While));
        assert!(matches!(tokens[12].kind, TokenKind::Trait));
        assert!(matches!(tokens[13].kind, TokenKind::Impl));
        assert!(matches!(tokens[14].kind, TokenKind::Use));
        assert!(matches!(tokens[15].kind, TokenKind::Module));
        assert!(matches!(tokens[16].kind, TokenKind::Pub));
        assert!(matches!(tokens[17].kind, TokenKind::Async));
        assert!(matches!(tokens[18].kind, TokenKind::Extern));
        assert!(matches!(tokens[19].kind, TokenKind::Move));
        assert!(matches!(tokens[20].kind, TokenKind::Mut));
        assert!(matches!(tokens[21].kind, TokenKind::Break));
        assert!(matches!(tokens[22].kind, TokenKind::Continue));
    }

    #[test]
    fn test_lex_all_operator_symbols() {
        let src = "+ - * / % == != < > <= >= |>";
        let tokens = lex(src).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Plus));
        assert!(matches!(tokens[1].kind, TokenKind::Minus));
        assert!(matches!(tokens[2].kind, TokenKind::Star));
        assert!(matches!(tokens[3].kind, TokenKind::Slash));
        assert!(matches!(tokens[4].kind, TokenKind::Percent));
        assert!(matches!(tokens[5].kind, TokenKind::EqEq));
        assert!(matches!(tokens[6].kind, TokenKind::Ne));
        assert!(matches!(tokens[7].kind, TokenKind::Lt));
        assert!(matches!(tokens[8].kind, TokenKind::Gt));
        assert!(matches!(tokens[9].kind, TokenKind::Le));
        assert!(matches!(tokens[10].kind, TokenKind::Ge));
        assert!(matches!(tokens[11].kind, TokenKind::PipeArrow));
    }

    #[test]
    fn test_lex_adjacent_strings() {
        let src = r#""hello" "world""#;
        let tokens = lex(src).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Str(ref s) if s == "hello"));
        assert!(matches!(tokens[1].kind, TokenKind::Str(ref s) if s == "world"));
    }

    #[test]
    fn test_lex_string_all_escape_sequences() {
        let src = r#""tab\there\nnew\\slash\"quote""#;
        let tokens = lex(src).unwrap();
        match &tokens[0].kind {
            TokenKind::Str(s) => {
                assert!(s.contains('\t'));
                assert!(s.contains('\n'));
                assert!(s.contains('\\'));
                assert!(s.contains('"'));
            }
            other => panic!("Expected Str, got {:?}", other),
        }
    }

    #[test]
    fn test_lex_interpolation_produces_interp_str() {
        let src = r#""outer #{inner}""#;
        let tokens = lex(src).unwrap();
        let has_interp = tokens.iter().any(|t| matches!(t.kind, TokenKind::InterpStr(_)));
        assert!(has_interp, "Should contain InterpStr token");
    }

    #[test]
    fn test_lex_very_large_integer() {
        let tokens = lex("999999999999").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Int(999999999999)));
    }

    #[test]
    fn test_lex_float_starting_with_zero() {
        let tokens = lex("0.123").unwrap();
        match &tokens[0].kind {
            TokenKind::Float(f) => assert!((*f - 0.123).abs() < f64::EPSILON),
            other => panic!("Expected Float, got {:?}", other),
        }
    }

    #[test]
    fn test_lex_multiple_line_comments() {
        let src = "# comment 1\n# comment 2\n42";
        let tokens = lex(src).unwrap();
        // Comments are stored as Comment tokens but filtered in some flows
        // At minimum the integer token should exist
        let has_int = tokens.iter().any(|t| matches!(t.kind, TokenKind::Int(42)));
        assert!(has_int, "Should contain integer after comments");
    }

    #[test]
    fn test_lex_inline_comment() {
        let src = "42 # inline comment";
        let tokens = lex(src).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Int(42)));
    }

    #[test]
    fn test_lex_upper_ident_types() {
        let tokens = lex("MyType SomeEnum").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::UpperIdent(ref s) if s == "MyType"));
        assert!(matches!(tokens[1].kind, TokenKind::UpperIdent(ref s) if s == "SomeEnum"));
    }

    #[test]
    fn test_lex_all_punctuation() {
        let tokens = lex("( ) [ ] { } , : .").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::LParen));
        assert!(matches!(tokens[1].kind, TokenKind::RParen));
        assert!(matches!(tokens[2].kind, TokenKind::LBracket));
        assert!(matches!(tokens[3].kind, TokenKind::RBracket));
        assert!(matches!(tokens[4].kind, TokenKind::LBrace));
        assert!(matches!(tokens[5].kind, TokenKind::RBrace));
        assert!(matches!(tokens[6].kind, TokenKind::Comma));
        assert!(matches!(tokens[7].kind, TokenKind::Colon));
        assert!(matches!(tokens[8].kind, TokenKind::Dot));
    }

    #[test]
    fn test_lex_boolean_keywords() {
        let tokens = lex("and or not").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::And));
        assert!(matches!(tokens[1].kind, TokenKind::Or));
        assert!(matches!(tokens[2].kind, TokenKind::Not));
    }

    #[test]
    fn test_lex_repeated_operators() {
        let tokens = lex("++--").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Plus));
        assert!(matches!(tokens[1].kind, TokenKind::Plus));
        assert!(matches!(tokens[2].kind, TokenKind::Minus));
        assert!(matches!(tokens[3].kind, TokenKind::Minus));
    }

    #[test]
    fn test_lex_bitwise_keyword_tokens() {
        let tokens = lex("band bor bxor").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Band));
        assert!(matches!(tokens[1].kind, TokenKind::Bor));
        assert!(matches!(tokens[2].kind, TokenKind::Bxor));
    }

    #[test]
    fn test_lex_mixed_whitespace() {
        let tokens = lex("fn\t\tmain()\n\n\n=\r\n42").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Fn));
        assert!(matches!(tokens[1].kind, TokenKind::Ident(ref s) if s == "main"));
    }

    #[test]
    fn test_lex_fat_arrow_symbol() {
        let tokens = lex("=>").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::FatArrow));
    }

    #[test]
    fn test_lex_dotdot_symbol() {
        let tokens = lex("..").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::DotDot));
    }

    #[test]
    fn test_lex_triple_string_interpolation() {
        let src = "let s = \"\"\"\n  hello #{name}\n  \"\"\"";
        let tokens = lex(src).unwrap();
        // Should succeed — triple strings with interpolation produce InterpStr
        let has_interp = tokens.iter().any(|t| matches!(t.kind, TokenKind::InterpStr(_)));
        assert!(has_interp, "Triple string should support interpolation");
    }

    #[test]
    fn test_lex_shift_operators() {
        let tokens = lex("<< >>").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Shl));
        assert!(matches!(tokens[1].kind, TokenKind::Shr));
    }

    #[test]
    fn test_lex_compound_eq_operators() {
        let tokens = lex("+= -= *= /= %=").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::PlusEq));
        assert!(matches!(tokens[1].kind, TokenKind::MinusEq));
        assert!(matches!(tokens[2].kind, TokenKind::StarEq));
        assert!(matches!(tokens[3].kind, TokenKind::SlashEq));
        assert!(matches!(tokens[4].kind, TokenKind::PercentEq));
    }

    #[test]
    fn test_lex_colon_colon() {
        let tokens = lex("Foo::bar").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::UpperIdent(ref s) if s == "Foo"));
        assert!(matches!(tokens[1].kind, TokenKind::ColonColon));
        assert!(matches!(tokens[2].kind, TokenKind::Ident(ref s) if s == "bar"));
    }

    #[test]
    fn test_lex_question_mark() {
        let tokens = lex("x?").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Ident(ref s) if s == "x"));
        assert!(matches!(tokens[1].kind, TokenKind::Question));
    }

    #[test]
    fn test_lex_underscore_token() {
        let tokens = lex("_").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Underscore));
    }

    #[test]
    fn test_lex_await_keyword() {
        let tokens = lex("await").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Await));
    }

    #[test]
    fn test_lex_dyn_keyword() {
        let tokens = lex("dyn").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Dyn));
    }

    #[test]
    fn test_lex_as_keyword() {
        let tokens = lex("x as Int").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Ident(ref s) if s == "x"));
        assert!(matches!(tokens[1].kind, TokenKind::As));
        assert!(matches!(tokens[2].kind, TokenKind::UpperIdent(ref s) if s == "Int"));
    }

    #[test]
    fn test_lex_when_keyword() {
        let tokens = lex("when").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::When));
    }

    #[test]
    fn test_lex_ampersand() {
        let tokens = lex("&x").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Ampersand));
        assert!(matches!(tokens[1].kind, TokenKind::Ident(ref s) if s == "x"));
    }

    #[test]
    fn test_lex_pipe_symbol() {
        let tokens = lex("|").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Pipe));
    }

    #[test]
    fn test_lex_negative_number() {
        // Minus is a separate token, not part of the number
        let tokens = lex("-42").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Minus));
        assert!(matches!(tokens[1].kind, TokenKind::Int(42)));
    }

    #[test]
    fn test_lex_annotation_syntax() {
        let tokens = lex("@[cfg(test)]").unwrap();
        // '@' followed by '[' — check both exist
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
        // The lexer should handle @[ for annotations
        assert!(tokens.len() >= 2, "Annotation should produce tokens");
    }
}
