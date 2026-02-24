mod error;
mod schema;
mod schema_string;
mod compat;

pub use notitia_core::Database;
pub use error::SchemaError;
pub use schema::{ActionSchema, FieldSchema, FieldType, ForeignKeySchema, Schema, TableSchema};
pub use schema_string::SchemaString;
pub use compat::{check_compatibility, CompatIssue, CompatResult};
