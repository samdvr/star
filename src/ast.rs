use crate::error::Span;

// ── AST Node Types ──────────────────────────────────────────────────────────

/// A type parameter with optional trait bounds, e.g., `T: Ord + Clone`
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParam {
    pub name: String,
    pub bounds: Vec<String>,
}

impl TypeParam {
    pub fn plain(name: String) -> Self {
        Self { name, bounds: vec![] }
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Function(Function),
    TypeDecl(TypeDecl),
    ModuleDecl(ModuleDecl),
    UseDecl(UseDecl),
    ExternFn(ExternFn),
    TraitDecl(TraitDecl),
    ImplBlock(ImplBlock),
    Const(ConstDecl),
    Expr(Expr),
}

// ── Constants ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub value: Expr,
    pub is_pub: bool,
    pub span: Span,
}

// ── Functions ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Expr,
    pub is_pub: bool,
    pub is_async: bool,
    pub type_params: Vec<TypeParam>,
    pub annotations: Vec<String>,  // e.g., ["cfg(target_os = \"linux\")"]
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub destructure: Option<Pattern>, // When present, destructure in param position
    pub span: Span,
}

// ── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TypeDecl {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub body: TypeBody,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeBody {
    Enum(Vec<Variant>),
    Struct(Vec<Field>),
    Alias(TypeExpr),
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeExpr {
    Named(String, Vec<TypeExpr>), // e.g., Int, List<T>, Result<T, E>
    Function(Vec<TypeExpr>, Box<TypeExpr>), // fn(A, B) -> C
    Tuple(Vec<TypeExpr>),
    Ref(Box<TypeExpr>),     // &T
    MutRef(Box<TypeExpr>),  // &mut T
    Move(Box<TypeExpr>),    // ~T
    Dyn(String),            // dyn Trait
    Lifetime(String),       // 'a
}

// ── Expressions ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    // Literals
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    BoolLit(bool),
    ListLit(Vec<Expr>),

    // Variables and paths
    Ident(String),
    FieldAccess(Box<Expr>, String),
    MethodCall(Box<Expr>, String, Vec<Expr>),

    // Operations
    BinOp(Box<Expr>, BinOp, Box<Expr>),
    UnaryOp(UnaryOp, Box<Expr>),
    Pipe(Box<Expr>, Box<Expr>),

    // Function-related
    Call(Box<Expr>, Vec<Expr>),
    Lambda(Vec<Param>, Option<TypeExpr>, Box<Expr>, bool), // params, return_type, body, is_move

    // Control flow
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    Match(Box<Expr>, Vec<MatchArm>),
    Block(Vec<Stmt>, Box<Expr>),

    // Bindings
    Let(Pattern, Option<TypeExpr>, Box<Expr>),

    // Struct construction
    StructLit(String, Vec<(String, Expr)>, Option<Box<Expr>>),

    // Tuple
    Tuple(Vec<Expr>),

    // String interpolation: "hello #{expr} world" becomes StringInterp([Lit("hello "), Expr(e), Lit(" world")])
    StringInterp(Vec<StringPart>),

    // Interop
    RustBlock(String),

    // Error handling
    Try(Box<Expr>), // expr?

    // Concurrency
    Await(Box<Expr>), // expr.await or await expr

    // Loops
    For(Pattern, Box<Expr>, Box<Expr>),    // for pattern in collection do body end
    While(Box<Expr>, Box<Expr>),           // while condition do body end
    Break,
    Continue,
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Band,
    Bor,
    Bxor,
    Shl,
    Shr,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    Ident(String),
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    BoolLit(bool),
    Constructor(String, Vec<Pattern>),
    Tuple(Vec<Pattern>),
    List(Vec<Pattern>, Option<String>), // [a, b | rest]
    Bind(String, Box<Pattern>),         // pattern as name
    Or(Vec<Pattern>),                    // pat1 | pat2 | pat3
    Range(i64, i64),                     // 1..10
}

// ── Statements ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Stmt {
    Let(bool, Pattern, Option<TypeExpr>, Expr), // is_mut, pattern, type, value
    Expr(Expr),
    Assign(String, Expr),
    CompoundAssign(String, BinOp, Expr),         // name += expr, name -= expr, etc.
    IndexAssign(Expr, Expr, Expr),               // collection[index] = value
}

// ── String Interpolation ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum StringPart {
    Lit(String),
    Expr(Expr),
}

// ── Modules ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ModuleDecl {
    pub name: String,
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct UseDecl {
    pub path: Vec<String>,
    pub imports: Option<Vec<String>>, // None = import all, Some = specific names
    pub span: Span,
}

// ── Extern Functions ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ExternFn {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub rust_name: Option<String>, // Optional Rust function path override
    pub span: Span,
}

// ── Traits ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TraitDecl {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub methods: Vec<TraitMethod>,
    pub associated_types: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub default_body: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImplBlock {
    pub trait_name: Option<String>,  // None for inherent impl
    pub type_name: String,
    pub type_params: Vec<TypeParam>,
    pub methods: Vec<Function>,
    pub associated_types: Vec<(String, TypeExpr)>, // type Name = ConcreteType
    pub span: Span,
}
