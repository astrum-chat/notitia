mod foreign_relationship;

pub use foreign_relationship::{ForeignRelationship, OnAction};

use crate::{
    Adapter, DatatypeKind, DatatypeKindMetadata, FieldsDef, Notitia, TableKind,
    utils::iter_join::Join,
};

pub struct EmbeddedTableDef {
    pub table_name: &'static str,
    pub embedded_fields: &'static [(&'static str, &'static str)],
    pub pk_field: &'static str,
}

pub trait Database: Send + Sync + Sized {
    type TableKind: TableKind;

    const _FOREIGN_RELATIONSHIPS: phf::Map<
        &'static str,
        phf::Map<&'static str, ForeignRelationship>,
    >;

    fn tables(&self) -> impl Iterator<Item = (&'static str, FieldsDef)>;

    fn schema_sql(&self, schema_builder: impl sea_query::SchemaBuilder) -> String {
        fn set_column_metadata<'a>(
            column: &'a mut sea_query::ColumnDef,
            metadata: &DatatypeKindMetadata,
        ) -> &'a mut sea_query::ColumnDef {
            if metadata.primary_key {
                column.primary_key();
            }

            if metadata.unique {
                column.unique_key();
            }

            if !metadata.optional {
                column.not_null();
            }

            column
        }

        fn set_column_type<'a>(
            column: &'a mut sea_query::ColumnDef,
            datatype: &DatatypeKind,
        ) -> &'a mut sea_query::ColumnDef {
            match datatype {
                DatatypeKind::Int(metadata) => set_column_metadata(column.integer(), metadata),
                DatatypeKind::BigInt(metadata) => {
                    set_column_metadata(column.big_integer(), metadata)
                }
                DatatypeKind::Float(metadata) => set_column_metadata(column.float(), metadata),
                DatatypeKind::Double(metadata) => set_column_metadata(column.double(), metadata),
                DatatypeKind::Text(metadata) => set_column_metadata(column.text(), metadata),
                DatatypeKind::Blob(metadata) => set_column_metadata(column.blob(), metadata),
                DatatypeKind::Bool(metadata) => set_column_metadata(column.boolean(), metadata),
            }
        }

        fn set_relationship_on_delete<'a>(
            relationship: &'a mut sea_query::ForeignKeyCreateStatement,
            on_delete: &OnAction,
        ) -> &'a mut sea_query::ForeignKeyCreateStatement {
            match on_delete {
                OnAction::NoAction => relationship.on_delete(sea_query::ForeignKeyAction::NoAction),
                OnAction::Restrict => relationship.on_delete(sea_query::ForeignKeyAction::Restrict),
                OnAction::SetNull => relationship.on_delete(sea_query::ForeignKeyAction::SetNull),
                OnAction::SetDefault => {
                    relationship.on_delete(sea_query::ForeignKeyAction::SetDefault)
                }
                OnAction::Cascade => relationship.on_delete(sea_query::ForeignKeyAction::Cascade),
            }
        }

        fn set_relationship_on_update<'a>(
            relationship: &'a mut sea_query::ForeignKeyCreateStatement,
            on_update: &OnAction,
        ) -> &'a mut sea_query::ForeignKeyCreateStatement {
            match on_update {
                OnAction::NoAction => relationship.on_update(sea_query::ForeignKeyAction::NoAction),
                OnAction::Restrict => relationship.on_update(sea_query::ForeignKeyAction::Restrict),
                OnAction::SetNull => relationship.on_update(sea_query::ForeignKeyAction::SetNull),
                OnAction::SetDefault => {
                    relationship.on_update(sea_query::ForeignKeyAction::SetDefault)
                }
                OnAction::Cascade => relationship.on_update(sea_query::ForeignKeyAction::Cascade),
            }
        }

        self.tables()
            .map(|(table_name, rows)| {
                let mut table = sea_query::Table::create()
                    .if_not_exists()
                    .table(table_name)
                    .to_owned();

                for (field_name, datatype) in rows.iter() {
                    table.col(set_column_type(
                        &mut sea_query::ColumnDef::new(*field_name),
                        datatype,
                    ));
                }

                if let Some(relationships) = Self::_FOREIGN_RELATIONSHIPS.get(table_name) {
                    for (local_field_name, foreign_table) in relationships {
                        table.foreign_key(set_relationship_on_update(
                            set_relationship_on_delete(
                                &mut sea_query::ForeignKey::create()
                                    .from(table_name, *local_field_name)
                                    .to(foreign_table.foreign_table, foreign_table.foreign_field),
                                &foreign_table.on_delete,
                            ),
                            &foreign_table.on_update,
                        ));
                    }
                }

                format!("{};", table.build_any(&schema_builder))
            })
            .join("\n\n")
    }

    fn embedded_tables(&self) -> Vec<EmbeddedTableDef> {
        Vec::new()
    }

    fn new() -> Self;

    fn connect<Adptr: Adapter>(
        options: impl Into<ConnectionOptions> + Send,
    ) -> impl Future<Output = Result<Notitia<Self, Adptr>, ConnectionError<Adptr::Error>>> + Send
    {
        async move {
            let options = options.into();

            let db = Adptr::open::<Self>(&options.uri)
                .await
                .map_err(ConnectionError::Adapter)?;

            #[cfg(feature = "embeddings")]
            {
                let embedded = db.database().embedded_tables();
                if !embedded.is_empty() {
                    let default_uri = options.default_embeddings_uri();
                    let embeddings_uri = options.embeddings_uri.unwrap_or(default_uri);
                    let embedder = options.embedder.ok_or(ConnectionError::EmbedderRequired)?;
                    let manager = crate::embeddings::EmbeddingManager::new(
                        &embeddings_uri,
                        embedder,
                        &embedded,
                    )
                    .map_err(|e| ConnectionError::Embeddings(e))?;
                    db.set_embedding_manager(std::sync::Arc::new(manager));
                }
            }

            Ok(db)
        }
    }
}

pub struct ConnectionOptions {
    pub uri: String,
    pub embeddings_uri: Option<String>,
    #[cfg(feature = "embeddings")]
    pub(crate) embedder: Option<Box<dyn crate::embeddings::DatabaseEmbedder>>,
}

impl ConnectionOptions {
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            embeddings_uri: None,
            #[cfg(feature = "embeddings")]
            embedder: None,
        }
    }

    pub fn embeddings_uri(mut self, uri: impl Into<String>) -> Self {
        self.embeddings_uri = Some(uri.into());
        self
    }

    #[cfg(feature = "embeddings")]
    pub fn embedder(
        mut self,
        embedder: impl crate::embeddings::DatabaseEmbedder + 'static,
    ) -> Self {
        self.embedder = Some(Box::new(embedder));
        self
    }

    #[cfg(feature = "embeddings")]
    fn default_embeddings_uri(&self) -> String {
        let raw = self.uri.strip_prefix("sqlite:").unwrap_or(&self.uri);
        let path = std::path::Path::new(raw);
        let parent = path.parent().unwrap_or(std::path::Path::new("."));
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("db");
        parent
            .join(format!("{stem}_embeddings"))
            .to_string_lossy()
            .into_owned()
    }
}

impl From<&str> for ConnectionOptions {
    fn from(uri: &str) -> Self {
        Self::new(uri)
    }
}

impl From<String> for ConnectionOptions {
    fn from(uri: String) -> Self {
        Self::new(uri)
    }
}

impl From<&String> for ConnectionOptions {
    fn from(uri: &String) -> Self {
        Self::new(uri.clone())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError<E: std::error::Error> {
    #[error("{0}")]
    Adapter(E),
    #[cfg(feature = "embeddings")]
    #[error("this database has embedded fields but no embedder was provided")]
    EmbedderRequired,
    #[cfg(feature = "embeddings")]
    #[error("embedding initialization failed: {0}")]
    Embeddings(crate::embeddings::EmbeddingError),
}

impl Database for () {
    type TableKind = ();

    const _FOREIGN_RELATIONSHIPS: phf::Map<
        &'static str,
        phf::Map<&'static str, crate::ForeignRelationship>,
    > = phf::Map::new();

    fn tables(&self) -> impl Iterator<Item = (&'static str, FieldsDef)> {
        std::iter::empty()
    }

    fn new() -> Self {
        ()
    }
}

impl TableKind for () {
    fn name(&self) -> &'static str {
        "()"
    }
}

pub trait OnStartup: Database {
    fn on_startup(&self) -> impl Future<Output = Result<(), ()>> + Send;
}
