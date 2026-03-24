use crate::ast::*;

// ── Public API ──────────────────────────────────────────────────────────────

pub fn format(program: &Program, comments: &[(usize, String)]) -> String {
    let mut f = Formatter::new(comments);
    f.format_program(program);
    // Flush any remaining comments (e.g., at end of file)
    f.emit_pending_comments(usize::MAX);
    f.output.trim_end().to_string() + "\n"
}

fn item_span_line(item: &Item) -> usize {
    match item {
        Item::Function(f) => f.span.line,
        Item::TypeDecl(t) => t.span.line,
        Item::ModuleDecl(m) => m.span.line,
        Item::UseDecl(u) => u.span.line,
        Item::ExternFn(e) => e.span.line,
        Item::TraitDecl(t) => t.span.line,
        Item::ImplBlock(i) => i.span.line,
        Item::Const(c) => c.span.line,
        Item::Expr(e) => e.span.line,
    }
}

// ── Formatter State ─────────────────────────────────────────────────────────

struct Formatter<'a> {
    output: String,
    indent: usize,
    comments: &'a [(usize, String)],
    comment_idx: usize,
}

impl<'a> Formatter<'a> {
    fn new(comments: &'a [(usize, String)]) -> Self {
        Self {
            output: String::new(),
            indent: 0,
            comments,
            comment_idx: 0,
        }
    }

    fn emit_pending_comments(&mut self, up_to_line: usize) {
        while self.comment_idx < self.comments.len() && self.comments[self.comment_idx].0 <= up_to_line {
            let (_line, text) = &self.comments[self.comment_idx];
            self.write_indent();
            self.output.push('#');
            self.output.push_str(text);
            self.output.push('\n');
            self.comment_idx += 1;
        }
    }

    fn push(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn pushln(&mut self, s: &str) {
        self.write_indent();
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn newline(&mut self) {
        self.output.push('\n');
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("  ");
        }
    }

    fn indented<F: FnOnce(&mut Self)>(&mut self, f: F) {
        self.indent += 1;
        f(self);
        self.indent -= 1;
    }

    fn format_type_params(&mut self, tps: &[TypeParam]) {
        if !tps.is_empty() {
            self.push("<");
            for (i, tp) in tps.iter().enumerate() {
                if i > 0 {
                    self.push(", ");
                }
                self.push(&tp.name);
                if !tp.bounds.is_empty() {
                    self.push(": ");
                    self.push(&tp.bounds.join(" + "));
                }
            }
            self.push(">");
        }
    }

    // ── Program ─────────────────────────────────────────────────────────

    fn format_program(&mut self, program: &Program) {
        for (i, item) in program.items.iter().enumerate() {
            if i > 0 {
                self.newline();
            }
            self.emit_pending_comments(item_span_line(item));
            self.format_item(item);
        }
    }

    // ── Items ───────────────────────────────────────────────────────────

    fn format_item(&mut self, item: &Item) {
        match item {
            Item::Function(f) => self.format_function(f),
            Item::TypeDecl(t) => self.format_type_decl(t),
            Item::ModuleDecl(m) => self.format_module_decl(m),
            Item::UseDecl(u) => self.format_use_decl(u),
            Item::ExternFn(e) => self.format_extern_fn(e),
            Item::TraitDecl(t) => self.format_trait_decl(t),
            Item::ImplBlock(i) => self.format_impl_block(i),
            Item::Const(c) => self.format_const(c),
            Item::Expr(e) => {
                self.write_indent();
                self.format_expr(e);
                self.newline();
            }
        }
    }

    fn format_function(&mut self, func: &Function) {
        for ann in &func.annotations {
            self.pushln(&format!("@[{ann}]"));
        }
        self.write_indent();
        if func.is_pub {
            self.push("pub ");
        }
        if func.is_async {
            self.push("async ");
        }
        self.push("fn ");
        self.push(&func.name);
        self.format_type_params(&func.type_params);
        self.push("(");
        self.format_params(&func.params);
        self.push(")");
        if let Some(rt) = &func.return_type {
            self.push(": ");
            self.format_type_expr(rt);
        }
        self.push(" =");
        self.format_body(&func.body);
    }

    fn format_body(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Block(stmts, tail) => {
                self.push(" do");
                self.newline();
                self.indented(|f| {
                    for stmt in stmts {
                        f.format_stmt(stmt);
                    }
                    f.write_indent();
                    f.format_expr(tail);
                    f.newline();
                });
                self.write_indent();
                self.pushln("end");
            }
            ExprKind::If(..) | ExprKind::Match(..) | ExprKind::For(..) | ExprKind::While(..) => {
                // These expressions already emit their own `end` keyword,
                // so we just indent them without adding another `end`.
                self.newline();
                self.indented(|f| {
                    f.write_indent();
                    f.format_expr(expr);
                    f.newline();
                });
            }
            _ => {
                self.push(" ");
                self.format_expr(expr);
                self.newline();
            }
        }
    }

    fn format_params(&mut self, params: &[Param]) {
        for (i, p) in params.iter().enumerate() {
            if i > 0 {
                self.push(", ");
            }
            if let Some(pattern) = &p.destructure {
                self.format_pattern(pattern);
            } else {
                self.push(&p.name);
            }
            if let Some(ty) = &p.ty {
                self.push(": ");
                self.format_type_expr(ty);
            }
        }
    }

    fn format_type_decl(&mut self, decl: &TypeDecl) {
        self.write_indent();
        self.push("type ");
        self.push(&decl.name);
        self.format_type_params(&decl.type_params);
        match &decl.body {
            TypeBody::Enum(variants) => {
                self.push(" =");
                self.newline();
                self.indented(|f| {
                    for v in variants {
                        f.write_indent();
                        f.push("| ");
                        f.push(&v.name);
                        if !v.fields.is_empty() {
                            f.push("(");
                            for (i, field) in v.fields.iter().enumerate() {
                                if i > 0 {
                                    f.push(", ");
                                }
                                f.format_type_expr(field);
                            }
                            f.push(")");
                        }
                        f.newline();
                    }
                });
            }
            TypeBody::Struct(fields) => {
                self.push(" = { ");
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.push(&field.name);
                    self.push(": ");
                    self.format_type_expr(&field.ty);
                }
                self.push(" }");
                self.newline();
            }
            TypeBody::Alias(ty) => {
                self.push(" = ");
                self.format_type_expr(ty);
                self.newline();
            }
        }
    }

    fn format_module_decl(&mut self, m: &ModuleDecl) {
        self.write_indent();
        self.push("module ");
        self.push(&m.name);
        self.newline();
        self.indented(|f| {
            for (i, item) in m.items.iter().enumerate() {
                if i > 0 {
                    f.newline();
                }
                f.format_item(item);
            }
        });
        self.pushln("end");
    }

    fn format_use_decl(&mut self, u: &UseDecl) {
        self.write_indent();
        self.push("use ");
        self.push(&u.path.join("::"));
        if let Some(imports) = &u.imports {
            self.push("::{");
            self.push(&imports.join(", "));
            self.push("}");
        }
        self.newline();
    }

    fn format_extern_fn(&mut self, e: &ExternFn) {
        self.write_indent();
        self.push("extern fn ");
        self.push(&e.name);
        self.push("(");
        self.format_params(&e.params);
        self.push(")");
        if let Some(rt) = &e.return_type {
            self.push(": ");
            self.format_type_expr(rt);
        }
        if let Some(rn) = &e.rust_name {
            self.push(" = \"");
            self.push(rn);
            self.push("\"");
        }
        self.newline();
    }

    fn format_trait_decl(&mut self, t: &TraitDecl) {
        self.write_indent();
        self.push("trait ");
        self.push(&t.name);
        self.format_type_params(&t.type_params);
        self.newline();
        self.indented(|f| {
            for assoc_ty in &t.associated_types {
                f.write_indent();
                f.push("type ");
                f.push(assoc_ty);
                f.newline();
            }
            for method in &t.methods {
                f.write_indent();
                f.push("fn ");
                f.push(&method.name);
                f.push("(");
                f.format_params(&method.params);
                f.push(")");
                if let Some(rt) = &method.return_type {
                    f.push(": ");
                    f.format_type_expr(rt);
                }
                if let Some(body) = &method.default_body {
                    f.push(" =");
                    f.format_body(body);
                } else {
                    f.newline();
                }
            }
        });
        self.pushln("end");
    }

    fn format_impl_block(&mut self, imp: &ImplBlock) {
        self.write_indent();
        self.push("impl ");
        if let Some(trait_name) = &imp.trait_name {
            self.push(trait_name);
            self.push(" for ");
        }
        self.push(&imp.type_name);
        self.format_type_params(&imp.type_params);
        self.newline();
        self.indented(|f| {
            for (name, ty) in &imp.associated_types {
                f.write_indent();
                f.push("type ");
                f.push(name);
                f.push(" = ");
                f.format_type_expr(ty);
                f.newline();
            }
            for (i, method) in imp.methods.iter().enumerate() {
                if i > 0 || !imp.associated_types.is_empty() {
                    f.newline();
                }
                f.format_function(method);
            }
        });
        self.pushln("end");
    }

    fn format_const(&mut self, c: &ConstDecl) {
        self.write_indent();
        if c.is_pub {
            self.push("pub ");
        }
        self.push("let ");
        self.push(&c.name);
        if let Some(ty) = &c.ty {
            self.push(": ");
            self.format_type_expr(ty);
        }
        self.push(" = ");
        self.format_expr(&c.value);
        self.newline();
    }

    // ── Type Expressions ────────────────────────────────────────────────

    fn format_type_expr(&mut self, ty: &TypeExpr) {
        match ty {
            TypeExpr::Named(name, params) => {
                self.push(name);
                if !params.is_empty() {
                    self.push("<");
                    for (i, p) in params.iter().enumerate() {
                        if i > 0 {
                            self.push(", ");
                        }
                        self.format_type_expr(p);
                    }
                    self.push(">");
                }
            }
            TypeExpr::Function(params, ret) => {
                self.push("fn(");
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.format_type_expr(p);
                }
                self.push("): ");
                self.format_type_expr(ret);
            }
            TypeExpr::Tuple(parts) => {
                self.push("(");
                for (i, p) in parts.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.format_type_expr(p);
                }
                self.push(")");
            }
            TypeExpr::Ref(inner) => {
                self.push("&");
                self.format_type_expr(inner);
            }
            TypeExpr::MutRef(inner) => {
                self.push("&mut ");
                self.format_type_expr(inner);
            }
            TypeExpr::Move(inner) => {
                self.push("~");
                self.format_type_expr(inner);
            }
            TypeExpr::Dyn(trait_name) => {
                self.push("dyn ");
                self.push(trait_name);
            }
            TypeExpr::Lifetime(name) => {
                self.push("'");
                self.push(name);
            }
        }
    }

    // ── Expressions ─────────────────────────────────────────────────────

    fn format_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::IntLit(n) => {
                self.push(&n.to_string());
            }
            ExprKind::FloatLit(f) => {
                let s = f.to_string();
                self.push(&s);
                // Ensure there's a decimal point
                if !s.contains('.') {
                    self.push(".0");
                }
            }
            ExprKind::StringLit(s) => {
                self.push("\"");
                self.push(&escape_string(s));
                self.push("\"");
            }
            ExprKind::BoolLit(b) => {
                self.push(if *b { "true" } else { "false" });
            }
            ExprKind::ListLit(items) => {
                self.push("[");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.format_expr(item);
                }
                self.push("]");
            }
            ExprKind::Ident(name) => {
                self.push(name);
            }
            ExprKind::FieldAccess(obj, field) => {
                self.format_expr(obj);
                self.push(".");
                self.push(field);
            }
            ExprKind::MethodCall(obj, method, args) => {
                self.format_expr(obj);
                self.push(".");
                self.push(method);
                self.push("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.format_expr(arg);
                }
                self.push(")");
            }
            ExprKind::BinOp(lhs, op, rhs) => {
                self.format_expr(lhs);
                self.push(" ");
                self.push(binop_str(op));
                self.push(" ");
                self.format_expr(rhs);
            }
            ExprKind::UnaryOp(op, operand) => {
                match op {
                    UnaryOp::Neg => self.push("-"),
                    UnaryOp::Not => self.push("not "),
                }
                self.format_expr(operand);
            }
            ExprKind::Pipe(lhs, rhs) => {
                self.format_expr(lhs);
                self.newline();
                self.write_indent();
                self.push("|> ");
                self.format_expr(rhs);
            }
            ExprKind::Call(callee, args) => {
                self.format_expr(callee);
                self.push("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.format_expr(arg);
                }
                self.push(")");
            }
            ExprKind::Lambda(params, return_type, body, is_move) => {
                if *is_move {
                    self.push("move ");
                }
                self.push("fn(");
                self.format_params(params);
                self.push(")");
                if let Some(ret_ty) = return_type {
                    self.push(": ");
                    self.format_type_expr(ret_ty);
                }
                self.push(" => ");
                self.format_expr(body);
            }
            ExprKind::If(cond, then_branch, else_branch) => {
                self.push("if ");
                self.format_expr(cond);
                self.push(" then");
                self.format_block_body(then_branch);
                if let Some(else_expr) = else_branch {
                    // Check if else branch is another if (else-if chain)
                    match &else_expr.kind {
                        ExprKind::If(..) => {
                            self.write_indent();
                            self.push("else ");
                            self.format_expr(else_expr);
                        }
                        _ => {
                            self.write_indent();
                            self.push("else");
                            self.format_block_body(else_expr);
                            self.pushln("end");
                        }
                    }
                } else {
                    self.pushln("end");
                }
            }
            ExprKind::Match(scrutinee, arms) => {
                self.push("match ");
                self.format_expr(scrutinee);
                self.newline();
                self.indented(|f| {
                    for arm in arms {
                        f.write_indent();
                        f.push("| ");
                        f.format_pattern(&arm.pattern);
                        if let Some(guard) = &arm.guard {
                            f.push(" when ");
                            f.format_expr(guard);
                        }
                        f.push(" => ");
                        f.format_expr(&arm.body);
                        f.newline();
                    }
                });
                self.pushln("end");
            }
            ExprKind::Block(stmts, tail) => {
                self.push("do");
                self.newline();
                self.indented(|f| {
                    for stmt in stmts {
                        f.format_stmt(stmt);
                    }
                    f.write_indent();
                    f.format_expr(tail);
                    f.newline();
                });
                self.write_indent();
                self.push("end");
            }
            ExprKind::Let(pattern, ty, value) => {
                self.push("let ");
                self.format_pattern(pattern);
                if let Some(t) = ty {
                    self.push(": ");
                    self.format_type_expr(t);
                }
                self.push(" = ");
                self.format_expr(value);
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
                    self.format_expr(fval);
                }
                if let Some(rest) = spread {
                    if !fields.is_empty() {
                        self.push(", ");
                    }
                    self.push("..");
                    self.format_expr(rest);
                }
                self.push(" }");
            }
            ExprKind::Tuple(items) => {
                self.push("(");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.format_expr(item);
                }
                self.push(")");
            }
            ExprKind::StringInterp(parts) => {
                self.push("\"");
                for part in parts {
                    match part {
                        StringPart::Lit(s) => self.push(&escape_string(s)),
                        StringPart::Expr(e) => {
                            self.push("#{");
                            self.format_expr(e);
                            self.push("}");
                        }
                    }
                }
                self.push("\"");
            }
            ExprKind::RustBlock(code) => {
                self.push("rust! {");
                self.newline();
                // Indent each line of the rust code
                for line in code.lines() {
                    if line.trim().is_empty() {
                        self.newline();
                    } else {
                        self.write_indent();
                        self.push("  ");
                        self.push(line.trim());
                        self.newline();
                    }
                }
                self.write_indent();
                self.push("}");
            }
            ExprKind::Try(inner) => {
                self.format_expr(inner);
                self.push("?");
            }
            ExprKind::Await(inner) => {
                self.format_expr(inner);
                self.push(".await");
            }
            ExprKind::For(pattern, collection, body) => {
                self.push("for ");
                self.format_pattern(pattern);
                self.push(" in ");
                self.format_expr(collection);
                self.push(" do");
                self.format_block_body(body);
                self.pushln("end");
            }
            ExprKind::While(cond, body) => {
                self.push("while ");
                self.format_expr(cond);
                self.push(" do");
                self.format_block_body(body);
                self.pushln("end");
            }
            ExprKind::Break => {
                self.push("break");
            }
            ExprKind::Continue => {
                self.push("continue");
            }
        }
    }

    /// Format the inner body of a block-like construct (if/for/while).
    /// Unwraps Block nodes to avoid double nesting.
    fn format_block_body(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Block(stmts, tail) => {
                self.newline();
                self.indented(|f| {
                    for stmt in stmts {
                        f.format_stmt(stmt);
                    }
                    f.write_indent();
                    f.format_expr(tail);
                    f.newline();
                });
            }
            _ => {
                self.newline();
                self.indented(|f| {
                    f.write_indent();
                    f.format_expr(expr);
                    f.newline();
                });
            }
        }
    }

    // ── Statements ──────────────────────────────────────────────────────

    fn format_stmt(&mut self, stmt: &Stmt) {
        self.write_indent();
        match stmt {
            Stmt::Let(is_mut, pattern, ty, value) => {
                self.push("let ");
                if *is_mut {
                    self.push("mut ");
                }
                self.format_pattern(pattern);
                if let Some(t) = ty {
                    self.push(": ");
                    self.format_type_expr(t);
                }
                self.push(" = ");
                self.format_expr(value);
            }
            Stmt::Expr(e) => {
                self.format_expr(e);
            }
            Stmt::Assign(name, value) => {
                self.push(name);
                self.push(" = ");
                self.format_expr(value);
            }
            Stmt::CompoundAssign(name, op, value) => {
                self.push(name);
                self.push(" ");
                self.push(binop_str(op));
                self.push("= ");
                self.format_expr(value);
            }
            Stmt::IndexAssign(collection, index, value) => {
                self.format_expr(collection);
                self.push("[");
                self.format_expr(index);
                self.push("] = ");
                self.format_expr(value);
            }
        }
        self.newline();
    }

    // ── Patterns ────────────────────────────────────────────────────────

    fn format_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Wildcard => self.push("_"),
            Pattern::Ident(name) => self.push(name),
            Pattern::IntLit(n) => self.push(&n.to_string()),
            Pattern::FloatLit(f) => {
                let s = f.to_string();
                self.push(&s);
                if !s.contains('.') {
                    self.push(".0");
                }
            }
            Pattern::StringLit(s) => {
                self.push("\"");
                self.push(&escape_string(s));
                self.push("\"");
            }
            Pattern::BoolLit(b) => {
                self.push(if *b { "true" } else { "false" });
            }
            Pattern::Constructor(name, fields) => {
                self.push(name);
                if !fields.is_empty() {
                    self.push("(");
                    for (i, f) in fields.iter().enumerate() {
                        if i > 0 {
                            self.push(", ");
                        }
                        self.format_pattern(f);
                    }
                    self.push(")");
                }
            }
            Pattern::Tuple(parts) => {
                self.push("(");
                for (i, p) in parts.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.format_pattern(p);
                }
                self.push(")");
            }
            Pattern::List(items, rest) => {
                self.push("[");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.format_pattern(item);
                }
                if let Some(rest_name) = rest {
                    if !items.is_empty() {
                        self.push(" | ");
                    }
                    self.push(rest_name);
                }
                self.push("]");
            }
            Pattern::Bind(name, inner) => {
                self.format_pattern(inner);
                self.push(" as ");
                self.push(name);
            }
            Pattern::Or(pats) => {
                for (i, pat) in pats.iter().enumerate() {
                    if i > 0 {
                        self.push(" | ");
                    }
                    self.format_pattern(pat);
                }
            }
            Pattern::Range(start, end) => {
                self.push(&start.to_string());
                self.push("..");
                self.push(&end.to_string());
            }
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn binop_str(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::Le => "<=",
        BinOp::Ge => ">=",
        BinOp::And => "and",
        BinOp::Or => "or",
        BinOp::Band => "band",
        BinOp::Bor => "bor",
        BinOp::Bxor => "bxor",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
    }
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Span;

    fn span() -> Span {
        Span::new(0, 0)
    }

    fn mk_expr(kind: ExprKind) -> Expr {
        Expr { kind, span: span() }
    }

    #[test]
    fn test_format_simple_function() {
        let program = Program {
            items: vec![Item::Function(Function {
                name: "add".to_string(),
                params: vec![
                    Param { name: "x".to_string(), ty: Some(TypeExpr::Named("Int".to_string(), vec![])), span: span(), destructure: None },
                    Param { name: "y".to_string(), ty: Some(TypeExpr::Named("Int".to_string(), vec![])), span: span(), destructure: None },
                ],
                return_type: Some(TypeExpr::Named("Int".to_string(), vec![])),
                body: mk_expr(ExprKind::BinOp(
                    Box::new(mk_expr(ExprKind::Ident("x".to_string()))),
                    BinOp::Add,
                    Box::new(mk_expr(ExprKind::Ident("y".to_string()))),
                )),
                is_pub: false,
                is_async: false,
                type_params: vec![],
                annotations: vec![],
                span: span(),
            })],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "fn add(x: Int, y: Int): Int = x + y\n");
    }

    #[test]
    fn test_format_pub_async_function() {
        let program = Program {
            items: vec![Item::Function(Function {
                name: "fetch".to_string(),
                params: vec![
                    Param { name: "url".to_string(), ty: Some(TypeExpr::Named("String".to_string(), vec![])), span: span(), destructure: None },
                ],
                return_type: Some(TypeExpr::Named("String".to_string(), vec![])),
                body: mk_expr(ExprKind::Ident("result".to_string())),
                is_pub: true,
                is_async: true,
                type_params: vec![],
                annotations: vec![],
                span: span(),
            })],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "pub async fn fetch(url: String): String = result\n");
    }

    #[test]
    fn test_format_enum_type() {
        let program = Program {
            items: vec![Item::TypeDecl(TypeDecl {
                name: "Option".to_string(),
                type_params: vec![TypeParam::plain("T".to_string())],
                body: TypeBody::Enum(vec![
                    Variant { name: "Some".to_string(), fields: vec![TypeExpr::Named("T".to_string(), vec![])], span: span() },
                    Variant { name: "None".to_string(), fields: vec![], span: span() },
                ]),
                span: span(),
            })],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "type Option<T> =\n  | Some(T)\n  | None\n");
    }

    #[test]
    fn test_format_struct_type() {
        let program = Program {
            items: vec![Item::TypeDecl(TypeDecl {
                name: "Point".to_string(),
                type_params: vec![],
                body: TypeBody::Struct(vec![
                    Field { name: "x".to_string(), ty: TypeExpr::Named("Int".to_string(), vec![]), span: span() },
                    Field { name: "y".to_string(), ty: TypeExpr::Named("Int".to_string(), vec![]), span: span() },
                ]),
                span: span(),
            })],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "type Point = { x: Int, y: Int }\n");
    }

    #[test]
    fn test_format_type_alias() {
        let program = Program {
            items: vec![Item::TypeDecl(TypeDecl {
                name: "StringList".to_string(),
                type_params: vec![],
                body: TypeBody::Alias(TypeExpr::Named(
                    "List".to_string(),
                    vec![TypeExpr::Named("String".to_string(), vec![])],
                )),
                span: span(),
            })],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "type StringList = List<String>\n");
    }

    #[test]
    fn test_format_if_then_else() {
        let expr = mk_expr(ExprKind::If(
            Box::new(mk_expr(ExprKind::BoolLit(true))),
            Box::new(mk_expr(ExprKind::IntLit(1))),
            Some(Box::new(mk_expr(ExprKind::IntLit(2)))),
        ));
        let program = Program {
            items: vec![Item::Expr(expr)],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "if true then\n  1\nelse\n  2\nend\n");
    }

    #[test]
    fn test_format_lambda() {
        let expr = mk_expr(ExprKind::Lambda(
            vec![Param { name: "x".to_string(), ty: None, span: span(), destructure: None }],
            None,
            Box::new(mk_expr(ExprKind::BinOp(
                Box::new(mk_expr(ExprKind::Ident("x".to_string()))),
                BinOp::Mul,
                Box::new(mk_expr(ExprKind::IntLit(2))),
            ))),
            false,
        ));
        let program = Program {
            items: vec![Item::Expr(expr)],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "fn(x) => x * 2\n");
    }

    #[test]
    fn test_format_match() {
        let expr = mk_expr(ExprKind::Match(
            Box::new(mk_expr(ExprKind::Ident("x".to_string()))),
            vec![
                MatchArm {
                    pattern: Pattern::IntLit(1),
                    guard: None,
                    body: mk_expr(ExprKind::StringLit("one".to_string())),
                    span: span(),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: mk_expr(ExprKind::StringLit("other".to_string())),
                    span: span(),
                },
            ],
        ));
        let program = Program {
            items: vec![Item::Expr(expr)],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "match x\n  | 1 => \"one\"\n  | _ => \"other\"\nend\n");
    }

    #[test]
    fn test_format_for_loop() {
        let expr = mk_expr(ExprKind::For(
            Pattern::Ident("x".to_string()),
            Box::new(mk_expr(ExprKind::Ident("items".to_string()))),
            Box::new(mk_expr(ExprKind::Call(
                Box::new(mk_expr(ExprKind::Ident("println".to_string()))),
                vec![mk_expr(ExprKind::Ident("x".to_string()))],
            ))),
        ));
        let program = Program {
            items: vec![Item::Expr(expr)],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "for x in items do\n  println(x)\nend\n");
    }

    #[test]
    fn test_format_pipe() {
        let expr = mk_expr(ExprKind::Pipe(
            Box::new(mk_expr(ExprKind::Ident("data".to_string()))),
            Box::new(mk_expr(ExprKind::Ident("process".to_string()))),
        ));
        let program = Program {
            items: vec![Item::Expr(expr)],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "data\n|> process\n");
    }

    #[test]
    fn test_format_string_interpolation() {
        let expr = mk_expr(ExprKind::StringInterp(vec![
            StringPart::Lit("hello ".to_string()),
            StringPart::Expr(mk_expr(ExprKind::Ident("name".to_string()))),
            StringPart::Lit("!".to_string()),
        ]));
        let program = Program {
            items: vec![Item::Expr(expr)],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "\"hello #{name}!\"\n");
    }

    #[test]
    fn test_format_use_decl() {
        let program = Program {
            items: vec![Item::UseDecl(UseDecl {
                path: vec!["Foo".to_string(), "Bar".to_string()],
                imports: Some(vec!["baz".to_string(), "qux".to_string()]),
                span: span(),
            })],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "use Foo::Bar::{baz, qux}\n");
    }

    #[test]
    fn test_format_do_block() {
        let expr = mk_expr(ExprKind::Block(
            vec![
                Stmt::Let(false, Pattern::Ident("x".to_string()), None, mk_expr(ExprKind::IntLit(1))),
            ],
            Box::new(mk_expr(ExprKind::Ident("x".to_string()))),
        ));
        let program = Program {
            items: vec![Item::Expr(expr)],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "do\n  let x = 1\n  x\nend\n");
    }

    #[test]
    fn test_format_function_with_block_body() {
        let program = Program {
            items: vec![Item::Function(Function {
                name: "main".to_string(),
                params: vec![],
                return_type: None,
                body: mk_expr(ExprKind::Block(
                    vec![
                        Stmt::Let(false, Pattern::Ident("x".to_string()), None, mk_expr(ExprKind::IntLit(42))),
                    ],
                    Box::new(mk_expr(ExprKind::Call(
                        Box::new(mk_expr(ExprKind::Ident("println".to_string()))),
                        vec![mk_expr(ExprKind::Ident("x".to_string()))],
                    ))),
                )),
                is_pub: false,
                is_async: false,
                type_params: vec![],
                annotations: vec![],
                span: span(),
            })],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "fn main() = do\n  let x = 42\n  println(x)\nend\n");
    }

    #[test]
    fn test_format_extern_fn() {
        let program = Program {
            items: vec![Item::ExternFn(ExternFn {
                name: "write".to_string(),
                params: vec![
                    Param { name: "s".to_string(), ty: Some(TypeExpr::Named("String".to_string(), vec![])), span: span(), destructure: None },
                ],
                return_type: None,
                rust_name: Some("std::io::write".to_string()),
                span: span(),
            })],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "extern fn write(s: String) = \"std::io::write\"\n");
    }

    #[test]
    fn test_format_pattern_constructor() {
        let expr = mk_expr(ExprKind::Match(
            Box::new(mk_expr(ExprKind::Ident("opt".to_string()))),
            vec![
                MatchArm {
                    pattern: Pattern::Constructor("Some".to_string(), vec![Pattern::Ident("v".to_string())]),
                    guard: None,
                    body: mk_expr(ExprKind::Ident("v".to_string())),
                    span: span(),
                },
                MatchArm {
                    pattern: Pattern::Constructor("None".to_string(), vec![]),
                    guard: None,
                    body: mk_expr(ExprKind::IntLit(0)),
                    span: span(),
                },
            ],
        ));
        let program = Program {
            items: vec![Item::Expr(expr)],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "match opt\n  | Some(v) => v\n  | None => 0\nend\n");
    }

    #[test]
    fn test_format_while_loop() {
        let expr = mk_expr(ExprKind::While(
            Box::new(mk_expr(ExprKind::BoolLit(true))),
            Box::new(mk_expr(ExprKind::Break)),
        ));
        let program = Program {
            items: vec![Item::Expr(expr)],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "while true do\n  break\nend\n");
    }

    #[test]
    fn test_format_impl_block() {
        let program = Program {
            items: vec![Item::ImplBlock(ImplBlock {
                trait_name: Some("Display".to_string()),
                type_name: "Point".to_string(),
                type_params: vec![],
                methods: vec![Function {
                    name: "show".to_string(),
                    params: vec![
                        Param { name: "self".to_string(), ty: None, span: span(), destructure: None },
                    ],
                    return_type: Some(TypeExpr::Named("String".to_string(), vec![])),
                    body: mk_expr(ExprKind::StringLit("point".to_string())),
                    is_pub: false,
                    is_async: false,
                    type_params: vec![],
                    annotations: vec![],
                    span: span(),
                }],
                associated_types: vec![],
                span: span(),
            })],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "impl Display for Point\n  fn show(self): String = \"point\"\nend\n");
    }

    #[test]
    fn test_format_struct_lit() {
        let expr = mk_expr(ExprKind::StructLit(
            "Point".to_string(),
            vec![
                ("x".to_string(), mk_expr(ExprKind::IntLit(1))),
                ("y".to_string(), mk_expr(ExprKind::IntLit(2))),
            ],
            None,
        ));
        let program = Program {
            items: vec![Item::Expr(expr)],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "Point { x: 1, y: 2 }\n");
    }

    #[test]
    fn test_format_try_await() {
        let expr = mk_expr(ExprKind::Try(
            Box::new(mk_expr(ExprKind::Await(
                Box::new(mk_expr(ExprKind::Call(
                    Box::new(mk_expr(ExprKind::Ident("fetch".to_string()))),
                    vec![],
                ))),
            ))),
        ));
        let program = Program {
            items: vec![Item::Expr(expr)],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "fetch().await?\n");
    }

    #[test]
    fn test_format_list_pattern() {
        let expr = mk_expr(ExprKind::Match(
            Box::new(mk_expr(ExprKind::Ident("xs".to_string()))),
            vec![
                MatchArm {
                    pattern: Pattern::List(
                        vec![Pattern::Ident("h".to_string())],
                        Some("t".to_string()),
                    ),
                    guard: None,
                    body: mk_expr(ExprKind::Ident("h".to_string())),
                    span: span(),
                },
            ],
        ));
        let program = Program {
            items: vec![Item::Expr(expr)],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "match xs\n  | [h | t] => h\nend\n");
    }

    #[test]
    fn test_format_compound_assign() {
        let program = Program {
            items: vec![Item::Function(Function {
                name: "inc".to_string(),
                params: vec![],
                return_type: None,
                body: mk_expr(ExprKind::Block(
                    vec![
                        Stmt::Let(true, Pattern::Ident("x".to_string()), None, mk_expr(ExprKind::IntLit(0))),
                        Stmt::CompoundAssign("x".to_string(), BinOp::Add, mk_expr(ExprKind::IntLit(1))),
                    ],
                    Box::new(mk_expr(ExprKind::Ident("x".to_string()))),
                )),
                is_pub: false,
                is_async: false,
                type_params: vec![],
                annotations: vec![],
                span: span(),
            })],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "fn inc() = do\n  let mut x = 0\n  x += 1\n  x\nend\n");
    }

    #[test]
    fn test_format_match_with_guard() {
        let expr = mk_expr(ExprKind::Match(
            Box::new(mk_expr(ExprKind::Ident("x".to_string()))),
            vec![
                MatchArm {
                    pattern: Pattern::Ident("n".to_string()),
                    guard: Some(mk_expr(ExprKind::BinOp(
                        Box::new(mk_expr(ExprKind::Ident("n".to_string()))),
                        BinOp::Gt,
                        Box::new(mk_expr(ExprKind::IntLit(0))),
                    ))),
                    body: mk_expr(ExprKind::StringLit("positive".to_string())),
                    span: span(),
                },
            ],
        ));
        let program = Program {
            items: vec![Item::Expr(expr)],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "match x\n  | n when n > 0 => \"positive\"\nend\n");
    }

    #[test]
    fn test_format_module() {
        let program = Program {
            items: vec![Item::ModuleDecl(ModuleDecl {
                name: "Math".to_string(),
                items: vec![Item::Function(Function {
                    name: "double".to_string(),
                    params: vec![
                        Param { name: "x".to_string(), ty: Some(TypeExpr::Named("Int".to_string(), vec![])), span: span(), destructure: None },
                    ],
                    return_type: Some(TypeExpr::Named("Int".to_string(), vec![])),
                    body: mk_expr(ExprKind::BinOp(
                        Box::new(mk_expr(ExprKind::Ident("x".to_string()))),
                        BinOp::Mul,
                        Box::new(mk_expr(ExprKind::IntLit(2))),
                    )),
                    is_pub: true,
                    is_async: false,
                    type_params: vec![],
                    annotations: vec![],
                    span: span(),
                })],
                span: span(),
            })],
        };
        let result = format(&program, &[]);
        assert_eq!(result, "module Math\n  pub fn double(x: Int): Int = x * 2\nend\n");
    }
}
