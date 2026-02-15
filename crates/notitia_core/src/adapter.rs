use std::error::Error;

use unions::IsUnion;

use crate::{
    Database, DeleteStmtBuilt, FieldKindGroup, InsertStmtBuilt, Notitia, PartialRecord, Record,
    SelectStmtBuilt, SelectStmtFetchMode, UpdateStmtBuilt,
};

pub trait Adapter: Sized + Send + Sync {
    type QueryBuilder: sea_query::QueryBuilder;
    type Connection: Send + Sync;
    type Error: Error;

    fn new(connection: Self::Connection) -> Self;

    fn initialize<Db: Database>(&self, database: &Db) -> impl Future<Output = ()> + Send;

    fn open<Db: Database>(
        url: &str,
    ) -> impl Future<Output = Result<Notitia<Db, Self>, Self::Error>> + Send;

    fn execute_select_stmt<Db, FieldUnion, FieldPath, Fields, Mode>(
        &self,
        stmt: &SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, Mode>,
    ) -> impl Future<Output = Result<Mode::Output, Self::Error>> + Send
    where
        Db: Database,
        FieldUnion: IsUnion + Send + Sync,
        FieldPath: Send + Sync,
        Fields: FieldKindGroup<FieldUnion, FieldPath> + Send + Sync,
        Mode: SelectStmtFetchMode<Fields::Type> + Sync;

    fn execute_insert_stmt<Db: Database, R: Record + Send>(
        &self,
        stmt: InsertStmtBuilt<Db, R>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    fn execute_update_stmt<Db: Database, Rec: Record + Send, P: PartialRecord + Send>(
        &self,
        stmt: UpdateStmtBuilt<Db, Rec, P>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    fn execute_delete_stmt<Db: Database, Rec: Record + Send>(
        &self,
        stmt: DeleteStmtBuilt<Db, Rec>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
