use crate::ast::*;
use crate::error::Span;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Resolve `use` declarations by loading external .star files.
///
/// Given a parsed Program from the main file, find all `use` declarations
/// that reference external modules (not inline `module ... end` blocks),
/// load and parse those files, and merge their items into the program.
pub fn resolve(program: Program, source_path: &str) -> Result<Program, Vec<String>> {
    let mut resolver = Resolver::new(source_path);
    resolver.resolve_program(program)
}

struct Resolver {
    /// Directory of the entry file (base for relative imports)
    base_dir: PathBuf,
    /// Set of already-loaded file paths (prevents circular deps)
    loaded: HashSet<PathBuf>,
    /// Inline module names defined in the current file
    inline_modules: HashSet<String>,
    /// Collected errors
    errors: Vec<String>,
}

impl Resolver {
    fn new(source_path: &str) -> Self {
        let path = Path::new(source_path);
        let base_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let mut loaded = HashSet::new();
        // Mark the entry file as loaded
        if let Ok(canonical) = std::fs::canonicalize(path) {
            loaded.insert(canonical);
        } else {
            loaded.insert(path.to_path_buf());
        }

        Self {
            base_dir,
            loaded,
            inline_modules: HashSet::new(),
            errors: Vec::new(),
        }
    }

    fn resolve_program(&mut self, mut program: Program) -> Result<Program, Vec<String>> {
        // First pass: collect inline module names
        for item in &program.items {
            if let Item::ModuleDecl(m) = item {
                self.inline_modules.insert(m.name.clone());
            }
        }

        // Second pass: resolve use declarations that reference external files
        let mut new_items = Vec::new();
        let use_decls: Vec<UseDecl> = program
            .items
            .iter()
            .filter_map(|item| {
                if let Item::UseDecl(u) = item {
                    Some(u.clone())
                } else {
                    None
                }
            })
            .collect();

        for use_decl in &use_decls {
            if let Some(module_name) = use_decl.path.first() {
                // Skip if this is an inline module
                if self.inline_modules.contains(module_name) {
                    continue;
                }

                // Try to find and load the external file
                match self.load_module(module_name, &use_decl.span) {
                    Ok(Some(items)) => new_items.extend(items),
                    Ok(None) => {} // File not found, might be a Rust import — skip
                    Err(e) => self.errors.push(e),
                }
            }
        }

        // Insert loaded module items before existing items
        // (types and functions need to be defined before use)
        if !new_items.is_empty() {
            new_items.extend(program.items);
            program.items = new_items;
        }

        if self.errors.is_empty() {
            Ok(program)
        } else {
            Err(self.errors.clone())
        }
    }

    /// Try to load a module from a .star file.
    /// Returns None if the file doesn't exist (not an error — could be a Rust module).
    fn load_module(
        &mut self,
        module_name: &str,
        span: &Span,
    ) -> Result<Option<Vec<Item>>, String> {
        // Convert PascalCase module name to snake_case filename
        let file_name = pascal_to_snake(module_name);

        // Search paths: same directory, then src/ subdirectory
        let candidates = [
            self.base_dir.join(format!("{file_name}.star")),
            self.base_dir.join("src").join(format!("{file_name}.star")),
        ];

        let file_path = match candidates.iter().find(|p| p.exists()) {
            Some(p) => p.clone(),
            None => return Ok(None), // Not found — not necessarily an error
        };

        // Check for circular dependency
        let canonical = std::fs::canonicalize(&file_path)
            .unwrap_or_else(|_| file_path.clone());

        if !self.loaded.insert(canonical.clone()) {
            return Err(format!(
                "{span} Circular dependency: module '{module_name}' ({})",
                file_path.display()
            ));
        }

        // Read and parse the file
        let source = std::fs::read_to_string(&file_path).map_err(|e| {
            format!(
                "{span} Cannot read module '{module_name}' ({}): {e}",
                file_path.display()
            )
        })?;

        let tokens = crate::lexer::lex(&source).map_err(|e| {
            format!(
                "In module '{module_name}' ({}): {e}",
                file_path.display()
            )
        })?;

        let (module_program, _comments) = crate::parser::parse(tokens).map_err(|e| {
            format!(
                "In module '{module_name}' ({}): {e}",
                file_path.display()
            )
        })?;

        // Wrap the module's items in a module declaration
        // so they get proper namespacing in codegen
        //
        // First, recursively resolve any use declarations in the loaded module.
        let module_base_dir = file_path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let old_base_dir = std::mem::replace(&mut self.base_dir, module_base_dir);
        let old_inline_modules = std::mem::replace(&mut self.inline_modules, HashSet::new());
        // Collect inline module names in the loaded module
        for item in &module_program.items {
            if let Item::ModuleDecl(m) = item {
                self.inline_modules.insert(m.name.clone());
            }
        }
        // Resolve transitive dependencies
        let resolved_program = self.resolve_program(module_program)
            .map_err(|errs| errs.join("\n"))?;
        // Restore original state
        self.base_dir = old_base_dir;
        self.inline_modules = old_inline_modules;

        let module_decl = Item::ModuleDecl(ModuleDecl {
            name: module_name.to_string(),
            items: resolved_program.items,
            span: *span,
        });

        Ok(Some(vec![module_decl]))
    }
}

/// Convert PascalCase to snake_case
fn pascal_to_snake(s: &str) -> String {
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

    #[test]
    fn test_pascal_to_snake() {
        assert_eq!(pascal_to_snake("Math"), "math");
        assert_eq!(pascal_to_snake("StringUtils"), "string_utils");
        assert_eq!(pascal_to_snake("HTTPClient"), "h_t_t_p_client");
        assert_eq!(pascal_to_snake("foo"), "foo");
    }

    #[test]
    fn test_resolve_no_external_deps() {
        // A program with no use declarations should pass through unchanged
        let program = Program {
            items: vec![Item::Expr(Expr {
                kind: ExprKind::IntLit(42),
                span: Span::new(1, 1),
            })],
        };
        let result = resolve(program, "test.star");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().items.len(), 1);
    }

    #[test]
    fn test_resolve_inline_module_skipped() {
        // A use declaration for an inline module should be skipped
        let program = Program {
            items: vec![
                Item::ModuleDecl(ModuleDecl {
                    name: "Math".to_string(),
                    items: vec![],
                    span: Span::new(1, 1),
                }),
                Item::UseDecl(UseDecl {
                    path: vec!["Math".to_string()],
                    imports: Some(vec!["square".to_string()]),
                    span: Span::new(3, 1),
                }),
            ],
        };
        let result = resolve(program, "test.star");
        assert!(result.is_ok());
        // Should have 2 items (module + use), no extra loaded modules
        assert_eq!(result.unwrap().items.len(), 2);
    }
}
