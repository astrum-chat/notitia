use notitia_migrations::check_compatibility;

use crate::config::Config;
use crate::extract::extract_schemas;
use crate::snapshot::{load_all_snapshots, read_crate_version, save_snapshot};

pub fn run(verbose: bool, tmp: bool, krate: Option<&str>) -> anyhow::Result<()> {
    let config = Config::load()?;
    let version = read_crate_version()?;

    println!("Extracting schemas...");
    let schemas = extract_schemas(verbose, tmp, krate)?;

    // Check compatibility with existing snapshots before saving.
    let mut any_issues = false;
    for (db_name, current) in &schemas {
        let named_snapshots = load_all_snapshots(&config.snapshots_dir, db_name)?;
        if named_snapshots.is_empty() {
            continue;
        }

        let (names, snapshots): (Vec<String>, Vec<_>) = named_snapshots.into_iter().unzip();
        let results = check_compatibility(current, &snapshots)?;

        for result in &results {
            if !result.is_compatible() {
                any_issues = true;
                println!("INCOMPATIBLE: {db_name}/{}", names[result.index]);
                for issue in &result.errors {
                    println!("  - {issue}");
                }
            }
        }
    }

    if any_issues {
        anyhow::bail!(
            "compatibility check failed. Fix the issues above or use `notitia check` for details."
        );
    }

    for (db_name, schema) in &schemas {
        match save_snapshot(&config.snapshots_dir, db_name, &version, schema)? {
            Some(path) => println!("  Saved: {path}"),
            None => println!("  {db_name}: unchanged, skipped."),
        }
    }

    Ok(())
}
