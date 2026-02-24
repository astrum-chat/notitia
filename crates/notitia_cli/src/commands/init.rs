use std::path::Path;

const TEMPLATE_TOML: &str = r#"# Notitia migration configuration

# Directory where snapshots are stored, relative to project root.
snapshots_dir = "snapshots"
"#;

pub fn run() -> anyhow::Result<()> {
    std::fs::create_dir_all("snapshots")?;
    println!("Created snapshots/");

    let toml_path = Path::new("notitia.toml");
    if toml_path.exists() {
        println!("notitia.toml already exists, skipping.");
    } else {
        std::fs::write(toml_path, TEMPLATE_TOML)?;
        println!("Created notitia.toml");
    }

    println!();
    println!("Next steps:");
    println!("  1. Add a `pub mod schemas` to your lib.rs");
    println!("  2. Define your #[database] structs in the schemas module");
    println!("  3. Run `notitia snapshot` to save your first schema snapshot");

    Ok(())
}
