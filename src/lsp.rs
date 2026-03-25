use lsp_server::{Connection, ExtractError, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::Notification as _;
use lsp_types::*;
use std::collections::HashMap;

use crate::{error, formatter, lexer, parser, resolver, typeck};

// ── Server State ────────────────────────────────────────────────────────────

struct ServerState {
    /// Open documents: URI → (source text, version)
    documents: HashMap<Uri, (String, i32)>,
    /// Cached analysis results per document
    analysis: HashMap<Uri, typeck::AnalysisResult>,
}

impl ServerState {
    fn new() -> Self {
        Self {
            documents: HashMap::new(),
            analysis: HashMap::new(),
        }
    }
}

// ── Entry Point ─────────────────────────────────────────────────────────────

pub fn run() {
    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::FULL,
        )),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
            ..Default::default()
        }),
        definition_provider: Some(OneOf::Left(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
            retrigger_characters: Some(vec![",".to_string()]),
            ..Default::default()
        }),
        inlay_hint_provider: Some(OneOf::Left(true)),
        semantic_tokens_provider: Some(
            SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                legend: SemanticTokensLegend {
                    token_types: SEMANTIC_TOKEN_TYPES.to_vec(),
                    token_modifiers: vec![],
                },
                full: Some(SemanticTokensFullOptions::Bool(true)),
                range: None,
                ..Default::default()
            }),
        ),
        ..Default::default()
    })
    .unwrap();

    let init_params = match connection.initialize(server_capabilities) {
        Ok(params) => params,
        Err(e) => {
            eprintln!("LSP initialization error: {e}");
            return;
        }
    };
    let _init_params: InitializeParams = serde_json::from_value(init_params).unwrap();

    let mut state = ServerState::new();
    main_loop(&connection, &mut state);

    io_threads.join().unwrap();
}

// ── Main Loop ───────────────────────────────────────────────────────────────

fn main_loop(connection: &Connection, state: &mut ServerState) {
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req).unwrap_or(false) {
                    return;
                }
                handle_request(connection, state, req);
            }
            Message::Notification(not) => {
                handle_notification(connection, state, not);
            }
            Message::Response(_) => {}
        }
    }
}

// ── Request Handling ────────────────────────────────────────────────────────

fn handle_request(connection: &Connection, state: &mut ServerState, req: Request) {
    let req = match cast_request::<request::HoverRequest>(req) {
        Ok((id, params)) => {
            let result = handle_hover(state, &params);
            let resp = Response::new_ok(id, result);
            connection.sender.send(Message::Response(resp)).unwrap();
            return;
        }
        Err(req) => req,
    };

    let req = match cast_request::<request::GotoDefinition>(req) {
        Ok((id, params)) => {
            let result = handle_goto_definition(state, &params);
            let resp = Response::new_ok(id, result);
            connection.sender.send(Message::Response(resp)).unwrap();
            return;
        }
        Err(req) => req,
    };

    let req = match cast_request::<request::Completion>(req) {
        Ok((id, params)) => {
            let result = handle_completion(state, &params);
            let resp = Response::new_ok(id, result);
            connection.sender.send(Message::Response(resp)).unwrap();
            return;
        }
        Err(req) => req,
    };

    let req = match cast_request::<request::Formatting>(req) {
        Ok((id, params)) => {
            let result = handle_formatting(state, &params);
            let resp = Response::new_ok(id, result);
            connection.sender.send(Message::Response(resp)).unwrap();
            return;
        }
        Err(req) => req,
    };

    let req = match cast_request::<request::DocumentSymbolRequest>(req) {
        Ok((id, params)) => {
            let result = handle_document_symbols(state, &params);
            let resp = Response::new_ok(id, result);
            connection.sender.send(Message::Response(resp)).unwrap();
            return;
        }
        Err(req) => req,
    };

    let req = match cast_request::<request::SignatureHelpRequest>(req) {
        Ok((id, params)) => {
            let result = handle_signature_help(state, &params);
            let resp = Response::new_ok(id, result);
            connection.sender.send(Message::Response(resp)).unwrap();
            return;
        }
        Err(req) => req,
    };

    let req = match cast_request::<request::InlayHintRequest>(req) {
        Ok((id, params)) => {
            let result = handle_inlay_hints(state, &params);
            let resp = Response::new_ok(id, result);
            connection.sender.send(Message::Response(resp)).unwrap();
            return;
        }
        Err(req) => req,
    };

    let _req = match cast_request::<request::SemanticTokensFullRequest>(req) {
        Ok((id, params)) => {
            let result = handle_semantic_tokens(state, &params);
            let resp = Response::new_ok(id, result);
            connection.sender.send(Message::Response(resp)).unwrap();
            return;
        }
        Err(req) => req,
    };
}

// ── Notification Handling ───────────────────────────────────────────────────

fn handle_notification(connection: &Connection, state: &mut ServerState, not: Notification) {
    if let Ok(params) = cast_notification::<notification::DidOpenTextDocument>(&not) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        let version = params.text_document.version;
        state.documents.insert(uri.clone(), (text, version));
        run_analysis(connection, state, &uri);
        return;
    }

    if let Ok(params) = cast_notification::<notification::DidChangeTextDocument>(&not) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        // Full sync: take the last content change
        if let Some(change) = params.content_changes.into_iter().last() {
            state.documents.insert(uri.clone(), (change.text, version));
            run_analysis(connection, state, &uri);
        }
        return;
    }

    if let Ok(params) = cast_notification::<notification::DidCloseTextDocument>(&not) {
        let uri = params.text_document.uri;
        state.documents.remove(&uri);
        state.analysis.remove(&uri);
        // Clear diagnostics for closed document
        publish_diagnostics(connection, &uri, vec![]);
    }
}

// ── Analysis Pipeline ───────────────────────────────────────────────────────

fn run_analysis(connection: &Connection, state: &mut ServerState, uri: &Uri) {
    let Some((source, _version)) = state.documents.get(uri) else {
        return;
    };
    let source: String = source.clone();
    let file_path = uri_to_path(uri);

    // Lex
    let tokens = match lexer::lex(&source) {
        Ok(tokens) => tokens,
        Err(e) => {
            let diag = parse_error_to_diagnostic(&e);
            publish_diagnostics(connection, uri, vec![diag]);
            return;
        }
    };

    // Parse
    let (program, _comments) = match parser::parse(tokens) {
        Ok(result) => result,
        Err(e) => {
            let diagnostics: Vec<Diagnostic> = e
                .lines()
                .map(|line| parse_error_to_diagnostic(line))
                .collect();
            publish_diagnostics(connection, uri, diagnostics);
            return;
        }
    };

    // Resolve (best-effort: if it fails, analyze what we have)
    let program = match resolver::resolve(program, &file_path) {
        Ok(p) => p,
        Err(errors) => {
            let diagnostics: Vec<Diagnostic> = errors
                .iter()
                .map(|e| parse_error_to_diagnostic(e))
                .collect();
            publish_diagnostics(connection, uri, diagnostics);
            return;
        }
    };

    // Type check / analyze
    let result = typeck::analyze(&program);

    // Publish diagnostics
    let mut diagnostics = Vec::new();
    for (span, msg) in &result.errors {
        diagnostics.push(Diagnostic {
            range: span_to_range(*span),
            severity: Some(DiagnosticSeverity::ERROR),
            message: msg.clone(),
            ..Default::default()
        });
    }
    for (span, msg) in &result.warnings {
        diagnostics.push(Diagnostic {
            range: span_to_range(*span),
            severity: Some(DiagnosticSeverity::WARNING),
            message: msg.clone(),
            ..Default::default()
        });
    }
    publish_diagnostics(connection, uri, diagnostics);

    state.analysis.insert(uri.clone(), result);
}

fn publish_diagnostics(connection: &Connection, uri: &Uri, diagnostics: Vec<Diagnostic>) {
    let params = PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics,
        version: None,
    };
    let not = Notification::new(
        notification::PublishDiagnostics::METHOD.to_string(),
        params,
    );
    connection.sender.send(Message::Notification(not)).unwrap();
}

// ── Hover ───────────────────────────────────────────────────────────────────

fn handle_hover(state: &ServerState, params: &HoverParams) -> Option<Hover> {
    let uri = &params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;

    let (source, _) = state.documents.get(uri)?;
    let analysis = state.analysis.get(uri)?;

    // Extract the word under the cursor from source text
    let word = word_at_position(source, pos)?;

    // Look up in type_at entries: find one matching the name and closest to cursor line
    let lsp_line = pos.line as usize + 1; // LSP is 0-indexed, Star spans are 1-indexed
    let mut best: Option<&(error::Span, String, String)> = None;
    let mut best_dist = usize::MAX;
    for entry in &analysis.type_at {
        if entry.1 == word {
            let dist = if entry.0.line >= lsp_line {
                entry.0.line - lsp_line
            } else {
                lsp_line - entry.0.line
            };
            if dist < best_dist {
                best_dist = dist;
                best = Some(entry);
            }
        }
    }

    if let Some((_span, name, ty)) = best {
        let contents = format!("```star\n{name}: {ty}\n```");
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: contents,
            }),
            range: None,
        })
    } else if analysis.builtin_names.contains(&word) {
        let contents = format!("```star\n{word} (builtin)\n```");
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: contents,
            }),
            range: None,
        })
    } else {
        None
    }
}

// ── Go-to-Definition ────────────────────────────────────────────────────────

fn handle_goto_definition(
    state: &ServerState,
    params: &GotoDefinitionParams,
) -> Option<GotoDefinitionResponse> {
    let uri = &params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;

    let (source, _) = state.documents.get(uri)?;
    let analysis = state.analysis.get(uri)?;

    let word = word_at_position(source, pos)?;

    // Find the definition in the same file
    for def in &analysis.definitions {
        if def.name == word {
            let range = span_to_range(def.span);
            return Some(GotoDefinitionResponse::Scalar(Location {
                uri: uri.clone(),
                range,
            }));
        }
    }
    None
}

// ── Completions ─────────────────────────────────────────────────────────────

fn handle_completion(
    state: &ServerState,
    params: &CompletionParams,
) -> Option<CompletionResponse> {
    let uri = &params.text_document_position.text_document.uri;
    let analysis = state.analysis.get(uri);

    let mut items: Vec<CompletionItem> = Vec::new();

    // Builtins
    if let Some(a) = analysis {
        for name in &a.builtin_names {
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("builtin".to_string()),
                ..Default::default()
            });
        }

        // User-defined symbols
        for def in &a.definitions {
            let kind = match def.kind {
                typeck::SymbolKind::Function => CompletionItemKind::FUNCTION,
                typeck::SymbolKind::Type => CompletionItemKind::CLASS,
                typeck::SymbolKind::Constructor => CompletionItemKind::ENUM_MEMBER,
                typeck::SymbolKind::Constant => CompletionItemKind::CONSTANT,
                typeck::SymbolKind::Module => CompletionItemKind::MODULE,
                typeck::SymbolKind::Trait => CompletionItemKind::INTERFACE,
            };
            items.push(CompletionItem {
                label: def.name.clone(),
                kind: Some(kind),
                detail: def.detail.clone(),
                ..Default::default()
            });
        }

        // Type names
        for name in &a.type_names {
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(CompletionItemKind::CLASS),
                ..Default::default()
            });
        }

        // Constructor names
        for name in &a.constructor_names {
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                ..Default::default()
            });
        }
    }

    // Keywords
    let keywords = [
        "fn", "let", "mut", "if", "else", "match", "do", "end", "type", "mod", "use",
        "pub", "trait", "impl", "for", "in", "while", "break", "continue", "true", "false",
        "async", "await", "extern", "return",
    ];
    for kw in keywords {
        items.push(CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        });
    }

    Some(CompletionResponse::Array(items))
}

// ── Document Formatting ─────────────────────────────────────────────────────

fn handle_formatting(
    state: &ServerState,
    params: &DocumentFormattingParams,
) -> Option<Vec<TextEdit>> {
    let uri = &params.text_document.uri;
    let (source, _) = state.documents.get(uri)?;

    // Try to lex + parse; if that fails, can't format
    let tokens = lexer::lex(source).ok()?;
    let (program, comments) = parser::parse(tokens).ok()?;

    let formatted = formatter::format(&program, &comments);

    // Return a single edit replacing the entire document
    let line_count = source.lines().count() as u32;
    let last_line_len = source.lines().last().map_or(0, |l| l.len()) as u32;
    Some(vec![TextEdit {
        range: Range {
            start: Position { line: 0, character: 0 },
            end: Position {
                line: line_count,
                character: last_line_len,
            },
        },
        new_text: formatted,
    }])
}

// ── Document Symbols ────────────────────────────────────────────────────────

#[allow(deprecated)] // SymbolInformation::deprecated field
fn handle_document_symbols(
    state: &ServerState,
    params: &DocumentSymbolParams,
) -> Option<DocumentSymbolResponse> {
    let uri = &params.text_document.uri;
    let analysis = state.analysis.get(uri)?;

    let symbols: Vec<SymbolInformation> = analysis
        .definitions
        .iter()
        .map(|def| {
            let kind = match def.kind {
                typeck::SymbolKind::Function => lsp_types::SymbolKind::FUNCTION,
                typeck::SymbolKind::Type => lsp_types::SymbolKind::CLASS,
                typeck::SymbolKind::Constructor => lsp_types::SymbolKind::ENUM_MEMBER,
                typeck::SymbolKind::Constant => lsp_types::SymbolKind::CONSTANT,
                typeck::SymbolKind::Module => lsp_types::SymbolKind::MODULE,
                typeck::SymbolKind::Trait => lsp_types::SymbolKind::INTERFACE,
            };
            #[allow(deprecated)]
            SymbolInformation {
                name: def.name.clone(),
                kind,
                tags: None,
                deprecated: None,
                location: Location {
                    uri: uri.clone(),
                    range: span_to_range(def.span),
                },
                container_name: None,
            }
        })
        .collect();

    Some(DocumentSymbolResponse::Flat(symbols))
}

// ── Signature Help ──────────────────────────────────────────────────────

fn handle_signature_help(
    state: &ServerState,
    params: &SignatureHelpParams,
) -> Option<SignatureHelp> {
    let uri = &params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;

    let (source, _) = state.documents.get(uri)?;
    let analysis = state.analysis.get(uri)?;

    // Find the function name at or before the cursor on the current line
    let line_text = source.lines().nth(pos.line as usize)?;
    let col = pos.character as usize;
    let before_cursor = &line_text[..col.min(line_text.len())];

    // Walk backwards from cursor to find the function call context
    // Look for the nearest unmatched '(' to find the function name
    let mut depth = 0i32;
    let mut paren_pos = None;
    for (i, ch) in before_cursor.char_indices().rev() {
        match ch {
            ')' => depth += 1,
            '(' => {
                if depth == 0 {
                    paren_pos = Some(i);
                    break;
                }
                depth -= 1;
            }
            _ => {}
        }
    }

    let paren_idx = paren_pos?;
    // Extract function name before the '('
    let before_paren = before_cursor[..paren_idx].trim_end();
    let func_name: String = before_paren
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .chars()
        .rev()
        .collect();

    if func_name.is_empty() {
        return None;
    }

    // Count active parameter (number of commas at depth 0 after the open paren)
    let after_paren = &before_cursor[paren_idx + 1..];
    let mut active_param = 0u32;
    let mut d = 0i32;
    for ch in after_paren.chars() {
        match ch {
            '(' | '[' | '{' => d += 1,
            ')' | ']' | '}' => d -= 1,
            ',' if d == 0 => active_param += 1,
            _ => {}
        }
    }

    // Look up the function in definitions
    for def in &analysis.definitions {
        if def.name == func_name {
            if let Some(detail) = &def.detail {
                let sig = SignatureInformation {
                    label: detail.clone(),
                    documentation: None,
                    parameters: extract_params(detail),
                    active_parameter: Some(active_param),
                };
                return Some(SignatureHelp {
                    signatures: vec![sig],
                    active_signature: Some(0),
                    active_parameter: Some(active_param),
                });
            }
        }
    }

    // Check builtins — provide basic signature from the builtin arity table
    if analysis.builtin_names.contains(&func_name) {
        let sig = SignatureInformation {
            label: format!("{func_name}(...)"),
            documentation: Some(Documentation::String("builtin function".to_string())),
            parameters: None,
            active_parameter: Some(active_param),
        };
        return Some(SignatureHelp {
            signatures: vec![sig],
            active_signature: Some(0),
            active_parameter: Some(active_param),
        });
    }

    None
}

/// Extract parameter labels from a function detail string like "fn foo(a, b, c)"
fn extract_params(detail: &str) -> Option<Vec<ParameterInformation>> {
    let start = detail.find('(')?;
    let end = detail.rfind(')')?;
    let inner = &detail[start + 1..end];
    if inner.trim().is_empty() {
        return Some(vec![]);
    }
    let params: Vec<ParameterInformation> = inner
        .split(',')
        .map(|p| ParameterInformation {
            label: ParameterLabel::Simple(p.trim().to_string()),
            documentation: None,
        })
        .collect();
    Some(params)
}

// ── Inlay Hints ─────────────────────────────────────────────────────────

fn handle_inlay_hints(
    state: &ServerState,
    params: &InlayHintParams,
) -> Option<Vec<InlayHint>> {
    let uri = &params.text_document.uri;
    let analysis = state.analysis.get(uri)?;

    let mut hints = Vec::new();

    // Show type hints for let bindings that have inferred types
    for (span, name, ty) in &analysis.type_at {
        let line = span.line.saturating_sub(1) as u32; // 1-indexed → 0-indexed
        let col = span.col.saturating_sub(1) as u32;

        // Only show for the range visible in the editor request
        if line < params.range.start.line || line > params.range.end.line {
            continue;
        }

        // Skip if the type is unhelpful
        if ty == "?" || ty.starts_with('_') || ty == "()" {
            continue;
        }

        // Position the hint after the name
        let position = Position::new(line, col + name.len() as u32);

        hints.push(InlayHint {
            position,
            label: InlayHintLabel::String(format!(": {ty}")),
            kind: Some(InlayHintKind::TYPE),
            text_edits: None,
            tooltip: None,
            padding_left: None,
            padding_right: Some(true),
            data: None,
        });
    }

    Some(hints)
}

// ── Semantic Tokens ──────────────────────────────────────────────────────

const SEMANTIC_TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,       // 0
    SemanticTokenType::FUNCTION,      // 1
    SemanticTokenType::VARIABLE,      // 2
    SemanticTokenType::STRING,        // 3
    SemanticTokenType::NUMBER,        // 4
    SemanticTokenType::COMMENT,       // 5
    SemanticTokenType::TYPE,          // 6
    SemanticTokenType::OPERATOR,      // 7
    SemanticTokenType::ENUM_MEMBER,   // 8
    SemanticTokenType::MACRO,         // 9
];

const TK_KEYWORD: u32 = 0;
const TK_FUNCTION: u32 = 1;
const TK_VARIABLE: u32 = 2;
const TK_STRING: u32 = 3;
const TK_NUMBER: u32 = 4;
const TK_COMMENT: u32 = 5;
const TK_TYPE: u32 = 6;
const TK_OPERATOR: u32 = 7;
const TK_ENUM_MEMBER: u32 = 8;
const TK_MACRO: u32 = 9;

fn handle_semantic_tokens(
    state: &ServerState,
    params: &SemanticTokensParams,
) -> Option<SemanticTokensResult> {
    let uri = &params.text_document.uri;
    let (source, _) = state.documents.get(uri)?;

    let tokens = lexer::lex(source).ok()?;

    let mut data: Vec<SemanticToken> = Vec::new();
    let mut prev_line: u32 = 0;
    let mut prev_start: u32 = 0;

    for tok in &tokens {
        let (token_type, length) = match &tok.kind {
            // Keywords
            lexer::TokenKind::Fn => (TK_KEYWORD, 2),
            lexer::TokenKind::Let => (TK_KEYWORD, 3),
            lexer::TokenKind::Mut => (TK_KEYWORD, 3),
            lexer::TokenKind::If => (TK_KEYWORD, 2),
            lexer::TokenKind::Then => (TK_KEYWORD, 4),
            lexer::TokenKind::Else => (TK_KEYWORD, 4),
            lexer::TokenKind::End => (TK_KEYWORD, 3),
            lexer::TokenKind::Match => (TK_KEYWORD, 5),
            lexer::TokenKind::Type => (TK_KEYWORD, 4),
            lexer::TokenKind::Module => (TK_KEYWORD, 6),
            lexer::TokenKind::Use => (TK_KEYWORD, 3),
            lexer::TokenKind::Pub => (TK_KEYWORD, 3),
            lexer::TokenKind::Do => (TK_KEYWORD, 2),
            lexer::TokenKind::And => (TK_KEYWORD, 3),
            lexer::TokenKind::Or => (TK_KEYWORD, 2),
            lexer::TokenKind::Not => (TK_KEYWORD, 3),
            lexer::TokenKind::When => (TK_KEYWORD, 4),
            lexer::TokenKind::As => (TK_KEYWORD, 2),
            lexer::TokenKind::Async => (TK_KEYWORD, 5),
            lexer::TokenKind::Await => (TK_KEYWORD, 5),
            lexer::TokenKind::Extern => (TK_KEYWORD, 6),
            lexer::TokenKind::Trait => (TK_KEYWORD, 5),
            lexer::TokenKind::Impl => (TK_KEYWORD, 4),
            lexer::TokenKind::For => (TK_KEYWORD, 3),
            lexer::TokenKind::While => (TK_KEYWORD, 5),
            lexer::TokenKind::In => (TK_KEYWORD, 2),
            lexer::TokenKind::Break => (TK_KEYWORD, 5),
            lexer::TokenKind::Continue => (TK_KEYWORD, 8),
            lexer::TokenKind::Dyn => (TK_KEYWORD, 3),
            lexer::TokenKind::Move => (TK_KEYWORD, 4),
            lexer::TokenKind::Band => (TK_KEYWORD, 4),
            lexer::TokenKind::Bor => (TK_KEYWORD, 3),
            lexer::TokenKind::Bxor => (TK_KEYWORD, 4),
            lexer::TokenKind::True => (TK_KEYWORD, 4),
            lexer::TokenKind::False => (TK_KEYWORD, 5),

            // Literals
            lexer::TokenKind::Int(n) => (TK_NUMBER, int_display_len(*n, source, tok.span)),
            lexer::TokenKind::Float(f) => (TK_NUMBER, float_display_len(*f, source, tok.span)),
            lexer::TokenKind::Str(s) => (TK_STRING, s.len() as u32 + 2), // +2 for quotes
            lexer::TokenKind::InterpStr(parts) => {
                // Compute full interpolated string length including quotes and #{} markers
                let inner: u32 = parts.iter().map(|(is_expr, text)| {
                    if *is_expr { text.len() as u32 + 3 } else { text.len() as u32 }
                }).sum();
                (TK_STRING, inner + 2) // +2 for quotes
            }

            // Identifiers
            lexer::TokenKind::Ident(name) => (TK_VARIABLE, name.len() as u32),
            lexer::TokenKind::UpperIdent(name) => (TK_TYPE, name.len() as u32),

            // Comments
            lexer::TokenKind::Comment(text) => (TK_COMMENT, text.len() as u32 + 1), // "#" + text

            // Operators
            lexer::TokenKind::Plus | lexer::TokenKind::Minus | lexer::TokenKind::Star
            | lexer::TokenKind::Slash | lexer::TokenKind::Percent
            | lexer::TokenKind::Lt | lexer::TokenKind::Gt
            | lexer::TokenKind::Ampersand | lexer::TokenKind::Pipe
            | lexer::TokenKind::Tilde | lexer::TokenKind::Question => (TK_OPERATOR, 1),
            lexer::TokenKind::EqEq | lexer::TokenKind::Ne
            | lexer::TokenKind::Le | lexer::TokenKind::Ge
            | lexer::TokenKind::Arrow | lexer::TokenKind::FatArrow
            | lexer::TokenKind::PipeArrow | lexer::TokenKind::DotDot
            | lexer::TokenKind::ColonColon
            | lexer::TokenKind::PlusEq | lexer::TokenKind::MinusEq
            | lexer::TokenKind::StarEq | lexer::TokenKind::SlashEq
            | lexer::TokenKind::PercentEq
            | lexer::TokenKind::Shl | lexer::TokenKind::Shr => (TK_OPERATOR, 2),

            // rust! macro
            lexer::TokenKind::RustBang => (TK_MACRO, 5),

            // Skip punctuation, newlines, EOF
            _ => continue,
        };

        let line = if tok.span.line > 0 { tok.span.line as u32 - 1 } else { 0 };
        let start = if tok.span.col > 0 { tok.span.col as u32 - 1 } else { 0 };

        let delta_line = line - prev_line;
        let delta_start = if delta_line == 0 {
            start - prev_start
        } else {
            start
        };

        data.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset: 0,
        });

        prev_line = line;
        prev_start = start;
    }

    Some(SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data,
    }))
}

/// Compute display length of an integer literal by reading the source at the span position.
fn int_display_len(n: i64, source: &str, span: error::Span) -> u32 {
    if let Some(line) = source.lines().nth(span.line.wrapping_sub(1)) {
        let col = span.col.saturating_sub(1);
        if col < line.len() {
            let rest = &line[col..];
            let len = rest.bytes().take_while(|b| b.is_ascii_digit() || *b == b'_').count();
            if len > 0 {
                return len as u32;
            }
        }
    }
    // Fallback: compute decimal length
    if n == 0 { return 1; }
    let mut len = 0u32;
    let mut v = n.unsigned_abs();
    while v > 0 { len += 1; v /= 10; }
    if n < 0 { len += 1; }
    len
}

/// Compute display length of a float literal by reading the source at the span position.
fn float_display_len(f: f64, source: &str, span: error::Span) -> u32 {
    // Try to read the actual source text length
    if let Some(line) = source.lines().nth(span.line.wrapping_sub(1)) {
        let col = span.col.saturating_sub(1);
        if col < line.len() {
            let rest = &line[col..];
            let len = rest.bytes().take_while(|b| b.is_ascii_digit() || *b == b'.' || *b == b'e' || *b == b'E' || *b == b'+' || *b == b'-').count();
            if len > 0 {
                return len as u32;
            }
        }
    }
    format!("{f}").len() as u32
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Convert a Star span (1-indexed line:col) to an LSP range (0-indexed).
fn span_to_range(span: error::Span) -> Range {
    let line = if span.line > 0 { span.line - 1 } else { 0 } as u32;
    let col = if span.col > 0 { span.col - 1 } else { 0 } as u32;
    Range {
        start: Position { line, character: col },
        end: Position { line, character: col },
    }
}

/// Extract the identifier word under the cursor position from source text.
fn word_at_position(source: &str, pos: Position) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    let line = lines.get(pos.line as usize)?;
    let col = pos.character as usize;

    if col > line.len() {
        return None;
    }

    let bytes = line.as_bytes();

    // Find start of word
    let mut start = col;
    while start > 0 && is_ident_char(bytes[start - 1]) {
        start -= 1;
    }

    // Find end of word
    let mut end = col;
    while end < bytes.len() && is_ident_char(bytes[end]) {
        end += 1;
    }

    if start == end {
        return None;
    }

    Some(line[start..end].to_string())
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Parse a "line:col message" error string into a Diagnostic.
fn parse_error_to_diagnostic(error_str: &str) -> Diagnostic {
    if let Some((loc, msg)) = error_str.split_once(' ') {
        if let Some((line_s, col_s)) = loc.split_once(':') {
            if let (Ok(line), Ok(col)) = (line_s.parse::<usize>(), col_s.parse::<usize>()) {
                let span = error::Span::new(line, col);
                return Diagnostic {
                    range: span_to_range(span),
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: msg.to_string(),
                    ..Default::default()
                };
            }
        }
    }
    Diagnostic {
        range: Range::default(),
        severity: Some(DiagnosticSeverity::ERROR),
        message: error_str.to_string(),
        ..Default::default()
    }
}

/// Convert a URI to a file path string (best effort).
fn uri_to_path(uri: &Uri) -> String {
    let s = uri.as_str();
    if let Some(path) = s.strip_prefix("file://") {
        path.to_string()
    } else {
        s.to_string()
    }
}

/// Try to cast a request to a specific type. Returns Ok((id, params)) or Err(original).
fn cast_request<R>(req: Request) -> Result<(RequestId, R::Params), Request>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    match req.extract::<R::Params>(R::METHOD) {
        Ok(result) => Ok(result),
        Err(ExtractError::MethodMismatch(req)) => Err(req),
        Err(ExtractError::JsonError { .. }) => {
            // Deserialization error — just drop the request
            Err(Request::new(RequestId::from(0), String::new(), serde_json::Value::Null))
        }
    }
}

/// Try to cast a notification to a specific type.
fn cast_notification<N>(not: &Notification) -> Result<N::Params, serde_json::Error>
where
    N: lsp_types::notification::Notification,
    N::Params: serde::de::DeserializeOwned,
{
    if not.method == N::METHOD {
        serde_json::from_value(not.params.clone())
    } else {
        Err(serde::de::Error::custom("method mismatch"))
    }
}
