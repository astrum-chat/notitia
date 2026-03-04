use std::path::Path;

use anyhow::Context;
use notitia_migrations::SchemaString;
use semver::Version;

/// Parse a semver version from a filename like "0.1.12.yml".
fn version_from_filename(name: &std::ffi::OsStr) -> Option<Version> {
    let s = name.to_str()?;
    let stem = s.strip_suffix(".yml").or_else(|| s.strip_suffix(".yaml"))?;
    Version::parse(stem).ok()
}

pub fn read_crate_version() -> anyhow::Result<String> {
    let contents =
        std::fs::read_to_string("Cargo.toml").context("no Cargo.toml found in current directory")?;
    let doc: toml::Table = toml::from_str(&contents).context("failed to parse Cargo.toml")?;
    let version = doc
        .get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .context("could not read [package] version from Cargo.toml")?;
    Ok(version.to_string())
}

/// Returns the most recent snapshot's YAML content, or None if no snapshots exist.
fn latest_snapshot(snapshots_dir: &Path, db_name: &str) -> anyhow::Result<Option<String>> {
    let dir = snapshots_dir.join(db_name);
    if !dir.exists() {
        return Ok(None);
    }

    let mut entries: Vec<_> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "yml" || ext == "yaml")
        })
        .collect();

    if entries.is_empty() {
        return Ok(None);
    }

    entries.sort_by(|a, b| {
        let va = version_from_filename(&a.file_name());
        let vb = version_from_filename(&b.file_name());
        va.cmp(&vb)
    });
    let last = entries.last().unwrap();
    let contents = std::fs::read_to_string(last.path())?;
    Ok(Some(contents))
}

/// Save a snapshot only if the schema has changed from the latest one.
/// Returns the relative path if saved, or None if unchanged.
pub fn save_snapshot(
    snapshots_dir: &Path,
    db_name: &str,
    version: &str,
    schema: &SchemaString,
) -> anyhow::Result<Option<String>> {
    if let Some(latest) = latest_snapshot(snapshots_dir, db_name)? {
        let latest_schema = SchemaString::new(latest.clone()).parse()?;
        let current_schema = schema.parse()?;
        if latest_schema == current_schema {
            return Ok(None);
        }
        eprintln!("--- latest snapshot ---\n{latest}");
        eprintln!("--- current schema ---\n{}", schema.as_str());
    }

    let dir = snapshots_dir.join(db_name);
    std::fs::create_dir_all(&dir)?;

    let filename = format!("{version}.yml");
    let path = dir.join(&filename);

    std::fs::write(&path, schema.as_str())
        .with_context(|| format!("failed to write snapshot to {}", path.display()))?;

    Ok(Some(format!("{db_name}/{filename}")))
}

pub fn load_all_snapshots(
    snapshots_dir: &Path,
    db_name: &str,
) -> anyhow::Result<Vec<(String, SchemaString)>> {
    let dir = snapshots_dir.join(db_name);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<_> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "yml" || ext == "yaml")
        })
        .collect();

    entries.sort_by(|a, b| {
        let va = version_from_filename(&a.file_name());
        let vb = version_from_filename(&b.file_name());
        va.cmp(&vb)
    });

    let mut snapshots = Vec::new();
    for entry in entries {
        let contents = std::fs::read_to_string(entry.path())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        snapshots.push((name, SchemaString::new(contents)));
    }

    Ok(snapshots)
}
