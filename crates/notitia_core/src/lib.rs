pub use phf;

mod database;
pub use database::*;

mod table;
pub use table::*;

mod stmts;
pub use stmts::*;

mod record;
pub use record::*;

mod field;
pub use field::*;

mod datatype;
pub use datatype::*;

mod adapter;
pub use adapter::*;

mod utils;
pub use utils::*;

mod subscription;
pub use subscription::*;

use std::sync::Arc;

pub(crate) struct NotitiaInner<Db, Adptr>
where
    Db: Database,
    Adptr: Adapter,
{
    database: Db,
    pub(crate) adapter: Adptr,
    pub(crate) subscriptions: SubscriptionRegistry,
}

pub struct Notitia<Db, Adptr>
where
    Db: Database,
    Adptr: Adapter,
{
    pub(crate) inner: Arc<NotitiaInner<Db, Adptr>>,
}

impl<Db, Adptr> Clone for Notitia<Db, Adptr>
where
    Db: Database,
    Adptr: Adapter,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<Db, Adptr> Notitia<Db, Adptr>
where
    Db: Database,
    Adptr: Adapter,
{
    pub async fn new(database: Db, adapter: Adptr) -> Self {
        adapter.initialize(&database).await;

        Self {
            inner: Arc::new(NotitiaInner {
                database,
                adapter,
                subscriptions: SubscriptionRegistry::new(),
            }),
        }
    }

    pub fn notify_subscribers(&self, event: &MutationEvent) {
        self.inner.subscriptions.broadcast(event);
    }

    pub fn query<FieldUnion, FieldPath, Fields, Mode>(
        &self,
        stmt: SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, Mode>,
    ) -> QueryExecutor<Db, Adptr, FieldUnion, FieldPath, Fields, Mode>
    where
        FieldUnion: unions::IsUnion,
        Fields: FieldKindGroup<FieldUnion, FieldPath>,
        Mode: SelectStmtFetchMode<Fields::Type>,
    {
        QueryExecutor {
            db: self.clone(),
            stmt,
        }
    }

    pub fn mutate<M: Mutation<Db>>(&self, stmt: M) -> MutateExecutor<Db, Adptr, M> {
        MutateExecutor {
            db: self.clone(),
            stmt,
        }
    }

    pub(crate) async fn execute_select_stmt<FieldUnion, FieldPath, Fields, Mode>(
        &self,
        stmt: &SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, Mode>,
    ) -> Result<Mode::Output, Adptr::Error>
    where
        FieldUnion: unions::IsUnion + Send + Sync,
        FieldPath: Send + Sync,
        Fields: FieldKindGroup<FieldUnion, FieldPath> + Send + Sync,
        Mode: SelectStmtFetchMode<Fields::Type> + Sync,
    {
        self.inner.adapter.execute_select_stmt(stmt).await
    }

    pub(crate) async fn execute_insert_stmt<R: Record + Send>(
        &self,
        stmt: InsertStmtBuilt<Db, R>,
    ) -> Result<(), Adptr::Error> {
        self.inner.adapter.execute_insert_stmt(stmt).await
    }

    pub(crate) async fn execute_update_stmt<Rec: Record + Send, P: PartialRecord + Send>(
        &self,
        stmt: UpdateStmtBuilt<Db, Rec, P>,
    ) -> Result<(), Adptr::Error> {
        self.inner.adapter.execute_update_stmt(stmt).await
    }

    pub(crate) async fn execute_delete_stmt<Rec: Record + Send>(
        &self,
        stmt: DeleteStmtBuilt<Db, Rec>,
    ) -> Result<(), Adptr::Error> {
        self.inner.adapter.execute_delete_stmt(stmt).await
    }
}

pub trait Connection {}
