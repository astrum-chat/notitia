use std::marker::PhantomData;

use smallvec::SmallVec;
use unions::{IntoUnion, UnionPath};

use crate::{
    Adapter, Database, FieldFilter, FieldKindOfDatabase, InnerFieldType, Mutation, MutationEvent,
    MutationEventKind, Notitia, PartialRecord, Record, StrongFieldFilter,
};

pub struct UpdateStmtBuilt<Db: Database, Rec: Record, P: PartialRecord> {
    pub table_name: &'static str,
    pub partial: P,
    pub filters: SmallVec<[FieldFilter; 1]>,
    _database: PhantomData<Db>,
    _record: PhantomData<Rec>,
}

impl<Db: Database, Rec: Record, P: PartialRecord> UpdateStmtBuilt<Db, Rec, P> {
    pub(crate) fn new(
        table_name: &'static str,
        partial: P,
        filters: SmallVec<[FieldFilter; 1]>,
    ) -> Self {
        Self {
            table_name,
            partial,
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

impl<Db, Rec, P> Mutation<Db> for UpdateStmtBuilt<Db, Rec, P>
where
    Db: Database,
    Rec: Record + Send,
    P: PartialRecord + Send,
{
    type Output = ();

    fn to_mutation_event(&self) -> MutationEvent {
        MutationEvent {
            table_name: self.table_name,
            kind: MutationEventKind::Update {
                changed: self.partial.clone().into_set_fields(),
                filters: self.filters.clone(),
            },
        }
    }

    async fn execute<Adptr: Adapter>(self, db: &Notitia<Db, Adptr>) -> Result<(), Adptr::Error> {
        db.execute_update_stmt(self).await
    }
}
