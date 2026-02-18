use std::marker::PhantomData;

use smallvec::SmallVec;
use unions::{IntoUnion, UnionPath};

use crate::{
    Adapter, Database, FieldFilter, FieldKindOfDatabase, InnerFieldType, Mutation, MutationEvent,
    MutationEventKind, Notitia, Record, StrongFieldFilter,
};

pub struct DeleteStmtBuilt<Db: Database, Rec: Record> {
    pub table_name: &'static str,
    pub filters: SmallVec<[FieldFilter; 1]>,
    _database: PhantomData<Db>,
    _record: PhantomData<Rec>,
}

impl<Db: Database, Rec: Record> DeleteStmtBuilt<Db, Rec> {
    pub(crate) fn new(table_name: &'static str, filters: SmallVec<[FieldFilter; 1]>) -> Self {
        Self {
            table_name,
            filters,
            _database: PhantomData,
            _record: PhantomData,
        }
    }

    pub fn filter<FieldPath: UnionPath, Field, T>(
        mut self,
        filter: StrongFieldFilter<Field, T>,
    ) -> Self
    where
        Field: FieldKindOfDatabase<Db> + IntoUnion<Rec::FieldKind, FieldPath>,
        T: InnerFieldType,
    {
        self.filters.push(filter.to_weak());
        self
    }
}

impl<Db, Rec> Mutation<Db> for DeleteStmtBuilt<Db, Rec>
where
    Db: Database,
    Rec: Record + Send,
{
    type Output = ();

    fn to_mutation_event(&self) -> MutationEvent {
        MutationEvent {
            table_name: self.table_name,
            kind: MutationEventKind::Delete {
                filters: self.filters.clone(),
            },
        }
    }

    async fn execute<Adptr: Adapter>(self, db: &Notitia<Db, Adptr>) -> Result<(), Adptr::Error> {
        db.execute_delete_stmt(self).await
    }
}
