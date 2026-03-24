use crate::ast::*;
use crate::error::Span;
use std::collections::{HashMap, HashSet};

// ── LSP Analysis Types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function,
    Type,
    Constructor,
    Constant,
    Module,
    Trait,
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
    pub detail: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub errors: Vec<(Span, String)>,
    pub warnings: Vec<(Span, String)>,
    pub type_at: Vec<(Span, String, String)>,  // (span, name, type_string)
    pub definitions: Vec<SymbolInfo>,
    pub builtin_names: Vec<String>,
    pub type_names: Vec<String>,
    pub constructor_names: Vec<String>,
}

fn type_param_names(tps: &[TypeParam]) -> Vec<String> {
    tps.iter().map(|tp| tp.name.clone()).collect()
}

// ── Type Representation ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Type {
    Int,
    Float,
    Bool,
    Str,
    Unit,
    Var(usize),
    List(Box<Type>),
    Tuple(Vec<Type>),
    Function(Vec<Type>, Box<Type>),
    Named(String, Vec<Type>),
    Error,
}

/// A polymorphic type scheme: forall vars . ty
#[derive(Debug, Clone)]
struct Scheme {
    vars: Vec<usize>,
    ty: Type,
}

/// Info about a constructor (enum variant).
#[derive(Debug, Clone)]
struct CtorSig {
    enum_name: String,
    type_params: Vec<String>,
    field_types: Vec<TypeExpr>,
}

/// Info about a struct type.
#[derive(Debug, Clone)]
struct StructDef {
    type_params: Vec<String>,
    fields: Vec<(String, TypeExpr)>,
}

/// Stored type definition — either enum or struct.
#[derive(Debug, Clone)]
enum TypeDef {
    Enum { type_params: Vec<String>, variants: Vec<String> },
    Struct(StructDef),
    Alias { type_params: Vec<String>, target: TypeExpr },
}

// ── Levenshtein Distance ─────────────────────────────────────────────────

fn levenshtein(a: &str, b: &str) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (m, n) = (a.len(), b.len());
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0; n + 1];
    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

// ── Inference Engine ───────────────────────────────────────────────────────

struct Infer {
    next_var: usize,
    substitution: Vec<Option<Type>>,
    types: HashMap<String, TypeDef>,
    constructors: HashMap<String, CtorSig>,
    functions: HashMap<String, Scheme>,
    scopes: Vec<HashMap<String, Type>>,
    builtins: HashMap<String, (usize, usize)>,
    errors: Vec<String>,
    warnings: Vec<(Span, String)>,
    used_vars: HashSet<String>,
    /// Maps variable names to their binding spans (for unused variable warnings)
    var_spans: HashMap<String, Span>,
    /// Tracks which scopes are function-parameter scopes (skip unused warnings)
    fn_scope_depth: HashSet<usize>,
    /// Registered traits: trait_name → vec of (method_name, param_count, has_default_body)
    traits: HashMap<String, Vec<(String, usize, bool)>>,
    /// LSP: recorded (span, name, type) for hover/go-to-definition
    type_at: Vec<(Span, String, String)>,
    /// LSP: structured errors with spans
    structured_errors: Vec<(Span, String)>,
    /// Inferred return types for functions without annotations (fn_name → Type)
    inferred_returns: HashMap<String, Type>,
}

impl Infer {
    fn new() -> Self {
        Self {
            next_var: 0,
            substitution: Vec::new(),
            types: HashMap::new(),
            constructors: HashMap::new(),
            functions: HashMap::new(),
            scopes: vec![HashMap::new()],
            builtins: Self::builtin_arities(),
            errors: Vec::new(),
            warnings: Vec::new(),
            used_vars: HashSet::new(),
            var_spans: HashMap::new(),
            fn_scope_depth: HashSet::new(),
            traits: HashMap::new(),
            type_at: Vec::new(),
            structured_errors: Vec::new(),
            inferred_returns: HashMap::new(),
        }
    }

    fn builtin_arities() -> HashMap<String, (usize, usize)> {
        let entries: Vec<(&str, usize, usize)> = vec![
            // I/O (variadic)
            ("println", 0, 255), ("print", 0, 255), ("eprintln", 0, 255), ("debug", 0, 255),
            ("read_line", 0, 0), ("read_all_stdin", 0, 0),
            // File system
            ("read_file", 1, 1), ("write_file", 2, 2), ("append_file", 2, 2),
            ("file_exists", 1, 1), ("delete_file", 1, 1), ("rename_file", 2, 2),
            ("copy_file", 2, 2), ("file_size", 1, 1), ("read_lines", 1, 1),
            // Directories
            ("list_dir", 1, 1), ("create_dir", 1, 1), ("create_dir_all", 1, 1),
            ("delete_dir", 1, 1), ("dir_exists", 1, 1),
            // Path operations
            ("path_join", 2, 2), ("path_parent", 1, 1), ("path_filename", 1, 1),
            ("path_extension", 1, 1), ("path_stem", 1, 1),
            ("path_is_absolute", 1, 1), ("path_is_relative", 1, 1),
            // Environment & process
            ("env_get", 1, 1), ("env_set", 2, 2), ("env_remove", 1, 1), ("env_vars", 0, 0),
            ("current_dir", 0, 0), ("set_current_dir", 1, 1), ("args", 0, 0),
            ("command", 1, 1), ("command_output", 1, 1), ("command_with_stdin", 2, 2),
            ("command_with_args", 2, 2), ("command_with_args_output", 2, 2),
            ("process_id", 0, 0), ("kill_process", 1, 1),
            // File metadata
            ("is_file", 1, 1), ("is_dir", 1, 1), ("is_symlink", 1, 1),
            ("file_modified", 1, 1), ("file_created", 1, 1),
            ("file_readonly", 1, 1), ("set_readonly", 2, 2),
            ("symlink", 2, 2), ("read_link", 1, 1), ("canonicalize", 1, 1),
            ("temp_dir", 0, 0), ("exe_path", 0, 0),
            // List operations
            ("map", 2, 2), ("filter", 2, 2), ("fold", 3, 3), ("each", 2, 2),
            ("flat_map", 2, 2), ("any", 2, 2), ("all", 2, 2), ("find", 2, 2),
            ("enumerate", 1, 1), ("take", 2, 2), ("drop", 2, 2), ("zip", 2, 2),
            ("flatten", 1, 1), ("reverse", 1, 1), ("sort", 1, 1), ("sort_by", 2, 2),
            ("head", 1, 1), ("tail", 1, 1), ("last", 1, 1), ("init", 1, 1),
            ("push", 2, 2), ("concat", 2, 2), ("dedup", 1, 1),
            ("sum", 1, 1), ("product", 1, 1), ("count", 1, 1),
            ("min_by", 2, 2), ("max_by", 2, 2),
            // Collection algorithms
            ("binary_search", 2, 2), ("position", 2, 2), ("contains_element", 2, 2),
            ("sort_desc", 1, 1), ("sort_by_key", 2, 2), ("is_sorted", 1, 1),
            ("chunks", 2, 2), ("windows", 2, 2), ("nth", 2, 2),
            ("take_while", 2, 2), ("drop_while", 2, 2), ("split_at", 2, 2),
            ("scan", 3, 3), ("reduce", 2, 2), ("partition", 2, 2), ("group_by", 2, 2),
            ("unique", 1, 1), ("intersperse", 2, 2),
            ("min_of", 1, 1), ("max_of", 1, 1), ("sum_float", 1, 1), ("product_float", 1, 1),
            ("unzip", 1, 1), ("zip_with", 3, 3),
            // String operations
            ("to_string", 1, 1), ("trim", 1, 1), ("trim_start", 1, 1), ("trim_end", 1, 1),
            ("split", 2, 2), ("join", 2, 2), ("contains", 2, 2),
            ("replace", 3, 3), ("replace_first", 3, 3),
            ("uppercase", 1, 1), ("lowercase", 1, 1), ("capitalize", 1, 1),
            ("starts_with", 2, 2), ("ends_with", 2, 2), ("chars", 1, 1),
            ("char_at", 2, 2), ("substring", 2, 3),
            ("index_of", 2, 2), ("last_index_of", 2, 2),
            ("pad_left", 2, 3), ("pad_right", 2, 3), ("repeat", 2, 2),
            ("is_empty", 1, 1), ("is_blank", 1, 1),
            ("reverse_string", 1, 1), ("lines", 1, 1), ("words", 1, 1),
            ("strip_prefix", 2, 2), ("strip_suffix", 2, 2),
            ("is_numeric", 1, 1), ("is_alphabetic", 1, 1), ("is_alphanumeric", 1, 1),
            // Regex & encoding
            ("regex_match", 2, 2), ("regex_find", 2, 2),
            ("regex_find_all", 2, 2), ("regex_replace", 3, 3),
            ("bytes", 1, 1), ("from_bytes", 1, 1),
            ("encode_base64", 1, 1), ("decode_base64", 1, 1),
            ("char_code", 1, 1), ("from_char_code", 1, 1),
            ("format", 2, 255),
            // Crypto
            ("sha256", 1, 1), ("sha512", 1, 1), ("md5", 1, 1), ("hash_bytes", 1, 1),
            ("secure_random_bytes", 1, 1), ("secure_random_hex", 1, 1), ("uuid_v4", 0, 0),
            // Ranges
            ("range", 2, 2), ("range_inclusive", 2, 2),
            // Math
            ("abs", 1, 1), ("min", 2, 2), ("max", 2, 2), ("pow", 2, 2),
            ("sqrt", 1, 1), ("clamp", 3, 3),
            ("sin", 1, 1), ("cos", 1, 1), ("tan", 1, 1),
            ("asin", 1, 1), ("acos", 1, 1), ("atan", 1, 1), ("atan2", 2, 2),
            ("floor", 1, 1), ("ceil", 1, 1), ("round", 1, 1), ("truncate", 1, 1),
            ("log", 1, 1), ("log2", 1, 1), ("log10", 1, 1),
            ("exp", 1, 1), ("exp2", 1, 1),
            ("signum", 1, 1), ("hypot", 2, 2), ("cbrt", 1, 1),
            ("pi", 0, 0), ("e_const", 0, 0),
            ("infinity", 0, 0), ("neg_infinity", 0, 0), ("nan", 0, 0),
            ("is_nan", 1, 1), ("is_infinite", 1, 1), ("is_finite", 1, 1),
            ("to_radians", 1, 1), ("to_degrees", 1, 1),
            ("random", 0, 0), ("random_range", 2, 2), ("random_float", 0, 0),
            ("gcd", 2, 2), ("lcm", 2, 2),
            // Date & time
            ("now", 0, 0), ("now_ms", 0, 0), ("now_ns", 0, 0),
            ("monotonic", 0, 0), ("elapsed", 1, 1), ("elapsed_ms", 1, 1),
            ("monotonic_elapsed_ms", 2, 2),
            ("timestamp_secs", 1, 1), ("timestamp_millis", 1, 1),
            ("format_timestamp", 1, 1), ("parse_timestamp", 1, 1),
            ("duration_secs", 1, 1), ("duration_ms", 1, 1),
            ("sleep_secs", 1, 1), ("sleep_millis", 1, 1),
            // Networking
            ("tcp_connect", 1, 1), ("tcp_listen", 1, 1), ("tcp_accept", 1, 1),
            ("tcp_read", 2, 2), ("tcp_write", 2, 2), ("tcp_close", 1, 1),
            ("tcp_read_line", 1, 1), ("tcp_write_line", 2, 2), ("tcp_set_timeout", 2, 2),
            ("udp_bind", 1, 1), ("udp_send_to", 3, 3), ("udp_recv_from", 2, 2),
            ("dns_lookup", 1, 1), ("url_parse", 1, 1),
            ("http_get", 1, 1), ("http", 2, 3), ("http_with_headers", 4, 4),
            // Conversions
            ("to_int", 1, 1), ("to_float", 1, 1), ("length", 1, 1),
            // Process
            ("exit", 1, 1), ("panic", 1, 1),
            // Testing & debugging
            ("assert", 1, 1), ("assert_msg", 2, 2),
            ("assert_eq", 2, 2), ("assert_ne", 2, 2),
            ("log_debug", 1, 1), ("log_info", 1, 1), ("log_warn", 1, 1), ("log_error", 1, 1),
            ("time_fn", 1, 1), ("bench", 2, 2), ("dbg", 1, 1), ("type_name_of", 1, 1),
            ("todo", 0, 0), ("todo_msg", 1, 1), ("unreachable_msg", 1, 1),
            // CLI & args
            ("arg_get", 1, 1), ("arg_count", 0, 0), ("arg_has", 1, 1),
            ("arg_value", 1, 1), ("arg_pairs", 0, 0),
            // JSON
            ("json_get", 2, 2), ("json_object", 1, 1), ("json_array", 1, 1),
            ("json_escape", 1, 1), ("json_parse", 1, 1), ("json_encode", 1, 1),
            // Env file
            ("parse_env_string", 1, 1), ("load_env_file", 1, 1),
            // Colors & styling
            ("color_red", 1, 1), ("color_green", 1, 1), ("color_blue", 1, 1),
            ("color_yellow", 1, 1), ("color_cyan", 1, 1), ("color_magenta", 1, 1),
            ("bold", 1, 1), ("dim", 1, 1), ("underline", 1, 1), ("strip_ansi", 1, 1),
            ("prompt", 1, 1), ("confirm", 1, 1),
            ("clear_screen", 0, 0), ("cursor_up", 1, 1), ("cursor_down", 1, 1),
            // Result/Option
            ("unwrap", 1, 1), ("unwrap_or", 2, 2), ("unwrap_or_else", 2, 2),
            ("expect", 2, 2), ("unwrap_err", 1, 1),
            ("map_result", 2, 2), ("map_option", 2, 2), ("map_err", 2, 2),
            ("and_then", 2, 2), ("or_else", 2, 2), ("map_or", 3, 3), ("or_default", 1, 1),
            ("ok", 1, 1), ("err", 1, 1),
            ("is_ok", 1, 1), ("is_err", 1, 1), ("is_some", 1, 1), ("is_none", 1, 1),
            ("some", 1, 1), ("none", 0, 0),
            ("ok_or", 2, 2), ("ok_or_else", 2, 2),
            ("flatten_result", 1, 1), ("flatten_option", 1, 1), ("transpose", 1, 1),
            // HashMap
            ("map_new", 0, 0), ("map_from_list", 1, 1),
            ("map_insert", 3, 3), ("map_remove", 2, 2), ("map_get", 2, 2),
            ("map_contains_key", 2, 2), ("map_keys", 1, 1), ("map_values", 1, 1),
            ("map_entries", 1, 1), ("map_size", 1, 1), ("map_merge", 2, 2),
            // HashSet
            ("set_new", 0, 0), ("set_from_list", 1, 1),
            ("set_insert", 2, 2), ("set_remove", 2, 2), ("set_contains", 2, 2),
            ("set_union", 2, 2), ("set_intersection", 2, 2), ("set_difference", 2, 2),
            ("set_size", 1, 1), ("set_to_list", 1, 1),
            // Deque
            ("deque_new", 0, 0), ("deque_from_list", 1, 1),
            ("deque_push_back", 2, 2), ("deque_push_front", 2, 2),
            ("deque_pop_back", 1, 1), ("deque_pop_front", 1, 1),
            ("deque_size", 1, 1), ("deque_to_list", 1, 1),
            // Heap
            ("heap_new", 0, 0), ("heap_from_list", 1, 1),
            ("heap_push", 2, 2), ("heap_pop", 1, 1), ("heap_peek", 1, 1),
            ("heap_size", 1, 1), ("heap_to_list", 1, 1),
            // Concurrency
            ("spawn", 1, 1), ("spawn_join", 1, 1),
            ("channel", 0, 0), ("send", 2, 2), ("recv", 1, 1), ("try_recv", 1, 1),
            ("mutex_new", 1, 1), ("mutex_lock", 1, 1),
            ("rwlock_new", 1, 1), ("rwlock_read", 1, 1), ("rwlock_write", 1, 1),
            ("atomic_new", 1, 1), ("atomic_get", 1, 1), ("atomic_set", 2, 2), ("atomic_add", 2, 2),
            ("sleep", 1, 1), ("sleep_ms", 1, 1), ("timeout", 2, 2),
            ("spawn_async", 1, 1), ("spawn_blocking", 1, 1), ("parallel_map", 2, 2),
            // Extra builtins from typeck that aren't in codegen match but are registered
            ("len", 1, 1), ("type_of", 1, 1),
            ("replace_all", 3, 3), ("trim_matches", 2, 2),
            ("map_len", 1, 1), ("map_is_empty", 1, 1), ("map_from_entries", 1, 1),
            ("set_len", 1, 1), ("set_is_empty", 1, 1),
            ("deque_len", 1, 1), ("deque_is_empty", 1, 1),
            ("heap_len", 1, 1), ("heap_is_empty", 1, 1),
        ];
        entries.into_iter().map(|(name, min, max)| (name.to_string(), (min, max))).collect()
    }

    // ── Fresh variables ────────────────────────────────────────────────────

    fn fresh_var(&mut self) -> Type {
        let id = self.next_var;
        self.next_var += 1;
        self.substitution.push(None);
        Type::Var(id)
    }

    // ── Substitution application ───────────────────────────────────────────

    fn apply(&self, ty: &Type) -> Type {
        match ty {
            Type::Var(id) => {
                if let Some(Some(resolved)) = self.substitution.get(*id) {
                    self.apply(resolved)
                } else {
                    ty.clone()
                }
            }
            Type::List(inner) => Type::List(Box::new(self.apply(inner))),
            Type::Tuple(elems) => Type::Tuple(elems.iter().map(|t| self.apply(t)).collect()),
            Type::Function(params, ret) => Type::Function(
                params.iter().map(|t| self.apply(t)).collect(),
                Box::new(self.apply(ret)),
            ),
            Type::Named(name, args) => {
                Type::Named(name.clone(), args.iter().map(|t| self.apply(t)).collect())
            }
            _ => ty.clone(),
        }
    }

    // ── Occurs check ───────────────────────────────────────────────────────

    fn occurs(&self, var: usize, ty: &Type) -> bool {
        match ty {
            Type::Var(id) => {
                if *id == var {
                    return true;
                }
                if let Some(Some(resolved)) = self.substitution.get(*id) {
                    self.occurs(var, resolved)
                } else {
                    false
                }
            }
            Type::List(inner) => self.occurs(var, inner),
            Type::Tuple(elems) => elems.iter().any(|t| self.occurs(var, t)),
            Type::Function(params, ret) => {
                params.iter().any(|t| self.occurs(var, t)) || self.occurs(var, ret)
            }
            Type::Named(_, args) => args.iter().any(|t| self.occurs(var, t)),
            _ => false,
        }
    }

    // ── Unification ────────────────────────────────────────────────────────

    fn unify(&mut self, a: &Type, b: &Type) -> bool {
        let a = self.apply(a);
        let b = self.apply(b);

        if a == b {
            return true;
        }

        match (&a, &b) {
            (Type::Error, _) | (_, Type::Error) => true,

            (Type::Var(id), _) => {
                if self.occurs(*id, &b) {
                    // Occurs check failure — just bind anyway (lenient)
                    self.substitution[*id] = Some(Type::Error);
                    true
                } else {
                    self.substitution[*id] = Some(b);
                    true
                }
            }
            (_, Type::Var(id)) => {
                if self.occurs(*id, &a) {
                    self.substitution[*id] = Some(Type::Error);
                    true
                } else {
                    self.substitution[*id] = Some(a);
                    true
                }
            }

            (Type::List(a_inner), Type::List(b_inner)) => self.unify(a_inner, b_inner),

            (Type::Tuple(as_), Type::Tuple(bs)) => {
                if as_.len() != bs.len() {
                    return false;
                }
                for (a, b) in as_.iter().zip(bs.iter()) {
                    if !self.unify(a, b) {
                        return false;
                    }
                }
                true
            }

            (Type::Function(ap, ar), Type::Function(bp, br)) => {
                if ap.len() != bp.len() {
                    return false;
                }
                for (a, b) in ap.iter().zip(bp.iter()) {
                    if !self.unify(a, b) {
                        return false;
                    }
                }
                self.unify(ar, br)
            }

            (Type::Named(an, aa), Type::Named(bn, ba)) => {
                if an != bn || aa.len() != ba.len() {
                    return false;
                }
                for (a, b) in aa.iter().zip(ba.iter()) {
                    if !self.unify(a, b) {
                        return false;
                    }
                }
                true
            }

            // Int and Float are both "numeric" — don't unify them with each other
            _ => false,
        }
    }

    // ── Scope management ───────────────────────────────────────────────────

    /// Record a (span, name, type) entry for LSP hover/go-to-definition.
    fn record_type(&mut self, span: Span, name: &str, ty: &Type) {
        let display = self.display_type(ty);
        self.type_at.push((span, name.to_string(), display));
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        let depth = self.scopes.len();
        let is_fn_scope = self.fn_scope_depth.remove(&depth);
        if let Some(scope) = self.scopes.last() {
            if !is_fn_scope {
                for name in scope.keys() {
                    if !name.starts_with('_') && !self.used_vars.contains(name) {
                        let span = self.var_spans.get(name).copied().unwrap_or(Span::new(0, 0));
                        self.warnings.push((span, format!(
                            "unused variable `{}`", name
                        )));
                    }
                }
            }
            // Clean up used_vars for this scope's bindings
            for name in scope.keys() {
                self.used_vars.remove(name);
            }
        }
        self.scopes.pop();
    }

    fn bind_var(&mut self, name: &str, ty: Type, span: Span) {
        self.scopes.last_mut().unwrap().insert(name.to_string(), ty);
        self.var_spans.insert(name.to_string(), span);
    }

    fn lookup_var(&mut self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                self.used_vars.insert(name.to_string());
                return Some(ty.clone());
            }
        }
        None
    }

    // ── Generalization / Instantiation ─────────────────────────────────────

    fn free_vars(&self, ty: &Type) -> HashSet<usize> {
        match ty {
            Type::Var(id) => {
                if let Some(Some(resolved)) = self.substitution.get(*id) {
                    self.free_vars(resolved)
                } else {
                    let mut s = HashSet::new();
                    s.insert(*id);
                    s
                }
            }
            Type::List(inner) => self.free_vars(inner),
            Type::Tuple(elems) => {
                let mut s = HashSet::new();
                for e in elems {
                    s.extend(self.free_vars(e));
                }
                s
            }
            Type::Function(params, ret) => {
                let mut s = HashSet::new();
                for p in params {
                    s.extend(self.free_vars(p));
                }
                s.extend(self.free_vars(ret));
                s
            }
            Type::Named(_, args) => {
                let mut s = HashSet::new();
                for a in args {
                    s.extend(self.free_vars(a));
                }
                s
            }
            _ => HashSet::new(),
        }
    }

    fn env_free_vars(&self) -> HashSet<usize> {
        let mut s = HashSet::new();
        for scope in &self.scopes {
            for ty in scope.values() {
                s.extend(self.free_vars(ty));
            }
        }
        s
    }

    fn generalize(&self, ty: &Type) -> Scheme {
        let applied = self.apply(ty);
        let ty_fv = self.free_vars(&applied);
        let env_fv = self.env_free_vars();
        let vars: Vec<usize> = ty_fv.difference(&env_fv).copied().collect();
        Scheme { vars, ty: applied }
    }

    fn instantiate(&mut self, scheme: &Scheme) -> Type {
        let mut mapping = HashMap::new();
        for &v in &scheme.vars {
            mapping.insert(v, self.fresh_var());
        }
        self.substitute_scheme(&scheme.ty, &mapping)
    }

    fn substitute_scheme(&self, ty: &Type, mapping: &HashMap<usize, Type>) -> Type {
        match ty {
            Type::Var(id) => {
                if let Some(replacement) = mapping.get(id) {
                    replacement.clone()
                } else if let Some(Some(resolved)) = self.substitution.get(*id) {
                    self.substitute_scheme(resolved, mapping)
                } else {
                    ty.clone()
                }
            }
            Type::List(inner) => {
                Type::List(Box::new(self.substitute_scheme(inner, mapping)))
            }
            Type::Tuple(elems) => {
                Type::Tuple(elems.iter().map(|t| self.substitute_scheme(t, mapping)).collect())
            }
            Type::Function(params, ret) => Type::Function(
                params.iter().map(|t| self.substitute_scheme(t, mapping)).collect(),
                Box::new(self.substitute_scheme(ret, mapping)),
            ),
            Type::Named(name, args) => Type::Named(
                name.clone(),
                args.iter().map(|t| self.substitute_scheme(t, mapping)).collect(),
            ),
            _ => ty.clone(),
        }
    }

    // ── TypeExpr → Type conversion ─────────────────────────────────────────

    fn resolve_type_expr(&mut self, ty: &TypeExpr, tparams: &HashMap<String, Type>) -> Type {
        match ty {
            TypeExpr::Named(name, args) => {
                // Check type parameter mapping first
                if args.is_empty() {
                    if let Some(t) = tparams.get(name) {
                        return t.clone();
                    }
                }
                match name.as_str() {
                    "Int" | "Int8" | "Int16" | "Int32"
                    | "UInt" | "UInt8" | "UInt16" | "UInt32" => Type::Int,
                    "Float" | "Float32" => Type::Float,
                    "Bool" => Type::Bool,
                    "String" => Type::Str,
                    "Unit" | "()" => Type::Unit,
                    "List" if args.len() == 1 => {
                        Type::List(Box::new(self.resolve_type_expr(&args[0], tparams)))
                    }
                    _ => {
                        let resolved_args: Vec<Type> =
                            args.iter().map(|a| self.resolve_type_expr(a, tparams)).collect();
                        Type::Named(name.clone(), resolved_args)
                    }
                }
            }
            TypeExpr::Function(params, ret) => {
                let ps: Vec<Type> = params
                    .iter()
                    .map(|p| self.resolve_type_expr(p, tparams))
                    .collect();
                Type::Function(ps, Box::new(self.resolve_type_expr(ret, tparams)))
            }
            TypeExpr::Tuple(types) => {
                if types.is_empty() {
                    Type::Unit
                } else {
                    Type::Tuple(
                        types.iter().map(|t| self.resolve_type_expr(t, tparams)).collect(),
                    )
                }
            }
            TypeExpr::Ref(inner) | TypeExpr::MutRef(inner) | TypeExpr::Move(inner) => {
                self.resolve_type_expr(inner, tparams)
            }
            TypeExpr::Dyn(trait_name) => {
                Type::Named(format!("dyn {}", trait_name), vec![])
            }
            TypeExpr::Lifetime(_) => {
                // Lifetimes are pass-through in type checking — they only matter in codegen
                Type::Unit
            }
        }
    }

    fn resolve_type_expr_simple(&mut self, ty: &TypeExpr) -> Type {
        self.resolve_type_expr(ty, &HashMap::new())
    }

    // ── Display ────────────────────────────────────────────────────────────

    fn display_type(&self, ty: &Type) -> String {
        let ty = self.apply(ty);
        match &ty {
            Type::Int => "Int".to_string(),
            Type::Float => "Float".to_string(),
            Type::Bool => "Bool".to_string(),
            Type::Str => "String".to_string(),
            Type::Unit => "Unit".to_string(),
            Type::Var(id) => format!("?{}", id),
            Type::List(inner) => format!("List<{}>", self.display_type(inner)),
            Type::Tuple(elems) => {
                let parts: Vec<String> = elems.iter().map(|t| self.display_type(t)).collect();
                format!("({})", parts.join(", "))
            }
            Type::Function(params, ret) => {
                let parts: Vec<String> = params.iter().map(|t| self.display_type(t)).collect();
                format!("fn({}) -> {}", parts.join(", "), self.display_type(ret))
            }
            Type::Named(name, args) => {
                if args.is_empty() {
                    name.clone()
                } else {
                    let parts: Vec<String> = args.iter().map(|t| self.display_type(t)).collect();
                    format!("{}<{}>", name, parts.join(", "))
                }
            }
            Type::Error => "Error".to_string(),
        }
    }

    /// Convert an internal Type to an AST TypeExpr (for filling in inferred return types).
    fn type_to_type_expr(&self, ty: &Type) -> TypeExpr {
        let ty = self.apply(ty);
        match &ty {
            Type::Int => TypeExpr::Named("Int".to_string(), vec![]),
            Type::Float => TypeExpr::Named("Float".to_string(), vec![]),
            Type::Bool => TypeExpr::Named("Bool".to_string(), vec![]),
            Type::Str => TypeExpr::Named("String".to_string(), vec![]),
            Type::Unit => TypeExpr::Named("Unit".to_string(), vec![]),
            Type::List(inner) => {
                TypeExpr::Named("List".to_string(), vec![self.type_to_type_expr(inner)])
            }
            Type::Tuple(elems) => {
                TypeExpr::Tuple(elems.iter().map(|t| self.type_to_type_expr(t)).collect())
            }
            Type::Function(params, ret) => {
                let ps: Vec<TypeExpr> = params.iter().map(|t| self.type_to_type_expr(t)).collect();
                TypeExpr::Function(ps, Box::new(self.type_to_type_expr(ret)))
            }
            Type::Named(name, args) => {
                let as_: Vec<TypeExpr> = args.iter().map(|t| self.type_to_type_expr(t)).collect();
                TypeExpr::Named(name.clone(), as_)
            }
            Type::Var(_) | Type::Error => {
                // Unresolved type variable — fall back to a generic marker
                TypeExpr::Named("_".to_string(), vec![])
            }
        }
    }

    /// Check if a type contains any unresolved type variables.
    fn has_unresolved_vars(&self, ty: &Type) -> bool {
        match ty {
            Type::Var(_) => true,
            Type::List(inner) => self.has_unresolved_vars(inner),
            Type::Tuple(elems) => elems.iter().any(|t| self.has_unresolved_vars(t)),
            Type::Function(params, ret) => {
                params.iter().any(|t| self.has_unresolved_vars(t))
                    || self.has_unresolved_vars(ret)
            }
            Type::Named(_, args) => args.iter().any(|t| self.has_unresolved_vars(t)),
            _ => false,
        }
    }

    /// Infer the return type of a builtin when used in a pipe expression.
    /// `left_ty` is the type flowing into the pipe, `extra_arg_types` are any extra args.
    fn infer_builtin_pipe_return(
        &mut self,
        name: &str,
        left_ty: &Type,
        extra_arg_types: &[Type],
    ) -> Option<Type> {
        let elem_ty = match left_ty {
            Type::List(inner) => Some(inner.as_ref().clone()),
            _ => None,
        };

        match name {
            // List -> List operations (preserve or transform element type)
            "map" => {
                // map(fn(A) -> B) returns List<B>
                if let Some(fn_ty) = extra_arg_types.first() {
                    let resolved = self.apply(fn_ty);
                    if let Type::Function(_, ret) = &resolved {
                        return Some(Type::List(ret.clone()));
                    }
                }
                // If we know the element type, return List<fresh>
                if elem_ty.is_some() {
                    return Some(Type::List(Box::new(self.fresh_var())));
                }
                None
            }
            "flat_map" => {
                // flat_map(fn(A) -> List<B>) returns List<B>
                if let Some(fn_ty) = extra_arg_types.first() {
                    let resolved = self.apply(fn_ty);
                    if let Type::Function(_, ret) = &resolved {
                        let resolved_ret = self.apply(ret);
                        // If callback returns List<B>, result is List<B>
                        if let Type::List(_) = &resolved_ret {
                            return Some(resolved_ret);
                        }
                        // Otherwise treat like map: wrap in List
                        return Some(Type::List(ret.clone()));
                    }
                }
                if elem_ty.is_some() {
                    return Some(Type::List(Box::new(self.fresh_var())));
                }
                None
            }
            "filter" | "take" | "drop" | "reverse" | "sort" | "sort_by" | "dedup"
            | "unique" | "take_while" | "drop_while" | "sort_desc" | "sort_by_key"
            | "tail" | "init" => {
                // These return List<same element type>
                elem_ty.map(|e| Type::List(Box::new(e)))
            }
            "flatten" => {
                // List<List<A>> → List<A>
                if let Some(Type::List(inner)) = &elem_ty {
                    return Some(Type::List(inner.clone()));
                }
                elem_ty.map(|e| Type::List(Box::new(e)))
            }
            "zip" => {
                // zip(other_list) returns List<(A, B)>
                if let (Some(a), Some(b_list)) = (&elem_ty, extra_arg_types.first()) {
                    let b_resolved = self.apply(b_list);
                    if let Type::List(b_elem) = b_resolved {
                        return Some(Type::List(Box::new(Type::Tuple(vec![
                            a.clone(),
                            *b_elem,
                        ]))));
                    }
                }
                None
            }
            "enumerate" => {
                // List<A> -> List<(Int, A)>
                elem_ty.map(|e| {
                    Type::List(Box::new(Type::Tuple(vec![Type::Int, e])))
                })
            }
            "fold" | "reduce" => {
                // fold(init, fn) returns the accumulator type
                if let Some(init_ty) = extra_arg_types.first() {
                    return Some(self.apply(init_ty));
                }
                elem_ty
            }
            "find" => {
                // find(predicate) returns Option<A>
                elem_ty.map(|e| Type::Named("Option".to_string(), vec![e]))
            }
            "any" | "all" => Some(Type::Bool),
            "count" => Some(Type::Int),
            "sum" | "product" => elem_ty,
            "sum_float" | "product_float" => Some(Type::Float),
            "head" | "last" => elem_ty,
            "join" => Some(Type::Str),
            "each" => Some(Type::Unit),
            "concat" => Some(left_ty.clone()),
            "chunks" | "windows" => {
                // Returns List<List<A>>
                elem_ty.map(|e| Type::List(Box::new(Type::List(Box::new(e)))))
            }
            "partition" => {
                // Returns (List<A>, List<A>)
                elem_ty.map(|e| {
                    Type::Tuple(vec![
                        Type::List(Box::new(e.clone())),
                        Type::List(Box::new(e)),
                    ])
                })
            }
            "group_by" => {
                // Returns Map<K, List<A>> — approximate as Named
                None // too complex to infer without knowing key type
            }
            "position" | "binary_search" => Some(Type::Named("Option".to_string(), vec![Type::Int])),
            "contains_element" | "is_sorted" => Some(Type::Bool),
            "nth" => elem_ty.map(|e| Type::Named("Option".to_string(), vec![e])),
            "split_at" => {
                elem_ty.map(|e| {
                    Type::Tuple(vec![
                        Type::List(Box::new(e.clone())),
                        Type::List(Box::new(e)),
                    ])
                })
            }
            "scan" => {
                if let Some(init_ty) = extra_arg_types.first() {
                    let resolved = self.apply(init_ty);
                    return Some(Type::List(Box::new(resolved)));
                }
                None
            }
            "unzip" => None, // complex
            "intersperse" => elem_ty.map(|e| Type::List(Box::new(e))),
            "min_of" | "max_of" | "min_by" | "max_by" => {
                elem_ty.map(|e| Type::Named("Option".to_string(), vec![e]))
            }
            "zip_with" => {
                // zip_with(other, fn) — return type is List<C> where fn(A,B)->C
                if extra_arg_types.len() >= 2 {
                    let fn_ty = self.apply(&extra_arg_types[1]);
                    if let Type::Function(_, ret) = fn_ty {
                        return Some(Type::List(ret));
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Find the closest known name to `name` (scope vars, functions, constructors, builtins).
    /// Returns Some(suggestion) if Levenshtein distance ≤ 3.
    fn suggest_name(&self, name: &str) -> Option<String> {
        let mut best: Option<(usize, String)> = None;
        let mut consider = |candidate: &str| {
            let d = levenshtein(name, candidate);
            if d > 0 && d <= 3 {
                if best.as_ref().map_or(true, |(bd, _)| d < *bd) {
                    best = Some((d, candidate.to_string()));
                }
            }
        };
        for scope in &self.scopes {
            for k in scope.keys() { consider(k); }
        }
        for k in self.functions.keys() { consider(k); }
        for k in self.constructors.keys() { consider(k); }
        for k in self.builtins.keys() { consider(k); }
        best.map(|(_, s)| s)
    }

    // ── Registration Phase ─────────────────────────────────────────────────

    fn register_program(&mut self, program: &Program) {
        for item in &program.items {
            self.register_item(item);
        }
    }

    fn register_item(&mut self, item: &Item) {
        self.register_item_in_module(item, false);
    }

    fn register_item_in_module(&mut self, item: &Item, in_module: bool) {
        match item {
            Item::TypeDecl(td) => self.register_type_decl(td),
            Item::Function(f) => {
                // In a module, only register pub functions
                if in_module && !f.is_pub {
                    return;
                }
                self.register_function(f);
            }
            Item::ModuleDecl(m) => {
                for item in &m.items {
                    self.register_item_in_module(item, true);
                }
            }
            Item::UseDecl(_) | Item::Expr(_) | Item::ExternFn(_) => {}
            Item::Const(c) => {
                let ty = c.ty.as_ref()
                    .map(|t| self.resolve_type_expr_simple(t))
                    .unwrap_or_else(|| self.fresh_var());
                self.bind_var(&c.name, ty, c.span);
            }
            Item::TraitDecl(t) => {
                let methods: Vec<(String, usize, bool)> = t.methods.iter().map(|m| {
                    (m.name.clone(), m.params.len(), m.default_body.is_some())
                }).collect();
                self.traits.insert(t.name.clone(), methods);
            }
            Item::ImplBlock(imp) => {
                for method in &imp.methods {
                    self.register_function(method);
                }
            }
        }
    }

    fn register_type_decl(&mut self, td: &TypeDecl) {
        let tp_names = type_param_names(&td.type_params);
        match &td.body {
            TypeBody::Enum(variants) => {
                let variant_names: Vec<String> =
                    variants.iter().map(|v| v.name.clone()).collect();
                self.types.insert(
                    td.name.clone(),
                    TypeDef::Enum {
                        type_params: tp_names.clone(),
                        variants: variant_names,
                    },
                );
                for variant in variants {
                    self.constructors.insert(
                        variant.name.clone(),
                        CtorSig {
                            enum_name: td.name.clone(),
                            type_params: tp_names.clone(),
                            field_types: variant.fields.clone(),
                        },
                    );
                }
            }
            TypeBody::Struct(fields) => {
                self.types.insert(
                    td.name.clone(),
                    TypeDef::Struct(StructDef {
                        type_params: tp_names,
                        fields: fields
                            .iter()
                            .map(|f| (f.name.clone(), f.ty.clone()))
                            .collect(),
                    }),
                );
            }
            TypeBody::Alias(target) => {
                self.types.insert(
                    td.name.clone(),
                    TypeDef::Alias {
                        type_params: tp_names,
                        target: target.clone(),
                    },
                );
            }
        }
    }

    fn register_function(&mut self, f: &Function) {
        // Build a type parameter mapping with fresh vars for generalization
        let mut tparams = HashMap::new();
        let mut scheme_vars = Vec::new();
        for tp in &f.type_params {
            let var = self.fresh_var();
            if let Type::Var(id) = var {
                scheme_vars.push(id);
                tparams.insert(tp.name.clone(), var);
            }
        }

        let param_types: Vec<Type> = f
            .params
            .iter()
            .map(|p| {
                p.ty.as_ref()
                    .map(|t| self.resolve_type_expr(t, &tparams))
                    .unwrap_or_else(|| self.fresh_var())
            })
            .collect();

        let ret_type = f
            .return_type
            .as_ref()
            .map(|t| self.resolve_type_expr(t, &tparams))
            .unwrap_or_else(|| self.fresh_var());

        let fn_type = Type::Function(param_types, Box::new(ret_type));

        // Collect all free vars generated for this function signature as scheme vars
        let all_fv = self.free_vars(&fn_type);
        let vars: Vec<usize> = all_fv.into_iter().collect();

        self.functions.insert(f.name.clone(), Scheme { vars, ty: fn_type });
    }

    // ── Inference: Expressions ─────────────────────────────────────────────

    fn infer_expr(&mut self, expr: &Expr) -> Type {
        match &expr.kind {
            ExprKind::IntLit(_) => Type::Int,
            ExprKind::FloatLit(_) => Type::Float,
            ExprKind::StringLit(_) => Type::Str,
            ExprKind::BoolLit(_) => Type::Bool,

            ExprKind::StringInterp(parts) => {
                for part in parts {
                    if let StringPart::Expr(e) = part {
                        self.infer_expr(e);
                    }
                }
                Type::Str
            }

            ExprKind::ListLit(elems) => {
                let elem_ty = self.fresh_var();
                for e in elems {
                    let t = self.infer_expr(e);
                    if !self.unify(&elem_ty, &t) {
                        self.errors.push(format!(
                            "{} List element type mismatch: expected {}, found {}",
                            expr.span,
                            self.display_type(&elem_ty),
                            self.display_type(&t),
                        ));
                    }
                }
                Type::List(Box::new(elem_ty))
            }

            ExprKind::Tuple(elems) => {
                let types: Vec<Type> = elems.iter().map(|e| self.infer_expr(e)).collect();
                Type::Tuple(types)
            }

            ExprKind::Ident(name) => {
                // Check local scope first
                if let Some(ty) = self.lookup_var(name) {
                    self.record_type(expr.span, name, &ty);
                    return ty;
                }
                // Check registered functions
                if let Some(scheme) = self.functions.get(name).cloned() {
                    let ty = self.instantiate(&scheme);
                    self.record_type(expr.span, name, &ty);
                    return ty;
                }
                // Check constructors (nullary: no fields means it's a value)
                if let Some(ctor) = self.constructors.get(name).cloned() {
                    if ctor.field_types.is_empty() {
                        let ty = self.instantiate_ctor(&ctor);
                        self.record_type(expr.span, name, &ty);
                        return ty;
                    } else {
                        // Constructor used as a function
                        let ty = self.ctor_as_function(&ctor);
                        self.record_type(expr.span, name, &ty);
                        return ty;
                    }
                }
                // Check if it's a known builtin
                if self.builtins.contains_key(name) {
                    return self.fresh_var();
                }
                // Unknown identifier — report error but continue with fresh var
                let mut msg = format!("{} Unknown identifier '{}'", expr.span, name);
                if let Some(suggestion) = self.suggest_name(name) {
                    msg.push_str(&format!("; did you mean '{}'?", suggestion));
                }
                self.errors.push(msg);
                self.structured_errors.push((expr.span, format!("Unknown identifier '{}'{}",
                    name,
                    self.suggest_name(name).map(|s| format!("; did you mean '{}'?", s)).unwrap_or_default()
                )));
                self.fresh_var()
            }

            ExprKind::BinOp(left, op, right) => {
                let lt = self.infer_expr(left);
                let rt = self.infer_expr(right);
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
                    | BinOp::Band | BinOp::Bor | BinOp::Bxor | BinOp::Shl | BinOp::Shr => {
                        // Both operands should be same numeric type
                        if !self.unify(&lt, &rt) {
                            self.errors.push(format!(
                                "{} Arithmetic operand type mismatch: {} vs {}",
                                expr.span,
                                self.display_type(&lt),
                                self.display_type(&rt),
                            ));
                        }
                        lt
                    }
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                        if !self.unify(&lt, &rt) {
                            self.errors.push(format!(
                                "{} Comparison operand type mismatch: {} vs {}",
                                expr.span,
                                self.display_type(&lt),
                                self.display_type(&rt),
                            ));
                        }
                        Type::Bool
                    }
                    BinOp::And | BinOp::Or => {
                        if !self.unify(&lt, &Type::Bool) {
                            self.errors.push(format!(
                                "{} Logical operator expects Bool, found {}",
                                expr.span,
                                self.display_type(&lt),
                            ));
                        }
                        if !self.unify(&rt, &Type::Bool) {
                            self.errors.push(format!(
                                "{} Logical operator expects Bool, found {}",
                                expr.span,
                                self.display_type(&rt),
                            ));
                        }
                        Type::Bool
                    }
                }
            }

            ExprKind::UnaryOp(op, inner) => {
                let t = self.infer_expr(inner);
                match op {
                    UnaryOp::Neg => {
                        // Should be numeric — we allow it and trust rustc
                        t
                    }
                    UnaryOp::Not => {
                        if !self.unify(&t, &Type::Bool) {
                            self.errors.push(format!(
                                "{} `not` expects Bool, found {}",
                                expr.span,
                                self.display_type(&t),
                            ));
                        }
                        Type::Bool
                    }
                }
            }

            ExprKind::Call(func, args) => {
                // Check builtin arity before type inference
                if let ExprKind::Ident(name) = &func.kind {
                    if let Some(&(min, max)) = self.builtins.get(name) {
                        if args.len() < min || args.len() > max {
                            let expected = if min == max {
                                format!("{}", min)
                            } else {
                                format!("{}-{}", min, max)
                            };
                            self.errors.push(format!(
                                "{} `{}` expects {} argument{}, got {}",
                                expr.span,
                                name,
                                expected,
                                if max == 1 { "" } else { "s" },
                                args.len(),
                            ));
                        }
                    }
                }

                let f_ty = self.infer_expr(func);
                let arg_types: Vec<Type> = args.iter().map(|a| self.infer_expr(a)).collect();

                let ret = self.fresh_var();
                let expected_fn = Type::Function(arg_types, Box::new(ret.clone()));

                if !self.unify(&f_ty, &expected_fn) {
                    // If the function type is completely unknown (fresh var), it will
                    // have unified fine. If not, we report but return the ret var.
                    let resolved_f = self.apply(&f_ty);
                    match &resolved_f {
                        Type::Function(params, _) => {
                            if params.len() != args.len() {
                                self.errors.push(format!(
                                    "{} Function expects {} arguments, got {}",
                                    expr.span,
                                    params.len(),
                                    args.len(),
                                ));
                            } else {
                                self.errors.push(format!(
                                    "{} Argument type mismatch in function call",
                                    expr.span,
                                ));
                            }
                        }
                        _ => {
                            // Could be calling something we can't resolve — be lenient
                        }
                    }
                }
                self.apply(&ret)
            }

            ExprKind::Lambda(params, return_type, body, _is_move) => {
                self.push_scope();
                let mut param_types = Vec::new();
                for p in params {
                    let ty = p
                        .ty
                        .as_ref()
                        .map(|t| self.resolve_type_expr_simple(t))
                        .unwrap_or_else(|| self.fresh_var());
                    self.bind_var(&p.name, ty.clone(), p.span);
                    param_types.push(ty);
                }
                let body_ty = self.infer_expr(body);
                if let Some(ret_ty) = return_type {
                    let resolved = self.resolve_type_expr_simple(ret_ty);
                    if !self.unify(&body_ty, &resolved) {
                        self.errors.push(format!(
                            "{} Lambda return type mismatch: expected {}, found {}",
                            expr.span,
                            self.display_type(&resolved),
                            self.display_type(&body_ty),
                        ));
                    }
                }
                self.pop_scope();
                Type::Function(param_types, Box::new(body_ty))
            }

            ExprKind::If(cond, then_branch, else_branch) => {
                let cond_ty = self.infer_expr(cond);
                if !self.unify(&cond_ty, &Type::Bool) {
                    self.errors.push(format!(
                        "{} If condition expects Bool, found {}",
                        expr.span,
                        self.display_type(&cond_ty),
                    ));
                }
                let then_ty = self.infer_expr(then_branch);
                if let Some(else_br) = else_branch {
                    let else_ty = self.infer_expr(else_br);
                    if !self.unify(&then_ty, &else_ty) {
                        self.errors.push(format!(
                            "{} If/else branch type mismatch: {} vs {}",
                            expr.span,
                            self.display_type(&then_ty),
                            self.display_type(&else_ty),
                        ));
                    }
                    self.apply(&then_ty)
                } else {
                    // No else branch — result is Unit
                    Type::Unit
                }
            }

            ExprKind::Match(scrutinee, arms) => {
                let scrut_ty = self.infer_expr(scrutinee);
                let result_ty = self.fresh_var();

                for arm in arms {
                    self.push_scope();
                    self.infer_pattern(&arm.pattern, &scrut_ty);
                    if let Some(guard) = &arm.guard {
                        let guard_ty = self.infer_expr(guard);
                        if !self.unify(&guard_ty, &Type::Bool) {
                            self.errors.push(format!(
                                "{} Match guard must be Bool",
                                arm.span,
                            ));
                        }
                    }
                    let arm_ty = self.infer_expr(&arm.body);
                    if !self.unify(&result_ty, &arm_ty) {
                        self.errors.push(format!(
                            "{} Match arm type mismatch: expected {}, found {}",
                            arm.span,
                            self.display_type(&result_ty),
                            self.display_type(&arm_ty),
                        ));
                    }
                    self.pop_scope();
                }

                // Exhaustiveness check: if scrutinee is a known enum, check all variants covered
                let resolved_scrut = self.apply(&scrut_ty);
                if let Type::Named(ref enum_name, _) = resolved_scrut {
                    if let Some(TypeDef::Enum { variants, .. }) = self.types.get(enum_name) {
                        let has_wildcard = arms.iter().any(|arm| self.pattern_has_wildcard(&arm.pattern));
                        if !has_wildcard {
                            let covered: std::collections::HashSet<String> = arms.iter()
                                .flat_map(|arm| self.collect_constructor_names(&arm.pattern))
                                .collect();
                            let missing: Vec<&String> = variants.iter()
                                .filter(|v| !covered.contains(*v))
                                .collect();
                            if !missing.is_empty() {
                                let names: Vec<&str> = missing.iter().map(|s| s.as_str()).collect();
                                self.warnings.push((expr.span, format!(
                                    "non-exhaustive match on `{}`. Missing variants: {}",
                                    enum_name,
                                    names.join(", "),
                                )));
                            }
                        }
                    }
                }

                self.apply(&result_ty)
            }

            ExprKind::Block(stmts, final_expr) => {
                self.push_scope();
                for stmt in stmts {
                    self.infer_stmt(stmt);
                }
                let ty = self.infer_expr(final_expr);
                self.pop_scope();
                ty
            }

            ExprKind::Let(pattern, ty_ann, value) => {
                let val_ty = self.infer_expr(value);
                if let Some(ann) = ty_ann {
                    let ann_ty = self.resolve_type_expr_simple(ann);
                    if !self.unify(&val_ty, &ann_ty) {
                        self.errors.push(format!(
                            "{} Let type annotation mismatch: expected {}, found {}",
                            expr.span,
                            self.display_type(&ann_ty),
                            self.display_type(&val_ty),
                        ));
                    }
                }
                // Record type for the bound name(s)
                if let Pattern::Ident(name) = pattern {
                    self.record_type(expr.span, name, &val_ty);
                }
                self.infer_pattern(pattern, &val_ty);
                Type::Unit
            }

            ExprKind::StructLit(name, fields, spread) => {
                if let Some(TypeDef::Struct(sdef)) = self.types.get(name).cloned() {
                    // Build type param mapping with fresh vars
                    let mut tparams = HashMap::new();
                    for tp in &sdef.type_params {
                        tparams.insert(tp.clone(), self.fresh_var());
                    }

                    // Check each provided field
                    let expected_fields: HashMap<String, TypeExpr> =
                        sdef.fields.iter().cloned().collect();
                    for (field_name, field_val) in fields {
                        let val_ty = self.infer_expr(field_val);
                        if let Some(expected_te) = expected_fields.get(field_name) {
                            let expected_ty = self.resolve_type_expr(expected_te, &tparams);
                            if !self.unify(&val_ty, &expected_ty) {
                                self.errors.push(format!(
                                    "{} Field `{}` type mismatch in {}: expected {}, found {}",
                                    expr.span,
                                    field_name,
                                    name,
                                    self.display_type(&expected_ty),
                                    self.display_type(&val_ty),
                                ));
                            }
                        } else {
                            self.errors.push(format!(
                                "{} Unknown field `{}` on struct {}",
                                expr.span, field_name, name,
                            ));
                        }
                    }

                    if let Some(s) = spread {
                        self.infer_expr(s);
                    }

                    // Check for missing fields (only if no spread)
                    if spread.is_none() {
                        let provided: HashSet<&str> =
                            fields.iter().map(|(n, _)| n.as_str()).collect();
                        for (fname, _) in &sdef.fields {
                            if !provided.contains(fname.as_str()) {
                                self.errors.push(format!(
                                    "{} Missing field `{}` in struct {}",
                                    expr.span, fname, name,
                                ));
                            }
                        }
                    }

                    let type_args: Vec<Type> = sdef
                        .type_params
                        .iter()
                        .map(|tp| {
                            tparams.get(tp).cloned().unwrap_or_else(|| self.fresh_var())
                        })
                        .collect();
                    Type::Named(name.clone(), type_args)
                } else {
                    // Unknown struct — lenient, just check field expressions
                    for (_, val) in fields {
                        self.infer_expr(val);
                    }
                    if let Some(s) = spread {
                        self.infer_expr(s);
                    }
                    Type::Named(name.clone(), vec![])
                }
            }

            ExprKind::FieldAccess(obj, _field) => {
                let obj_ty = self.infer_expr(obj);
                // We could resolve known struct fields here, but for v1
                // we return a fresh var — rustc will catch field errors.
                let resolved = self.apply(&obj_ty);
                if let Type::Named(name, args) = &resolved {
                    if let Some(TypeDef::Struct(sdef)) = self.types.get(name).cloned() {
                        let mut tparams = HashMap::new();
                        for (tp, arg) in sdef.type_params.iter().zip(args.iter()) {
                            tparams.insert(tp.clone(), arg.clone());
                        }
                        for (fname, fty) in &sdef.fields {
                            if fname == _field {
                                return self.resolve_type_expr(fty, &tparams);
                            }
                        }
                    }
                }
                self.fresh_var()
            }

            ExprKind::MethodCall(obj, _method, args) => {
                self.infer_expr(obj);
                for arg in args {
                    self.infer_expr(arg);
                }
                self.fresh_var()
            }

            ExprKind::Pipe(left, right) => {
                let left_ty = self.infer_expr(left);
                // Pipe desugars: left |> f  =>  f(left)
                // or:            left |> f(x)  =>  f(left, x)
                // Infer the right side and try to apply it
                match &right.kind {
                    ExprKind::Call(func, extra_args) => {
                        // Check for known builtin pipe return types
                        if let ExprKind::Ident(name) = &func.kind {
                            if self.builtins.contains_key(name.as_str()) {
                                let extra_tys: Vec<Type> =
                                    extra_args.iter().map(|a| self.infer_expr(a)).collect();
                                let resolved_left = self.apply(&left_ty);
                                if let Some(ret) =
                                    self.infer_builtin_pipe_return(name, &resolved_left, &extra_tys)
                                {
                                    return ret;
                                }
                            }
                        }
                        let f_ty = self.infer_expr(func);
                        let mut all_arg_types = vec![left_ty];
                        for a in extra_args {
                            all_arg_types.push(self.infer_expr(a));
                        }
                        let ret = self.fresh_var();
                        let expected_fn =
                            Type::Function(all_arg_types, Box::new(ret.clone()));
                        self.unify(&f_ty, &expected_fn);
                        self.apply(&ret)
                    }
                    _ => {
                        // Check for known builtin pipe return types (bare ident)
                        if let ExprKind::Ident(name) = &right.kind {
                            if self.builtins.contains_key(name.as_str()) {
                                let resolved_left = self.apply(&left_ty);
                                if let Some(ret) =
                                    self.infer_builtin_pipe_return(name, &resolved_left, &[])
                                {
                                    return ret;
                                }
                            }
                        }
                        let f_ty = self.infer_expr(right);
                        let ret = self.fresh_var();
                        let expected_fn =
                            Type::Function(vec![left_ty], Box::new(ret.clone()));
                        self.unify(&f_ty, &expected_fn);
                        self.apply(&ret)
                    }
                }
            }

            ExprKind::For(pattern, iter, body) => {
                let iter_ty = self.infer_expr(iter);
                let elem_ty = self.fresh_var();
                // Try to unify iter with List<elem_ty>
                let _ = self.unify(&iter_ty, &Type::List(Box::new(elem_ty.clone())));
                self.push_scope();
                self.infer_pattern(pattern, &self.apply(&elem_ty));
                self.infer_expr(body);
                self.pop_scope();
                Type::Unit
            }

            ExprKind::While(cond, body) => {
                let cond_ty = self.infer_expr(cond);
                if !self.unify(&cond_ty, &Type::Bool) {
                    self.errors.push(format!(
                        "{} While condition expects Bool, found {}",
                        expr.span,
                        self.display_type(&cond_ty),
                    ));
                }
                self.push_scope();
                self.infer_expr(body);
                self.pop_scope();
                Type::Unit
            }

            ExprKind::Break | ExprKind::Continue => Type::Unit,

            ExprKind::RustBlock(_) => self.fresh_var(),

            ExprKind::Try(inner) => {
                let inner_ty = self.infer_expr(inner);
                // Try unwraps Result<T, E> to T — but we can't easily extract T
                // from a Named("Result", [T, E]). Try our best.
                let resolved = self.apply(&inner_ty);
                if let Type::Named(name, args) = &resolved {
                    if name == "Result" && args.len() == 2 {
                        return args[0].clone();
                    }
                    if name == "Option" && args.len() == 1 {
                        return args[0].clone();
                    }
                }
                self.fresh_var()
            }

            ExprKind::Await(inner) => {
                self.infer_expr(inner);
                self.fresh_var()
            }
        }
    }

    // ── Inference: Patterns ────────────────────────────────────────────────

    fn infer_pattern(&mut self, pattern: &Pattern, expected: &Type) {
        match pattern {
            Pattern::Wildcard => {}

            Pattern::Ident(name) => {
                self.bind_var(name, self.apply(expected), Span::new(0, 0));
            }

            Pattern::IntLit(_) => {
                self.unify(expected, &Type::Int);
            }

            Pattern::FloatLit(_) => {
                self.unify(expected, &Type::Float);
            }

            Pattern::StringLit(_) => {
                self.unify(expected, &Type::Str);
            }

            Pattern::BoolLit(_) => {
                self.unify(expected, &Type::Bool);
            }

            Pattern::Tuple(pats) => {
                let elem_types: Vec<Type> =
                    pats.iter().map(|_| self.fresh_var()).collect();
                let tuple_ty = Type::Tuple(elem_types.clone());
                self.unify(expected, &tuple_ty);
                let resolved = self.apply(&tuple_ty);
                if let Type::Tuple(resolved_elems) = &resolved {
                    for (p, t) in pats.iter().zip(resolved_elems.iter()) {
                        self.infer_pattern(p, t);
                    }
                } else {
                    for (p, t) in pats.iter().zip(elem_types.iter()) {
                        self.infer_pattern(p, &self.apply(t));
                    }
                }
            }

            Pattern::List(pats, rest) => {
                let elem_ty = self.fresh_var();
                self.unify(expected, &Type::List(Box::new(elem_ty.clone())));
                let resolved_elem = self.apply(&elem_ty);
                for p in pats {
                    self.infer_pattern(p, &resolved_elem);
                }
                if let Some(rest_name) = rest {
                    self.bind_var(rest_name, self.apply(expected), Span::new(0, 0));
                }
            }

            Pattern::Constructor(name, pats) => {
                if let Some(ctor) = self.constructors.get(name).cloned() {
                    let result_ty = self.instantiate_ctor_with_fields(&ctor, pats);
                    self.unify(expected, &result_ty);
                } else {
                    // Unknown constructor — bind sub-patterns as fresh vars
                    for p in pats {
                        let fv = self.fresh_var();
                        self.infer_pattern(p, &fv);
                    }
                }
            }

            Pattern::Bind(name, inner) => {
                self.bind_var(name, self.apply(expected), Span::new(0, 0));
                self.infer_pattern(inner, expected);
            }

            Pattern::Or(pats) => {
                for pat in pats {
                    self.infer_pattern(pat, expected);
                }
            }

            Pattern::Range(_, _) => {
                self.unify(expected, &Type::Int);
            }
        }
    }

    // ── Pattern analysis helpers ──────────────────────────────────────────

    fn pattern_has_wildcard(&self, pattern: &Pattern) -> bool {
        match pattern {
            Pattern::Wildcard | Pattern::Ident(_) => true,
            Pattern::Or(pats) => pats.iter().any(|p| self.pattern_has_wildcard(p)),
            Pattern::Bind(_, inner) => self.pattern_has_wildcard(inner),
            _ => false,
        }
    }

    fn collect_constructor_names(&self, pattern: &Pattern) -> Vec<String> {
        match pattern {
            Pattern::Constructor(name, _) => vec![name.clone()],
            Pattern::Or(pats) => pats.iter()
                .flat_map(|p| self.collect_constructor_names(p))
                .collect(),
            Pattern::Bind(_, inner) => self.collect_constructor_names(inner),
            _ => vec![],
        }
    }

    // ── Constructor helpers ────────────────────────────────────────────────

    fn instantiate_ctor(&mut self, ctor: &CtorSig) -> Type {
        let mut tparams = HashMap::new();
        for tp in &ctor.type_params {
            tparams.insert(tp.clone(), self.fresh_var());
        }
        let type_args: Vec<Type> = ctor
            .type_params
            .iter()
            .map(|tp| tparams.get(tp).unwrap().clone())
            .collect();
        Type::Named(ctor.enum_name.clone(), type_args)
    }

    fn ctor_as_function(&mut self, ctor: &CtorSig) -> Type {
        let mut tparams = HashMap::new();
        for tp in &ctor.type_params {
            tparams.insert(tp.clone(), self.fresh_var());
        }
        let field_types: Vec<Type> = ctor
            .field_types
            .iter()
            .map(|ft| self.resolve_type_expr(ft, &tparams))
            .collect();
        let type_args: Vec<Type> = ctor
            .type_params
            .iter()
            .map(|tp| tparams.get(tp).unwrap().clone())
            .collect();
        let ret = Type::Named(ctor.enum_name.clone(), type_args);
        Type::Function(field_types, Box::new(ret))
    }

    fn instantiate_ctor_with_fields(&mut self, ctor: &CtorSig, pats: &[Pattern]) -> Type {
        let mut tparams = HashMap::new();
        for tp in &ctor.type_params {
            tparams.insert(tp.clone(), self.fresh_var());
        }

        // Unify patterns with field types
        if pats.len() == ctor.field_types.len() {
            for (p, ft) in pats.iter().zip(ctor.field_types.iter()) {
                let expected = self.resolve_type_expr(ft, &tparams);
                self.infer_pattern(p, &expected);
            }
        } else {
            // Arity mismatch
            self.errors.push(format!(
                "Constructor {} expects {} fields, got {}",
                ctor.enum_name,
                ctor.field_types.len(),
                pats.len(),
            ));
            for p in pats {
                let fv = self.fresh_var();
                self.infer_pattern(p, &fv);
            }
        }

        let type_args: Vec<Type> = ctor
            .type_params
            .iter()
            .map(|tp| tparams.get(tp).unwrap().clone())
            .collect();
        Type::Named(ctor.enum_name.clone(), type_args)
    }

    // ── Inference: Statements ──────────────────────────────────────────────

    fn infer_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(_is_mut, pattern, ty_ann, value) => {
                let val_ty = self.infer_expr(value);
                if let Some(ann) = ty_ann {
                    let ann_ty = self.resolve_type_expr_simple(ann);
                    if !self.unify(&val_ty, &ann_ty) {
                        self.errors.push(format!(
                            "Let type annotation mismatch: expected {}, found {}",
                            self.display_type(&ann_ty),
                            self.display_type(&val_ty),
                        ));
                    }
                }
                // Generalize for let-polymorphism
                let scheme = self.generalize(&val_ty);
                // For simple ident patterns, bind with the generalized type.
                // For complex patterns, just bind directly.
                match pattern {
                    Pattern::Ident(name) => {
                        // Store the monomorphic type — full let-poly would store scheme
                        // and instantiate at use sites. We simplify: bind the applied type.
                        self.bind_var(name, self.apply(&scheme.ty), value.span);
                    }
                    _ => {
                        let applied = self.apply(&val_ty);
                        self.infer_pattern(pattern, &applied);
                    }
                }
            }

            Stmt::Expr(e) => {
                self.infer_expr(e);
            }

            Stmt::Assign(name, value) => {
                let val_ty = self.infer_expr(value);
                if let Some(var_ty) = self.lookup_var(name) {
                    if !self.unify(&var_ty, &val_ty) {
                        self.errors.push(format!(
                            "Assignment type mismatch for `{}`: expected {}, found {}",
                            name,
                            self.display_type(&var_ty),
                            self.display_type(&val_ty),
                        ));
                    }
                }
                // If not in scope, just silently accept (might be a module-level thing)
            }

            Stmt::CompoundAssign(name, _op, value) => {
                let val_ty = self.infer_expr(value);
                if let Some(var_ty) = self.lookup_var(name) {
                    if !self.unify(&var_ty, &val_ty) {
                        self.errors.push(format!(
                            "Compound assignment type mismatch for `{}`",
                            name,
                        ));
                    }
                }
            }

            Stmt::IndexAssign(obj, index, value) => {
                self.infer_expr(obj);
                self.infer_expr(index);
                self.infer_expr(value);
            }
        }
    }

    // ── Check a function body ──────────────────────────────────────────────

    fn check_function(&mut self, f: &Function) {
        self.push_scope();
        self.fn_scope_depth.insert(self.scopes.len());

        // Build type param mapping
        let mut tparams = HashMap::new();
        for tp in &f.type_params {
            tparams.insert(tp.name.clone(), self.fresh_var());
        }

        // Bind parameters
        for param in &f.params {
            let ty = param
                .ty
                .as_ref()
                .map(|t| self.resolve_type_expr(t, &tparams))
                .unwrap_or_else(|| self.fresh_var());
            if let Some(pattern) = &param.destructure {
                self.infer_pattern(pattern, &ty);
            } else {
                self.bind_var(&param.name, ty, param.span);
            }
        }

        // Infer body
        let body_ty = self.infer_expr(&f.body);

        // Check return type annotation
        if let Some(ret_ann) = &f.return_type {
            let ret_ty = self.resolve_type_expr(ret_ann, &tparams);
            if !self.unify(&body_ty, &ret_ty) {
                self.errors.push(format!(
                    "{} Function `{}` return type mismatch: declared {}, body produces {}",
                    f.span,
                    f.name,
                    self.display_type(&ret_ty),
                    self.display_type(&body_ty),
                ));
            }
        } else {
            // No annotation — record inferred return type for codegen
            let resolved = self.apply(&body_ty);
            if !matches!(resolved, Type::Unit | Type::Error | Type::Var(_)) {
                if !self.has_unresolved_vars(&resolved) {
                    self.inferred_returns.insert(f.name.clone(), resolved);
                }
            }
        }

        self.pop_scope();
    }

    // ── Check entire program ───────────────────────────────────────────────

    fn check_program(&mut self, program: &Program) {
        self.register_program(program);

        for item in &program.items {
            self.check_item(item);
        }
    }

    fn check_item(&mut self, item: &Item) {
        match item {
            Item::Function(f) => self.check_function(f),
            Item::ModuleDecl(m) => {
                for item in &m.items {
                    self.check_item(item);
                }
            }
            Item::Expr(e) => {
                self.infer_expr(e);
            }
            Item::Const(c) => {
                self.infer_expr(&c.value);
            }
            Item::TypeDecl(_) | Item::UseDecl(_) | Item::ExternFn(_) => {}
            Item::TraitDecl(t) => {
                for method in &t.methods {
                    if let Some(body) = &method.default_body {
                        self.infer_expr(body);
                    }
                }
            }
            Item::ImplBlock(imp) => {
                for method in &imp.methods {
                    self.check_function(method);
                }
                // Validate trait satisfaction
                if let Some(trait_name) = &imp.trait_name {
                    if let Some(trait_methods) = self.traits.get(trait_name).cloned() {
                        let impl_methods: HashMap<&str, usize> = imp.methods.iter()
                            .map(|m| (m.name.as_str(), m.params.len()))
                            .collect();
                        for (method_name, param_count, has_default) in &trait_methods {
                            match impl_methods.get(method_name.as_str()) {
                                Some(&impl_param_count) => {
                                    if impl_param_count != *param_count {
                                        self.errors.push(format!(
                                            "{} Method '{}' in impl {} for {} has {} parameters, but trait requires {}",
                                            imp.span, method_name, trait_name, imp.type_name,
                                            impl_param_count, param_count,
                                        ));
                                    }
                                }
                                None if !has_default => {
                                    self.errors.push(format!(
                                        "{} Missing method '{}' required by trait {} in impl for {}",
                                        imp.span, method_name, trait_name, imp.type_name,
                                    ));
                                }
                                None => {} // has default body — not required
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── Public API ─────────────────────────────────────────────────────────────

pub fn check(program: Program) -> Result<(Program, Vec<(Span, String)>), String> {
    let mut infer = Infer::new();
    infer.check_program(&program);

    if infer.errors.is_empty() {
        let mut program = program;
        // Fill in inferred return types for functions without annotations
        fill_inferred_returns(&mut program.items, &infer);
        Ok((program, infer.warnings))
    } else {
        Err(infer.errors.join("\n"))
    }
}

/// Walk AST items and fill in inferred return types for functions without annotations.
fn fill_inferred_returns(items: &mut [Item], infer: &Infer) {
    for item in items.iter_mut() {
        match item {
            Item::Function(f) => {
                if f.return_type.is_none() && f.name != "main" {
                    if let Some(ty) = infer.inferred_returns.get(&f.name) {
                        f.return_type = Some(infer.type_to_type_expr(ty));
                    }
                }
            }
            Item::ModuleDecl(m) => {
                fill_inferred_returns(&mut m.items, infer);
            }
            _ => {}
        }
    }
}

/// Analyze a program for LSP: returns diagnostics, type info, definitions, completions.
/// Unlike `check()`, this always succeeds and returns partial results on errors.
pub fn analyze(program: &Program) -> AnalysisResult {
    let mut infer = Infer::new();
    infer.check_program(program);

    // Parse errors from the legacy "line:col message" format into structured errors
    let mut errors: Vec<(Span, String)> = infer.structured_errors.clone();
    for err_str in &infer.errors {
        if let Some((loc, msg)) = err_str.split_once(' ') {
            if let Some((line_s, col_s)) = loc.split_once(':') {
                if let (Ok(line), Ok(col)) = (line_s.parse::<usize>(), col_s.parse::<usize>()) {
                    errors.push((Span::new(line, col), msg.to_string()));
                    continue;
                }
            }
        }
        errors.push((Span::new(1, 1), err_str.clone()));
    }

    let definitions = collect_definitions(program);
    let builtin_names: Vec<String> = infer.builtins.keys().cloned().collect();
    let type_names: Vec<String> = infer.types.keys().cloned().collect();
    let constructor_names: Vec<String> = infer.constructors.keys().cloned().collect();

    AnalysisResult {
        errors,
        warnings: infer.warnings,
        type_at: infer.type_at,
        definitions,
        builtin_names,
        type_names,
        constructor_names,
    }
}

/// Walk AST items and collect definition symbols.
fn collect_definitions(program: &Program) -> Vec<SymbolInfo> {
    let mut defs = Vec::new();
    for item in &program.items {
        collect_item_defs(item, &mut defs);
    }
    defs
}

fn collect_item_defs(item: &Item, defs: &mut Vec<SymbolInfo>) {
    match item {
        Item::Function(f) => {
            let params: Vec<String> = f.params.iter().map(|p| p.name.clone()).collect();
            let detail = format!("fn {}({})", f.name, params.join(", "));
            defs.push(SymbolInfo {
                name: f.name.clone(),
                kind: SymbolKind::Function,
                span: f.span,
                detail: Some(detail),
            });
        }
        Item::TypeDecl(t) => {
            defs.push(SymbolInfo {
                name: t.name.clone(),
                kind: SymbolKind::Type,
                span: t.span,
                detail: None,
            });
            if let TypeBody::Enum(variants) = &t.body {
                for v in variants {
                    defs.push(SymbolInfo {
                        name: v.name.clone(),
                        kind: SymbolKind::Constructor,
                        span: v.span,
                        detail: Some(format!("{}::{}", t.name, v.name)),
                    });
                }
            }
        }
        Item::ModuleDecl(m) => {
            defs.push(SymbolInfo {
                name: m.name.clone(),
                kind: SymbolKind::Module,
                span: m.span,
                detail: None,
            });
            for sub in &m.items {
                collect_item_defs(sub, defs);
            }
        }
        Item::TraitDecl(t) => {
            defs.push(SymbolInfo {
                name: t.name.clone(),
                kind: SymbolKind::Trait,
                span: t.span,
                detail: None,
            });
        }
        Item::Const(c) => {
            defs.push(SymbolInfo {
                name: c.name.clone(),
                kind: SymbolKind::Constant,
                span: c.span,
                detail: None,
            });
        }
        Item::ImplBlock(imp) => {
            for method in &imp.methods {
                let params: Vec<String> = method.params.iter().map(|p| p.name.clone()).collect();
                let detail = format!("fn {}.{}({})", imp.type_name, method.name, params.join(", "));
                defs.push(SymbolInfo {
                    name: method.name.clone(),
                    kind: SymbolKind::Function,
                    span: method.span,
                    detail: Some(detail),
                });
            }
        }
        _ => {}
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Span;

    fn span() -> Span {
        Span::new(1, 1)
    }

    fn mk_expr(kind: ExprKind) -> Expr {
        Expr { kind, span: span() }
    }

    fn mk_fn(name: &str, params: Vec<Param>, ret: Option<TypeExpr>, body: Expr) -> Function {
        Function {
            name: name.to_string(),
            params,
            return_type: ret,
            body,
            is_pub: false,
            is_async: false,
            type_params: vec![],
            annotations: vec![],
            span: span(),
        }
    }

    fn mk_param(name: &str, ty: Option<TypeExpr>) -> Param {
        Param { name: name.to_string(), ty, destructure: None, span: span() }
    }

    fn int_ty() -> TypeExpr {
        TypeExpr::Named("Int".to_string(), vec![])
    }

    fn str_ty() -> TypeExpr {
        TypeExpr::Named("String".to_string(), vec![])
    }

    fn bool_ty() -> TypeExpr {
        TypeExpr::Named("Bool".to_string(), vec![])
    }

    #[test]
    fn test_literal_types() {
        let mut infer = Infer::new();
        assert_eq!(infer.infer_expr(&mk_expr(ExprKind::IntLit(42))), Type::Int);
        assert_eq!(infer.infer_expr(&mk_expr(ExprKind::FloatLit(3.14))), Type::Float);
        assert_eq!(
            infer.infer_expr(&mk_expr(ExprKind::StringLit("hi".to_string()))),
            Type::Str
        );
        assert_eq!(infer.infer_expr(&mk_expr(ExprKind::BoolLit(true))), Type::Bool);
    }

    #[test]
    fn test_unify_basic() {
        let mut infer = Infer::new();
        assert!(infer.unify(&Type::Int, &Type::Int));
        assert!(!infer.unify(&Type::Int, &Type::Float));
        assert!(!infer.unify(&Type::Int, &Type::Str));
    }

    #[test]
    fn test_unify_vars() {
        let mut infer = Infer::new();
        let v = infer.fresh_var();
        assert!(infer.unify(&v, &Type::Int));
        assert_eq!(infer.apply(&v), Type::Int);
    }

    #[test]
    fn test_unify_list() {
        let mut infer = Infer::new();
        let v = infer.fresh_var();
        let list_v = Type::List(Box::new(v.clone()));
        let list_int = Type::List(Box::new(Type::Int));
        assert!(infer.unify(&list_v, &list_int));
        assert_eq!(infer.apply(&v), Type::Int);
    }

    #[test]
    fn test_list_lit_inference() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::ListLit(vec![
            mk_expr(ExprKind::IntLit(1)),
            mk_expr(ExprKind::IntLit(2)),
        ]));
        let ty = infer.infer_expr(&expr);
        assert_eq!(infer.apply(&ty), Type::List(Box::new(Type::Int)));
        assert!(infer.errors.is_empty());
    }

    #[test]
    fn test_list_lit_mismatch() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::ListLit(vec![
            mk_expr(ExprKind::IntLit(1)),
            mk_expr(ExprKind::StringLit("oops".to_string())),
        ]));
        infer.infer_expr(&expr);
        assert!(!infer.errors.is_empty());
    }

    #[test]
    fn test_binop_arithmetic() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(1))),
            BinOp::Add,
            Box::new(mk_expr(ExprKind::IntLit(2))),
        ));
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Int);
        assert!(infer.errors.is_empty());
    }

    #[test]
    fn test_binop_comparison() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(1))),
            BinOp::Lt,
            Box::new(mk_expr(ExprKind::IntLit(2))),
        ));
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_if_expr() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::If(
            Box::new(mk_expr(ExprKind::BoolLit(true))),
            Box::new(mk_expr(ExprKind::IntLit(1))),
            Some(Box::new(mk_expr(ExprKind::IntLit(2)))),
        ));
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Int);
        assert!(infer.errors.is_empty());
    }

    #[test]
    fn test_if_branch_mismatch() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::If(
            Box::new(mk_expr(ExprKind::BoolLit(true))),
            Box::new(mk_expr(ExprKind::IntLit(1))),
            Some(Box::new(mk_expr(ExprKind::StringLit("x".to_string())))),
        ));
        infer.infer_expr(&expr);
        assert!(!infer.errors.is_empty());
    }

    #[test]
    fn test_lambda_inference() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::Lambda(
            vec![mk_param("x", Some(int_ty()))],
            None,
            Box::new(mk_expr(ExprKind::Ident("x".to_string()))),
            false,
        ));
        let ty = infer.infer_expr(&expr);
        let applied = infer.apply(&ty);
        assert_eq!(applied, Type::Function(vec![Type::Int], Box::new(Type::Int)));
    }

    #[test]
    fn test_function_return_type_check() {
        let f = mk_fn(
            "foo",
            vec![mk_param("x", Some(int_ty()))],
            Some(str_ty()),
            mk_expr(ExprKind::Ident("x".to_string())),
        );
        let program = Program {
            items: vec![Item::Function(f)],
        };
        let result = check(program);
        assert!(result.is_err());
    }

    #[test]
    fn test_function_return_type_ok() {
        let f = mk_fn(
            "foo",
            vec![mk_param("x", Some(int_ty()))],
            Some(int_ty()),
            mk_expr(ExprKind::Ident("x".to_string())),
        );
        let program = Program {
            items: vec![Item::Function(f)],
        };
        let result = check(program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_builtin_call_lenient() {
        // Calling a builtin like println should not error
        let expr = mk_expr(ExprKind::Call(
            Box::new(mk_expr(ExprKind::Ident("println".to_string()))),
            vec![mk_expr(ExprKind::StringLit("hello".to_string()))],
        ));
        let mut infer = Infer::new();
        infer.infer_expr(&expr);
        assert!(infer.errors.is_empty());
    }

    #[test]
    fn test_struct_lit_check() {
        let td = TypeDecl {
            name: "Point".to_string(),
            type_params: vec![],
            body: TypeBody::Struct(vec![
                Field { name: "x".to_string(), ty: int_ty(), span: span() },
                Field { name: "y".to_string(), ty: int_ty(), span: span() },
            ]),
            span: span(),
        };
        let expr = mk_expr(ExprKind::StructLit(
            "Point".to_string(),
            vec![
                ("x".to_string(), mk_expr(ExprKind::IntLit(1))),
                ("y".to_string(), mk_expr(ExprKind::IntLit(2))),
            ],
            None,
        ));
        let program = Program {
            items: vec![
                Item::TypeDecl(td),
                Item::Expr(expr),
            ],
        };
        let result = check(program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_struct_missing_field() {
        let td = TypeDecl {
            name: "Point".to_string(),
            type_params: vec![],
            body: TypeBody::Struct(vec![
                Field { name: "x".to_string(), ty: int_ty(), span: span() },
                Field { name: "y".to_string(), ty: int_ty(), span: span() },
            ]),
            span: span(),
        };
        let expr = mk_expr(ExprKind::StructLit(
            "Point".to_string(),
            vec![("x".to_string(), mk_expr(ExprKind::IntLit(1)))],
            None,
        ));
        let program = Program {
            items: vec![
                Item::TypeDecl(td),
                Item::Expr(expr),
            ],
        };
        let result = check(program);
        assert!(result.is_err());
    }

    #[test]
    fn test_enum_constructor() {
        let td = TypeDecl {
            name: "Option".to_string(),
            type_params: vec![TypeParam::plain("T".to_string())],
            body: TypeBody::Enum(vec![
                Variant {
                    name: "Some".to_string(),
                    fields: vec![TypeExpr::Named("T".to_string(), vec![])],
                    span: span(),
                },
                Variant {
                    name: "None".to_string(),
                    fields: vec![],
                    span: span(),
                },
            ]),
            span: span(),
        };
        let expr = mk_expr(ExprKind::Call(
            Box::new(mk_expr(ExprKind::Ident("Some".to_string()))),
            vec![mk_expr(ExprKind::IntLit(42))],
        ));
        let program = Program {
            items: vec![
                Item::TypeDecl(td),
                Item::Expr(expr),
            ],
        };
        let result = check(program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_match_pattern_binding() {
        let mut infer = Infer::new();
        infer.push_scope();
        let scrut_ty = Type::Int;
        infer.infer_pattern(&Pattern::Ident("x".to_string()), &scrut_ty);
        assert_eq!(infer.lookup_var("x"), Some(Type::Int));
        infer.pop_scope();
    }

    #[test]
    fn test_tuple_inference() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::Tuple(vec![
            mk_expr(ExprKind::IntLit(1)),
            mk_expr(ExprKind::StringLit("hi".to_string())),
        ]));
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Tuple(vec![Type::Int, Type::Str]));
    }

    #[test]
    fn test_block_returns_final() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::Block(
            vec![Stmt::Expr(mk_expr(ExprKind::IntLit(1)))],
            Box::new(mk_expr(ExprKind::StringLit("result".to_string()))),
        ));
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Str);
    }

    #[test]
    fn test_for_loop_unit() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::For(
            Pattern::Ident("x".to_string()),
            Box::new(mk_expr(ExprKind::ListLit(vec![mk_expr(ExprKind::IntLit(1))]))),
            Box::new(mk_expr(ExprKind::Ident("x".to_string()))),
        ));
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Unit);
    }

    #[test]
    fn test_unknown_function_error() {
        // Calling a completely unknown function should produce an error
        let expr = mk_expr(ExprKind::Call(
            Box::new(mk_expr(ExprKind::Ident("some_unknown_fn".to_string()))),
            vec![mk_expr(ExprKind::IntLit(1))],
        ));
        let mut infer = Infer::new();
        infer.infer_expr(&expr);
        assert!(!infer.errors.is_empty());
        assert!(infer.errors[0].contains("Unknown identifier 'some_unknown_fn'"));
    }

    // ── Let-polymorphism tests ───────────────────────────────────

    #[test]
    fn test_let_polymorphism_identity() {
        // let id = fn(x) => x; id(1); id("hello") — should not error
        let mut infer = Infer::new();
        // First define id with a generic type
        let lambda = mk_expr(ExprKind::Lambda(
            vec![mk_param("x", None)],
            None,
            Box::new(mk_expr(ExprKind::Ident("x".to_string()))),
            false,
        ));
        let lambda_ty = infer.infer_expr(&lambda);
        // Generalize and bind
        let scheme = infer.generalize(&lambda_ty);
        infer.push_scope();
        let instantiated1 = infer.instantiate(&scheme);
        // Using it as Int -> Int
        assert!(infer.unify(&instantiated1, &Type::Function(vec![Type::Int], Box::new(Type::Int))));
        // Using it as Str -> Str (fresh instantiation)
        let instantiated2 = infer.instantiate(&scheme);
        assert!(infer.unify(&instantiated2, &Type::Function(vec![Type::Str], Box::new(Type::Str))));
        infer.pop_scope();
        assert!(infer.errors.is_empty());
    }

    // ── Nested function call type inference ──────────────────────

    #[test]
    fn test_nested_call_unknown_errors() {
        // f(g(1)) where both are unknown — should produce errors for both
        let mut infer = Infer::new();
        let inner_call = mk_expr(ExprKind::Call(
            Box::new(mk_expr(ExprKind::Ident("g".to_string()))),
            vec![mk_expr(ExprKind::IntLit(1))],
        ));
        let outer_call = mk_expr(ExprKind::Call(
            Box::new(mk_expr(ExprKind::Ident("f".to_string()))),
            vec![inner_call],
        ));
        infer.infer_expr(&outer_call);
        assert!(infer.errors.iter().any(|e| e.contains("Unknown identifier 'g'")));
        assert!(infer.errors.iter().any(|e| e.contains("Unknown identifier 'f'")));
    }

    // ── Pattern matching type inference ──────────────────────────

    #[test]
    fn test_pattern_wildcard() {
        let mut infer = Infer::new();
        infer.push_scope();
        infer.infer_pattern(&Pattern::Wildcard, &Type::Int);
        // Wildcard should not introduce any bindings
        assert!(infer.lookup_var("_").is_none());
        infer.pop_scope();
    }

    #[test]
    fn test_pattern_int_lit() {
        let mut infer = Infer::new();
        infer.push_scope();
        infer.infer_pattern(&Pattern::IntLit(42), &Type::Int);
        // IntLit pattern should not introduce bindings
        infer.pop_scope();
        assert!(infer.errors.is_empty());
    }

    #[test]
    fn test_pattern_bool_lit() {
        let mut infer = Infer::new();
        infer.push_scope();
        infer.infer_pattern(&Pattern::BoolLit(true), &Type::Bool);
        infer.pop_scope();
        assert!(infer.errors.is_empty());
    }

    #[test]
    fn test_pattern_string_lit() {
        let mut infer = Infer::new();
        infer.push_scope();
        infer.infer_pattern(&Pattern::StringLit("hi".to_string()), &Type::Str);
        infer.pop_scope();
        assert!(infer.errors.is_empty());
    }

    #[test]
    fn test_pattern_tuple() {
        let mut infer = Infer::new();
        infer.push_scope();
        let scrut_ty = Type::Tuple(vec![Type::Int, Type::Str]);
        infer.infer_pattern(
            &Pattern::Tuple(vec![
                Pattern::Ident("a".to_string()),
                Pattern::Ident("b".to_string()),
            ]),
            &scrut_ty,
        );
        assert_eq!(infer.lookup_var("a"), Some(Type::Int));
        assert_eq!(infer.lookup_var("b"), Some(Type::Str));
        infer.pop_scope();
    }

    // ── Constructor with wrong number of args ────────────────────

    #[test]
    fn test_enum_constructor_wrong_arg_count() {
        let td = TypeDecl {
            name: "Shape".to_string(),
            type_params: vec![],
            body: TypeBody::Enum(vec![
                Variant {
                    name: "Circle".to_string(),
                    fields: vec![TypeExpr::Named("Float".to_string(), vec![])],
                    span: span(),
                },
            ]),
            span: span(),
        };
        let expr = mk_expr(ExprKind::Call(
            Box::new(mk_expr(ExprKind::Ident("Circle".to_string()))),
            vec![
                mk_expr(ExprKind::FloatLit(1.0)),
                mk_expr(ExprKind::FloatLit(2.0)),
            ], // 2 args, but Circle takes 1
        ));
        let program = Program {
            items: vec![
                Item::TypeDecl(td),
                Item::Expr(expr),
            ],
        };
        let result = check(program);
        assert!(result.is_err());
    }

    // ── While loop returns unit type ─────────────────────────────

    #[test]
    fn test_while_loop_unit_type() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::While(
            Box::new(mk_expr(ExprKind::BoolLit(true))),
            Box::new(mk_expr(ExprKind::IntLit(1))),
        ));
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Unit);
    }

    // ── Lambda return type inference ─────────────────────────────

    #[test]
    fn test_lambda_body_determines_return_type() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::Lambda(
            vec![mk_param("x", Some(str_ty()))],
            None,
            Box::new(mk_expr(ExprKind::IntLit(42))),
            false,
        ));
        let ty = infer.infer_expr(&expr);
        let applied = infer.apply(&ty);
        assert_eq!(applied, Type::Function(vec![Type::Str], Box::new(Type::Int)));
    }

    #[test]
    fn test_lambda_no_annotation_infers_param_type() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::Lambda(
            vec![mk_param("x", None)],
            None,
            Box::new(mk_expr(ExprKind::IntLit(42))),
            false,
        ));
        let ty = infer.infer_expr(&expr);
        let applied = infer.apply(&ty);
        // Return type should be Int; param type should be a type var
        match applied {
            Type::Function(params, ret) => {
                assert_eq!(params.len(), 1);
                assert_eq!(*ret, Type::Int);
            }
            _ => panic!("Expected function type, got {:?}", applied),
        }
    }

    // ── BinOp edge cases ─────────────────────────────────────────

    #[test]
    fn test_binop_equality_returns_bool() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(1))),
            BinOp::Eq,
            Box::new(mk_expr(ExprKind::IntLit(2))),
        ));
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_binop_ne_returns_bool() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(1))),
            BinOp::Ne,
            Box::new(mk_expr(ExprKind::IntLit(2))),
        ));
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_binop_and_or_bools() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::BoolLit(true))),
            BinOp::And,
            Box::new(mk_expr(ExprKind::BoolLit(false))),
        ));
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_binop_mul_returns_int() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(3))),
            BinOp::Mul,
            Box::new(mk_expr(ExprKind::IntLit(4))),
        ));
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Int);
    }

    // ── Unary ops ────────────────────────────────────────────────

    #[test]
    fn test_unary_neg() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::UnaryOp(
            UnaryOp::Neg,
            Box::new(mk_expr(ExprKind::IntLit(5))),
        ));
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_unary_not() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::UnaryOp(
            UnaryOp::Not,
            Box::new(mk_expr(ExprKind::BoolLit(true))),
        ));
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Bool);
    }

    // ── Let expression ───────────────────────────────────────────

    #[test]
    fn test_let_binding_type() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::Let(
            Pattern::Ident("x".to_string()),
            None,
            Box::new(mk_expr(ExprKind::IntLit(42))),
        ));
        infer.infer_expr(&expr);
        assert_eq!(infer.lookup_var("x"), Some(Type::Int));
        assert!(infer.errors.is_empty());
    }

    #[test]
    fn test_let_binding_with_annotation_ok() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::Let(
            Pattern::Ident("x".to_string()),
            Some(int_ty()),
            Box::new(mk_expr(ExprKind::IntLit(42))),
        ));
        infer.infer_expr(&expr);
        assert_eq!(infer.lookup_var("x"), Some(Type::Int));
        assert!(infer.errors.is_empty());
    }

    #[test]
    fn test_let_binding_with_annotation_mismatch() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::Let(
            Pattern::Ident("x".to_string()),
            Some(str_ty()),
            Box::new(mk_expr(ExprKind::IntLit(42))),
        ));
        infer.infer_expr(&expr);
        assert!(!infer.errors.is_empty());
    }

    // ── Empty list ───────────────────────────────────────────────

    #[test]
    fn test_empty_list_inference() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::ListLit(vec![]));
        let ty = infer.infer_expr(&expr);
        // Empty list has type List<var>
        match infer.apply(&ty) {
            Type::List(_) => {}
            other => panic!("Expected List type, got {:?}", other),
        }
    }

    // ── String literal ───────────────────────────────────────────

    #[test]
    fn test_string_interp_type() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::StringInterp(vec![
            StringPart::Lit("hello ".to_string()),
            StringPart::Expr(mk_expr(ExprKind::IntLit(42))),
        ]));
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Str);
    }

    // ── Pipe type ────────────────────────────────────────────────

    #[test]
    fn test_pipe_unknown_function_error() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::Pipe(
            Box::new(mk_expr(ExprKind::IntLit(5))),
            Box::new(mk_expr(ExprKind::Ident("double".to_string()))),
        ));
        infer.infer_expr(&expr);
        // Pipe with unknown function should now error
        assert!(infer.errors.iter().any(|e| e.contains("Unknown identifier 'double'")));
    }

    #[test]
    fn test_did_you_mean_builtin() {
        // "pritnln" is close to "println" — should suggest it
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::Ident("pritnln".to_string()));
        infer.infer_expr(&expr);
        assert!(infer.errors.len() == 1);
        assert!(infer.errors[0].contains("did you mean 'println'?"));
    }

    #[test]
    fn test_did_you_mean_user_function() {
        // Register a function "calculate", then mistype as "calculat"
        let mut infer = Infer::new();
        let scheme = Scheme { vars: vec![], ty: Type::Function(vec![Type::Int], Box::new(Type::Int)) };
        infer.functions.insert("calculate".to_string(), scheme);
        let expr = mk_expr(ExprKind::Ident("calculat".to_string()));
        infer.infer_expr(&expr);
        assert!(infer.errors[0].contains("did you mean 'calculate'?"));
    }

    #[test]
    fn test_levenshtein_distances() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("foo", "foo"), 0);
        assert_eq!(levenshtein("foo", "bar"), 3);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", ""), 3);
    }

    // ── Trait validation tests ────────────────────────────────────

    #[test]
    fn test_trait_impl_missing_method() {
        let trait_decl = Item::TraitDecl(TraitDecl {
            name: "Printable".to_string(),
            type_params: vec![],
            methods: vec![
                TraitMethod {
                    name: "to_str".to_string(),
                    params: vec![mk_param("self", None)],
                    return_type: Some(str_ty()),
                    default_body: None,
                    span: span(),
                },
            ],
            associated_types: vec![],
            span: span(),
        });
        let impl_block = Item::ImplBlock(ImplBlock {
            trait_name: Some("Printable".to_string()),
            type_name: "Foo".to_string(),
            type_params: vec![],
            methods: vec![], // Missing to_str!
            associated_types: vec![],
            span: span(),
        });
        let program = Program { items: vec![trait_decl, impl_block] };
        let result = check(program);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing method 'to_str'"));
    }

    #[test]
    fn test_trait_impl_satisfied() {
        let trait_decl = Item::TraitDecl(TraitDecl {
            name: "Printable".to_string(),
            type_params: vec![],
            methods: vec![
                TraitMethod {
                    name: "to_str".to_string(),
                    params: vec![mk_param("self", None)],
                    return_type: Some(str_ty()),
                    default_body: None,
                    span: span(),
                },
            ],
            associated_types: vec![],
            span: span(),
        });
        let impl_block = Item::ImplBlock(ImplBlock {
            trait_name: Some("Printable".to_string()),
            type_name: "Foo".to_string(),
            type_params: vec![],
            methods: vec![mk_fn(
                "to_str",
                vec![mk_param("self", None)],
                Some(str_ty()),
                mk_expr(ExprKind::StringLit("foo".to_string())),
            )],
            associated_types: vec![],
            span: span(),
        });
        let program = Program { items: vec![trait_decl, impl_block] };
        let result = check(program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_trait_default_method_not_required() {
        let trait_decl = Item::TraitDecl(TraitDecl {
            name: "Describable".to_string(),
            type_params: vec![],
            methods: vec![
                TraitMethod {
                    name: "describe".to_string(),
                    params: vec![mk_param("self", None)],
                    return_type: Some(str_ty()),
                    default_body: Some(mk_expr(ExprKind::StringLit("default".to_string()))),
                    span: span(),
                },
            ],
            associated_types: vec![],
            span: span(),
        });
        let impl_block = Item::ImplBlock(ImplBlock {
            trait_name: Some("Describable".to_string()),
            type_name: "Bar".to_string(),
            type_params: vec![],
            methods: vec![], // No methods — should be OK since describe has default
            associated_types: vec![],
            span: span(),
        });
        let program = Program { items: vec![trait_decl, impl_block] };
        let result = check(program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_trait_impl_wrong_param_count() {
        let trait_decl = Item::TraitDecl(TraitDecl {
            name: "Addable".to_string(),
            type_params: vec![],
            methods: vec![
                TraitMethod {
                    name: "add".to_string(),
                    params: vec![mk_param("self", None), mk_param("other", None)],
                    return_type: None,
                    default_body: None,
                    span: span(),
                },
            ],
            associated_types: vec![],
            span: span(),
        });
        let impl_block = Item::ImplBlock(ImplBlock {
            trait_name: Some("Addable".to_string()),
            type_name: "Num".to_string(),
            type_params: vec![],
            methods: vec![mk_fn(
                "add",
                vec![mk_param("self", None)], // Only 1 param, trait expects 2
                None,
                mk_expr(ExprKind::IntLit(0)),
            )],
            associated_types: vec![],
            span: span(),
        });
        let program = Program { items: vec![trait_decl, impl_block] };
        let result = check(program);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has 1 parameters, but trait requires 2"));
    }

    // ── Pub visibility tests ─────────────────────────────────────

    #[test]
    fn test_pub_function_accessible_from_module() {
        // A pub function in a module should be callable
        let module = Item::ModuleDecl(ModuleDecl {
            name: "Math".to_string(),
            items: vec![Item::Function(Function {
                name: "square".to_string(),
                params: vec![mk_param("x", Some(int_ty()))],
                return_type: Some(int_ty()),
                body: mk_expr(ExprKind::BinOp(
                    Box::new(mk_expr(ExprKind::Ident("x".to_string()))),
                    BinOp::Mul,
                    Box::new(mk_expr(ExprKind::Ident("x".to_string()))),
                )),
                is_pub: true,
                is_async: false,
                type_params: vec![],
                annotations: vec![],
                span: span(),
            })],
            span: span(),
        });
        let call = Item::Expr(mk_expr(ExprKind::Call(
            Box::new(mk_expr(ExprKind::Ident("square".to_string()))),
            vec![mk_expr(ExprKind::IntLit(5))],
        )));
        let program = Program { items: vec![module, call] };
        let result = check(program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_non_pub_function_inaccessible_from_module() {
        // A non-pub function in a module should NOT be callable
        let module = Item::ModuleDecl(ModuleDecl {
            name: "Math".to_string(),
            items: vec![Item::Function(Function {
                name: "secret".to_string(),
                params: vec![mk_param("x", Some(int_ty()))],
                return_type: Some(int_ty()),
                body: mk_expr(ExprKind::Ident("x".to_string())),
                is_pub: false,
                is_async: false,
                type_params: vec![],
                annotations: vec![],
                span: span(),
            })],
            span: span(),
        });
        let call = Item::Expr(mk_expr(ExprKind::Call(
            Box::new(mk_expr(ExprKind::Ident("secret".to_string()))),
            vec![mk_expr(ExprKind::IntLit(5))],
        )));
        let program = Program { items: vec![module, call] };
        let result = check(program);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown identifier 'secret'"));
    }

    // ── Multiple functions type check ────────────────────────────

    #[test]
    fn test_program_two_functions_ok() {
        let f1 = mk_fn(
            "double",
            vec![mk_param("x", Some(int_ty()))],
            Some(int_ty()),
            mk_expr(ExprKind::BinOp(
                Box::new(mk_expr(ExprKind::Ident("x".to_string()))),
                BinOp::Mul,
                Box::new(mk_expr(ExprKind::IntLit(2))),
            )),
        );
        let f2 = mk_fn(
            "main",
            vec![],
            None,
            mk_expr(ExprKind::Call(
                Box::new(mk_expr(ExprKind::Ident("double".to_string()))),
                vec![mk_expr(ExprKind::IntLit(21))],
            )),
        );
        let program = Program {
            items: vec![Item::Function(f1), Item::Function(f2)],
        };
        let result = check(program);
        assert!(result.is_ok());
    }

    // ── Scope isolation ──────────────────────────────────────────

    #[test]
    fn test_scope_push_pop() {
        let mut infer = Infer::new();
        infer.push_scope();
        infer.bind_var("x", Type::Int, Span::new(1, 1));
        assert_eq!(infer.lookup_var("x"), Some(Type::Int));
        infer.pop_scope();
        assert!(infer.lookup_var("x").is_none());
    }

    // ── Unification edge cases ───────────────────────────────────

    #[test]
    fn test_unify_two_vars() {
        let mut infer = Infer::new();
        let v1 = infer.fresh_var();
        let v2 = infer.fresh_var();
        assert!(infer.unify(&v1, &v2));
        // After unifying v1=v2 and then v1=Int, both should resolve to Int
        assert!(infer.unify(&v1, &Type::Int));
        assert_eq!(infer.apply(&v2), Type::Int);
    }

    #[test]
    fn test_unify_function_types() {
        let mut infer = Infer::new();
        let f1 = Type::Function(vec![Type::Int], Box::new(Type::Str));
        let f2 = Type::Function(vec![Type::Int], Box::new(Type::Str));
        assert!(infer.unify(&f1, &f2));
    }

    #[test]
    fn test_unify_function_types_mismatch() {
        let mut infer = Infer::new();
        let f1 = Type::Function(vec![Type::Int], Box::new(Type::Str));
        let f2 = Type::Function(vec![Type::Int], Box::new(Type::Int));
        assert!(!infer.unify(&f1, &f2));
    }

    #[test]
    fn test_unify_tuple_types() {
        let mut infer = Infer::new();
        let t1 = Type::Tuple(vec![Type::Int, Type::Str]);
        let t2 = Type::Tuple(vec![Type::Int, Type::Str]);
        assert!(infer.unify(&t1, &t2));
    }

    #[test]
    fn test_unify_tuple_length_mismatch() {
        let mut infer = Infer::new();
        let t1 = Type::Tuple(vec![Type::Int, Type::Str]);
        let t2 = Type::Tuple(vec![Type::Int]);
        assert!(!infer.unify(&t1, &t2));
    }

    // ── Additional type checker tests ──────────────────────────────

    #[test]
    fn test_infer_list_int_literal() {
        let mut infer = Infer::new();
        let list = mk_expr(ExprKind::ListLit(vec![
            mk_expr(ExprKind::IntLit(1)),
            mk_expr(ExprKind::IntLit(2)),
            mk_expr(ExprKind::IntLit(3)),
        ]));
        let ty = infer.infer_expr(&list);
        match ty {
            Type::List(_) => {} // List type is correct regardless of inner inference
            _ => panic!("Expected List type, got {:?}", ty),
        }
    }

    #[test]
    fn test_infer_empty_list_type() {
        let mut infer = Infer::new();
        let list = mk_expr(ExprKind::ListLit(vec![]));
        let ty = infer.infer_expr(&list);
        match ty {
            Type::List(_) => {}
            _ => panic!("Expected List type, got {:?}", ty),
        }
    }

    #[test]
    fn test_infer_tuple_int_str() {
        let mut infer = Infer::new();
        let tuple = mk_expr(ExprKind::Tuple(vec![
            mk_expr(ExprKind::IntLit(1)),
            mk_expr(ExprKind::StringLit("hello".to_string())),
        ]));
        let ty = infer.infer_expr(&tuple);
        match ty {
            Type::Tuple(types) => {
                assert_eq!(types.len(), 2);
                assert_eq!(types[0], Type::Int);
                assert_eq!(types[1], Type::Str);
            }
            _ => panic!("Expected Tuple, got {:?}", ty),
        }
    }

    #[test]
    fn test_infer_if_with_else() {
        let mut infer = Infer::new();
        let if_expr = mk_expr(ExprKind::If(
            Box::new(mk_expr(ExprKind::BoolLit(true))),
            Box::new(mk_expr(ExprKind::IntLit(1))),
            Some(Box::new(mk_expr(ExprKind::IntLit(2)))),
        ));
        let ty = infer.infer_expr(&if_expr);
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_infer_addition() {
        let mut infer = Infer::new();
        let add = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(1))),
            crate::ast::BinOp::Add,
            Box::new(mk_expr(ExprKind::IntLit(2))),
        ));
        assert_eq!(infer.infer_expr(&add), Type::Int);
    }

    #[test]
    fn test_infer_less_than() {
        let mut infer = Infer::new();
        let cmp = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(1))),
            crate::ast::BinOp::Lt,
            Box::new(mk_expr(ExprKind::IntLit(2))),
        ));
        assert_eq!(infer.infer_expr(&cmp), Type::Bool);
    }

    #[test]
    fn test_infer_eq_returns_bool() {
        let mut infer = Infer::new();
        let eq = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(1))),
            crate::ast::BinOp::Eq,
            Box::new(mk_expr(ExprKind::IntLit(2))),
        ));
        assert_eq!(infer.infer_expr(&eq), Type::Bool);
    }

    #[test]
    fn test_infer_and_expr() {
        let mut infer = Infer::new();
        let and_expr = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::BoolLit(true))),
            crate::ast::BinOp::And,
            Box::new(mk_expr(ExprKind::BoolLit(false))),
        ));
        assert_eq!(infer.infer_expr(&and_expr), Type::Bool);
    }

    #[test]
    fn test_infer_not_expr() {
        let mut infer = Infer::new();
        let not_expr = mk_expr(ExprKind::UnaryOp(
            crate::ast::UnaryOp::Not,
            Box::new(mk_expr(ExprKind::BoolLit(true))),
        ));
        assert_eq!(infer.infer_expr(&not_expr), Type::Bool);
    }

    #[test]
    fn test_infer_neg_int() {
        let mut infer = Infer::new();
        let neg = mk_expr(ExprKind::UnaryOp(
            crate::ast::UnaryOp::Neg,
            Box::new(mk_expr(ExprKind::IntLit(42))),
        ));
        assert_eq!(infer.infer_expr(&neg), Type::Int);
    }

    #[test]
    fn test_unify_var_to_int() {
        let mut infer = Infer::new();
        let var = infer.fresh_var();
        assert!(infer.unify(&var, &Type::Int));
    }

    #[test]
    fn test_unify_two_fresh_vars() {
        let mut infer = Infer::new();
        let v1 = infer.fresh_var();
        let v2 = infer.fresh_var();
        assert!(infer.unify(&v1, &v2));
        // After unifying with each other, unifying one with Int should succeed
        assert!(infer.unify(&v1, &Type::Int));
    }

    #[test]
    fn test_unify_list_int_int() {
        let mut infer = Infer::new();
        let l1 = Type::List(Box::new(Type::Int));
        let l2 = Type::List(Box::new(Type::Int));
        assert!(infer.unify(&l1, &l2));
    }

    #[test]
    fn test_unify_list_int_str_fails() {
        let mut infer = Infer::new();
        let l1 = Type::List(Box::new(Type::Int));
        let l2 = Type::List(Box::new(Type::Str));
        assert!(!infer.unify(&l1, &l2));
    }

    #[test]
    fn test_unify_nested_list_int() {
        let mut infer = Infer::new();
        let l1 = Type::List(Box::new(Type::List(Box::new(Type::Int))));
        let l2 = Type::List(Box::new(Type::List(Box::new(Type::Int))));
        assert!(infer.unify(&l1, &l2));
    }

    #[test]
    fn test_unify_fn_arity_mismatch() {
        let mut infer = Infer::new();
        let f1 = Type::Function(vec![Type::Int], Box::new(Type::Int));
        let f2 = Type::Function(vec![Type::Int, Type::Int], Box::new(Type::Int));
        assert!(!infer.unify(&f1, &f2));
    }

    #[test]
    fn test_check_function_double() {
        let mut infer = Infer::new();
        let f = mk_fn(
            "double",
            vec![mk_param("x", Some(int_ty()))],
            Some(int_ty()),
            mk_expr(ExprKind::BinOp(
                Box::new(mk_expr(ExprKind::Ident("x".to_string()))),
                crate::ast::BinOp::Mul,
                Box::new(mk_expr(ExprKind::IntLit(2))),
            )),
        );
        let errors_before = infer.errors.len();
        infer.check_function(&f);
        assert_eq!(infer.errors.len(), errors_before, "double(x: Int): Int should type check");
    }

    #[test]
    fn test_check_function_bad_return() {
        let mut infer = Infer::new();
        let f = mk_fn(
            "bad",
            vec![mk_param("x", Some(int_ty()))],
            Some(str_ty()),
            mk_expr(ExprKind::Ident("x".to_string())),
        );
        infer.check_function(&f);
        assert!(!infer.errors.is_empty(), "Return type mismatch should produce error");
    }

    #[test]
    fn test_infer_block_last_expr() {
        let mut infer = Infer::new();
        let block = mk_expr(ExprKind::Block(
            vec![
                Stmt::Expr(mk_expr(ExprKind::IntLit(1))),
                Stmt::Expr(mk_expr(ExprKind::IntLit(2))),
            ],
            Box::new(mk_expr(ExprKind::StringLit("hello".to_string()))),
        ));
        let ty = infer.infer_expr(&block);
        assert_eq!(ty, Type::Str, "Block should return type of last expression");
    }

    #[test]
    fn test_infer_single_block() {
        let mut infer = Infer::new();
        let block = mk_expr(ExprKind::Block(
            vec![],
            Box::new(mk_expr(ExprKind::IntLit(42))),
        ));
        assert_eq!(infer.infer_expr(&block), Type::Int);
    }

    #[test]
    fn test_infer_float_add() {
        let mut infer = Infer::new();
        let add = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::FloatLit(1.0))),
            crate::ast::BinOp::Add,
            Box::new(mk_expr(ExprKind::FloatLit(2.0))),
        ));
        assert_eq!(infer.infer_expr(&add), Type::Float);
    }

    #[test]
    fn test_infer_string_plus() {
        let mut infer = Infer::new();
        let concat = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::StringLit("hello".to_string()))),
            crate::ast::BinOp::Add,
            Box::new(mk_expr(ExprKind::StringLit(" world".to_string()))),
        ));
        let ty = infer.infer_expr(&concat);
        assert_eq!(ty, Type::Str, "String + String should be String");
    }

    #[test]
    fn test_unify_named_type_same() {
        let mut infer = Infer::new();
        let t1 = Type::Named("Foo".to_string(), vec![]);
        let t2 = Type::Named("Foo".to_string(), vec![]);
        assert!(infer.unify(&t1, &t2));
    }

    #[test]
    fn test_unify_named_type_different() {
        let mut infer = Infer::new();
        let t1 = Type::Named("Foo".to_string(), vec![]);
        let t2 = Type::Named("Bar".to_string(), vec![]);
        assert!(!infer.unify(&t1, &t2));
    }

    #[test]
    fn test_unify_named_type_with_params() {
        let mut infer = Infer::new();
        let t1 = Type::Named("Option".to_string(), vec![Type::Int]);
        let t2 = Type::Named("Option".to_string(), vec![Type::Int]);
        assert!(infer.unify(&t1, &t2));
    }

    #[test]
    fn test_unify_named_type_params_mismatch() {
        let mut infer = Infer::new();
        let t1 = Type::Named("Option".to_string(), vec![Type::Int]);
        let t2 = Type::Named("Option".to_string(), vec![Type::Str]);
        assert!(!infer.unify(&t1, &t2));
    }

    #[test]
    fn test_infer_or_expr() {
        let mut infer = Infer::new();
        let or_expr = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::BoolLit(false))),
            crate::ast::BinOp::Or,
            Box::new(mk_expr(ExprKind::BoolLit(true))),
        ));
        assert_eq!(infer.infer_expr(&or_expr), Type::Bool);
    }

    #[test]
    fn test_infer_subtraction() {
        let mut infer = Infer::new();
        let sub = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(10))),
            crate::ast::BinOp::Sub,
            Box::new(mk_expr(ExprKind::IntLit(3))),
        ));
        assert_eq!(infer.infer_expr(&sub), Type::Int);
    }

    #[test]
    fn test_infer_multiplication() {
        let mut infer = Infer::new();
        let mul = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(4))),
            crate::ast::BinOp::Mul,
            Box::new(mk_expr(ExprKind::IntLit(5))),
        ));
        assert_eq!(infer.infer_expr(&mul), Type::Int);
    }

    #[test]
    fn test_infer_division() {
        let mut infer = Infer::new();
        let div = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(10))),
            crate::ast::BinOp::Div,
            Box::new(mk_expr(ExprKind::IntLit(2))),
        ));
        assert_eq!(infer.infer_expr(&div), Type::Int);
    }

    #[test]
    fn test_infer_modulo() {
        let mut infer = Infer::new();
        let modulo = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(10))),
            crate::ast::BinOp::Mod,
            Box::new(mk_expr(ExprKind::IntLit(3))),
        ));
        assert_eq!(infer.infer_expr(&modulo), Type::Int);
    }

    #[test]
    fn test_infer_gt_comparison() {
        let mut infer = Infer::new();
        let gt = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(5))),
            crate::ast::BinOp::Gt,
            Box::new(mk_expr(ExprKind::IntLit(3))),
        ));
        assert_eq!(infer.infer_expr(&gt), Type::Bool);
    }

    #[test]
    fn test_infer_ne_comparison() {
        let mut infer = Infer::new();
        let ne = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(1))),
            crate::ast::BinOp::Ne,
            Box::new(mk_expr(ExprKind::IntLit(2))),
        ));
        assert_eq!(infer.infer_expr(&ne), Type::Bool);
    }

    #[test]
    fn test_fresh_var_unique() {
        let mut infer = Infer::new();
        let v1 = infer.fresh_var();
        let v2 = infer.fresh_var();
        let v3 = infer.fresh_var();
        // Each should be a different Var
        assert_ne!(v1, v2);
        assert_ne!(v2, v3);
    }

    #[test]
    fn test_unify_error_type() {
        let mut infer = Infer::new();
        // Error type should unify with anything (to prevent cascading errors)
        assert!(infer.unify(&Type::Error, &Type::Int));
        assert!(infer.unify(&Type::Str, &Type::Error));
    }

    #[test]
    fn test_unify_unit_types() {
        let mut infer = Infer::new();
        assert!(infer.unify(&Type::Unit, &Type::Unit));
        assert!(!infer.unify(&Type::Unit, &Type::Int));
    }

    // ── Unification: transitive variable chains ─────────────────

    #[test]
    fn test_unify_transitive_vars() {
        // a ~ b, b ~ Int  →  a resolves to Int
        let mut infer = Infer::new();
        let a = infer.fresh_var();
        let b = infer.fresh_var();
        assert!(infer.unify(&a, &b));
        assert!(infer.unify(&b, &Type::Int));
        assert_eq!(infer.apply(&a), Type::Int);
        assert_eq!(infer.apply(&b), Type::Int);
    }

    #[test]
    fn test_unify_var_both_directions() {
        // Int ~ a  should work the same as a ~ Int
        let mut infer = Infer::new();
        let a = infer.fresh_var();
        assert!(infer.unify(&Type::Str, &a));
        assert_eq!(infer.apply(&a), Type::Str);
    }

    #[test]
    fn test_unify_function_types_same() {
        let mut infer = Infer::new();
        let f1 = Type::Function(vec![Type::Int], Box::new(Type::Bool));
        let f2 = Type::Function(vec![Type::Int], Box::new(Type::Bool));
        assert!(infer.unify(&f1, &f2));
    }

    #[test]
    fn test_unify_function_types_return_mismatch() {
        let mut infer = Infer::new();
        let f1 = Type::Function(vec![Type::Int], Box::new(Type::Bool));
        let f2 = Type::Function(vec![Type::Int], Box::new(Type::Str));
        assert!(!infer.unify(&f1, &f2));
    }

    #[test]
    fn test_unify_function_types_arity_mismatch() {
        let mut infer = Infer::new();
        let f1 = Type::Function(vec![Type::Int], Box::new(Type::Bool));
        let f2 = Type::Function(vec![Type::Int, Type::Int], Box::new(Type::Bool));
        assert!(!infer.unify(&f1, &f2));
    }

    #[test]
    fn test_unify_function_with_vars() {
        // fn(a) -> b  ~  fn(Int) -> Bool  →  a=Int, b=Bool
        let mut infer = Infer::new();
        let a = infer.fresh_var();
        let b = infer.fresh_var();
        let f1 = Type::Function(vec![a.clone()], Box::new(b.clone()));
        let f2 = Type::Function(vec![Type::Int], Box::new(Type::Bool));
        assert!(infer.unify(&f1, &f2));
        assert_eq!(infer.apply(&a), Type::Int);
        assert_eq!(infer.apply(&b), Type::Bool);
    }

    #[test]
    fn test_unify_tuple_types_equal() {
        let mut infer = Infer::new();
        let t1 = Type::Tuple(vec![Type::Int, Type::Str]);
        let t2 = Type::Tuple(vec![Type::Int, Type::Str]);
        assert!(infer.unify(&t1, &t2));
    }

    #[test]
    fn test_unify_tuple_length_differs() {
        let mut infer = Infer::new();
        let t1 = Type::Tuple(vec![Type::Int]);
        let t2 = Type::Tuple(vec![Type::Int, Type::Str]);
        assert!(!infer.unify(&t1, &t2));
    }

    #[test]
    fn test_unify_tuple_element_mismatch() {
        let mut infer = Infer::new();
        let t1 = Type::Tuple(vec![Type::Int, Type::Str]);
        let t2 = Type::Tuple(vec![Type::Int, Type::Bool]);
        assert!(!infer.unify(&t1, &t2));
    }

    #[test]
    fn test_unify_nested_list() {
        // List<List<a>>  ~  List<List<Int>>
        let mut infer = Infer::new();
        let a = infer.fresh_var();
        let t1 = Type::List(Box::new(Type::List(Box::new(a.clone()))));
        let t2 = Type::List(Box::new(Type::List(Box::new(Type::Int))));
        assert!(infer.unify(&t1, &t2));
        assert_eq!(infer.apply(&a), Type::Int);
    }

    #[test]
    fn test_unify_named_type_param_count_mismatch() {
        let mut infer = Infer::new();
        let t1 = Type::Named("Result".to_string(), vec![Type::Int]);
        let t2 = Type::Named("Result".to_string(), vec![Type::Int, Type::Str]);
        assert!(!infer.unify(&t1, &t2));
    }

    // ── Error type absorbs mismatches ───────────────────────────

    #[test]
    fn test_error_unifies_with_function() {
        let mut infer = Infer::new();
        let f = Type::Function(vec![Type::Int], Box::new(Type::Str));
        assert!(infer.unify(&Type::Error, &f));
    }

    #[test]
    fn test_error_unifies_with_list() {
        let mut infer = Infer::new();
        assert!(infer.unify(&Type::Error, &Type::List(Box::new(Type::Int))));
    }

    #[test]
    fn test_error_unifies_with_tuple() {
        let mut infer = Infer::new();
        assert!(infer.unify(&Type::Error, &Type::Tuple(vec![Type::Int, Type::Str])));
    }

    // ── Let-polymorphism: multiple instantiations ───────────────

    #[test]
    fn test_let_polymorphism_pair() {
        // A polymorphic function fn(x) => (x, x) should instantiate differently
        let mut infer = Infer::new();
        let lambda = mk_expr(ExprKind::Lambda(
            vec![mk_param("x", None)],
            None,
            Box::new(mk_expr(ExprKind::Tuple(vec![
                mk_expr(ExprKind::Ident("x".to_string())),
                mk_expr(ExprKind::Ident("x".to_string())),
            ]))),
            false,
        ));
        let ty = infer.infer_expr(&lambda);
        let scheme = infer.generalize(&ty);
        // Instantiate and use with Int
        let inst = infer.instantiate(&scheme);
        assert!(infer.unify(
            &inst,
            &Type::Function(vec![Type::Int], Box::new(Type::Tuple(vec![Type::Int, Type::Int])))
        ));
        // Instantiate again and use with Str
        let inst2 = infer.instantiate(&scheme);
        assert!(infer.unify(
            &inst2,
            &Type::Function(vec![Type::Str], Box::new(Type::Tuple(vec![Type::Str, Type::Str])))
        ));
        assert!(infer.errors.is_empty());
    }

    // ── Scope isolation ─────────────────────────────────────────

    #[test]
    fn test_scope_isolation() {
        let mut infer = Infer::new();
        infer.push_scope();
        infer.infer_pattern(&Pattern::Ident("x".to_string()), &Type::Int);
        assert_eq!(infer.lookup_var("x"), Some(Type::Int));
        infer.pop_scope();
        // After popping, x should not be visible
        assert!(infer.lookup_var("x").is_none());
    }

    #[test]
    fn test_nested_scopes() {
        let mut infer = Infer::new();
        infer.push_scope();
        infer.infer_pattern(&Pattern::Ident("x".to_string()), &Type::Int);
        infer.push_scope();
        infer.infer_pattern(&Pattern::Ident("y".to_string()), &Type::Str);
        assert_eq!(infer.lookup_var("x"), Some(Type::Int));
        assert_eq!(infer.lookup_var("y"), Some(Type::Str));
        infer.pop_scope();
        assert_eq!(infer.lookup_var("x"), Some(Type::Int));
        assert!(infer.lookup_var("y").is_none());
        infer.pop_scope();
    }

    #[test]
    fn test_inner_scope_shadows_outer() {
        let mut infer = Infer::new();
        infer.push_scope();
        infer.infer_pattern(&Pattern::Ident("x".to_string()), &Type::Int);
        infer.push_scope();
        infer.infer_pattern(&Pattern::Ident("x".to_string()), &Type::Str);
        assert_eq!(infer.lookup_var("x"), Some(Type::Str));
        infer.pop_scope();
        assert_eq!(infer.lookup_var("x"), Some(Type::Int));
        infer.pop_scope();
    }

    // ── Multi-function programs ─────────────────────────────────

    #[test]
    fn test_function_calls_another() {
        let double = mk_fn(
            "double",
            vec![mk_param("x", Some(int_ty()))],
            Some(int_ty()),
            mk_expr(ExprKind::BinOp(
                Box::new(mk_expr(ExprKind::Ident("x".to_string()))),
                BinOp::Mul,
                Box::new(mk_expr(ExprKind::IntLit(2))),
            )),
        );
        let main_fn = mk_fn(
            "main",
            vec![],
            Some(int_ty()),
            mk_expr(ExprKind::Call(
                Box::new(mk_expr(ExprKind::Ident("double".to_string()))),
                vec![mk_expr(ExprKind::IntLit(5))],
            )),
        );
        let program = Program {
            items: vec![Item::Function(double), Item::Function(main_fn)],
        };
        let result = check(program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_wrong_arg_type() {
        let greet = mk_fn(
            "greet",
            vec![mk_param("name", Some(str_ty()))],
            Some(str_ty()),
            mk_expr(ExprKind::Ident("name".to_string())),
        );
        let main_fn = mk_fn(
            "main",
            vec![],
            Some(str_ty()),
            mk_expr(ExprKind::Call(
                Box::new(mk_expr(ExprKind::Ident("greet".to_string()))),
                vec![mk_expr(ExprKind::IntLit(42))],  // Int where String expected
            )),
        );
        let program = Program {
            items: vec![Item::Function(greet), Item::Function(main_fn)],
        };
        let result = check(program);
        assert!(result.is_err());
    }

    #[test]
    fn test_function_wrong_arg_count() {
        let add = mk_fn(
            "add",
            vec![mk_param("a", Some(int_ty())), mk_param("b", Some(int_ty()))],
            Some(int_ty()),
            mk_expr(ExprKind::BinOp(
                Box::new(mk_expr(ExprKind::Ident("a".to_string()))),
                BinOp::Add,
                Box::new(mk_expr(ExprKind::Ident("b".to_string()))),
            )),
        );
        let main_fn = mk_fn(
            "main",
            vec![],
            Some(int_ty()),
            mk_expr(ExprKind::Call(
                Box::new(mk_expr(ExprKind::Ident("add".to_string()))),
                vec![mk_expr(ExprKind::IntLit(1))],  // 1 arg instead of 2
            )),
        );
        let program = Program {
            items: vec![Item::Function(add), Item::Function(main_fn)],
        };
        let result = check(program);
        assert!(result.is_err());
    }

    // ── Match expression type inference ─────────────────────────

    #[test]
    fn test_match_all_arms_same_type() {
        let mut infer = Infer::new();
        infer.push_scope();
        infer.infer_pattern(&Pattern::Ident("x".to_string()), &Type::Int);
        let expr = mk_expr(ExprKind::Match(
            Box::new(mk_expr(ExprKind::Ident("x".to_string()))),
            vec![
                MatchArm {
                    pattern: Pattern::IntLit(0),
                    body: mk_expr(ExprKind::StringLit("zero".to_string())),
                    guard: None,
                    span: span(),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    body: mk_expr(ExprKind::StringLit("other".to_string())),
                    guard: None,
                    span: span(),
                },
            ],
        ));
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Str);
        assert!(infer.errors.is_empty());
        infer.pop_scope();
    }

    #[test]
    fn test_match_arm_type_mismatch() {
        let mut infer = Infer::new();
        infer.push_scope();
        infer.infer_pattern(&Pattern::Ident("x".to_string()), &Type::Int);
        let expr = mk_expr(ExprKind::Match(
            Box::new(mk_expr(ExprKind::Ident("x".to_string()))),
            vec![
                MatchArm {
                    pattern: Pattern::IntLit(0),
                    body: mk_expr(ExprKind::StringLit("zero".to_string())),
                    guard: None,
                    span: span(),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    body: mk_expr(ExprKind::IntLit(1)),  // Int vs String
                    guard: None,
                    span: span(),
                },
            ],
        ));
        infer.infer_expr(&expr);
        assert!(!infer.errors.is_empty());
        infer.pop_scope();
    }

    // ── Struct field type checking ──────────────────────────────

    #[test]
    fn test_struct_wrong_field_type() {
        let td = TypeDecl {
            name: "Pair".to_string(),
            type_params: vec![],
            body: TypeBody::Struct(vec![
                Field { name: "a".to_string(), ty: int_ty(), span: span() },
                Field { name: "b".to_string(), ty: str_ty(), span: span() },
            ]),
            span: span(),
        };
        let expr = mk_expr(ExprKind::StructLit(
            "Pair".to_string(),
            vec![
                ("a".to_string(), mk_expr(ExprKind::IntLit(1))),
                ("b".to_string(), mk_expr(ExprKind::IntLit(2))),  // Int where String expected
            ],
            None,
        ));
        let program = Program {
            items: vec![Item::TypeDecl(td), Item::Expr(expr)],
        };
        let result = check(program);
        assert!(result.is_err());
    }

    #[test]
    fn test_struct_extra_field() {
        let td = TypeDecl {
            name: "Single".to_string(),
            type_params: vec![],
            body: TypeBody::Struct(vec![
                Field { name: "x".to_string(), ty: int_ty(), span: span() },
            ]),
            span: span(),
        };
        let expr = mk_expr(ExprKind::StructLit(
            "Single".to_string(),
            vec![
                ("x".to_string(), mk_expr(ExprKind::IntLit(1))),
                ("y".to_string(), mk_expr(ExprKind::IntLit(2))),  // extra field
            ],
            None,
        ));
        let program = Program {
            items: vec![Item::TypeDecl(td), Item::Expr(expr)],
        };
        let result = check(program);
        assert!(result.is_err());
    }

    // ── If without else returns unit ────────────────────────────

    #[test]
    fn test_if_no_else_returns_unit() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::If(
            Box::new(mk_expr(ExprKind::BoolLit(true))),
            Box::new(mk_expr(ExprKind::IntLit(1))),
            None,
        ));
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Unit);
    }

    // ── Levenshtein "did you mean?" suggestions ─────────────────

    #[test]
    fn test_did_you_mean_suggestion() {
        // Misspelling "println" as "prinln" should produce a suggestion
        let f = mk_fn(
            "main",
            vec![],
            None,
            mk_expr(ExprKind::Call(
                Box::new(mk_expr(ExprKind::Ident("prinln".to_string()))),
                vec![mk_expr(ExprKind::StringLit("hi".to_string()))],
            )),
        );
        let program = Program {
            items: vec![Item::Function(f)],
        };
        let result = check(program);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("println"), "Should suggest 'println', got: {err}");
    }

    // ── Tuple pattern with wrong arity ──────────────────────────

    #[test]
    fn test_tuple_pattern_correct_arity() {
        let mut infer = Infer::new();
        infer.push_scope();
        let scrut_ty = Type::Tuple(vec![Type::Int, Type::Str]);
        infer.infer_pattern(
            &Pattern::Tuple(vec![
                Pattern::Ident("a".to_string()),
                Pattern::Ident("b".to_string()),
            ]),
            &scrut_ty,
        );
        assert_eq!(infer.lookup_var("a"), Some(Type::Int));
        assert_eq!(infer.lookup_var("b"), Some(Type::Str));
        assert!(infer.errors.is_empty());
        infer.pop_scope();
    }

    // ── Constructor pattern binds variables ──────────────────────

    #[test]
    fn test_constructor_pattern_binds() {
        let td = TypeDecl {
            name: "Wrapper".to_string(),
            type_params: vec![],
            body: TypeBody::Enum(vec![
                Variant {
                    name: "Wrap".to_string(),
                    fields: vec![TypeExpr::Named("Int".to_string(), vec![])],
                    span: span(),
                },
            ]),
            span: span(),
        };
        let f = mk_fn(
            "unwrap",
            vec![mk_param("w", Some(TypeExpr::Named("Wrapper".to_string(), vec![])))],
            Some(int_ty()),
            mk_expr(ExprKind::Match(
                Box::new(mk_expr(ExprKind::Ident("w".to_string()))),
                vec![MatchArm {
                    pattern: Pattern::Constructor("Wrap".to_string(), vec![Pattern::Ident("x".to_string())]),
                    body: mk_expr(ExprKind::Ident("x".to_string())),
                    guard: None,
                    span: span(),
                }],
            )),
        );
        let program = Program {
            items: vec![Item::TypeDecl(td), Item::Function(f)],
        };
        let result = check(program);
        assert!(result.is_ok());
    }

    // ── Bitwise operators return Int ────────────────────────────

    #[test]
    fn test_bitwise_and_returns_int() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(0xFF))),
            BinOp::Band,
            Box::new(mk_expr(ExprKind::IntLit(0x0F))),
        ));
        assert_eq!(infer.infer_expr(&expr), Type::Int);
    }

    #[test]
    fn test_bitwise_or_returns_int() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(0xA0))),
            BinOp::Bor,
            Box::new(mk_expr(ExprKind::IntLit(0x05))),
        ));
        assert_eq!(infer.infer_expr(&expr), Type::Int);
    }

    #[test]
    fn test_bitwise_xor_returns_int() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(0xFF))),
            BinOp::Bxor,
            Box::new(mk_expr(ExprKind::IntLit(0x0F))),
        ));
        assert_eq!(infer.infer_expr(&expr), Type::Int);
    }

    #[test]
    fn test_shift_left_returns_int() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(1))),
            BinOp::Shl,
            Box::new(mk_expr(ExprKind::IntLit(4))),
        ));
        assert_eq!(infer.infer_expr(&expr), Type::Int);
    }

    #[test]
    fn test_shift_right_returns_int() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::IntLit(16))),
            BinOp::Shr,
            Box::new(mk_expr(ExprKind::IntLit(2))),
        ));
        assert_eq!(infer.infer_expr(&expr), Type::Int);
    }

    // ── Float arithmetic ────────────────────────────────────────

    #[test]
    fn test_float_mul() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::FloatLit(2.5))),
            BinOp::Mul,
            Box::new(mk_expr(ExprKind::FloatLit(4.0))),
        ));
        assert_eq!(infer.infer_expr(&expr), Type::Float);
    }

    #[test]
    fn test_float_div() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::FloatLit(10.0))),
            BinOp::Div,
            Box::new(mk_expr(ExprKind::FloatLit(3.0))),
        ));
        assert_eq!(infer.infer_expr(&expr), Type::Float);
    }

    #[test]
    fn test_float_comparison_returns_bool() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::BinOp(
            Box::new(mk_expr(ExprKind::FloatLit(1.5))),
            BinOp::Le,
            Box::new(mk_expr(ExprKind::FloatLit(2.5))),
        ));
        assert_eq!(infer.infer_expr(&expr), Type::Bool);
    }

    // ── Negate float ────────────────────────────────────────────

    #[test]
    fn test_unary_neg_float() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::UnaryOp(
            UnaryOp::Neg,
            Box::new(mk_expr(ExprKind::FloatLit(3.14))),
        ));
        assert_eq!(infer.infer_expr(&expr), Type::Float);
    }

    // ── Empty list ──────────────────────────────────────────────

    #[test]
    fn test_empty_list_infers_var() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::ListLit(vec![]));
        let ty = infer.infer_expr(&expr);
        match infer.apply(&ty) {
            Type::List(_) => {} // Good — List<somevar>
            other => panic!("Expected List type, got {:?}", other),
        }
    }

    // ── String interpolation ────────────────────────────────────

    #[test]
    fn test_string_interp_returns_string() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::StringInterp(vec![
            StringPart::Lit("x is ".to_string()),
            StringPart::Expr(mk_expr(ExprKind::IntLit(42))),
        ]));
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Str);
    }

    // ── Try expression ──────────────────────────────────────────

    #[test]
    fn test_try_expr_infers() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::Try(
            Box::new(mk_expr(ExprKind::IntLit(42))),
        ));
        // Try should produce some type (not crash)
        let _ty = infer.infer_expr(&expr);
    }

    // ── Break and Continue ──────────────────────────────────────

    #[test]
    fn test_break_is_unit() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::Break);
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Unit);
    }

    #[test]
    fn test_continue_is_unit() {
        let mut infer = Infer::new();
        let expr = mk_expr(ExprKind::Continue);
        let ty = infer.infer_expr(&expr);
        assert_eq!(ty, Type::Unit);
    }

    // ── Trait declarations type check ───────────────────────────

    #[test]
    fn test_trait_and_impl_ok() {
        let program = Program {
            items: vec![
                Item::TypeDecl(TypeDecl {
                    name: "Dog".to_string(),
                    type_params: vec![],
                    body: TypeBody::Struct(vec![
                        Field { name: "name".to_string(), ty: str_ty(), span: span() },
                    ]),
                    span: span(),
                }),
                Item::TraitDecl(TraitDecl {
                    name: "Speak".to_string(),
                    type_params: vec![],
                    methods: vec![TraitMethod {
                        name: "speak".to_string(),
                        params: vec![mk_param("self", None)],
                        return_type: Some(str_ty()),
                        default_body: None,
                        span: span(),
                    }],
                    associated_types: vec![],
                    span: span(),
                }),
                Item::ImplBlock(ImplBlock {
                    type_name: "Dog".to_string(),
                    trait_name: Some("Speak".to_string()),
                    type_params: vec![],
                    methods: vec![Function {
                        name: "speak".to_string(),
                        params: vec![mk_param("self", None)],
                        return_type: Some(str_ty()),
                        body: mk_expr(ExprKind::StringLit("Woof!".to_string())),
                        is_pub: false,
                        is_async: false,
                        type_params: vec![],
                        annotations: vec![],
                        span: span(),
                    }],
                    associated_types: vec![],
                    span: span(),
                }),
            ],
        };
        let result = check(program);
        assert!(result.is_ok());
    }

    // ── Multiple enum variants type check ───────────────────────

    #[test]
    fn test_enum_multiple_variant_constructors() {
        let td = TypeDecl {
            name: "Shape".to_string(),
            type_params: vec![],
            body: TypeBody::Enum(vec![
                Variant { name: "Circle".to_string(), fields: vec![TypeExpr::Named("Float".to_string(), vec![])], span: span() },
                Variant { name: "Rect".to_string(), fields: vec![TypeExpr::Named("Float".to_string(), vec![]), TypeExpr::Named("Float".to_string(), vec![])], span: span() },
                Variant { name: "Point".to_string(), fields: vec![], span: span() },
            ]),
            span: span(),
        };
        // Circle(1.0) should typecheck
        let expr1 = mk_expr(ExprKind::Call(
            Box::new(mk_expr(ExprKind::Ident("Circle".to_string()))),
            vec![mk_expr(ExprKind::FloatLit(5.0))],
        ));
        let program = Program {
            items: vec![Item::TypeDecl(td), Item::Expr(expr1)],
        };
        let result = check(program);
        assert!(result.is_ok());
    }

    // ── Const declaration type check ────────────────────────────

    #[test]
    fn test_const_type_ok() {
        let program = Program {
            items: vec![Item::Const(ConstDecl {
                name: "X".to_string(),
                ty: Some(int_ty()),
                value: mk_expr(ExprKind::IntLit(42)),
                is_pub: false,
                span: span(),
            })],
        };
        let result = check(program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_const_infers_type() {
        let program = Program {
            items: vec![Item::Const(ConstDecl {
                name: "Y".to_string(),
                ty: None,
                value: mk_expr(ExprKind::StringLit("hello".to_string())),
                is_pub: false,
                span: span(),
            })],
        };
        let result = check(program);
        assert!(result.is_ok());
    }
}
