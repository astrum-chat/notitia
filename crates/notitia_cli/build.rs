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

fn main() {
    let cargo_toml = std::fs::read_to_string("Cargo.toml").expect("failed to read Cargo.toml");
    let doc: toml::Table = toml::from_str(&cargo_toml).expect("failed to parse Cargo.toml");

    let deps = doc["dependencies"].as_table().expect("no [dependencies]");
    let mut migrations_dep = deps
        .get("notitia_migrations")
        .expect("notitia_migrations not in [dependencies]")
        .clone();

    // Resolve relative `path` to absolute so it works from the temp project directory.
    if let toml::Value::Table(ref mut t) = migrations_dep {
        if let Some(toml::Value::String(p)) = t.get_mut("path") {
            let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let abs = manifest_dir.join(&*p).canonicalize().expect("failed to resolve notitia_migrations path");
            *p = abs.to_string_lossy().into_owned();
        }
    }

    let dep_toml = inline_toml(&migrations_dep);

    println!("cargo::rerun-if-changed=Cargo.toml");
    println!("cargo::rustc-env=NOTITIA_MIGRATIONS_DEP={dep_toml}");
}
