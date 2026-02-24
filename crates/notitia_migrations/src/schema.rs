use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Schema {
    pub tables: IndexMap<String, TableSchema>,

    /// Tables that were intentionally removed from the schema.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_tables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableSchema {
    pub fields: IndexMap<String, FieldSchema>,

    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub foreign_keys: IndexMap<String, ForeignKeySchema>,

    /// Fields that were intentionally removed from this table.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_fields: Vec<String>,

    /// Previous names this table was known by (for rename tracking).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub migrate_from: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldSchema {
    #[serde(rename = "type")]
    pub field_type: FieldType,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub primary_key: bool,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unique: bool,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,

    /// Previous names this field was known by (for rename tracking).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub migrate_from: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FieldType {
    Int,
    BigInt,
    Float,
    Double,
    Text,
    Blob,
    Bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForeignKeySchema {
    pub foreign_table: String,
    pub foreign_field: String,

    #[serde(default)]
    pub on_delete: ActionSchema,

    #[serde(default)]
    pub on_update: ActionSchema,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActionSchema {
    #[default]
    NoAction,
    Restrict,
    SetNull,
    SetDefault,
    Cascade,
}
