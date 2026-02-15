mod foreign_relationship;

pub use foreign_relationship::{ForeignRelationship, OnAction};

use crate::{
    Adapter, DatatypeKind, DatatypeKindMetadata, FieldsDef, Notitia, TableKind,
    utils::iter_join::Join,
};

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

    fn new() -> Self;

    fn connect<Adptr: Adapter>(
        url: &str,
    ) -> impl Future<Output = Result<Notitia<Self, Adptr>, <Adptr as Adapter>::Error>> + Send {
        async move { Adptr::open::<Self>(url).await }
    }
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
