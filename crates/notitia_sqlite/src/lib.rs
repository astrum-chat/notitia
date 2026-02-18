#[cfg(test)]
extern crate notitia_core as notitia;

mod convert_stmts;
pub use convert_stmts::*;

use std::{path::Path, sync::Arc};

use notitia_core::{
    Adapter, Database, Datatype, DeleteStmtBuilt, FieldKindGroup, InsertStmtBuilt, Notitia,
    OrderKey, PartialRecord, Record, SelectStmtBuilt, SelectStmtFetchMode, UpdateStmtBuilt,
};
use smallvec::SmallVec;
use sqlx::{Column, Pool, Row, Sqlite, TypeInfo, sqlite::SqlitePoolOptions};
use unions::IsUnion;

fn sqlite_row_column_to_datatype(row: &sqlx::sqlite::SqliteRow, index: usize) -> Datatype {
    let col = &row.columns()[index];
    let type_name = col.type_info().name();

    match type_name {
        "TEXT" => {
            let v: String = row.get(index);
            Datatype::Text(v)
        }
        "INTEGER" | "INT" | "BIGINT" => {
            let v: i64 = row.get(index);
            Datatype::BigInt(v)
        }
        "REAL" | "FLOAT" | "DOUBLE" => {
            let v: f64 = row.get(index);
            Datatype::Double(v)
        }
        "BLOB" => {
            let v: Vec<u8> = row.get(index);
            Datatype::Blob(v)
        }
        "BOOLEAN" => {
            let v: bool = row.get(index);
            Datatype::Bool(v)
        }
        "NULL" => Datatype::Null,
        _ => {
            // Fall back: try text, then blob
            if let Ok(v) = row.try_get::<String, _>(index) {
                Datatype::Text(v)
            } else if let Ok(v) = row.try_get::<Vec<u8>, _>(index) {
                Datatype::Blob(v)
            } else {
                Datatype::Null
            }
        }
    }
}

pub struct SqliteAdapter
where
    Self: Send + Sync,
{
    connection: Arc<Pool<Sqlite>>,
}

impl Adapter for SqliteAdapter {
    type QueryBuilder = sea_query::SqliteQueryBuilder;
    type Connection = Arc<Pool<Sqlite>>;
    type Error = sqlx::Error;

    fn new(connection: Self::Connection) -> Self {
        Self { connection }
    }

    async fn initialize<Db: Database>(&self, database: &Db) {
        let mut schema_sql = database.schema_sql(Self::QueryBuilder::default());

        if Db::_FOREIGN_RELATIONSHIPS.len() != 0 {
            schema_sql = format!("PRAGMA foreign_keys = ON;\n\n{}", schema_sql);
        };

        sqlx::query(&schema_sql)
            .execute(self.connection.as_ref())
            .await
            .unwrap();
    }

    async fn open<Db: Database>(url: &str) -> Result<Notitia<Db, Self>, Self::Error> {
        fn create_local_file(url: &str) -> std::io::Result<()> {
            if let Some(path) = url
                .strip_prefix("sqlite://")
                .or_else(|| url.strip_prefix("sqlite:"))
            {
                if path != ":memory:" && !path.starts_with(":memory:") {
                    let path = Path::new(path.split('?').next().unwrap_or(path));

                    // Create parent directories if needed.
                    if let Some(parent) = path.parent() {
                        if !parent.as_os_str().is_empty() {
                            std::fs::create_dir_all(parent)?;
                        }
                    }

                    if !path.exists() {
                        std::fs::File::create(path)?;
                    }
                }
            }

            Ok(())
        }

        // TODO: better error handling via early return with Result::Err.
        create_local_file(url).unwrap();

        let connection = SqlitePoolOptions::new().connect(url).await?;

        Ok(Notitia::new(Db::new(), Self::new(Arc::new(connection))).await)
    }

    async fn execute_select_stmt<Db, FieldUnion, FieldPath, Fields, Mode>(
        &self,
        stmt: &SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, Mode>,
    ) -> Result<Mode::Output, Self::Error>
    where
        Db: Database,
        FieldUnion: IsUnion + Send + Sync,
        FieldPath: Send + Sync,
        Fields: FieldKindGroup<FieldUnion, FieldPath> + Send + Sync,
        Mode: SelectStmtFetchMode<Fields::Type> + Sync,
    {
        let sql = select_stmt_to_sql(stmt);
        let rows = sqlx::query(&sql)
            .fetch_all(self.connection.as_ref())
            .await?;

        let needs_order_keys = stmt.mode.needs_order_keys();
        let field_names = stmt.fields.field_names();
        let user_field_count = field_names.len();

        // Build column index mapping for ORDER BY fields (only when needed).
        let order_key_indices: SmallVec<[usize; 1]> = if needs_order_keys {
            let mut indices = SmallVec::new();
            let mut extra_col_idx = user_field_count;
            for order in &stmt.order_by {
                if let Some(pos) = field_names.iter().position(|n| *n == order.field) {
                    indices.push(pos);
                } else {
                    indices.push(extra_col_idx);
                    extra_col_idx += 1;
                }
            }
            indices
        } else {
            SmallVec::new()
        };

        let (typed_rows, order_keys): (Vec<_>, Vec<_>) = rows
            .into_iter()
            .map(|row| {
                let all_values: Vec<Datatype> = (0..row.columns().len())
                    .map(|i| sqlite_row_column_to_datatype(&row, i))
                    .collect();

                let order_key = if needs_order_keys {
                    OrderKey::new(
                        order_key_indices
                            .iter()
                            .map(|&idx| all_values[idx].clone())
                            .collect(),
                        stmt.order_by
                            .iter()
                            .map(|o| matches!(o.direction, notitia_core::OrderDirection::Desc))
                            .collect(),
                    )
                } else {
                    OrderKey::default()
                };

                let user_values: Vec<Datatype> =
                    all_values.into_iter().take(user_field_count).collect();
                let typed = Fields::from_datatypes(&mut user_values.into_iter())
                    .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
                Ok((typed, order_key))
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()?
            .into_iter()
            .unzip();

        stmt.mode
            .from_rows(typed_rows, order_keys)
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))
    }

    async fn execute_insert_stmt<Db: Database, R: Record + Send>(
        &self,
        stmt: InsertStmtBuilt<Db, R>,
    ) -> Result<(), Self::Error> {
        let fields = stmt.record.into_datatypes();
        let sql = insert_stmt_to_sql(stmt.table_name, &fields);
        sqlx::query(&sql).execute(self.connection.as_ref()).await?;
        Ok(())
    }

    async fn execute_update_stmt<Db: Database, Rec: Record + Send, P: PartialRecord + Send>(
        &self,
        stmt: UpdateStmtBuilt<Db, Rec, P>,
    ) -> Result<(), Self::Error> {
        let fields = stmt.partial.into_set_fields();
        let sql = update_stmt_to_sql(stmt.table_name, &fields, &stmt.filters);
        sqlx::query(&sql).execute(self.connection.as_ref()).await?;
        Ok(())
    }

    async fn execute_delete_stmt<Db: Database, Rec: Record + Send>(
        &self,
        stmt: DeleteStmtBuilt<Db, Rec>,
    ) -> Result<(), Self::Error> {
        let sql = delete_stmt_to_sql(stmt.table_name, &stmt.filters);
        sqlx::query(&sql).execute(self.connection.as_ref()).await?;
        Ok(())
    }
}
