use indexmap::IndexMap;
use notitia_core::{Database, DatatypeKind, OnAction};

use crate::error::SchemaError;
use crate::schema::*;

#[derive(Debug, Clone)]
pub struct SchemaString(String);

impl SchemaString {
    pub fn new(yaml: String) -> Self {
        Self(yaml)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }

    pub fn parse(&self) -> Result<Schema, SchemaError> {
        serde_yaml::from_str(&self.0).map_err(SchemaError::Yaml)
    }

    pub fn from_database<Db: Database>(db: &Db) -> Result<Self, SchemaError> {
        let schema = extract_schema::<Db>(db);
        let yaml = serde_yaml::to_string(&schema).map_err(SchemaError::Yaml)?;
        Ok(Self(yaml))
    }

    pub fn extract<Db: Database>() -> Result<Self, SchemaError> {
        let db = Db::new();
        Self::from_database(&db)
    }
}

impl std::fmt::Display for SchemaString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for SchemaString {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<SchemaString> for String {
    fn from(s: SchemaString) -> Self {
        s.0
    }
}

fn extract_schema<Db: Database>(db: &Db) -> Schema {
    use std::collections::HashMap;

    // Collect migration metadata keyed by table name.
    let migration_meta: HashMap<&str, notitia_core::TableMigrationMeta> =
        db.table_migration_metadata().collect();

    let mut tables = IndexMap::new();

    for (table_name, fields_def) in db.tables() {
        let mut fields = IndexMap::new();

        // Build a map of field_name -> [old_names] from migration metadata.
        let field_migration_map: HashMap<&str, &[&str]> = migration_meta
            .get(table_name)
            .map(|meta| {
                meta.field_migrations
                    .iter()
                    .map(|(current, old_names)| (*current, *old_names))
                    .collect()
            })
            .unwrap_or_default();

        for (field_name, datatype_kind) in fields_def.iter() {
            let mut field_schema = convert_field(datatype_kind);

            // Populate migrate_from for this field.
            if let Some(old_names) = field_migration_map.get(field_name) {
                field_schema.migrate_from = old_names.iter().map(|s| s.to_string()).collect();
            }

            fields.insert(field_name.to_string(), field_schema);
        }

        let foreign_keys = extract_foreign_keys::<Db>(table_name);

        let (table_migrate_from, removed_fields) = match migration_meta.get(table_name) {
            Some(meta) => (
                meta.migrate_from.iter().map(|s| s.to_string()).collect(),
                meta.removed_fields.iter().map(|s| s.to_string()).collect(),
            ),
            None => (Vec::new(), Vec::new()),
        };

        tables.insert(
            table_name.to_string(),
            TableSchema {
                fields,
                foreign_keys,
                removed_fields,
                migrate_from: table_migrate_from,
            },
        );
    }

    Schema {
        tables,
        removed_tables: Db::_REMOVED_TABLES.iter().map(|s| s.to_string()).collect(),
    }
}

fn convert_field(kind: &DatatypeKind) -> FieldSchema {
    let (field_type, metadata) = match kind {
        DatatypeKind::Int(m) => (FieldType::Int, m),
        DatatypeKind::BigInt(m) => (FieldType::BigInt, m),
        DatatypeKind::Float(m) => (FieldType::Float, m),
        DatatypeKind::Double(m) => (FieldType::Double, m),
        DatatypeKind::Text(m) => (FieldType::Text, m),
        DatatypeKind::Blob(m) => (FieldType::Blob, m),
        DatatypeKind::Bool(m) => (FieldType::Bool, m),
    };

    FieldSchema {
        field_type,
        primary_key: metadata.primary_key,
        unique: metadata.unique,
        optional: metadata.optional,
        migrate_from: Vec::new(),
    }
}

fn extract_foreign_keys<Db: Database>(table_name: &str) -> IndexMap<String, ForeignKeySchema> {
    let mut fks = IndexMap::new();

    if let Some(relationships) = Db::_FOREIGN_RELATIONSHIPS.get(table_name) {
        for (local_field, rel) in relationships {
            fks.insert(
                local_field.to_string(),
                ForeignKeySchema {
                    foreign_table: rel.foreign_table.to_string(),
                    foreign_field: rel.foreign_field.to_string(),
                    on_delete: convert_on_action(&rel.on_delete),
                    on_update: convert_on_action(&rel.on_update),
                },
            );
        }
    }

    fks
}

fn convert_on_action(action: &OnAction) -> ActionSchema {
    match action {
        OnAction::NoAction => ActionSchema::NoAction,
        OnAction::Restrict => ActionSchema::Restrict,
        OnAction::SetNull => ActionSchema::SetNull,
        OnAction::SetDefault => ActionSchema::SetDefault,
        OnAction::Cascade => ActionSchema::Cascade,
    }
}
