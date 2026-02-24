use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};
use notitia_migrations::SchemaString;

const MIGRATIONS_DEP: &str = env!("NOTITIA_MIGRATIONS_DEP");

fn inline_toml(value: &toml::Value) -> String {
    match value {
        toml::Value::Table(t) => {
            let entries: Vec<String> = t
                .iter()
                .map(|(k, v)| format!("{k} = {}", inline_toml(v)))
                .collect();
            format!("{{ {} }}", entries.join(", "))
        }
        toml::Value::String(s) => format!("\"{s}\""),
        toml::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(|v| inline_toml(v)).collect();
            format!("[{}]", items.join(", "))
        }
        other => other.to_string(),
    }
}

/// Read the `[package] name` from a Cargo.toml at the given path.
fn read_crate_name(cargo_toml: &Path) -> anyhow::Result<String> {
    let contents = std::fs::read_to_string(cargo_toml)
        .with_context(|| format!("failed to read {}", cargo_toml.display()))?;
    let doc: toml::Table =
        toml::from_str(&contents).with_context(|| format!("failed to parse {}", cargo_toml.display()))?;
    let name = doc
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .with_context(|| format!("could not read [package] name from {}", cargo_toml.display()))?;
    Ok(name.to_string())
}

/// Resolve a workspace member crate by name. Returns the member's directory.
fn resolve_workspace_member(krate: &str) -> anyhow::Result<PathBuf> {
    let contents =
        std::fs::read_to_string("Cargo.toml").context("no Cargo.toml found in current directory")?;
    let doc: toml::Table = toml::from_str(&contents).context("failed to parse Cargo.toml")?;

    let members = doc
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .context("no [workspace] members found in Cargo.toml")?;

    let cwd = std::env::current_dir()?;

    for member in members {
        let Some(pattern) = member.as_str() else {
            continue;
        };

        for entry in glob::glob(&pattern).into_iter().flatten().flatten() {
            let candidate = cwd.join(&entry);
            let cargo_toml = candidate.join("Cargo.toml");
            if !cargo_toml.exists() {
                continue;
            }
            if let Ok(name) = read_crate_name(&cargo_toml) {
                if name == krate {
                    return Ok(candidate);
                }
            }
        }
    }

    bail!(
        "crate '{krate}' not found in workspace members. \
         Check your [workspace] members in Cargo.toml."
    );
}

/// Extract type names from `pub use` statements in source text.
fn extract_pub_use_names(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        // `pub use some::path::Name;`
        if let Some(rest) = trimmed.strip_prefix("pub use ") {
            let rest = rest.trim_end_matches(';').trim();
            // Handle glob re-exports: `pub use super::schema::*;`
            if rest.ends_with("::*") {
                continue;
            }
            // Handle braced groups: `pub use super::schema::{A, B};`
            if let Some(brace_start) = rest.find('{') {
                if let Some(brace_end) = rest.find('}') {
                    let inner = &rest[brace_start + 1..brace_end];
                    for item in inner.split(',') {
                        let item = item.trim();
                        if !item.is_empty() {
                            names.push(item.to_string());
                        }
                    }
                }
            } else {
                // Simple path: take the last segment
                if let Some(name) = rest.rsplit("::").next() {
                    names.push(name.to_string());
                }
            }
        }
    }
    names
}

/// Read database names from the schemas module.
///
/// Checks (in order):
/// 1. `src/schemas/mod.rs`
/// 2. `src/schemas.rs`
/// 3. Inline `pub mod schemas { ... }` in `src/lib.rs`
pub fn read_database_names(base: &Path) -> anyhow::Result<Vec<String>> {
    // Check file-based schemas module first.
    for rel in &["src/schemas/mod.rs", "src/schemas.rs"] {
        let path = base.join(rel);
        if path.exists() {
            let source = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let names = extract_pub_use_names(&source);
            if names.is_empty() {
                bail!("no `pub use` items found in {}", path.display());
            }
            return Ok(names);
        }
    }

    // Check for inline `pub mod schemas { ... }` in lib.rs.
    let lib_path = base.join("src/lib.rs");
    if lib_path.exists() {
        let source = std::fs::read_to_string(&lib_path)
            .with_context(|| format!("failed to read {}", lib_path.display()))?;
        let mut in_schemas_mod = false;
        let mut brace_depth: i32 = 0;
        let mut block_source = String::new();

        for line in source.lines() {
            let trimmed = line.trim();
            if !in_schemas_mod {
                if trimmed.starts_with("pub mod schemas") && trimmed.contains('{') {
                    in_schemas_mod = true;
                    brace_depth = trimmed.chars().filter(|&c| c == '{').count() as i32
                        - trimmed.chars().filter(|&c| c == '}').count() as i32;
                    // Grab content after the opening brace
                    if let Some(after) = trimmed.splitn(2, '{').nth(1) {
                        block_source.push_str(after);
                        block_source.push('\n');
                    }
                    if brace_depth <= 0 {
                        break;
                    }
                    continue;
                }
            } else {
                brace_depth += trimmed.chars().filter(|&c| c == '{').count() as i32;
                brace_depth -= trimmed.chars().filter(|&c| c == '}').count() as i32;
                block_source.push_str(line);
                block_source.push('\n');
                if brace_depth <= 0 {
                    break;
                }
            }
        }

        if in_schemas_mod {
            let names = extract_pub_use_names(&block_source);
            if names.is_empty() {
                bail!("no `pub use` items found in `pub mod schemas` block in {}", lib_path.display());
            }
            return Ok(names);
        }
    }

    bail!(
        "no schemas module found in {}. Define it as:\n\
         - src/schemas/mod.rs\n\
         - src/schemas.rs\n\
         - pub mod schemas {{ ... }} in src/lib.rs",
        base.display()
    );
}

/// Generate the temp main.rs source that extracts schemas for each database type.
fn generate_main_rs(crate_name: &str, databases: &[String]) -> String {
    let mut src = String::new();
    src.push_str("use notitia_migrations::SchemaString;\n");

    for db in databases {
        src.push_str(&format!("use {crate_name}::schemas::{db};\n"));
    }

    src.push_str("\nfn main() {\n");

    for db in databases {
        src.push_str(&format!(
            "    let schema = SchemaString::extract::<{db}>().expect(\"failed to extract {db}\");\n"
        ));
        src.push_str(&format!("    println!(\"---{db}\");\n"));
        src.push_str(&format!("    print!(\"{{schema}}\");\n"));
    }

    src.push_str("}\n");
    src
}

/// Parse the output back into named schemas.
/// Format: "---DbName\n{yaml}---DbName2\n{yaml2}"
fn parse_schemas(output: &str) -> Vec<(String, SchemaString)> {
    let mut result = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_yaml = String::new();

    for line in output.lines() {
        if let Some(name) = line.strip_prefix("---") {
            if let Some(prev_name) = current_name.take() {
                result.push((prev_name, SchemaString::new(current_yaml.clone())));
                current_yaml.clear();
            }
            current_name = Some(name.to_string());
        } else if current_name.is_some() {
            current_yaml.push_str(line);
            current_yaml.push('\n');
        }
    }

    if let Some(name) = current_name {
        result.push((name, SchemaString::new(current_yaml)));
    }

    result
}

/// Build and run a temp project that extracts schemas from the user's crate.
pub fn extract_schemas(
    verbose: bool,
    tmp: bool,
    krate: Option<&str>,
) -> anyhow::Result<Vec<(String, SchemaString)>> {
    let cwd = std::env::current_dir()?;

    // Resolve the target crate directory and name.
    let (crate_dir, crate_name) = match krate {
        Some(name) => {
            let member_dir = resolve_workspace_member(name)?;
            let crate_name = read_crate_name(&member_dir.join("Cargo.toml"))?;
            (member_dir, crate_name)
        }
        None => {
            let crate_name = read_crate_name(&cwd.join("Cargo.toml"))?;
            (cwd.clone(), crate_name)
        }
    };

    let databases = read_database_names(&crate_dir)?;

    if !crate_dir.join("src/lib.rs").exists() {
        bail!(
            "src/lib.rs not found in {}. Your crate must have a library target \
             so the schema can be imported.\n\
             Create src/lib.rs with:\n\n  \
             pub mod schemas;",
            crate_dir.display()
        );
    }

    // Patches and lock file come from the workspace root (cwd).
    let tmp_dir = build_temp_project(&cwd, &crate_dir, &crate_name, &databases, tmp)?;
    let result = run_temp_project(&tmp_dir, verbose);

    if tmp {
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    let output = result?;
    Ok(parse_schemas(&output))
}

/// Extract `[patch]` sections from the user's Cargo.toml as raw TOML text.
fn read_patch_sections(project_dir: &Path) -> anyhow::Result<String> {
    let contents = std::fs::read_to_string(project_dir.join("Cargo.toml"))
        .context("failed to read user's Cargo.toml")?;
    let doc: toml::Table = toml::from_str(&contents).context("failed to parse user's Cargo.toml")?;

    let mut patch_toml = String::new();
    if let Some(patch) = doc.get("patch") {
        if let Some(patch_table) = patch.as_table() {
            for (registry, overrides) in patch_table {
                patch_toml.push_str(&format!("[patch.{registry}]\n"));
                if let Some(overrides_table) = overrides.as_table() {
                    for (name, spec) in overrides_table {
                        patch_toml.push_str(&format!("{name} = {}\n", inline_toml(spec)));
                    }
                }
                patch_toml.push('\n');
            }
        }
    }
    Ok(patch_toml)
}

fn build_temp_project(
    workspace_dir: &Path,
    crate_dir: &Path,
    crate_name: &str,
    databases: &[String],
    tmp: bool,
) -> anyhow::Result<PathBuf> {
    let tmp_dir = if tmp {
        std::env::temp_dir().join("notitia_schema_export")
    } else {
        workspace_dir.join(".notitia")
    };

    if tmp {
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
    std::fs::create_dir_all(tmp_dir.join("src"))?;

    let patch_sections = read_patch_sections(workspace_dir)?;

    let cargo_toml = format!(
        r#"[package]
name = "notitia_schema_export"
version = "0.0.0"
edition = "2024"

[dependencies]
{crate_name} = {{ path = "{crate_dir}" }}
notitia_migrations = {migrations_dep}

[workspace]

{patch_sections}"#,
        crate_name = crate_name,
        crate_dir = crate_dir.display(),
        migrations_dep = MIGRATIONS_DEP,
        patch_sections = patch_sections,
    );

    std::fs::write(tmp_dir.join("Cargo.toml"), cargo_toml)?;
    std::fs::write(
        tmp_dir.join("src/main.rs"),
        generate_main_rs(crate_name, databases),
    )?;

    // Copy the workspace's Cargo.lock so the temp project uses the same dependency versions.
    let lock_src = workspace_dir.join("Cargo.lock");
    if lock_src.exists() {
        std::fs::copy(&lock_src, tmp_dir.join("Cargo.lock"))?;
    }

    Ok(tmp_dir)
}

fn run_temp_project(tmp_dir: &Path, verbose: bool) -> anyhow::Result<String> {
    use std::process::Stdio;

    let mut cmd = Command::new("cargo");
    cmd.arg("run");
    if !verbose {
        cmd.arg("--quiet");
    }
    cmd.current_dir(tmp_dir).stdout(Stdio::piped());

    if verbose {
        cmd.stderr(Stdio::inherit());
    }

    let output = cmd
        .output()
        .context("failed to run cargo on schema export project")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "schema export failed (exit code {:?}):\n{}",
            output.status.code(),
            stderr
        );
    }

    String::from_utf8(output.stdout).context("schema export output is not valid UTF-8")
}
