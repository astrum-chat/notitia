use std::collections::HashMap;

use rayon::prelude::*;

use crate::error::SchemaError;
use crate::schema::{FieldType, Schema, TableSchema};
use crate::schema_string::SchemaString;

#[derive(Debug)]
pub struct CompatResult {
    pub index: usize,
    pub errors: Vec<CompatIssue>,
}

impl CompatResult {
    pub fn is_compatible(&self) -> bool {
        self.errors.is_empty()
    }
}

/// An incompatibility that prevents migration.
#[derive(Debug, Clone)]
pub enum CompatIssue {
    /// A table from the old schema is missing in the current schema and was
    /// neither renamed (via `migrate_from`) nor declared in `removed_tables`.
    UndeclaredTableRemoval { table: String },

    /// A field from the old schema is missing in the current schema and was
    /// neither renamed (via `migrate_from`) nor declared in `removed_fields`.
    UndeclaredFieldRemoval { table: String, field: String },

    /// A field's type changed between schemas.
    FieldTypeChanged {
        table: String,
        field: String,
        old_type: FieldType,
        new_type: FieldType,
    },

    /// A field that was optional became required.
    FieldBecameRequired { table: String, field: String },
}

impl std::fmt::Display for CompatIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UndeclaredTableRemoval { table } => {
                write!(
                    f,
                    "table '{table}' was removed without being declared in \
                     `removed_tables` or accounted for by `migrate_from`\n \
                     - If the table was renamed, add to the new table field in your database:\n    \
                     #[db(migrate_from({table}))]\n\n \
                     - If the table was intentionally deleted, add to your database struct:\n    \
                     #[database(removed_tables({table}))]"
                )
            }
            Self::UndeclaredFieldRemoval { table, field } => {
                write!(
                    f,
                    "field '{table}.{field}' was removed without being declared in \
                     `removed_fields` or accounted for by `migrate_from`\n \
                     - If the field was renamed, add to the new field in your record:\n    \
                     #[db(migrate_from({field}))]\n\n \
                     - If the field was intentionally deleted, add to your record struct:\n    \
                     #[record(removed_fields({field}))]"
                )
            }
            Self::FieldTypeChanged {
                table,
                field,
                old_type,
                new_type,
            } => {
                write!(
                    f,
                    "field '{table}.{field}' changed type from {old_type:?} to {new_type:?}\n\
                     \n\
                     Type changes require a migration strategy. Consider adding a new field \
                     with the desired type and using `#[db(migrate_from({field}))]` to map \
                     from the old field."
                )
            }
            Self::FieldBecameRequired { table, field } => {
                write!(
                    f,
                    "field '{table}.{field}' changed from optional to required\n\
                     \n\
                     Existing rows may have NULL values for this field. Ensure your migration \
                     populates a default value before making the column required."
                )
            }
        }
    }
}

pub fn check_compatibility(
    current: &SchemaString,
    snapshots: &[SchemaString],
) -> Result<Vec<CompatResult>, SchemaError> {
    let current_schema = current.parse()?;
    let table_rename_map = build_table_rename_map(&current_schema);
    let field_rename_map = build_field_rename_map(&current_schema);

    let results: Vec<CompatResult> = snapshots
        .par_iter()
        .enumerate()
        .map(|(index, snapshot)| {
            let errors = match snapshot.parse() {
                Ok(old_schema) => check_one(
                    &current_schema,
                    &old_schema,
                    &table_rename_map,
                    &field_rename_map,
                ),
                Err(_) => vec![],
            };
            CompatResult { index, errors }
        })
        .collect();

    Ok(results)
}

/// Build a map from old_table_name -> new_table_name
/// using `migrate_from` annotations on current tables.
fn build_table_rename_map(current: &Schema) -> HashMap<&str, &str> {
    let mut map = HashMap::new();

    for (table_name, table_schema) in &current.tables {
        for old_name in &table_schema.migrate_from {
            map.insert(old_name.as_str(), table_name.as_str());
        }
    }

    map
}

/// Build a map from (current_table_name, old_field_name) -> new_field_name
/// using `migrate_from` annotations on current fields.
fn build_field_rename_map(current: &Schema) -> HashMap<(&str, &str), &str> {
    let mut map = HashMap::new();

    for (table_name, table_schema) in &current.tables {
        for (field_name, field_schema) in &table_schema.fields {
            for old_name in &field_schema.migrate_from {
                map.insert(
                    (table_name.as_str(), old_name.as_str()),
                    field_name.as_str(),
                );
            }
        }
    }

    map
}

fn check_one(
    current: &Schema,
    old: &Schema,
    table_rename_map: &HashMap<&str, &str>,
    field_rename_map: &HashMap<(&str, &str), &str>,
) -> Vec<CompatIssue> {
    let mut issues = Vec::new();

    for (old_table_name, old_table_schema) in &old.tables {
        // Resolve the current table for this old table.
        let current_table = resolve_table(current, old_table_name, table_rename_map);

        let Some((current_table_name, current_table)) = current_table else {
            // Check if declared in removed_tables.
            if current.removed_tables.iter().any(|t| t == old_table_name) {
                continue;
            }

            issues.push(CompatIssue::UndeclaredTableRemoval {
                table: old_table_name.clone(),
            });
            continue;
        };

        check_table_fields(
            &mut issues,
            old_table_name,
            old_table_schema,
            current_table_name,
            current_table,
            field_rename_map,
        );
    }

    issues
}

/// Resolve an old table name to the current table.
/// Tries direct match first, then rename via `migrate_from`.
fn resolve_table<'a>(
    current: &'a Schema,
    old_table_name: &str,
    table_rename_map: &HashMap<&str, &'a str>,
) -> Option<(&'a str, &'a TableSchema)> {
    // 1. Direct match.
    if let Some(table) = current.tables.get(old_table_name) {
        // Find the key in the IndexMap to get the &str with the right lifetime.
        let name = current
            .tables
            .get_key_value(old_table_name)
            .unwrap()
            .0
            .as_str();
        return Some((name, table));
    }

    // 2. Rename — a current table has migrate_from pointing to this old name.
    if let Some(&new_name) = table_rename_map.get(old_table_name) {
        if let Some(table) = current.tables.get(new_name) {
            return Some((new_name, table));
        }
    }

    None
}

fn check_table_fields(
    issues: &mut Vec<CompatIssue>,
    old_table_name: &str,
    old_table: &TableSchema,
    current_table_name: &str,
    current_table: &TableSchema,
    field_rename_map: &HashMap<(&str, &str), &str>,
) {
    for (old_field_name, old_field_schema) in &old_table.fields {
        // 1. Direct match — field still exists with the same name.
        if let Some(current_field) = current_table.fields.get(old_field_name.as_str()) {
            check_field_compat(
                issues,
                old_table_name,
                old_field_name,
                old_field_schema,
                current_field,
            );
            continue;
        }

        // 2. Rename — current table has a field with migrate_from pointing here.
        if let Some(&new_name) =
            field_rename_map.get(&(current_table_name, old_field_name.as_str()))
        {
            if let Some(current_field) = current_table.fields.get(new_name) {
                check_field_compat(
                    issues,
                    old_table_name,
                    old_field_name,
                    old_field_schema,
                    current_field,
                );
                continue;
            }
        }

        // 3. Declared removal — field is listed in removed_fields.
        if current_table
            .removed_fields
            .iter()
            .any(|f| f == old_field_name)
        {
            continue;
        }

        // 4. None of the above — undeclared removal, this is an error.
        issues.push(CompatIssue::UndeclaredFieldRemoval {
            table: old_table_name.to_string(),
            field: old_field_name.clone(),
        });
    }
}

fn check_field_compat(
    issues: &mut Vec<CompatIssue>,
    table: &str,
    field: &str,
    old: &crate::schema::FieldSchema,
    current: &crate::schema::FieldSchema,
) {
    if old.field_type != current.field_type {
        issues.push(CompatIssue::FieldTypeChanged {
            table: table.to_string(),
            field: field.to_string(),
            old_type: old.field_type,
            new_type: current.field_type,
        });
    }

    if old.optional && !current.optional {
        issues.push(CompatIssue::FieldBecameRequired {
            table: table.to_string(),
            field: field.to_string(),
        });
    }
}
