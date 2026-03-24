mod lexer;
mod parser;
mod ast;
mod typeck;
mod codegen;
mod error;
mod resolver;
mod formatter;
mod manifest;
mod optimize;
mod borrow;
mod lsp;

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: star <command> [file.star]");
        eprintln!("Run 'star --help' for usage information");
        std::process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "--version" | "-V" => {
            println!("star {}", env!("CARGO_PKG_VERSION"));
        }
        "--help" | "-h" => {
            println!("star {} — a functional language compiling to Rust", env!("CARGO_PKG_VERSION"));
            println!();
            println!("Usage: star <command> [file.star]");
            println!();
            println!("Commands:");
            println!("  build       Compile a Star program");
            println!("  run         Compile and run a Star program");
            println!("  check       Type-check without compiling");
            println!("  emit-rust   Print generated Rust code");
            println!("  fmt         Format a Star source file");
            println!("  test        Run test functions (fn test_*())");
            println!("  new         Create a new Star project");
            println!("  init        Initialize a Star project in the current directory");
            println!("  clean       Remove build artifacts (.star-build/)");
            println!("  lsp         Start the Language Server Protocol server");
            println!();
            println!("Options:");
            println!("  --release          Build in release mode (optimized)");
            println!("  --filter <pattern> Run only tests matching pattern (test mode)");
            println!("  --verbose, -v      Show detailed test output with timing");
            println!("  -h, --help         Show this help message");
            println!("  -V, --version      Show version");
        }
        "new" => {
            let Some(project_name) = args.get(2) else {
                eprintln!("Usage: star new <project-name>");
                std::process::exit(1);
            };
            match create_project(project_name) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        "init" => {
            match init_project() {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        "clean" => {
            let build_dir = Path::new(".star-build");
            if build_dir.exists() {
                fs::remove_dir_all(build_dir)
                    .map_err(|e| format!("Cannot remove .star-build: {e}"))
                    .unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                println!("Removed .star-build/");
            } else {
                println!("Nothing to clean");
            }
        }
        "fmt" => {
            let file = args.get(2).map(|s| s.as_str());
            match run_formatter(file) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        "lsp" => {
            lsp::run();
        }
        "build" | "run" | "check" | "emit-rust" | "test" => {
            let release = args.iter().any(|a| a == "--release");
            let verbose = args.iter().any(|a| a == "--verbose" || a == "-v");
            let filter = args.iter().position(|a| a == "--filter").and_then(|i| args.get(i + 1).cloned());
            let file = args.get(2).map(|s| s.as_str()).filter(|s| !s.starts_with('-'));
            match run_compiler(command, file, release, filter.as_deref(), verbose) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Unknown command: {command}");
            eprintln!("Run 'star --help' for usage information");
            std::process::exit(1);
        }
    }
}

/// Initialize a Star project in the current directory.
fn init_project() -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("Cannot get current dir: {e}"))?;

    if cwd.join("Star.toml").exists() {
        return Err("Star.toml already exists in this directory".to_string());
    }

    let name = cwd
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my-project")
        .to_string();

    fs::create_dir_all(cwd.join("src"))
        .map_err(|e| format!("Cannot create src directory: {e}"))?;

    fs::write(cwd.join("Star.toml"), manifest::default_manifest(&name))
        .map_err(|e| format!("Cannot write Star.toml: {e}"))?;

    if !cwd.join(".gitignore").exists() {
        fs::write(cwd.join(".gitignore"), "/.star-build\n")
            .map_err(|e| format!("Cannot write .gitignore: {e}"))?;
    }

    if !cwd.join("src/main.star").exists() {
        fs::write(cwd.join("src/main.star"), manifest::default_main_star())
            .map_err(|e| format!("Cannot write src/main.star: {e}"))?;
        println!("Initialized project '{name}'");
        println!("  Star.toml");
        println!("  src/main.star");
        println!("  .gitignore");
    } else {
        println!("Initialized project '{name}'");
        println!("  Star.toml");
        println!("  src/main.star (already exists, not overwritten)");
        println!("  .gitignore");
    }

    Ok(())
}

/// Create a new Star project with Star.toml and src/main.star.
fn create_project(name: &str) -> Result<(), String> {
    let project_dir = Path::new(name);
    if project_dir.exists() {
        return Err(format!("Directory '{}' already exists", name));
    }

    fs::create_dir_all(project_dir.join("src"))
        .map_err(|e| format!("Cannot create project directory: {e}"))?;

    fs::write(project_dir.join("Star.toml"), manifest::default_manifest(name))
        .map_err(|e| format!("Cannot write Star.toml: {e}"))?;

    fs::write(project_dir.join("src/main.star"), manifest::default_main_star())
        .map_err(|e| format!("Cannot write src/main.star: {e}"))?;

    fs::write(project_dir.join(".gitignore"), "/.star-build\n")
        .map_err(|e| format!("Cannot write .gitignore: {e}"))?;

    // Try to run git init (non-fatal if git is not available)
    let _ = Command::new("git")
        .arg("init")
        .current_dir(project_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    println!("Created project '{name}'");
    println!("  {name}/Star.toml");
    println!("  {name}/src/main.star");
    println!("  {name}/.gitignore");

    Ok(())
}

fn run_formatter(file_arg: Option<&str>) -> Result<(), String> {
    let file = match file_arg {
        Some(f) => f.to_string(),
        None => {
            let cwd = env::current_dir().map_err(|e| format!("Cannot get current dir: {e}"))?;
            if manifest::find_and_parse(&cwd)?.is_some() {
                "src/main.star".to_string()
            } else {
                return Err(
                    "No file specified and no Star.toml found in current directory".to_string(),
                );
            }
        }
    };

    let source = fs::read_to_string(&file).map_err(|e| format!("Cannot read {file}: {e}"))?;

    let tokens = lexer::lex(&source).map_err(|e| {
        error::format_error_from_string(&source, &file, &e)
    })?;

    let (program, comments) = parser::parse(tokens).map_err(|e| {
        e.lines()
            .map(|line| error::format_error_from_string(&source, &file, line))
            .collect::<Vec<_>>()
            .join("")
    })?;

    let formatted = formatter::format(&program, &comments);
    fs::write(&file, &formatted).map_err(|e| format!("Cannot write {file}: {e}"))?;
    println!("Formatted {file}");
    Ok(())
}

fn run_compiler(command: &str, file_arg: Option<&str>, release: bool, test_filter: Option<&str>, verbose: bool) -> Result<(), String> {
    // Determine the source file and optional manifest
    let (file, maybe_manifest) = match file_arg {
        Some(f) => (f.to_string(), None),
        None => {
            // No file given — look for Star.toml in the current directory
            let cwd = env::current_dir().map_err(|e| format!("Cannot get current dir: {e}"))?;
            match manifest::find_and_parse(&cwd)? {
                Some(m) => ("src/main.star".to_string(), Some(m)),
                None => {
                    return Err(
                        "No file specified and no Star.toml found in current directory".to_string(),
                    );
                }
            }
        }
    };

    let source = fs::read_to_string(&file).map_err(|e| format!("Cannot read {file}: {e}"))?;

    // Lex
    let tokens = lexer::lex(&source).map_err(|e| {
        error::format_error_from_string(&source, &file, &e)
    })?;

    // Parse
    let (program, _comments) = parser::parse(tokens).map_err(|e| {
        // The parser may return multiple errors separated by newlines
        e.lines()
            .map(|line| error::format_error_from_string(&source, &file, line))
            .collect::<Vec<_>>()
            .join("")
    })?;

    // Resolve external modules
    let program = resolver::resolve(program, &file).map_err(|errors| {
        errors
            .iter()
            .map(|e| error::format_error_from_string(&source, &file, e))
            .collect::<Vec<_>>()
            .join("")
    })?;

    // Type check
    let (typed_program, warnings) = typeck::check(program).map_err(|e| {
        error::format_error_from_string(&source, &file, &e)
    })?;

    // Print warnings with source context
    for (span, msg) in &warnings {
        let warn = error::StarError::warning(*span, msg.clone());
        eprint!("{}", error::format_error(&source, &file, &warn));
    }

    if command == "check" {
        println!("OK");
        return Ok(());
    }

    // Codegen
    let test_mode = command == "test";
    let rust_code = codegen::generate(&typed_program, test_mode);

    // Optimize: remove provably unnecessary clones
    let rust_code = optimize::optimize(&rust_code);

    // Borrow inference: convert String params to &str where safe
    let rust_code = borrow::infer_borrows(&rust_code);

    if command == "emit-rust" {
        println!("{rust_code}");
        return Ok(());
    }

    // Write to build directory and invoke cargo
    let build_dir = ".star-build";
    fs::create_dir_all(format!("{build_dir}/src"))
        .map_err(|e| format!("Cannot create build dir: {e}"))?;

    // Detect crate dependencies from generated code
    let mut auto_deps = String::new();
    if rust_code.contains("regex::") {
        auto_deps.push_str("regex = \"1\"\n");
    }
    if rust_code.contains("base64::") {
        auto_deps.push_str("base64 = \"0.22\"\n");
    }
    if rust_code.contains("tokio::") || rust_code.contains("#[tokio::main]") {
        auto_deps.push_str("tokio = { version = \"1\", features = [\"full\"] }\n");
    }
    if rust_code.contains("native_tls::") {
        auto_deps.push_str("native-tls = \"0.2\"\n");
    }

    // Merge manifest deps with auto-detected deps
    let (pkg_name, deps_section, metadata_comments) = match &maybe_manifest {
        Some(m) => {
            let deps = if test_mode {
                m.cargo_test_dependencies(&auto_deps)
            } else {
                m.cargo_dependencies(&auto_deps)
            };
            (m.package.name.clone(), deps, m.cargo_metadata_comments())
        }
        None => ("star-output".to_string(), auto_deps.trim_end().to_string(), String::new()),
    };

    fs::write(
        format!("{build_dir}/Cargo.toml"),
        format!(
            r#"{metadata_comments}[package]
name = "{pkg_name}"
version = "0.1.0"
edition = "2024"

[dependencies]
{deps_section}"#
        ),
    )
    .map_err(|e| format!("Cannot write Cargo.toml: {e}"))?;

    fs::write(format!("{build_dir}/src/main.rs"), &rust_code)
        .map_err(|e| format!("Cannot write main.rs: {e}"))?;

    // Cargo.lock preservation: copy Star.lock → .star-build/Cargo.lock before build
    let has_manifest = maybe_manifest.is_some();
    if has_manifest && Path::new("Star.lock").exists() {
        let _ = fs::copy("Star.lock", format!("{build_dir}/Cargo.lock"));
    }

    // Build
    let mut build_cmd = Command::new("cargo");
    build_cmd.arg("build");
    if release {
        build_cmd.arg("--release");
    }
    let output = build_cmd
        .current_dir(build_dir)
        .output()
        .map_err(|e| format!("Cannot run cargo: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            eprint!("{stdout}");
        }
        if !stderr.is_empty() {
            eprint!("{stderr}");
        }
        return Err("Rust compilation failed".to_string());
    }

    // Cargo.lock preservation: copy .star-build/Cargo.lock → Star.lock after build
    if has_manifest {
        let cargo_lock = format!("{build_dir}/Cargo.lock");
        if Path::new(&cargo_lock).exists() {
            let _ = fs::copy(&cargo_lock, "Star.lock");
        }
    }

    if command == "run" || command == "test" {
        let mut run_cmd = Command::new("cargo");
        run_cmd.arg("run").arg("--quiet");
        if release {
            run_cmd.arg("--release");
        }
        // Pass test flags as env vars
        if test_mode {
            if let Some(filter) = test_filter {
                run_cmd.env("STAR_TEST_FILTER", filter);
            }
            if verbose {
                run_cmd.env("STAR_TEST_VERBOSE", "1");
            }
        }
        let status = run_cmd
            .current_dir(build_dir)
            .status()
            .map_err(|e| format!("Cannot run binary: {e}"))?;

        if !status.success() {
            return Err(if command == "test" {
                "Tests failed".to_string()
            } else {
                "Program exited with error".to_string()
            });
        }
    }

    Ok(())
}
