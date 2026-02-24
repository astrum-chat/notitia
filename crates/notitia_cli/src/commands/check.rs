use std::fmt::Write;

use notitia_migrations::check_compatibility;

use crate::config::Config;
use crate::extract::extract_schemas;
use crate::snapshot::load_all_snapshots;

fn format_issues(errors: &[notitia_migrations::CompatIssue]) -> String {
    let mut out = String::new();
    for (idx, issue) in errors.iter().enumerate() {
        if idx > 0 {
            out.push_str("\n\n");
        }
        let msg = issue.to_string();
        for (i, line) in msg.lines().enumerate() {
            if i == 0 {
                let _ = writeln!(out, "  - {line}");
            } else {
                let _ = writeln!(out, "    {line}");
            }
        }
    }
    out
}

pub fn run(verbose: bool, tmp: bool, krate: Option<&str>) -> anyhow::Result<()> {
    let config = Config::load()?;
    let mut output = String::from("Extracting current schemas...\n");
    let schemas = extract_schemas(verbose, tmp, krate)?;
    let mut any_issues = false;

    for (db_name, current) in &schemas {
        let named_snapshots = load_all_snapshots(&config.snapshots_dir, db_name)?;
        if named_snapshots.is_empty() {
            let _ = writeln!(output, "{db_name}: no snapshots found, skipping.");
            continue;
        }

        let (names, snapshots): (Vec<String>, Vec<_>) = named_snapshots.into_iter().unzip();
        let results = check_compatibility(current, &snapshots)?;

        for result in &results {
            if !result.is_compatible() {
                if any_issues {
                    output.push('\n');
                }
                any_issues = true;
                let _ = writeln!(output, "INCOMPATIBLE: {db_name}/{}", names[result.index]);
                output.push_str(&format_issues(&result.errors));
            }
        }

        if results.iter().all(|r| r.is_compatible()) {
            let _ = writeln!(output, "{db_name}: all {} snapshot(s) compatible.", snapshots.len());
        }
    }

    print!("{output}");

    if any_issues {
        std::process::exit(1);
    }

    Ok(())
}
