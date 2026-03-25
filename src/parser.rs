use crate::ast::*;
use crate::error::Span;
use crate::lexer::{Token, TokenKind};

pub fn parse(tokens: Vec<Token>) -> Result<(Program, Vec<(usize, String)>), String> {
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;
    Ok((program, parser.comments))
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    errors: Vec<String>,
    comments: Vec<(usize, String)>,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0, errors: Vec::new(), comments: Vec::new() }
    }

    fn span(&self) -> Span {
        // Find next non-comment token for span
        let mut i = self.pos;
        while i < self.tokens.len() {
            if let TokenKind::Comment(_) = &self.tokens[i].kind {
                i += 1;
            } else {
                break;
            }
        }
        self.tokens
            .get(i)
            .map(|t| t.span)
            .unwrap_or(Span::new(0, 0))
    }

    fn peek(&self) -> &TokenKind {
        // Find next non-comment token
        let mut i = self.pos;
        while i < self.tokens.len() {
            if let TokenKind::Comment(_) = &self.tokens[i].kind {
                i += 1;
            } else {
                break;
            }
        }
        self.tokens
            .get(i)
            .map(|t| &t.kind)
            .unwrap_or(&TokenKind::Eof)
    }

    fn at(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(kind)
    }

    fn advance(&mut self) -> &Token {
        self.skip_comments();
        let idx = self.pos;
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        // Note: we don't skip_comments after advance here because
        // peek/span already skip comments transparently
        &self.tokens[idx]
    }

    fn skip_comments(&mut self) {
        while self.pos < self.tokens.len() {
            if let TokenKind::Comment(text) = &self.tokens[self.pos].kind {
                let line = self.tokens[self.pos].span.line;
                self.comments.push((line, text.clone()));
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, expected: &TokenKind) -> Result<&Token, String> {
        if self.at(expected) {
            Ok(self.advance())
        } else {
            Err(format!(
                "{} Expected {:?}, got {:?}",
                self.span(),
                expected,
                self.peek()
            ))
        }
    }

    /// Expect a closing `>` for generic types. Handles the `>>` case where the
    /// lexer produces a single `Shr` token for nested generics like `List<List<Int>>`.
    fn expect_gt(&mut self) -> Result<(), String> {
        if self.at(&TokenKind::Gt) {
            self.advance();
            Ok(())
        } else if self.at(&TokenKind::Shr) {
            // Split `>>` into `>` (consumed) + `>` (left for the outer generic)
            self.split_shr_to_gt();
            Ok(())
        } else {
            Err(format!(
                "{} Expected Gt, got {:?}",
                self.span(),
                self.peek()
            ))
        }
    }

    /// Replace the current `Shr` (`>>`) token with `Gt` (`>`), effectively
    /// consuming one `>` and leaving the other for the next parse step.
    fn split_shr_to_gt(&mut self) {
        self.skip_comments();
        let span = self.tokens[self.pos].span;
        self.tokens[self.pos] = Token {
            kind: TokenKind::Gt,
            span: Span::new(span.line, span.col + 1),
        };
    }

    fn skip_newlines(&mut self) {
        while self.at(&TokenKind::Newline) {
            self.advance();
        }
    }

    /// Look past newlines to see if a PipeArrow follows
    fn peek_is_pipe_arrow(&self) -> bool {
        let mut i = self.pos;
        while i < self.tokens.len() {
            match &self.tokens[i].kind {
                TokenKind::Newline => i += 1,
                TokenKind::PipeArrow => return true,
                _ => return false,
            }
        }
        false
    }

    fn expect_ident(&mut self) -> Result<String, String> {
        match self.peek().clone() {
            TokenKind::Ident(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            _ => Err(format!("{} Expected identifier, got {:?}", self.span(), self.peek())),
        }
    }

    fn expect_upper_ident(&mut self) -> Result<String, String> {
        match self.peek().clone() {
            TokenKind::UpperIdent(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            _ => Err(format!(
                "{} Expected type name, got {:?}",
                self.span(),
                self.peek()
            )),
        }
    }

    // ── Program ─────────────────────────────────────────────────────────────

    fn parse_program(&mut self) -> Result<Program, String> {
        let mut items = Vec::new();
        self.skip_newlines();

        while !self.at(&TokenKind::Eof) {
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(e) => {
                    self.errors.push(e);
                    // Recovery: skip tokens until we find a new item start
                    self.recover_to_next_item();
                }
            }
            self.skip_newlines();
        }

        if !self.errors.is_empty() {
            return Err(self.errors.join("\n"));
        }

        Ok(Program { items })
    }

    /// Skip tokens until we reach something that looks like the start of a new item
    fn recover_to_next_item(&mut self) {
        loop {
            match self.peek() {
                TokenKind::Fn | TokenKind::Pub | TokenKind::Type
                | TokenKind::Module | TokenKind::Use | TokenKind::Async
                | TokenKind::Extern | TokenKind::Trait | TokenKind::Impl
                | TokenKind::Let | TokenKind::At
                | TokenKind::Eof => break,
                TokenKind::Newline => {
                    self.advance();
                    match self.peek() {
                        TokenKind::Fn | TokenKind::Pub | TokenKind::Type
                        | TokenKind::Module | TokenKind::Use | TokenKind::Async
                        | TokenKind::Extern | TokenKind::Trait | TokenKind::Impl
                        | TokenKind::Let | TokenKind::At
                        | TokenKind::Eof => break,
                        _ => {}
                    }
                }
                _ => { self.advance(); }
            }
        }
    }

    fn parse_item(&mut self) -> Result<Item, String> {
        // Parse annotations (@[...]) before items
        let annotations = self.parse_annotations()?;

        let mut item = match self.peek() {
            TokenKind::Fn => Ok(Item::Function(self.parse_function(false, false)?)),
            TokenKind::Async => {
                self.advance();
                match self.peek() {
                    TokenKind::Fn => Ok(Item::Function(self.parse_function(false, true)?)),
                    _ => Err(format!("{} Expected 'fn' after 'async'", self.span())),
                }
            }
            TokenKind::Pub => {
                self.advance();
                match self.peek() {
                    TokenKind::Fn => Ok(Item::Function(self.parse_function(true, false)?)),
                    TokenKind::Async => {
                        self.advance();
                        match self.peek() {
                            TokenKind::Fn => Ok(Item::Function(self.parse_function(true, true)?)),
                            _ => Err(format!("{} Expected 'fn' after 'pub async'", self.span())),
                        }
                    }
                    TokenKind::Let => Ok(Item::Const(self.parse_const_decl(true)?)),
                    _ => Err(format!("{} Expected 'fn', 'async', or 'let' after 'pub'", self.span())),
                }
            }
            TokenKind::Let => Ok(Item::Const(self.parse_const_decl(false)?)),
            TokenKind::Type => Ok(Item::TypeDecl(self.parse_type_decl()?)),
            TokenKind::Module => Ok(Item::ModuleDecl(self.parse_module_decl()?)),
            TokenKind::Use => Ok(Item::UseDecl(self.parse_use_decl()?)),
            TokenKind::Extern => Ok(Item::ExternFn(self.parse_extern_fn()?)),
            TokenKind::Trait => Ok(Item::TraitDecl(self.parse_trait_decl()?)),
            TokenKind::Impl => Ok(Item::ImplBlock(self.parse_impl_block()?)),
            _ => {
                let expr = self.parse_expr()?;
                Ok(Item::Expr(expr))
            }
        }?;

        // Attach annotations to functions
        if !annotations.is_empty() {
            if let Item::Function(ref mut f) = item {
                f.annotations = annotations;
            }
        }

        Ok(item)
    }

    // ── Functions ───────────────────────────────────────────────────────────

    fn parse_function(&mut self, is_pub: bool, is_async: bool) -> Result<Function, String> {
        let span = self.span();
        self.expect(&TokenKind::Fn)?;
        let name = self.expect_ident()?;

        let type_params = self.parse_optional_type_params()?;

        self.expect(&TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;

        let return_type = if self.at(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        let where_clauses = self.parse_optional_where_clauses()?;

        self.expect(&TokenKind::Eq)?;
        self.skip_newlines();
        let body = self.parse_function_body()?;

        Ok(Function {
            name,
            params,
            return_type,
            body,
            is_pub,
            is_async,
            type_params,
            where_clauses,
            annotations: vec![],
            span,
        })
    }

    fn parse_function_body(&mut self) -> Result<Expr, String> {
        let span = self.span();
        let first = self.parse_statement()?;

        // Check if there are more statements on subsequent lines
        if !self.at(&TokenKind::Newline) {
            return match first {
                Stmt::Expr(e) => Ok(e),
                other => Ok(Expr {
                    kind: ExprKind::Block(vec![other], Box::new(Expr {
                        kind: ExprKind::IntLit(0),
                        span,
                    })),
                    span,
                }),
            };
        }

        self.skip_newlines();

        // Check if the next token starts a new top-level item
        if self.at(&TokenKind::Fn) || self.at(&TokenKind::Pub) || self.at(&TokenKind::Type)
            || self.at(&TokenKind::Module) || self.at(&TokenKind::Use) || self.at(&TokenKind::Eof)
            || self.at(&TokenKind::Extern) || self.at(&TokenKind::Trait) || self.at(&TokenKind::Impl)
            || self.at(&TokenKind::End) || self.at(&TokenKind::At)
        {
            return match first {
                Stmt::Expr(e) => Ok(e),
                other => Ok(Expr {
                    kind: ExprKind::Block(vec![other], Box::new(Expr {
                        kind: ExprKind::IntLit(0),
                        span,
                    })),
                    span,
                }),
            };
        }

        // Multiple statements — collect into a block
        let mut stmts = vec![first];

        loop {
            if self.at(&TokenKind::Fn) || self.at(&TokenKind::Pub) || self.at(&TokenKind::Type)
                || self.at(&TokenKind::Module) || self.at(&TokenKind::Use) || self.at(&TokenKind::Eof)
                || self.at(&TokenKind::Extern) || self.at(&TokenKind::Trait) || self.at(&TokenKind::Impl)
                || self.at(&TokenKind::End)
            {
                break;
            }
            stmts.push(self.parse_statement()?);
            if !self.at(&TokenKind::Newline) {
                break;
            }
            self.skip_newlines();
        }

        // Last statement becomes the return expression
        let last = stmts.pop().unwrap();
        let final_expr = match last {
            Stmt::Expr(e) => e,
            other => {
                stmts.push(other);
                Expr { kind: ExprKind::IntLit(0), span }
            }
        };

        Ok(Expr {
            kind: ExprKind::Block(stmts, Box::new(final_expr)),
            span,
        })
    }

    // ── Constants ──────────────────────────────────────────────────────────

    fn parse_const_decl(&mut self, is_pub: bool) -> Result<ConstDecl, String> {
        let span = self.span();
        self.expect(&TokenKind::Let)?;
        // Accept both lowercase and UPPERCASE constant names
        let name = match self.peek().clone() {
            TokenKind::Ident(s) | TokenKind::UpperIdent(s) => {
                let s = s.clone();
                self.advance();
                s
            }
            _ => return Err(format!("{} Expected constant name, got {:?}", self.span(), self.peek())),
        };
        let ty = if self.at(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.expect(&TokenKind::Eq)?;
        self.skip_newlines();
        let value = self.parse_expr()?;
        Ok(ConstDecl { name, ty, value, is_pub, span })
    }

    // ── Annotations ────────────────────────────────────────────────────────

    fn parse_annotations(&mut self) -> Result<Vec<String>, String> {
        let mut annotations = Vec::new();
        while self.at(&TokenKind::At) {
            self.advance();
            self.expect(&TokenKind::LBracket)?;
            // Collect everything until ]
            let mut content = String::new();
            let mut depth = 1;
            loop {
                match self.peek() {
                    TokenKind::LBracket => { depth += 1; content.push('['); self.advance(); }
                    TokenKind::RBracket => {
                        depth -= 1;
                        if depth == 0 { self.advance(); break; }
                        content.push(']');
                        self.advance();
                    }
                    TokenKind::Eof => return Err(format!("{} Unterminated annotation", self.span())),
                    _ => {
                        let tok = self.advance();
                        content.push_str(&token_to_string(&tok.kind));
                    }
                }
            }
            annotations.push(content.trim().to_string());
            self.skip_newlines();
        }
        Ok(annotations)
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>, String> {
        let mut params = Vec::new();
        if self.at(&TokenKind::RParen) {
            return Ok(params);
        }

        params.push(self.parse_param()?);
        while self.at(&TokenKind::Comma) {
            self.advance();
            params.push(self.parse_param()?);
        }

        Ok(params)
    }

    fn parse_param(&mut self) -> Result<Param, String> {
        let span = self.span();

        // Check for tuple destructuring: (a, b): (Int, Int)
        if self.at(&TokenKind::LParen) {
            let pattern = self.parse_pattern()?;
            let ty = if self.at(&TokenKind::Colon) {
                self.advance();
                Some(self.parse_type_expr()?)
            } else {
                None
            };
            // Extract a name for internal use from the pattern
            let name = format!("_destruct");
            return Ok(Param { name, ty, destructure: Some(pattern), span });
        }

        let name = self.expect_ident()?;
        let ty = if self.at(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        Ok(Param { name, ty, destructure: None, span })
    }

    fn parse_optional_type_params(&mut self) -> Result<Vec<TypeParam>, String> {
        if !self.at(&TokenKind::Lt) {
            return Ok(Vec::new());
        }
        self.advance();
        let mut params = vec![self.parse_type_param()?];
        while self.at(&TokenKind::Comma) {
            self.advance();
            params.push(self.parse_type_param()?);
        }
        self.expect_gt()?;
        Ok(params)
    }

    fn parse_type_param(&mut self) -> Result<TypeParam, String> {
        // Support lifetime parameters: 'a, 'b, etc.
        if let TokenKind::Tick(lifetime) = self.peek().clone() {
            self.advance();
            return Ok(TypeParam { name: format!("'{lifetime}"), bounds: vec![] });
        }
        let name = self.expect_upper_ident()?;
        let bounds = if self.at(&TokenKind::Colon) {
            self.advance();
            let mut bounds = vec![self.expect_upper_ident()?];
            while self.at(&TokenKind::Plus) {
                self.advance();
                bounds.push(self.expect_upper_ident()?);
            }
            bounds
        } else {
            vec![]
        };
        Ok(TypeParam { name, bounds })
    }

    fn parse_optional_where_clauses(&mut self) -> Result<Vec<WhereClause>, String> {
        // Check for `where` keyword (parsed as an Ident, not a keyword token)
        if let TokenKind::Ident(ref s) = self.peek().clone() {
            if s == "where" {
                self.advance();
                let mut clauses = Vec::new();
                loop {
                    // Parse TypeName: Bound + Bound
                    let type_name = self.expect_upper_ident()?;
                    self.expect(&TokenKind::Colon)?;
                    let mut bounds = vec![self.expect_upper_ident()?];
                    while self.at(&TokenKind::Plus) {
                        self.advance();
                        bounds.push(self.expect_upper_ident()?);
                    }
                    clauses.push(WhereClause { type_name, bounds });
                    // Continue if comma-separated
                    if self.at(&TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                return Ok(clauses);
            }
        }
        Ok(vec![])
    }

    // ── Types ───────────────────────────────────────────────────────────────

    fn parse_type_decl(&mut self) -> Result<TypeDecl, String> {
        let span = self.span();
        self.expect(&TokenKind::Type)?;
        let name = self.expect_upper_ident()?;
        let type_params = self.parse_optional_type_params()?;
        self.expect(&TokenKind::Eq)?;
        self.skip_newlines();

        let body = if self.at(&TokenKind::Pipe) || self.at(&TokenKind::Newline) {
            self.skip_newlines();
            // Enum variants
            let mut variants = Vec::new();
            while self.at(&TokenKind::Pipe) {
                self.advance();
                let vspan = self.span();
                let vname = self.expect_upper_ident()?;
                let (fields, named_fields) = if self.at(&TokenKind::LParen) {
                    self.advance();
                    let types = self.parse_type_list()?;
                    self.expect(&TokenKind::RParen)?;
                    (types, None)
                } else if self.at(&TokenKind::LBrace) {
                    // Struct-like variant: | Variant { name: Type, name2: Type2 }
                    self.advance();
                    self.skip_newlines();
                    let mut nfields = Vec::new();
                    while !self.at(&TokenKind::RBrace) {
                        let fspan = self.span();
                        let fname = self.expect_ident()?;
                        self.expect(&TokenKind::Colon)?;
                        let ftype = self.parse_type_expr()?;
                        nfields.push(Field {
                            name: fname,
                            ty: ftype,
                            is_pub: false,
                            span: fspan,
                        });
                        if self.at(&TokenKind::Comma) {
                            self.advance();
                        }
                        self.skip_newlines();
                    }
                    self.expect(&TokenKind::RBrace)?;
                    (Vec::new(), Some(nfields))
                } else {
                    (Vec::new(), None)
                };
                variants.push(Variant {
                    name: vname,
                    fields,
                    named_fields,
                    span: vspan,
                });
                self.skip_newlines();
            }
            TypeBody::Enum(variants)
        } else if self.at(&TokenKind::LBrace) {
            // Struct
            self.advance();
            self.skip_newlines();
            let mut fields = Vec::new();
            while !self.at(&TokenKind::RBrace) {
                let fspan = self.span();
                let is_pub = if self.at(&TokenKind::Pub) {
                    self.advance();
                    true
                } else {
                    false
                };
                let fname = self.expect_ident()?;
                self.expect(&TokenKind::Colon)?;
                let ftype = self.parse_type_expr()?;
                fields.push(Field {
                    name: fname,
                    ty: ftype,
                    is_pub,
                    span: fspan,
                });
                if self.at(&TokenKind::Comma) {
                    self.advance();
                }
                self.skip_newlines();
            }
            self.expect(&TokenKind::RBrace)?;
            TypeBody::Struct(fields)
        } else {
            // Type alias: type Name = ExistingType
            let ty = self.parse_type_expr()?;
            TypeBody::Alias(ty)
        };

        Ok(TypeDecl {
            name,
            type_params,
            body,
            span,
        })
    }

    fn parse_type_expr(&mut self) -> Result<TypeExpr, String> {
        // Lifetime: 'a
        if let TokenKind::Tick(name) = self.peek().clone() {
            self.advance();
            return Ok(TypeExpr::Lifetime(name));
        }

        if self.at(&TokenKind::Fn) {
            // fn(A, B) -> C
            self.advance();
            self.expect(&TokenKind::LParen)?;
            let params = self.parse_type_list()?;
            self.expect(&TokenKind::RParen)?;
            self.expect(&TokenKind::Arrow)?;
            let ret = self.parse_type_expr()?;
            return Ok(TypeExpr::Function(params, Box::new(ret)));
        }

        if self.at(&TokenKind::Ampersand) {
            self.advance();
            if self.at(&TokenKind::Mut) {
                self.advance();
                let inner = self.parse_type_expr()?;
                return Ok(TypeExpr::MutRef(Box::new(inner)));
            }
            let inner = self.parse_type_expr()?;
            return Ok(TypeExpr::Ref(Box::new(inner)));
        }

        if self.at(&TokenKind::Tilde) {
            self.advance();
            let inner = self.parse_type_expr()?;
            return Ok(TypeExpr::Move(Box::new(inner)));
        }

        if self.at(&TokenKind::Dyn) {
            self.advance();
            let trait_name = self.expect_upper_ident()?;
            return Ok(TypeExpr::Dyn(trait_name));
        }

        if self.at(&TokenKind::LParen) {
            self.advance();
            let types = self.parse_type_list()?;
            self.expect(&TokenKind::RParen)?;
            return Ok(TypeExpr::Tuple(types));
        }

        // Named type with optional type args
        let name = match self.peek().clone() {
            TokenKind::UpperIdent(s) => {
                let s = s.clone();
                self.advance();
                s
            }
            TokenKind::Ident(s) => {
                // Allow lowercase built-in type names
                let s = s.clone();
                self.advance();
                s
            }
            _ => {
                return Err(format!(
                    "{} Expected type expression, got {:?}",
                    self.span(),
                    self.peek()
                ))
            }
        };

        let args = if self.at(&TokenKind::Lt) {
            self.advance();
            let args = self.parse_type_list()?;
            self.expect_gt()?;
            args
        } else {
            Vec::new()
        };

        Ok(TypeExpr::Named(name, args))
    }

    fn parse_type_list(&mut self) -> Result<Vec<TypeExpr>, String> {
        let mut types = Vec::new();
        if self.at(&TokenKind::RParen) || self.at(&TokenKind::Gt) || self.at(&TokenKind::Shr) {
            return Ok(types);
        }

        types.push(self.parse_type_expr()?);
        while self.at(&TokenKind::Comma) {
            self.advance();
            types.push(self.parse_type_expr()?);
        }
        Ok(types)
    }

    // ── Modules ─────────────────────────────────────────────────────────────

    fn parse_module_decl(&mut self) -> Result<ModuleDecl, String> {
        let span = self.span();
        self.expect(&TokenKind::Module)?;
        let name = self.expect_upper_ident()?;
        self.skip_newlines();

        let mut items = Vec::new();
        while !self.at(&TokenKind::End) && !self.at(&TokenKind::Eof) {
            items.push(self.parse_item()?);
            self.skip_newlines();
        }
        self.expect(&TokenKind::End)?;

        Ok(ModuleDecl { name, items, span })
    }

    fn parse_use_decl(&mut self) -> Result<UseDecl, String> {
        let span = self.span();
        self.expect(&TokenKind::Use)?;

        let mut path = Vec::new();
        // First segment can be upper or lower ident
        let first = match self.peek().clone() {
            TokenKind::UpperIdent(s) | TokenKind::Ident(s) => {
                let s = s.clone();
                self.advance();
                s
            }
            _ => return Err(format!("{} Expected module path", self.span())),
        };
        path.push(first);

        while self.at(&TokenKind::ColonColon) {
            self.advance();
            if self.at(&TokenKind::LBrace) {
                // use Foo::{a, b}
                self.advance();
                let mut imports = Vec::new();
                loop {
                    let name = match self.peek().clone() {
                        TokenKind::Ident(s) | TokenKind::UpperIdent(s) => {
                            let s = s.clone();
                            self.advance();
                            s
                        }
                        _ => break,
                    };
                    imports.push(name);
                    if !self.at(&TokenKind::Comma) {
                        break;
                    }
                    self.advance();
                }
                self.expect(&TokenKind::RBrace)?;
                return Ok(UseDecl {
                    path,
                    imports: Some(imports),
                    alias: None,
                    span,
                });
            }
            let segment = match self.peek().clone() {
                TokenKind::UpperIdent(s) | TokenKind::Ident(s) => {
                    let s = s.clone();
                    self.advance();
                    s
                }
                _ => return Err(format!("{} Expected module path segment", self.span())),
            };
            path.push(segment);
        }

        // Check for alias: `use Foo as Bar`
        let alias = if self.at(&TokenKind::As) {
            self.advance();
            let alias_name = match self.peek().clone() {
                TokenKind::UpperIdent(s) | TokenKind::Ident(s) => {
                    let s = s.clone();
                    self.advance();
                    s
                }
                _ => return Err(format!("{} Expected alias name after 'as'", self.span())),
            };
            Some(alias_name)
        } else {
            None
        };

        Ok(UseDecl {
            path,
            imports: None,
            alias,
            span,
        })
    }

    // ── Expressions ─────────────────────────────────────────────────────────

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_pipe_expr()
    }

    fn parse_pipe_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_or_expr()?;

        loop {
            if self.at(&TokenKind::PipeArrow) {
                let span = self.span();
                self.advance();
                self.skip_newlines();
                let right = self.parse_or_expr()?;
                left = Expr {
                    kind: ExprKind::Pipe(Box::new(left), Box::new(right)),
                    span,
                };
            } else if self.at(&TokenKind::Newline) && self.peek_is_pipe_arrow() {
                self.skip_newlines();
                // Now we should be at PipeArrow
                continue;
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_or_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and_expr()?;

        while self.at(&TokenKind::Or) {
            let span = self.span();
            self.advance();
            let right = self.parse_and_expr()?;
            left = Expr {
                kind: ExprKind::BinOp(Box::new(left), BinOp::Or, Box::new(right)),
                span,
            };
        }

        Ok(left)
    }

    fn parse_and_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;

        while self.at(&TokenKind::And) {
            let span = self.span();
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr {
                kind: ExprKind::BinOp(Box::new(left), BinOp::And, Box::new(right)),
                span,
            };
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise()?;

        loop {
            let op = match self.peek() {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::Ne => BinOp::Ne,
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::Le => BinOp::Le,
                TokenKind::Ge => BinOp::Ge,
                _ => break,
            };
            let span = self.span();
            self.advance();
            let right = self.parse_bitwise()?;
            left = Expr {
                kind: ExprKind::BinOp(Box::new(left), op, Box::new(right)),
                span,
            };
        }

        Ok(left)
    }

    fn parse_bitwise(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_addition()?;

        loop {
            let op = match self.peek() {
                TokenKind::Band => BinOp::Band,
                TokenKind::Bor => BinOp::Bor,
                TokenKind::Bxor => BinOp::Bxor,
                TokenKind::Shl => BinOp::Shl,
                TokenKind::Shr => BinOp::Shr,
                _ => break,
            };
            let span = self.span();
            self.advance();
            let right = self.parse_addition()?;
            left = Expr {
                kind: ExprKind::BinOp(Box::new(left), op, Box::new(right)),
                span,
            };
        }

        Ok(left)
    }

    fn parse_addition(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplication()?;

        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            let span = self.span();
            self.advance();
            let right = self.parse_multiplication()?;
            left = Expr {
                kind: ExprKind::BinOp(Box::new(left), op, Box::new(right)),
                span,
            };
        }

        Ok(left)
    }

    fn parse_multiplication(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match self.peek() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            let span = self.span();
            self.advance();
            let right = self.parse_unary()?;
            left = Expr {
                kind: ExprKind::BinOp(Box::new(left), op, Box::new(right)),
                span,
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            TokenKind::Not => {
                let span = self.span();
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr {
                    kind: ExprKind::UnaryOp(UnaryOp::Not, Box::new(expr)),
                    span,
                })
            }
            TokenKind::Minus => {
                let span = self.span();
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr {
                    kind: ExprKind::UnaryOp(UnaryOp::Neg, Box::new(expr)),
                    span,
                })
            }
            TokenKind::Star => {
                let span = self.span();
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr {
                    kind: ExprKind::UnaryOp(UnaryOp::Deref, Box::new(expr)),
                    span,
                })
            }
            TokenKind::Ampersand => {
                let span = self.span();
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr {
                    kind: ExprKind::UnaryOp(UnaryOp::Ref, Box::new(expr)),
                    span,
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek() {
                TokenKind::LParen => {
                    let span = self.span();
                    self.advance();
                    let args = self.parse_arg_list()?;
                    self.expect(&TokenKind::RParen)?;
                    expr = Expr {
                        kind: ExprKind::Call(Box::new(expr), args),
                        span,
                    };
                }
                TokenKind::Dot => {
                    let span = self.span();
                    self.advance();
                    // Check for .await
                    if self.at(&TokenKind::Await) {
                        self.advance();
                        expr = Expr {
                            kind: ExprKind::Await(Box::new(expr)),
                            span,
                        };
                    } else {
                        let field = self.expect_ident()?;
                        if self.at(&TokenKind::LParen) {
                            // Method call
                            self.advance();
                            let args = self.parse_arg_list()?;
                            self.expect(&TokenKind::RParen)?;
                            expr = Expr {
                                kind: ExprKind::MethodCall(Box::new(expr), field, args),
                                span,
                            };
                        } else {
                            expr = Expr {
                                kind: ExprKind::FieldAccess(Box::new(expr), field),
                                span,
                            };
                        }
                    }
                }
                TokenKind::Question => {
                    let span = self.span();
                    self.advance();
                    expr = Expr {
                        kind: ExprKind::Try(Box::new(expr)),
                        span,
                    };
                }
                TokenKind::LBracket => {
                    let span = self.span();
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(&TokenKind::RBracket)?;
                    // Represent indexing as a method call on __index
                    expr = Expr {
                        kind: ExprKind::MethodCall(Box::new(expr), "__index".to_string(), vec![index]),
                        span,
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let span = self.span();

        match self.peek().clone() {
            TokenKind::Int(n) => {
                let n = n;
                self.advance();
                Ok(Expr {
                    kind: ExprKind::IntLit(n),
                    span,
                })
            }
            TokenKind::Float(f) => {
                let f = f;
                self.advance();
                Ok(Expr {
                    kind: ExprKind::FloatLit(f),
                    span,
                })
            }
            TokenKind::Str(ref s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr {
                    kind: ExprKind::StringLit(s),
                    span,
                })
            }
            TokenKind::InterpStr(ref parts) => {
                let parts = parts.clone();
                self.advance();
                self.parse_interp_string(parts, span)
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::BoolLit(true),
                    span,
                })
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::BoolLit(false),
                    span,
                })
            }
            TokenKind::CharLit(c) => {
                let c = c;
                self.advance();
                Ok(Expr {
                    kind: ExprKind::CharLit(c),
                    span,
                })
            }
            TokenKind::Ident(ref s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Ident(s),
                    span,
                })
            }
            TokenKind::UpperIdent(ref s) => {
                let s = s.clone();
                self.advance();

                // Check for struct literal: Name { field: value }
                if self.at(&TokenKind::LBrace) {
                    return self.parse_struct_lit(s, span);
                }

                Ok(Expr {
                    kind: ExprKind::Ident(s),
                    span,
                })
            }
            TokenKind::LParen => {
                self.advance();
                if self.at(&TokenKind::RParen) {
                    self.advance();
                    return Ok(Expr {
                        kind: ExprKind::IntLit(0), // unit
                        span,
                    });
                }
                let first = self.parse_expr()?;
                if self.at(&TokenKind::Comma) {
                    // Tuple expression
                    let mut elems = vec![first];
                    while self.at(&TokenKind::Comma) {
                        self.advance();
                        if self.at(&TokenKind::RParen) {
                            break;
                        }
                        elems.push(self.parse_expr()?);
                    }
                    self.expect(&TokenKind::RParen)?;
                    Ok(Expr {
                        kind: ExprKind::Tuple(elems),
                        span,
                    })
                } else {
                    self.expect(&TokenKind::RParen)?;
                    Ok(first)
                }
            }
            TokenKind::LBracket => {
                self.advance();
                let mut elems = Vec::new();
                if !self.at(&TokenKind::RBracket) {
                    elems.push(self.parse_expr()?);
                    while self.at(&TokenKind::Comma) {
                        self.advance();
                        if self.at(&TokenKind::RBracket) {
                            break;
                        }
                        elems.push(self.parse_expr()?);
                    }
                }
                self.expect(&TokenKind::RBracket)?;
                Ok(Expr {
                    kind: ExprKind::ListLit(elems),
                    span,
                })
            }
            TokenKind::If => self.parse_if_expr(),
            TokenKind::Match => self.parse_match_expr(),
            TokenKind::Fn => self.parse_lambda(),
            TokenKind::Move => {
                self.advance();
                self.parse_lambda_inner(true)
            }
            TokenKind::Let => self.parse_let_expr(),
            TokenKind::Do => self.parse_do_block(),
            TokenKind::For => self.parse_for_loop(),
            TokenKind::While => self.parse_while_loop(),
            TokenKind::Loop => self.parse_loop_expr(),
            TokenKind::Break => {
                self.advance();
                // Check for break-with-value: break expr
                // Only parse an expression if the next token could start one
                // (not newline, end, pipe, etc.)
                match self.peek() {
                    TokenKind::Newline | TokenKind::End | TokenKind::Pipe
                    | TokenKind::Eof | TokenKind::Else => {
                        Ok(Expr {
                            kind: ExprKind::Break,
                            span,
                        })
                    }
                    _ => {
                        let value = self.parse_expr()?;
                        Ok(Expr {
                            kind: ExprKind::BreakValue(Box::new(value)),
                            span,
                        })
                    }
                }
            }
            TokenKind::Continue => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Continue,
                    span,
                })
            }
            TokenKind::RustBang => self.parse_rust_block(),
            TokenKind::Await => {
                self.advance();
                let inner = self.parse_postfix()?;
                Ok(Expr {
                    kind: ExprKind::Await(Box::new(inner)),
                    span,
                })
            }
            _ => Err(format!(
                "{} Unexpected token {:?}",
                self.span(),
                self.peek()
            )),
        }
    }

    fn parse_struct_lit(&mut self, name: String, span: Span) -> Result<Expr, String> {
        self.expect(&TokenKind::LBrace)?;
        self.skip_newlines();
        let mut fields = Vec::new();
        let mut spread = None;

        while !self.at(&TokenKind::RBrace) {
            if self.at(&TokenKind::DotDot) {
                self.advance();
                spread = Some(Box::new(self.parse_expr()?));
                self.skip_newlines();
                break;
            }

            let fname = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let fval = self.parse_expr()?;
            fields.push((fname, fval));

            if self.at(&TokenKind::Comma) {
                self.advance();
            }
            self.skip_newlines();
        }
        self.expect(&TokenKind::RBrace)?;

        Ok(Expr {
            kind: ExprKind::StructLit(name, fields, spread),
            span,
        })
    }

    fn parse_if_expr(&mut self) -> Result<Expr, String> {
        let span = self.span();
        self.expect(&TokenKind::If)?;
        let cond = self.parse_expr()?;
        self.expect(&TokenKind::Then)?;
        self.skip_newlines();
        let then_branch = self.parse_expr()?;
        self.skip_newlines();
        let else_branch = if self.at(&TokenKind::Else) {
            self.advance();
            self.skip_newlines();
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };
        self.skip_newlines();
        self.expect(&TokenKind::End)?;

        Ok(Expr {
            kind: ExprKind::If(Box::new(cond), Box::new(then_branch), else_branch),
            span,
        })
    }

    fn parse_match_expr(&mut self) -> Result<Expr, String> {
        let span = self.span();
        self.expect(&TokenKind::Match)?;
        let scrutinee = self.parse_expr()?;
        self.skip_newlines();

        let mut arms = Vec::new();
        while self.at(&TokenKind::Pipe) {
            arms.push(self.parse_match_arm()?);
            self.skip_newlines();
        }
        self.expect(&TokenKind::End)?;

        Ok(Expr {
            kind: ExprKind::Match(Box::new(scrutinee), arms),
            span,
        })
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm, String> {
        let span = self.span();
        self.expect(&TokenKind::Pipe)?;
        let first_pattern = self.parse_pattern()?;

        // Check for or-patterns: | Pat1 | Pat2 | Pat3 => ...
        // If next token is `|` and the token after is NOT `=>`, it's an or-pattern
        let pattern = if self.at(&TokenKind::Pipe) {
            let mut pats = vec![first_pattern];
            while self.at(&TokenKind::Pipe) {
                // Peek ahead: if the token after `|` is another pattern (not =>), consume
                let next_pos = self.pos + 1;
                let next_kind = self.tokens.get(next_pos).map(|t| &t.kind).unwrap_or(&TokenKind::Eof);
                if matches!(next_kind, TokenKind::FatArrow) {
                    break;
                }
                self.advance(); // consume `|`
                pats.push(self.parse_pattern()?);
            }
            if pats.len() == 1 {
                pats.into_iter().next().unwrap()
            } else {
                Pattern::Or(pats)
            }
        } else {
            first_pattern
        };

        let guard = if self.at(&TokenKind::When) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.expect(&TokenKind::FatArrow)?;
        self.skip_newlines();
        let body = self.parse_expr()?;

        Ok(MatchArm {
            pattern,
            guard,
            body,
            span,
        })
    }

    fn parse_pattern(&mut self) -> Result<Pattern, String> {
        let pat = match self.peek().clone() {
            TokenKind::Underscore => {
                self.advance();
                Pattern::Wildcard
            }
            TokenKind::Int(n) => {
                self.advance();
                if self.at(&TokenKind::DotDot) {
                    self.advance();
                    if let TokenKind::Int(end) = self.peek().clone() {
                        self.advance();
                        Pattern::Range(n, end)
                    } else {
                        return Err(format!(
                            "{} Expected integer after '..' in range pattern",
                            self.span()
                        ));
                    }
                } else {
                    Pattern::IntLit(n)
                }
            }
            TokenKind::Float(f) => {
                self.advance();
                Pattern::FloatLit(f)
            }
            TokenKind::Str(ref s) => {
                let s = s.clone();
                self.advance();
                Pattern::StringLit(s)
            }
            TokenKind::True => {
                self.advance();
                Pattern::BoolLit(true)
            }
            TokenKind::False => {
                self.advance();
                Pattern::BoolLit(false)
            }
            TokenKind::Ident(ref s) => {
                let s = s.clone();
                self.advance();
                Pattern::Ident(s)
            }
            TokenKind::UpperIdent(ref s) => {
                let s = s.clone();
                self.advance();
                if self.at(&TokenKind::LParen) {
                    self.advance();
                    let mut pats = Vec::new();
                    if !self.at(&TokenKind::RParen) {
                        pats.push(self.parse_pattern()?);
                        while self.at(&TokenKind::Comma) {
                            self.advance();
                            pats.push(self.parse_pattern()?);
                        }
                    }
                    self.expect(&TokenKind::RParen)?;
                    Pattern::Constructor(s, pats)
                } else {
                    Pattern::Constructor(s, Vec::new())
                }
            }
            TokenKind::LParen => {
                self.advance();
                let mut pats = Vec::new();
                if !self.at(&TokenKind::RParen) {
                    pats.push(self.parse_pattern()?);
                    while self.at(&TokenKind::Comma) {
                        self.advance();
                        pats.push(self.parse_pattern()?);
                    }
                }
                self.expect(&TokenKind::RParen)?;
                Pattern::Tuple(pats)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut pats = Vec::new();
                let mut rest = None;
                if !self.at(&TokenKind::RBracket) {
                    pats.push(self.parse_pattern()?);
                    while self.at(&TokenKind::Comma) {
                        self.advance();
                        pats.push(self.parse_pattern()?);
                    }
                    if self.at(&TokenKind::Pipe) {
                        self.advance();
                        rest = Some(self.expect_ident()?);
                    }
                }
                self.expect(&TokenKind::RBracket)?;
                Pattern::List(pats, rest)
            }
            _ => {
                return Err(format!(
                    "{} Unexpected token in pattern: {:?}",
                    self.span(),
                    self.peek()
                ))
            }
        };

        // Check for `as` binding
        if self.at(&TokenKind::As) {
            self.advance();
            let name = self.expect_ident()?;
            Ok(Pattern::Bind(name, Box::new(pat)))
        } else {
            Ok(pat)
        }
    }

    fn parse_lambda(&mut self) -> Result<Expr, String> {
        self.parse_lambda_inner(false)
    }

    fn parse_lambda_inner(&mut self, is_move: bool) -> Result<Expr, String> {
        let span = self.span();
        self.expect(&TokenKind::Fn)?;
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;
        let return_type = if self.at(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.expect(&TokenKind::FatArrow)?;
        self.skip_newlines();
        let body = self.parse_expr()?;

        Ok(Expr {
            kind: ExprKind::Lambda(params, return_type, Box::new(body), is_move),
            span,
        })
    }

    fn parse_let_expr(&mut self) -> Result<Expr, String> {
        let span = self.span();
        self.expect(&TokenKind::Let)?;
        let _is_mut = if self.at(&TokenKind::Mut) {
            self.advance();
            true
        } else {
            false
        };
        let pattern = self.parse_pattern()?;
        let ty = if self.at(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.expect(&TokenKind::Eq)?;
        self.skip_newlines();
        let value = self.parse_expr()?;

        Ok(Expr {
            kind: ExprKind::Let(pattern, ty, Box::new(value)),
            span,
        })
    }

    // ── Extern Functions ────────────────────────────────────────────────────

    // extern fn name(params): ReturnType
    // extern fn name(params): ReturnType = "rust_path"
    fn parse_extern_fn(&mut self) -> Result<ExternFn, String> {
        let span = self.span();
        self.expect(&TokenKind::Extern)?;
        self.expect(&TokenKind::Fn)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;

        let return_type = if self.at(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        // Optional Rust function path: extern fn foo(): Int = "std::process::id"
        let rust_name = if self.at(&TokenKind::Eq) {
            self.advance();
            match self.peek().clone() {
                TokenKind::Str(s) => {
                    let s = s.clone();
                    self.advance();
                    Some(s)
                }
                _ => return Err(format!("{} Expected string literal for extern fn path", self.span())),
            }
        } else {
            None
        };

        Ok(ExternFn {
            name,
            params,
            return_type,
            rust_name,
            span,
        })
    }

    // ── Traits ──────────────────────────────────────────────────────────────

    // trait Name<T>
    //   fn method(params): ReturnType
    //   fn method_with_default(params): ReturnType = body
    // end
    fn parse_trait_decl(&mut self) -> Result<TraitDecl, String> {
        let span = self.span();
        self.expect(&TokenKind::Trait)?;
        let name = self.expect_upper_ident()?;
        let type_params = self.parse_optional_type_params()?;
        self.skip_newlines();

        let mut methods = Vec::new();
        let mut associated_types = Vec::new();
        while !self.at(&TokenKind::End) && !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Type) {
                // Associated type: `type Output`
                self.advance();
                let type_name = self.expect_upper_ident()?;
                associated_types.push(type_name);
                self.skip_newlines();
            } else {
                methods.push(self.parse_trait_method()?);
                self.skip_newlines();
            }
        }
        self.expect(&TokenKind::End)?;

        Ok(TraitDecl {
            name,
            type_params,
            methods,
            associated_types,
            span,
        })
    }

    fn parse_trait_method(&mut self) -> Result<TraitMethod, String> {
        let span = self.span();
        self.expect(&TokenKind::Fn)?;
        let name = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;

        let return_type = if self.at(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        // Optional default body
        let default_body = if self.at(&TokenKind::Eq) {
            self.advance();
            self.skip_newlines();
            Some(self.parse_expr()?)
        } else {
            None
        };

        Ok(TraitMethod {
            name,
            params,
            return_type,
            default_body,
            span,
        })
    }

    // ── Impl Blocks ─────────────────────────────────────────────────────────

    // impl Type
    //   fn method(params) = body
    // end
    //
    // impl Trait for Type
    //   fn method(params) = body
    // end
    fn parse_impl_block(&mut self) -> Result<ImplBlock, String> {
        let span = self.span();
        self.expect(&TokenKind::Impl)?;

        let first_name = self.expect_upper_ident()?;
        let type_params = self.parse_optional_type_params()?;

        // Check for "for Type" (trait impl) vs direct type impl
        let (trait_name, type_name) = if self.at(&TokenKind::For) {
            self.advance();
            let type_name = self.expect_upper_ident()?;
            (Some(first_name), type_name)
        } else {
            (None, first_name)
        };

        self.skip_newlines();
        let mut methods = Vec::new();
        let mut associated_types = Vec::new();
        while !self.at(&TokenKind::End) && !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Type) {
                // Associated type: `type Output = ConcreteType`
                self.advance();
                let assoc_name = self.expect_upper_ident()?;
                self.expect(&TokenKind::Eq)?;
                let assoc_ty = self.parse_type_expr()?;
                associated_types.push((assoc_name, assoc_ty));
                self.skip_newlines();
            } else {
                let is_pub = if self.at(&TokenKind::Pub) {
                    self.advance();
                    true
                } else {
                    false
                };
                methods.push(self.parse_function(is_pub, false)?);
                self.skip_newlines();
            }
        }
        self.expect(&TokenKind::End)?;

        Ok(ImplBlock {
            trait_name,
            type_name,
            type_params,
            methods,
            associated_types,
            where_clauses: vec![],
            span,
        })
    }

    fn parse_do_block(&mut self) -> Result<Expr, String> {
        let span = self.span();
        self.expect(&TokenKind::Do)?;
        self.skip_newlines();

        let mut stmts = Vec::new();
        while !self.at(&TokenKind::End) && !self.at(&TokenKind::Eof) {
            let stmt = self.parse_statement()?;
            stmts.push(stmt);
            self.skip_newlines();
        }
        self.expect(&TokenKind::End)?;

        // Last statement becomes the return expression
        if stmts.is_empty() {
            return Ok(Expr {
                kind: ExprKind::IntLit(0), // unit
                span,
            });
        }

        let last = stmts.pop().unwrap();
        let final_expr = match last {
            Stmt::Expr(e) => e,
            Stmt::Let(is_mut, pat, ty, val) => {
                // A let as the last statement returns unit
                stmts.push(Stmt::Let(is_mut, pat, ty, val));
                Expr {
                    kind: ExprKind::IntLit(0),
                    span,
                }
            }
            Stmt::Assign(name, val) => {
                stmts.push(Stmt::Assign(name, val));
                Expr {
                    kind: ExprKind::IntLit(0),
                    span,
                }
            }
            Stmt::CompoundAssign(name, op, val) => {
                stmts.push(Stmt::CompoundAssign(name, op, val));
                Expr {
                    kind: ExprKind::IntLit(0),
                    span,
                }
            }
            Stmt::IndexAssign(obj, index, val) => {
                stmts.push(Stmt::IndexAssign(obj, index, val));
                Expr {
                    kind: ExprKind::IntLit(0),
                    span,
                }
            }
        };

        Ok(Expr {
            kind: ExprKind::Block(stmts, Box::new(final_expr)),
            span,
        })
    }

    // for name in collection do ... end
    fn parse_for_loop(&mut self) -> Result<Expr, String> {
        let span = self.span();
        self.expect(&TokenKind::For)?;
        let pattern = self.parse_pattern()?;
        self.expect(&TokenKind::In)?;
        let iter = self.parse_expr()?;
        self.expect(&TokenKind::Do)?;
        self.skip_newlines();

        let mut stmts = Vec::new();
        while !self.at(&TokenKind::End) && !self.at(&TokenKind::Eof) {
            let stmt = self.parse_statement()?;
            stmts.push(stmt);
            self.skip_newlines();
        }
        self.expect(&TokenKind::End)?;

        let body = if stmts.is_empty() {
            Expr { kind: ExprKind::IntLit(0), span }
        } else {
            let last = stmts.pop().unwrap();
            let final_expr = match last {
                Stmt::Expr(e) => e,
                other => {
                    stmts.push(other);
                    Expr { kind: ExprKind::IntLit(0), span }
                }
            };
            if stmts.is_empty() {
                final_expr
            } else {
                Expr {
                    kind: ExprKind::Block(stmts, Box::new(final_expr)),
                    span,
                }
            }
        };

        Ok(Expr {
            kind: ExprKind::For(pattern, Box::new(iter), Box::new(body)),
            span,
        })
    }

    // while condition do ... end
    fn parse_while_loop(&mut self) -> Result<Expr, String> {
        let span = self.span();
        self.expect(&TokenKind::While)?;
        let cond = self.parse_expr()?;
        self.expect(&TokenKind::Do)?;
        self.skip_newlines();

        let mut stmts = Vec::new();
        while !self.at(&TokenKind::End) && !self.at(&TokenKind::Eof) {
            let stmt = self.parse_statement()?;
            stmts.push(stmt);
            self.skip_newlines();
        }
        self.expect(&TokenKind::End)?;

        let body = if stmts.is_empty() {
            Expr { kind: ExprKind::IntLit(0), span }
        } else {
            let last = stmts.pop().unwrap();
            let final_expr = match last {
                Stmt::Expr(e) => e,
                other => {
                    stmts.push(other);
                    Expr { kind: ExprKind::IntLit(0), span }
                }
            };
            if stmts.is_empty() {
                final_expr
            } else {
                Expr {
                    kind: ExprKind::Block(stmts, Box::new(final_expr)),
                    span,
                }
            }
        };

        Ok(Expr {
            kind: ExprKind::While(Box::new(cond), Box::new(body)),
            span,
        })
    }

    fn parse_loop_expr(&mut self) -> Result<Expr, String> {
        let span = self.span();
        self.expect(&TokenKind::Loop)?;
        self.expect(&TokenKind::Do)?;
        self.skip_newlines();

        let mut stmts = Vec::new();
        while !self.at(&TokenKind::End) && !self.at(&TokenKind::Eof) {
            let stmt = self.parse_statement()?;
            stmts.push(stmt);
            self.skip_newlines();
        }
        self.expect(&TokenKind::End)?;

        let body = if stmts.is_empty() {
            Expr { kind: ExprKind::IntLit(0), span }
        } else {
            let last = stmts.pop().unwrap();
            let final_expr = match last {
                Stmt::Expr(e) => e,
                other => {
                    stmts.push(other);
                    Expr { kind: ExprKind::IntLit(0), span }
                }
            };
            if stmts.is_empty() {
                final_expr
            } else {
                Expr {
                    kind: ExprKind::Block(stmts, Box::new(final_expr)),
                    span,
                }
            }
        };

        Ok(Expr {
            kind: ExprKind::Loop(Box::new(body)),
            span,
        })
    }

    fn parse_statement(&mut self) -> Result<Stmt, String> {
        if self.at(&TokenKind::Let) {
            self.advance();
            let is_mut = if self.at(&TokenKind::Mut) {
                self.advance();
                true
            } else {
                false
            };
            let pattern = self.parse_pattern()?;
            let ty = if self.at(&TokenKind::Colon) {
                self.advance();
                Some(self.parse_type_expr()?)
            } else {
                None
            };
            self.expect(&TokenKind::Eq)?;
            self.skip_newlines();
            let value = self.parse_expr()?;
            Ok(Stmt::Let(is_mut, pattern, ty, value))
        } else {
            let expr = self.parse_expr()?;

            // Check for compound assignment: ident += expr, etc.
            if let ExprKind::Ident(ref name) = expr.kind {
                let compound_op = match self.peek() {
                    TokenKind::PlusEq => Some(BinOp::Add),
                    TokenKind::MinusEq => Some(BinOp::Sub),
                    TokenKind::StarEq => Some(BinOp::Mul),
                    TokenKind::SlashEq => Some(BinOp::Div),
                    TokenKind::PercentEq => Some(BinOp::Mod),
                    _ => None,
                };
                if let Some(op) = compound_op {
                    let name = name.clone();
                    self.advance();
                    self.skip_newlines();
                    let value = self.parse_expr()?;
                    return Ok(Stmt::CompoundAssign(name, op, value));
                }
            }

            // Check for assignment: ident = expr
            if let ExprKind::Ident(ref name) = expr.kind {
                if self.at(&TokenKind::Eq) {
                    let name = name.clone();
                    self.advance();
                    self.skip_newlines();
                    let value = self.parse_expr()?;
                    return Ok(Stmt::Assign(name, value));
                }
            }

            // Check for index assignment: expr[index] = value
            if let ExprKind::MethodCall(ref obj, ref method, ref args) = expr.kind {
                if method == "__index" && args.len() == 1 && self.at(&TokenKind::Eq) {
                    let obj = (**obj).clone();
                    let index = args[0].clone();
                    self.advance();
                    self.skip_newlines();
                    let value = self.parse_expr()?;
                    return Ok(Stmt::IndexAssign(obj, index, value));
                }
            }

            Ok(Stmt::Expr(expr))
        }
    }

    fn parse_interp_string(
        &mut self,
        parts: Vec<(bool, String)>,
        span: Span,
    ) -> Result<Expr, String> {
        use crate::lexer;

        let mut ast_parts = Vec::new();
        for (is_expr, text) in parts {
            if is_expr {
                // Parse the expression source
                let tokens = lexer::lex(&text)
                    .map_err(|e| format!("{span} In string interpolation: {e}"))?;
                let mut sub_parser = Parser::new(tokens);
                let expr = sub_parser.parse_expr()
                    .map_err(|e| format!("{span} In string interpolation: {e}"))?;
                ast_parts.push(StringPart::Expr(expr));
            } else {
                ast_parts.push(StringPart::Lit(text));
            }
        }
        Ok(Expr {
            kind: ExprKind::StringInterp(ast_parts),
            span,
        })
    }

    fn parse_rust_block(&mut self) -> Result<Expr, String> {
        let span = self.span();
        self.expect(&TokenKind::RustBang)?;
        self.expect(&TokenKind::LParen)?;
        let code = match self.peek().clone() {
            TokenKind::Str(s) => {
                let s = s.clone();
                self.advance();
                s
            }
            _ => return Err(format!("{} Expected string after rust!", span)),
        };
        self.expect(&TokenKind::RParen)?;
        Ok(Expr {
            kind: ExprKind::RustBlock(code),
            span,
        })
    }

    fn parse_arg_list(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();
        if self.at(&TokenKind::RParen) {
            return Ok(args);
        }

        args.push(self.parse_expr()?);
        while self.at(&TokenKind::Comma) {
            self.advance();
            args.push(self.parse_expr()?);
        }
        Ok(args)
    }
}

fn token_to_string(tok: &TokenKind) -> String {
    match tok {
        TokenKind::Ident(s) | TokenKind::UpperIdent(s) | TokenKind::Str(s) => s.clone(),
        TokenKind::Int(n) => n.to_string(),
        TokenKind::Float(f) => f.to_string(),
        TokenKind::LParen => "(".to_string(),
        TokenKind::RParen => ")".to_string(),
        TokenKind::Comma => ", ".to_string(),
        TokenKind::Eq => "=".to_string(),
        TokenKind::EqEq => "==".to_string(),
        TokenKind::Ne => "!=".to_string(),
        TokenKind::Lt => "<".to_string(),
        TokenKind::Gt => ">".to_string(),
        TokenKind::Le => "<=".to_string(),
        TokenKind::Ge => ">=".to_string(),
        TokenKind::Colon => ":".to_string(),
        TokenKind::ColonColon => "::".to_string(),
        TokenKind::Dot => ".".to_string(),
        TokenKind::DotDot => "..".to_string(),
        TokenKind::Plus => "+".to_string(),
        TokenKind::Minus => "-".to_string(),
        TokenKind::Star => "*".to_string(),
        TokenKind::Slash => "/".to_string(),
        TokenKind::Percent => "%".to_string(),
        TokenKind::Ampersand => "&".to_string(),
        TokenKind::Pipe => "|".to_string(),
        TokenKind::True => "true".to_string(),
        TokenKind::False => "false".to_string(),
        TokenKind::Underscore => "_".to_string(),
        TokenKind::Tick(s) => format!("'{s}"),
        _ => format!("{:?}", tok),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;

    fn parse_str(src: &str) -> Result<Program, String> {
        let tokens = lexer::lex(src)?;
        let (program, _comments) = parse(tokens)?;
        Ok(program)
    }

    #[test]
    fn test_parse_function() {
        let prog = parse_str("fn add(a: Int, b: Int): Int = a + b").unwrap();
        assert_eq!(prog.items.len(), 1);
        match &prog.items[0] {
            Item::Function(f) => {
                assert_eq!(f.name, "add");
                assert_eq!(f.params.len(), 2);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_type_decl() {
        let prog = parse_str(
            "type Shape =\n  | Circle(Float)\n  | Rectangle(Float, Float)\n  | Point",
        )
        .unwrap();
        assert_eq!(prog.items.len(), 1);
        match &prog.items[0] {
            Item::TypeDecl(td) => {
                assert_eq!(td.name, "Shape");
                match &td.body {
                    TypeBody::Enum(variants) => assert_eq!(variants.len(), 3),
                    _ => panic!("Expected enum"),
                }
            }
            _ => panic!("Expected type decl"),
        }
    }

    #[test]
    fn test_parse_type_alias_simple() {
        let prog = parse_str("type StringList = List<String>").unwrap();
        assert_eq!(prog.items.len(), 1);
        match &prog.items[0] {
            Item::TypeDecl(td) => {
                assert_eq!(td.name, "StringList");
                match &td.body {
                    TypeBody::Alias(TypeExpr::Named(name, args)) => {
                        assert_eq!(name, "List");
                        assert_eq!(args.len(), 1);
                    }
                    _ => panic!("Expected alias"),
                }
            }
            _ => panic!("Expected type decl"),
        }
    }

    #[test]
    fn test_parse_type_alias_tuple() {
        let prog = parse_str("type IntPair = (Int, Int)").unwrap();
        match &prog.items[0] {
            Item::TypeDecl(td) => {
                assert_eq!(td.name, "IntPair");
                match &td.body {
                    TypeBody::Alias(TypeExpr::Tuple(elems)) => {
                        assert_eq!(elems.len(), 2);
                    }
                    _ => panic!("Expected tuple alias"),
                }
            }
            _ => panic!("Expected type decl"),
        }
    }

    #[test]
    fn test_parse_type_alias_function() {
        let prog = parse_str("type Callback = fn(Int) -> String").unwrap();
        match &prog.items[0] {
            Item::TypeDecl(td) => {
                assert_eq!(td.name, "Callback");
                match &td.body {
                    TypeBody::Alias(TypeExpr::Function(params, _ret)) => {
                        assert_eq!(params.len(), 1);
                    }
                    _ => panic!("Expected function alias"),
                }
            }
            _ => panic!("Expected type decl"),
        }
    }

    #[test]
    fn test_parse_type_alias_with_type_params() {
        let prog = parse_str("type Pair<A, B> = (A, B)").unwrap();
        match &prog.items[0] {
            Item::TypeDecl(td) => {
                assert_eq!(td.name, "Pair");
                assert_eq!(td.type_params.len(), 2);
                assert_eq!(td.type_params[0].name, "A");
                assert_eq!(td.type_params[1].name, "B");
                match &td.body {
                    TypeBody::Alias(TypeExpr::Tuple(elems)) => {
                        assert_eq!(elems.len(), 2);
                    }
                    _ => panic!("Expected tuple alias"),
                }
            }
            _ => panic!("Expected type decl"),
        }
    }

    #[test]
    fn test_parse_pipe() {
        let prog = parse_str("fn main() = 5 |> double |> add_one").unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Pipe(_, _) => {}
                _ => panic!("Expected pipe expression"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_match() {
        let src = r#"fn f(x: Int) = match x
  | 0 => "zero"
  | 1 => "one"
  | _ => "other"
  end"#;
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Match(_, arms) => assert_eq!(arms.len(), 3),
                _ => panic!("Expected match"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_if() {
        let prog = parse_str("fn f(x: Int) = if x > 0 then x else 0 - x end").unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::If(_, _, Some(_)) => {}
                _ => panic!("Expected if-else"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_lambda() {
        let prog = parse_str("fn main() = fn(x) => x + 1").unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Lambda(params, _, _, _) => assert_eq!(params.len(), 1),
                _ => panic!("Expected lambda"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_or_pattern() {
        let src = r#"fn f(x: Int) = match x
  | 0 | 1 => "small"
  | _ => "big"
  end"#;
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Match(_, arms) => {
                    assert_eq!(arms.len(), 2);
                    match &arms[0].pattern {
                        Pattern::Or(pats) => {
                            assert_eq!(pats.len(), 2);
                            assert!(matches!(&pats[0], Pattern::IntLit(0)));
                            assert!(matches!(&pats[1], Pattern::IntLit(1)));
                        }
                        _ => panic!("Expected or-pattern"),
                    }
                }
                _ => panic!("Expected match"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_or_pattern_constructors() {
        let src = r#"fn f(x) = match x
  | Some(a) | None => "done"
  end"#;
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Match(_, arms) => {
                    assert_eq!(arms.len(), 1);
                    match &arms[0].pattern {
                        Pattern::Or(pats) => assert_eq!(pats.len(), 2),
                        _ => panic!("Expected or-pattern"),
                    }
                }
                _ => panic!("Expected match"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_range_pattern() {
        let src = r#"fn f(x: Int) = match x
  | 1..10 => "small"
  | _ => "big"
  end"#;
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Match(_, arms) => {
                    assert_eq!(arms.len(), 2);
                    match &arms[0].pattern {
                        Pattern::Range(1, 10) => {}
                        _ => panic!("Expected range pattern 1..10"),
                    }
                }
                _ => panic!("Expected match"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_string_pattern() {
        let src = r#"fn f(x: String) = match x
  | "hello" => 1
  | "world" => 2
  | _ => 0
  end"#;
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Match(_, arms) => {
                    assert_eq!(arms.len(), 3);
                    match &arms[0].pattern {
                        Pattern::StringLit(s) => assert_eq!(s, "hello"),
                        _ => panic!("Expected string pattern"),
                    }
                }
                _ => panic!("Expected match"),
            },
            _ => panic!("Expected function"),
        }
    }

    // ── Nested if/else chains ────────────────────────────────────

    #[test]
    fn test_parse_nested_if_else() {
        let src = r#"fn f(x: Int) = if x > 10 then "big" else if x > 0 then "small" else "neg" end end"#;
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::If(_, _, Some(else_branch)) => match &else_branch.kind {
                    ExprKind::If(_, _, Some(_)) => {}
                    _ => panic!("Expected nested if-else"),
                },
                _ => panic!("Expected if-else"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_if_without_else() {
        let src = "fn f(x: Int) = if x > 0 then println(x) end";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::If(_, _, None) => {}
                _ => panic!("Expected if without else"),
            },
            _ => panic!("Expected function"),
        }
    }

    // ── Match with guards ────────────────────────────────────────

    #[test]
    fn test_parse_match_with_guard() {
        let src = r#"fn f(x: Int) = match x
  | n when n > 0 => "positive"
  | _ => "non-positive"
  end"#;
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Match(_, arms) => {
                    assert_eq!(arms.len(), 2);
                    assert!(arms[0].guard.is_some());
                    assert!(arms[1].guard.is_none());
                }
                _ => panic!("Expected match"),
            },
            _ => panic!("Expected function"),
        }
    }

    // ── Lambda with type annotations ─────────────────────────────

    #[test]
    fn test_parse_lambda_with_type_annotations() {
        let src = "fn main() = fn(x: Int, y: String) => x";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Lambda(params, _, _, _) => {
                    assert_eq!(params.len(), 2);
                    assert_eq!(params[0].name, "x");
                    assert!(params[0].ty.is_some());
                    assert_eq!(params[1].name, "y");
                    assert!(params[1].ty.is_some());
                }
                _ => panic!("Expected lambda"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_lambda_no_params() {
        let src = "fn main() = fn() => 42";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Lambda(params, _, _, _) => assert_eq!(params.len(), 0),
                _ => panic!("Expected lambda"),
            },
            _ => panic!("Expected function"),
        }
    }

    // ── Do blocks ────────────────────────────────────────────────

    #[test]
    fn test_parse_do_block() {
        let src = "fn main() = do\n  let x = 1\n  let y = 2\n  x + y\nend";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Block(stmts, _) => {
                    assert_eq!(stmts.len(), 2); // two let statements, final expr is separate
                }
                _ => panic!("Expected block, got {:?}", f.body.kind),
            },
            _ => panic!("Expected function"),
        }
    }

    // ── For and while loops ──────────────────────────────────────

    #[test]
    fn test_parse_for_loop() {
        let src = "fn main() = for x in [1, 2, 3] do\n  println(x)\nend";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::For(Pattern::Ident(var), _, _) => assert_eq!(var, "x"),
                _ => panic!("Expected for loop"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_while_loop() {
        let src = "fn main() = while true do\n  println(1)\nend";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::While(_, _) => {}
                _ => panic!("Expected while loop"),
            },
            _ => panic!("Expected function"),
        }
    }

    // ── Pipe chains ──────────────────────────────────────────────

    #[test]
    fn test_parse_pipe_chain_multiline() {
        let src = "fn main() = [1, 2, 3]\n  |> map(fn(x) => x + 1)\n  |> filter(fn(x) => x > 2)";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Pipe(_, _) => {}
                _ => panic!("Expected pipe expression"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_long_pipe_chain() {
        let src = "fn main() = x |> f |> g |> h";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Pipe(_, _) => {} // pipes nest left-to-right
                _ => panic!("Expected pipe expression"),
            },
            _ => panic!("Expected function"),
        }
    }

    // ── Struct literals ──────────────────────────────────────────

    #[test]
    fn test_parse_struct_literal() {
        let src = "type Point = {\n  x: Float,\n  y: Float\n}\nfn main() = Point { x: 1.0, y: 2.0 }";
        let prog = parse_str(src).unwrap();
        assert_eq!(prog.items.len(), 2);
        match &prog.items[1] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::StructLit(name, fields, _) => {
                    assert_eq!(name, "Point");
                    assert_eq!(fields.len(), 2);
                    assert_eq!(fields[0].0, "x");
                    assert_eq!(fields[1].0, "y");
                }
                _ => panic!("Expected struct literal"),
            },
            _ => panic!("Expected function"),
        }
    }

    // ── Module and use declarations ──────────────────────────────

    #[test]
    fn test_parse_use_decl() {
        let src = "use Foo";
        let prog = parse_str(src).unwrap();
        assert_eq!(prog.items.len(), 1);
        match &prog.items[0] {
            Item::UseDecl(u) => {
                assert_eq!(u.path, vec!["Foo"]);
            }
            _ => panic!("Expected use decl"),
        }
    }

    // ── Negative cases: syntax errors ────────────────────────────

    #[test]
    fn test_parse_error_missing_equals() {
        let result = parse_str("fn foo(x: Int) x + 1");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_missing_end_in_match() {
        let result = parse_str("fn f(x: Int) = match x\n  | 0 => 1\n  | _ => 2");
        assert!(result.is_err());
    }

    // ── Type declarations ────────────────────────────────────────

    #[test]
    fn test_parse_enum_with_fields() {
        let src = "type Expr =\n  | Num(Int)\n  | Add(Expr, Expr)\n  | Neg(Expr)";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::TypeDecl(td) => match &td.body {
                TypeBody::Enum(variants) => {
                    assert_eq!(variants.len(), 3);
                    assert_eq!(variants[0].name, "Num");
                    assert_eq!(variants[0].fields.len(), 1);
                    assert_eq!(variants[1].name, "Add");
                    assert_eq!(variants[1].fields.len(), 2);
                    assert_eq!(variants[2].name, "Neg");
                    assert_eq!(variants[2].fields.len(), 1);
                }
                _ => panic!("Expected enum"),
            },
            _ => panic!("Expected type decl"),
        }
    }

    #[test]
    fn test_parse_struct_decl() {
        let src = "type Person = {\n  name: String,\n  age: Int\n}";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::TypeDecl(td) => {
                assert_eq!(td.name, "Person");
                match &td.body {
                    TypeBody::Struct(fields) => {
                        assert_eq!(fields.len(), 2);
                        assert_eq!(fields[0].name, "name");
                        assert_eq!(fields[1].name, "age");
                    }
                    _ => panic!("Expected struct"),
                }
            }
            _ => panic!("Expected type decl"),
        }
    }

    // ── Multiple items ───────────────────────────────────────────

    #[test]
    fn test_parse_multiple_functions() {
        let src = "fn add(a: Int, b: Int): Int = a + b\n\nfn main() = add(1, 2)";
        let prog = parse_str(src).unwrap();
        assert_eq!(prog.items.len(), 2);
        match &prog.items[0] {
            Item::Function(f) => assert_eq!(f.name, "add"),
            _ => panic!("Expected function"),
        }
        match &prog.items[1] {
            Item::Function(f) => assert_eq!(f.name, "main"),
            _ => panic!("Expected function"),
        }
    }

    // ── Pub function ─────────────────────────────────────────────

    #[test]
    fn test_parse_pub_function() {
        let src = "pub fn hello(): String = \"world\"";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => {
                assert_eq!(f.name, "hello");
                assert!(f.is_pub);
            }
            _ => panic!("Expected function"),
        }
    }

    // ── Async function ───────────────────────────────────────────

    #[test]
    fn test_parse_async_function() {
        let src = "async fn fetch() = 42";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => {
                assert_eq!(f.name, "fetch");
                assert!(f.is_async);
            }
            _ => panic!("Expected function"),
        }
    }

    // ── Expressions ──────────────────────────────────────────────

    #[test]
    fn test_parse_list_literal() {
        let src = "fn main() = [1, 2, 3]";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::ListLit(elems) => assert_eq!(elems.len(), 3),
                _ => panic!("Expected list literal"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_tuple() {
        let src = "fn main() = (1, 2, 3)";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Tuple(elems) => assert_eq!(elems.len(), 3),
                _ => panic!("Expected tuple, got {:?}", f.body.kind),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_unary_neg() {
        let src = "fn main() = -5";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::UnaryOp(UnaryOp::Neg, _) => {}
                _ => panic!("Expected unary negation"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_unary_not() {
        let src = "fn main() = not true";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::UnaryOp(UnaryOp::Not, _) => {}
                _ => panic!("Expected unary not"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_field_access() {
        let src = "fn main() = point.x";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::FieldAccess(_, field) => assert_eq!(field, "x"),
                _ => panic!("Expected field access"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_function_call_multiple_args() {
        let src = "fn main() = add(1, 2, 3)";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Call(_, args) => assert_eq!(args.len(), 3),
                _ => panic!("Expected call"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_try_operator() {
        let src = r#"fn main() = read_file("test.txt")?"#;
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Try(_) => {}
                _ => panic!("Expected try expression"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_break_continue() {
        let src = "fn main() = for x in [1, 2, 3] do\n  if x == 2 then break end\n  continue\nend";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::For(_, _, _) => {}
                _ => panic!("Expected for loop"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_let_with_pattern() {
        let src = "fn main() = do\n  let (a, b) = (1, 2)\n  a + b\nend";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Block(stmts, _) => {
                    assert!(!stmts.is_empty());
                }
                _ => panic!("Expected block"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_match_constructor_pattern() {
        let src = "type Option<T> =\n  | Some(T)\n  | None\n\nfn f(x) = match x\n  | Some(v) => v\n  | None => 0\n  end";
        let prog = parse_str(src).unwrap();
        assert_eq!(prog.items.len(), 2);
        match &prog.items[1] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Match(_, arms) => {
                    assert_eq!(arms.len(), 2);
                    match &arms[0].pattern {
                        Pattern::Constructor(name, args) => {
                            assert_eq!(name, "Some");
                            assert_eq!(args.len(), 1);
                        }
                        _ => panic!("Expected constructor pattern"),
                    }
                }
                _ => panic!("Expected match"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_match_tuple_pattern() {
        let src = "fn f(p) = match p\n  | (0, 0) => \"origin\"\n  | (x, y) => \"other\"\n  end";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Match(_, arms) => {
                    assert_eq!(arms.len(), 2);
                    match &arms[0].pattern {
                        Pattern::Tuple(elems) => assert_eq!(elems.len(), 2),
                        _ => panic!("Expected tuple pattern"),
                    }
                }
                _ => panic!("Expected match"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_generic_function() {
        let src = "fn identity<T>(x: T): T = x";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => {
                assert_eq!(f.name, "identity");
                assert_eq!(f.type_params.len(), 1);
                assert_eq!(f.type_params[0].name, "T");
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_bool_operators() {
        let src = "fn f(a: Bool, b: Bool) = a and b or not a";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::BinOp(_, BinOp::Or, _) => {}
                _ => panic!("Expected 'or' as top-level binop"),
            },
            _ => panic!("Expected function"),
        }
    }

    // ── Additional parser tests ────────────────────────────────────

    #[test]
    fn test_parse_struct_type_fields() {
        let src = "type Point = {\n  x: Float,\n  y: Float\n}";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::TypeDecl(td) => {
                assert_eq!(td.name, "Point");
                match &td.body {
                    TypeBody::Struct(fields) => {
                        assert_eq!(fields.len(), 2);
                        assert_eq!(fields[0].name, "x");
                        assert_eq!(fields[1].name, "y");
                    }
                    _ => panic!("Expected struct"),
                }
            }
            _ => panic!("Expected type decl"),
        }
    }

    #[test]
    fn test_parse_trait_with_methods() {
        let src = "trait Printable\n  fn to_str(self): String\nend";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::TraitDecl(t) => {
                assert_eq!(t.name, "Printable");
                assert_eq!(t.methods.len(), 1);
                assert_eq!(t.methods[0].name, "to_str");
            }
            _ => panic!("Expected trait decl"),
        }
    }

    #[test]
    fn test_parse_impl_block_methods() {
        let src = "type Foo = { x: Int }\n\nimpl Foo\n  fn get_x(self): Int = self.x\nend";
        let prog = parse_str(src).unwrap();
        let has_impl = prog.items.iter().any(|item| matches!(item, Item::ImplBlock(_)));
        assert!(has_impl, "Should parse impl block");
    }

    #[test]
    fn test_parse_match_four_arms() {
        let src = r#"fn f(x: Int): String = match x
  | 0 => "zero"
  | 1 => "one"
  | 2 => "two"
  | _ => "other"
  end"#;
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Match(_, arms) => assert_eq!(arms.len(), 4),
                _ => panic!("Expected match expression"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_do_block_body() {
        let src = "fn main() = do\n  let x = 1\n  let y = 2\n  x + y\nend";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Block(_, _) => {}
                _ => panic!("Expected block expression, got {:?}", f.body.kind),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_for_loop_body() {
        let src = "fn main() = for x in items do\n  println(x)\nend";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::For(_, _, _) => {}
                _ => panic!("Expected for loop"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_while_loop_body() {
        let src = "fn main() = while true do\n  println(1)\nend";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::While(_, _) => {}
                _ => panic!("Expected while loop"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_lambda_expression() {
        let src = "fn main() = fn(x) => x + 1";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Lambda(_, _, _, _) => {}
                _ => panic!("Expected lambda, got {:?}", f.body.kind),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_lambda_typed_params() {
        let src = "fn main() = fn(x: Int, y: Int): Int => x + y";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Lambda(params, _, _, _) => {
                    assert_eq!(params.len(), 2);
                    assert!(params[0].ty.is_some());
                    assert!(params[1].ty.is_some());
                }
                _ => panic!("Expected lambda"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_pipe_chain() {
        let src = "fn main() = 5 |> double |> add_one";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Pipe(_, _) => {}
                _ => panic!("Expected pipe, got {:?}", f.body.kind),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_list_three_elements() {
        let src = "fn main() = [1, 2, 3]";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::ListLit(items) => assert_eq!(items.len(), 3),
                _ => panic!("Expected list literal"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_empty_list_literal() {
        let src = "fn main() = []";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::ListLit(items) => assert_eq!(items.len(), 0),
                _ => panic!("Expected empty list literal, got {:?}", f.body.kind),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_tuple_three_elements() {
        let src = "fn main() = (1, 2, 3)";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Tuple(items) => assert_eq!(items.len(), 3),
                _ => panic!("Expected tuple, got {:?}", f.body.kind),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_if_else_branches() {
        let src = "fn f(x: Int): Int = if x > 0 then x else 0 end";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::If(_, _, Some(_)) => {}
                _ => panic!("Expected if-else expression"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_if_no_else() {
        let src = "fn f(x: Int) = if x > 0 then println(x) end";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::If(_, _, None) => {}
                _ => panic!("Expected if without else"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_interp_string() {
        let src = r#"fn main() = "hello #{name}""#;
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::StringInterp(_) => {}
                _ => panic!("Expected string interpolation, got {:?}", f.body.kind),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_struct_construction() {
        let src = "fn main() = Point { x: 1.0, y: 2.0 }";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::StructLit(name, _, _) => assert_eq!(name, "Point"),
                _ => panic!("Expected struct literal, got {:?}", f.body.kind),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_dot_field_access() {
        let src = "fn get_x(p: Point): Float = p.x";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::FieldAccess(_, field) => assert_eq!(field, "x"),
                _ => panic!("Expected field access, got {:?}", f.body.kind),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_method_call_args() {
        let src = "fn main() = obj.method(1, 2)";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::MethodCall(_, method, args) => {
                    assert_eq!(method, "method");
                    assert_eq!(args.len(), 2);
                }
                _ => panic!("Expected method call, got {:?}", f.body.kind),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_module_declaration() {
        let src = "module Math\n  pub fn double(x: Int): Int = x * 2\nend";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::ModuleDecl(m) => {
                assert_eq!(m.name, "Math");
                assert!(!m.items.is_empty());
            }
            _ => panic!("Expected module decl"),
        }
    }

    #[test]
    fn test_parse_use_statement() {
        let src = "use Math";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::UseDecl(u) => {
                assert_eq!(u.path, vec!["Math"]);
            }
            _ => panic!("Expected use decl"),
        }
    }

    #[test]
    fn test_parse_async_fn() {
        let src = "async fn fetch(): Int = 42";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => {
                assert!(f.is_async);
                assert_eq!(f.name, "fetch");
            }
            _ => panic!("Expected async function"),
        }
    }

    #[test]
    fn test_parse_pub_fn() {
        let src = "pub fn add(x: Int, y: Int): Int = x + y";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => {
                assert!(f.is_pub);
                assert_eq!(f.name, "add");
            }
            _ => panic!("Expected pub function"),
        }
    }

    #[test]
    fn test_parse_three_items() {
        let src = "type Foo = { x: Int }\n\nfn make_foo(): Foo = Foo { x: 42 }\n\nfn main() = println(make_foo())";
        let prog = parse_str(src).unwrap();
        assert_eq!(prog.items.len(), 3);
    }

    #[test]
    fn test_parse_recursive_enum() {
        let src = "type Tree =\n  | Leaf(Int)\n  | Node(Tree, Tree)";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::TypeDecl(td) => {
                assert_eq!(td.name, "Tree");
                match &td.body {
                    TypeBody::Enum(variants) => {
                        assert_eq!(variants.len(), 2);
                        assert_eq!(variants[0].name, "Leaf");
                        assert_eq!(variants[1].name, "Node");
                    }
                    _ => panic!("Expected enum"),
                }
            }
            _ => panic!("Expected type decl"),
        }
    }

    #[test]
    fn test_parse_const_declaration() {
        let src = "let MAX: Int = 100";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Const(c) => {
                assert_eq!(c.name, "MAX");
            }
            _ => panic!("Expected constant, got {:?}", prog.items[0]),
        }
    }

    #[test]
    fn test_parse_annotation_on_function() {
        let src = "@[cfg(test)]\nfn test_fn() = 42";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => {
                assert!(!f.annotations.is_empty());
            }
            _ => panic!("Expected annotated function"),
        }
    }

    #[test]
    fn test_parse_error_no_equals() {
        let result = parse_str("fn foo(x: Int) x + 1");
        assert!(result.is_err(), "Missing = should fail");
    }

    #[test]
    fn test_parse_error_unclosed_do() {
        let result = parse_str("fn main() = do\n  let x = 1");
        assert!(result.is_err(), "Missing end should fail");
    }

    #[test]
    fn test_parse_unit_type_parens() {
        let src = "fn noop(): () = ()";
        let _result = parse_str(src); // Just ensure it doesn't crash
    }

    #[test]
    fn test_parse_index_subscript() {
        let src = "fn main() = list[0]";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                // Index access is compiled as a Call or special node
                _ => {} // Just verify it parsed successfully
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_gte_operator() {
        let src = "fn f(a: Int, b: Int) = a >= b";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::BinOp(_, BinOp::Ge, _) => {}
                _ => panic!("Expected >= operator, got {:?}", f.body.kind),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_not_operator() {
        let src = "fn f(x: Bool) = not x";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::UnaryOp(UnaryOp::Not, _) => {}
                _ => panic!("Expected not operator, got {:?}", f.body.kind),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_neg_operator() {
        let src = "fn f(x: Int) = -x";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::UnaryOp(UnaryOp::Neg, _) => {}
                _ => panic!("Expected negate operator, got {:?}", f.body.kind),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_deref_operator() {
        let src = "fn f(x: Int) = *x";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::UnaryOp(UnaryOp::Deref, _) => {}
                _ => panic!("Expected deref operator, got {:?}", f.body.kind),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_ref_operator() {
        let src = "fn f(x: Int) = &x";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::UnaryOp(UnaryOp::Ref, _) => {}
                _ => panic!("Expected ref operator, got {:?}", f.body.kind),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_char_literal() {
        let prog = parse_str("fn main() = 'a'").unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::CharLit('a') => {}
                other => panic!("Expected CharLit('a'), got {:?}", other),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_char_literal_escape() {
        let prog = parse_str(r"fn main() = '\n'").unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::CharLit('\n') => {}
                other => panic!("Expected CharLit('\\n'), got {:?}", other),
            },
            _ => panic!("Expected function"),
        }
    }

    // ── Tests for new parser features ──────────────────────────────

    #[test]
    fn test_parse_where_clause_single() {
        let src = "fn foo<T>(x: T): String where T: Display = x";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => {
                assert_eq!(f.name, "foo");
                assert_eq!(f.where_clauses.len(), 1);
                assert_eq!(f.where_clauses[0].type_name, "T");
                assert_eq!(f.where_clauses[0].bounds, vec!["Display"]);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_where_clause_multiple_bounds() {
        let src = "fn foo<T>(x: T): String where T: Display + Clone = x";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => {
                assert_eq!(f.where_clauses.len(), 1);
                assert_eq!(f.where_clauses[0].bounds, vec!["Display", "Clone"]);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_where_clause_multiple_types() {
        let src = "fn foo<T, U>(x: T, y: U) where T: Display, U: Clone = x";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => {
                assert_eq!(f.where_clauses.len(), 2);
                assert_eq!(f.where_clauses[0].type_name, "T");
                assert_eq!(f.where_clauses[1].type_name, "U");
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_function_no_where_clause() {
        let src = "fn foo(x: Int) = x";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => {
                assert!(f.where_clauses.is_empty());
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_struct_like_variant() {
        let src = "type Shape =\n  | Point\n  | Named { x: Int, y: Int }";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::TypeDecl(td) => match &td.body {
                TypeBody::Enum(variants) => {
                    assert_eq!(variants.len(), 2);
                    assert_eq!(variants[0].name, "Point");
                    assert!(variants[0].named_fields.is_none());
                    assert_eq!(variants[1].name, "Named");
                    let nf = variants[1].named_fields.as_ref().unwrap();
                    assert_eq!(nf.len(), 2);
                    assert_eq!(nf[0].name, "x");
                    assert_eq!(nf[1].name, "y");
                    assert!(!nf[0].is_pub);
                }
                _ => panic!("Expected enum"),
            },
            _ => panic!("Expected type decl"),
        }
    }

    #[test]
    fn test_parse_variant_named_fields_empty_positional() {
        let src = "type Msg =\n  | Hello { name: String }";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::TypeDecl(td) => match &td.body {
                TypeBody::Enum(variants) => {
                    assert_eq!(variants[0].fields.len(), 0);
                    assert!(variants[0].named_fields.is_some());
                }
                _ => panic!("Expected enum"),
            },
            _ => panic!("Expected type decl"),
        }
    }

    #[test]
    fn test_parse_pub_struct_field() {
        let src = "type Config = {\n  pub name: String,\n  secret: Int\n}";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::TypeDecl(td) => match &td.body {
                TypeBody::Struct(fields) => {
                    assert_eq!(fields.len(), 2);
                    assert!(fields[0].is_pub);
                    assert_eq!(fields[0].name, "name");
                    assert!(!fields[1].is_pub);
                    assert_eq!(fields[1].name, "secret");
                }
                _ => panic!("Expected struct"),
            },
            _ => panic!("Expected type decl"),
        }
    }

    #[test]
    fn test_parse_struct_fields_default_not_pub() {
        let src = "type Point = {\n  x: Float,\n  y: Float\n}";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::TypeDecl(td) => match &td.body {
                TypeBody::Struct(fields) => {
                    assert!(!fields[0].is_pub);
                    assert!(!fields[1].is_pub);
                }
                _ => panic!("Expected struct"),
            },
            _ => panic!("Expected type decl"),
        }
    }

    #[test]
    fn test_parse_use_alias() {
        let src = "use Foo as Bar";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::UseDecl(u) => {
                assert_eq!(u.path, vec!["Foo"]);
                assert_eq!(u.alias, Some("Bar".to_string()));
            }
            _ => panic!("Expected use decl"),
        }
    }

    #[test]
    fn test_parse_use_no_alias() {
        let src = "use Foo";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::UseDecl(u) => {
                assert_eq!(u.path, vec!["Foo"]);
                assert!(u.alias.is_none());
            }
            _ => panic!("Expected use decl"),
        }
    }

    #[test]
    fn test_parse_use_selective_has_no_alias() {
        let src = "use Foo::{bar, baz}";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::UseDecl(u) => {
                assert_eq!(u.path, vec!["Foo"]);
                assert!(u.alias.is_none());
                assert_eq!(u.imports, Some(vec!["bar".to_string(), "baz".to_string()]));
            }
            _ => panic!("Expected use decl"),
        }
    }

    #[test]
    fn test_parse_impl_block_has_where_clauses() {
        let src = "type Foo = { x: Int }\n\nimpl Foo\n  fn get_x(self): Int = self.x\nend";
        let prog = parse_str(src).unwrap();
        let has_impl = prog.items.iter().find_map(|item| {
            if let Item::ImplBlock(ib) = item { Some(ib) } else { None }
        });
        let ib = has_impl.expect("Should have impl block");
        assert!(ib.where_clauses.is_empty());
    }

    #[test]
    fn test_parse_loop_expr() {
        let src = "fn main() = loop do\n  break\nend";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Loop(_) => {}
                _ => panic!("Expected loop expression, got {:?}", f.body.kind),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_loop_with_body() {
        let src = "fn main() = loop do\n  let x = 1\n  break\nend";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Loop(body) => match &body.kind {
                    ExprKind::Block(stmts, _) => assert_eq!(stmts.len(), 1),
                    _ => panic!("Expected block in loop body"),
                },
                _ => panic!("Expected loop expression"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_break_with_value() {
        let src = "fn main() = loop do\n  break 42\nend";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::Loop(body) => match &body.kind {
                    ExprKind::BreakValue(val) => match &val.kind {
                        ExprKind::IntLit(42) => {}
                        _ => panic!("Expected IntLit(42)"),
                    },
                    _ => panic!("Expected break-with-value, got {:?}", body.kind),
                },
                _ => panic!("Expected loop expression"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_parse_break_without_value() {
        let src = "fn main() = while true do\n  break\nend";
        let prog = parse_str(src).unwrap();
        match &prog.items[0] {
            Item::Function(f) => match &f.body.kind {
                ExprKind::While(_, body) => match &body.kind {
                    ExprKind::Break => {}
                    _ => panic!("Expected plain break, got {:?}", body.kind),
                },
                _ => panic!("Expected while loop"),
            },
            _ => panic!("Expected function"),
        }
    }
}
