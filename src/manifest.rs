use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Represents a parsed Star.toml manifest.
#[derive(Debug, Clone)]
pub struct Manifest {
    pub package: Package,
    pub dependencies: Vec<Dependency>,
    pub dev_dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub license: Option<String>,
    pub edition: Option<String>,
}

/// A Rust crate dependency from [dependencies].
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
}

impl Dependency {
    /// Render as a Cargo.toml dependency line.
    pub fn to_cargo_toml_line(&self) -> String {
        if self.features.is_empty() {
            format!("{} = \"{}\"", self.name, self.version)
        } else {
            let feats: Vec<String> = self.features.iter().map(|f| format!("\"{}\"", f)).collect();
            format!(
                "{} = {{ version = \"{}\", features = [{}] }}",
                self.name,
                self.version,
                feats.join(", ")
            )
        }
    }
}

impl Manifest {
    /// Generate the [dependencies] section content for Cargo.toml,
    /// merging manifest deps with auto-detected deps from codegen output.
    /// Auto-detected deps are only included if the manifest doesn't already
    /// specify that crate.
    pub fn cargo_dependencies(&self, auto_detected: &str) -> String {
        let mut lines: Vec<String> = Vec::new();
        let mut declared: HashMap<&str, bool> = HashMap::new();

        for dep in &self.dependencies {
            declared.insert(&dep.name, true);
            lines.push(dep.to_cargo_toml_line());
        }

        // Add auto-detected deps that weren't explicitly declared
        for line in auto_detected.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Extract crate name (everything before " = ")
            if let Some(name) = line.split(" = ").next() {
                let name = name.trim();
                if !declared.contains_key(name) {
                    lines.push(line.to_string());
                }
            }
        }

        lines.join("\n")
    }

    /// Generate dependencies for test mode: merges regular + dev dependencies.
    /// Dev-deps override regular deps if they declare the same crate.
    pub fn cargo_test_dependencies(&self, auto_detected: &str) -> String {
        let mut lines: Vec<String> = Vec::new();
        let mut declared: HashMap<&str, bool> = HashMap::new();

        // Dev-deps override regular deps
        let mut dev_names: HashMap<&str, bool> = HashMap::new();
        for dep in &self.dev_dependencies {
            dev_names.insert(&dep.name, true);
        }

        for dep in &self.dependencies {
            if !dev_names.contains_key(dep.name.as_str()) {
                declared.insert(&dep.name, true);
                lines.push(dep.to_cargo_toml_line());
            }
        }
        for dep in &self.dev_dependencies {
            declared.insert(&dep.name, true);
            lines.push(dep.to_cargo_toml_line());
        }

        // Add auto-detected deps that weren't explicitly declared
        for line in auto_detected.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(name) = line.split(" = ").next() {
                let name = name.trim();
                if !declared.contains_key(name) {
                    lines.push(line.to_string());
                }
            }
        }

        lines.join("\n")
    }

    /// Emit package metadata as comments for the generated Cargo.toml.
    pub fn cargo_metadata_comments(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        if let Some(desc) = &self.package.description {
            lines.push(format!("# description: {desc}"));
        }
        if !self.package.authors.is_empty() {
            lines.push(format!("# authors: {}", self.package.authors.join(", ")));
        }
        if let Some(lic) = &self.package.license {
            lines.push(format!("# license: {lic}"));
        }
        if lines.is_empty() {
            String::new()
        } else {
            format!("{}\n", lines.join("\n"))
        }
    }
}

/// Look for Star.toml in the given directory, parse and return it.
pub fn find_and_parse(dir: &Path) -> Result<Option<Manifest>, String> {
    let path = dir.join("Star.toml");
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Cannot read Star.toml: {e}"))?;
    let manifest = parse(&content)?;
    Ok(Some(manifest))
}

/// Parse a Star.toml string into a Manifest.
pub fn parse(input: &str) -> Result<Manifest, String> {
    let mut package_name: Option<String> = None;
    let mut package_version: Option<String> = None;
    let mut package_description: Option<String> = None;
    let mut package_authors: Vec<String> = Vec::new();
    let mut package_license: Option<String> = None;
    let mut package_edition: Option<String> = None;
    let mut dependencies: Vec<Dependency> = Vec::new();
    let mut dev_dependencies: Vec<Dependency> = Vec::new();

    let mut current_section: Option<&str> = None;

    for (line_num, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Section header
        if line.starts_with('[') && line.ends_with(']') {
            let section = &line[1..line.len() - 1].trim();
            match *section {
                "package" | "dependencies" | "dev-dependencies" => {
                    current_section = Some(match *section {
                        "package" => "package",
                        "dependencies" => "dependencies",
                        "dev-dependencies" => "dev-dependencies",
                        _ => unreachable!(),
                    });
                }
                _other => {
                    // Silently ignore unknown sections for forward compatibility
                    current_section = None;
                }
            }
            continue;
        }

        // Key = value
        let Some((key, value)) = line.split_once('=') else {
            if current_section.is_some() {
                return Err(format!(
                    "Star.toml line {}: expected key = value, got: {}",
                    line_num + 1,
                    line
                ));
            }
            continue;
        };
        let key = key.trim();
        let value = value.trim();

        match current_section {
            Some("package") => match key {
                "name" => {
                    package_name = Some(parse_string_value(value, line_num)?);
                }
                "version" => {
                    package_version = Some(parse_string_value(value, line_num)?);
                }
                "description" => {
                    package_description = Some(parse_string_value(value, line_num)?);
                }
                "authors" => {
                    package_authors = parse_string_array(value, line_num)?;
                }
                "license" => {
                    package_license = Some(parse_string_value(value, line_num)?);
                }
                "edition" => {
                    let ed = parse_string_value(value, line_num)?;
                    match ed.as_str() {
                        "2024" => {}
                        other => {
                            return Err(format!(
                                "Star.toml line {}: unknown edition '{}' (known: \"2024\")",
                                line_num + 1,
                                other
                            ));
                        }
                    }
                    package_edition = Some(ed);
                }
                _other => {
                    // Silently ignore unknown package fields for forward compatibility
                }
            },
            Some("dependencies") => {
                let dep = parse_dependency(key, value, line_num)?;
                dependencies.push(dep);
            }
            Some("dev-dependencies") => {
                let dep = parse_dependency(key, value, line_num)?;
                dev_dependencies.push(dep);
            }
            _ => {
                // Inside an unknown section or outside any section — skip
            }
        }
    }

    let name = package_name.unwrap_or_else(|| "star-project".to_string());
    let version = package_version.unwrap_or_else(|| "0.1.0".to_string());

    Ok(Manifest {
        package: Package {
            name,
            version,
            description: package_description,
            authors: package_authors,
            license: package_license,
            edition: package_edition,
        },
        dependencies,
        dev_dependencies,
    })
}

/// Parse a quoted string value like `"hello"`.
fn parse_string_value(value: &str, line_num: usize) -> Result<String, String> {
    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        Ok(value[1..value.len() - 1].to_string())
    } else {
        Err(format!(
            "Star.toml line {}: expected quoted string, got: {}",
            line_num + 1,
            value
        ))
    }
}

/// Parse a dependency value. Either:
/// - `"1.0"` (simple version string)
/// - `{ version = "1", features = ["full"] }` (inline table)
fn parse_dependency(name: &str, value: &str, line_num: usize) -> Result<Dependency, String> {
    let value = value.trim();

    if value.starts_with('"') {
        // Simple: name = "version"
        let version = parse_string_value(value, line_num)?;
        return Ok(Dependency {
            name: name.to_string(),
            version,
            features: Vec::new(),
        });
    }

    if value.starts_with('{') && value.ends_with('}') {
        // Inline table: { version = "1", features = ["full"] }
        let inner = &value[1..value.len() - 1].trim();
        let mut version = String::new();
        let mut features: Vec<String> = Vec::new();

        // Split by comma, but be careful about commas inside brackets
        let parts = split_inline_table(inner);

        for part in &parts {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let Some((k, v)) = part.split_once('=') else {
                return Err(format!(
                    "Star.toml line {}: malformed inline table entry: {}",
                    line_num + 1,
                    part
                ));
            };
            let k = k.trim();
            let v = v.trim();

            match k {
                "version" => {
                    version = parse_string_value(v, line_num)?;
                }
                "features" => {
                    features = parse_string_array(v, line_num)?;
                }
                other => {
                    return Err(format!(
                        "Star.toml line {}: unknown dependency field '{}'",
                        line_num + 1,
                        other
                    ));
                }
            }
        }

        if version.is_empty() {
            return Err(format!(
                "Star.toml line {}: dependency '{}' missing version",
                line_num + 1,
                name
            ));
        }

        return Ok(Dependency {
            name: name.to_string(),
            version,
            features,
        });
    }

    Err(format!(
        "Star.toml line {}: expected \"version\" or {{ version = \"...\" }}, got: {}",
        line_num + 1,
        value
    ))
}

/// Split an inline table by commas, respecting brackets.
fn split_inline_table(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut bracket_depth = 0;

    for ch in input.chars() {
        match ch {
            '[' => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' => {
                bracket_depth -= 1;
                current.push(ch);
            }
            ',' if bracket_depth == 0 => {
                parts.push(current.clone());
                current.clear();
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.trim().is_empty() {
        parts.push(current);
    }
    parts
}

/// Parse `["a", "b", "c"]` into a Vec<String>.
fn parse_string_array(value: &str, line_num: usize) -> Result<Vec<String>, String> {
    let value = value.trim();
    if !value.starts_with('[') || !value.ends_with(']') {
        return Err(format!(
            "Star.toml line {}: expected array [...], got: {}",
            line_num + 1,
            value
        ));
    }

    let inner = &value[1..value.len() - 1];
    let mut result = Vec::new();

    for item in inner.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        result.push(parse_string_value(item, line_num)?);
    }

    Ok(result)
}

/// Generate a default Star.toml for `star new`.
pub fn default_manifest(project_name: &str) -> String {
    format!(
        r#"[package]
name = "{project_name}"
version = "0.1.0"
# description = "A Star project"
# authors = ["Your Name"]
# license = "MIT"

[dependencies]
"#
    )
}

/// Default main.star for `star new`.
pub fn default_main_star() -> &'static str {
    r#"# Main entry point
fn main() =
  println("Hello from Star!")
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal() {
        let input = r#"
[package]
name = "my-project"
version = "0.1.0"
"#;
        let m = parse(input).unwrap();
        assert_eq!(m.package.name, "my-project");
        assert_eq!(m.package.version, "0.1.0");
        assert!(m.dependencies.is_empty());
        assert!(m.dev_dependencies.is_empty());
        assert!(m.package.description.is_none());
        assert!(m.package.authors.is_empty());
        assert!(m.package.license.is_none());
        assert!(m.package.edition.is_none());
    }

    #[test]
    fn test_parse_simple_dep() {
        let input = r#"
[package]
name = "test"
version = "1.0.0"

[dependencies]
regex = "1"
"#;
        let m = parse(input).unwrap();
        assert_eq!(m.dependencies.len(), 1);
        assert_eq!(m.dependencies[0].name, "regex");
        assert_eq!(m.dependencies[0].version, "1");
        assert!(m.dependencies[0].features.is_empty());
    }

    #[test]
    fn test_parse_inline_table_dep() {
        let input = r#"
[package]
name = "test"
version = "1.0.0"

[dependencies]
tokio = { version = "1", features = ["full"] }
"#;
        let m = parse(input).unwrap();
        assert_eq!(m.dependencies.len(), 1);
        assert_eq!(m.dependencies[0].name, "tokio");
        assert_eq!(m.dependencies[0].version, "1");
        assert_eq!(m.dependencies[0].features, vec!["full"]);
    }

    #[test]
    fn test_parse_multiple_features() {
        let input = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
tokio = { version = "1", features = ["rt", "macros", "net"] }
"#;
        let m = parse(input).unwrap();
        assert_eq!(
            m.dependencies[0].features,
            vec!["rt", "macros", "net"]
        );
    }

    #[test]
    fn test_cargo_toml_line_simple() {
        let dep = Dependency {
            name: "regex".to_string(),
            version: "1".to_string(),
            features: vec![],
        };
        assert_eq!(dep.to_cargo_toml_line(), "regex = \"1\"");
    }

    #[test]
    fn test_cargo_toml_line_features() {
        let dep = Dependency {
            name: "tokio".to_string(),
            version: "1".to_string(),
            features: vec!["full".to_string()],
        };
        assert_eq!(
            dep.to_cargo_toml_line(),
            "tokio = { version = \"1\", features = [\"full\"] }"
        );
    }

    #[test]
    fn test_cargo_dependencies_merge() {
        let m = Manifest {
            package: Package {
                name: "test".to_string(),
                version: "0.1.0".to_string(),
                description: None,
                authors: vec![],
                license: None,
                edition: None,
            },
            dependencies: vec![Dependency {
                name: "tokio".to_string(),
                version: "1".to_string(),
                features: vec!["full".to_string()],
            }],
            dev_dependencies: vec![],
        };
        // Auto-detected has tokio and regex; tokio should be skipped (manifest wins)
        let auto = "tokio = { version = \"1\", features = [\"full\"] }\nregex = \"1\"\n";
        let result = m.cargo_dependencies(auto);
        assert!(result.contains("tokio = { version = \"1\", features = [\"full\"] }"));
        assert!(result.contains("regex = \"1\""));
        // tokio should only appear once
        assert_eq!(result.matches("tokio").count(), 1);
    }

    #[test]
    fn test_comments_and_blank_lines() {
        let input = r#"
# This is a Star project
[package]
name = "demo"
version = "0.1.0"

# No deps yet
[dependencies]
"#;
        let m = parse(input).unwrap();
        assert_eq!(m.package.name, "demo");
        assert!(m.dependencies.is_empty());
    }

    #[test]
    fn test_defaults_when_missing() {
        let input = "[dependencies]\n";
        let m = parse(input).unwrap();
        assert_eq!(m.package.name, "star-project");
        assert_eq!(m.package.version, "0.1.0");
    }

    #[test]
    fn test_parse_metadata_fields() {
        let input = r#"
[package]
name = "my-app"
version = "1.0.0"
description = "A cool app"
authors = ["Alice", "Bob"]
license = "MIT"
edition = "2024"
"#;
        let m = parse(input).unwrap();
        assert_eq!(m.package.description.as_deref(), Some("A cool app"));
        assert_eq!(m.package.authors, vec!["Alice", "Bob"]);
        assert_eq!(m.package.license.as_deref(), Some("MIT"));
        assert_eq!(m.package.edition.as_deref(), Some("2024"));
    }

    #[test]
    fn test_parse_dev_dependencies() {
        let input = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = "1"

[dev-dependencies]
criterion = "0.5"
"#;
        let m = parse(input).unwrap();
        assert_eq!(m.dependencies.len(), 1);
        assert_eq!(m.dependencies[0].name, "serde");
        assert_eq!(m.dev_dependencies.len(), 1);
        assert_eq!(m.dev_dependencies[0].name, "criterion");
        assert_eq!(m.dev_dependencies[0].version, "0.5");
    }

    #[test]
    fn test_unknown_fields_ignored() {
        let input = r#"
[package]
name = "test"
version = "0.1.0"
homepage = "https://example.com"
repository = "https://github.com/test"

[dependencies]
"#;
        // Should not error — unknown fields are silently ignored
        let m = parse(input).unwrap();
        assert_eq!(m.package.name, "test");
    }

    #[test]
    fn test_unknown_sections_ignored() {
        let input = r#"
[package]
name = "test"
version = "0.1.0"

[build]
optimize = "true"

[dependencies]
"#;
        // Should not error — unknown sections are silently ignored
        let m = parse(input).unwrap();
        assert_eq!(m.package.name, "test");
    }

    #[test]
    fn test_cargo_test_dependencies_merge() {
        let m = Manifest {
            package: Package {
                name: "test".to_string(),
                version: "0.1.0".to_string(),
                description: None,
                authors: vec![],
                license: None,
                edition: None,
            },
            dependencies: vec![Dependency {
                name: "serde".to_string(),
                version: "1".to_string(),
                features: vec![],
            }],
            dev_dependencies: vec![Dependency {
                name: "criterion".to_string(),
                version: "0.5".to_string(),
                features: vec![],
            }],
        };
        let result = m.cargo_test_dependencies("");
        assert!(result.contains("serde = \"1\""));
        assert!(result.contains("criterion = \"0.5\""));
    }

    #[test]
    fn test_cargo_metadata_comments() {
        let m = Manifest {
            package: Package {
                name: "test".to_string(),
                version: "0.1.0".to_string(),
                description: Some("A test project".to_string()),
                authors: vec!["Alice".to_string()],
                license: Some("MIT".to_string()),
                edition: None,
            },
            dependencies: vec![],
            dev_dependencies: vec![],
        };
        let comments = m.cargo_metadata_comments();
        assert!(comments.contains("# description: A test project"));
        assert!(comments.contains("# authors: Alice"));
        assert!(comments.contains("# license: MIT"));
    }

    #[test]
    fn test_invalid_edition() {
        let input = r#"
[package]
name = "test"
version = "0.1.0"
edition = "2020"
"#;
        let result = parse(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown edition"));
    }
}
