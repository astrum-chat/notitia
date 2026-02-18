use std::marker::PhantomData;

use smallvec::SmallVec;
use unions::{IntoUnion, UnionPath};

use crate::{
    Adapter, Database, DeleteStmtBuilt, FieldKindOfDatabase, InnerFieldType, Mutation,
    MutationEvent, MutationEventKind, Notitia, Record, StrongFieldFilter,
};

pub struct DeleteStmtUnbuilt<Db: Database, Rec: Record> {
    pub table_name: &'static str,
    _database: PhantomData<Db>,
    _record: PhantomData<Rec>,
}

impl<Db: Database, Rec: Record> DeleteStmtUnbuilt<Db, Rec> {
    pub(crate) fn new(table_name: &'static str) -> Self {
        Self {
            table_name,
            _database: PhantomData,
            _record: PhantomData,
        }
    }

    pub fn filter<FieldPath: UnionPath, Field, T>(
        self,
        filter: StrongFieldFilter<Field, T>,
    ) -> DeleteStmtBuilt<Db, Rec>
    where
        Field: FieldKindOfDatabase<Db> + IntoUnion<Rec::FieldKind, FieldPath>,
        T: InnerFieldType,
    {
        let mut filters = SmallVec::new();
        filters.push(filter.to_weak());

        DeleteStmtBuilt::new(self.table_name, filters)
    }
}

impl<Db, Rec> Mutation<Db> for DeleteStmtUnbuilt<Db, Rec>
where
    Db: Database,
    Rec: Record + Send,
{
    type Output = ();

    fn to_mutation_event(&self) -> MutationEvent {
        MutationEvent {
            table_name: self.table_name,
            kind: MutationEventKind::Delete {
                filters: SmallVec::new(),
            },
        }
    }

    async fn execute<Adptr: Adapter>(self, db: &Notitia<Db, Adptr>) -> Result<(), Adptr::Error> {
        let built: DeleteStmtBuilt<Db, Rec> =
            DeleteStmtBuilt::new(self.table_name, SmallVec::new());
        db.execute_delete_stmt(built).await
    }
}
