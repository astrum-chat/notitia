use std::marker::PhantomData;

use smallvec::SmallVec;
use unions::{IntoUnion, UnionPath};

use crate::{
    Adapter, Database, Datatype, FieldKindOfDatabase, Mutation, MutationEvent, MutationEventKind,
    Notitia, PartialRecord, Record, StrongFieldFilter, UpdateStmtBuilt,
};

pub struct UpdateStmtUnbuilt<Db: Database, Rec: Record, P: PartialRecord> {
    pub table_name: &'static str,
    pub partial: P,
    _database: PhantomData<Db>,
    _record: PhantomData<Rec>,
}

impl<Db: Database, Rec: Record, P: PartialRecord> UpdateStmtUnbuilt<Db, Rec, P> {
    pub(crate) fn new(table_name: &'static str, partial: P) -> Self {
        Self {
            table_name,
            partial,
            _database: PhantomData,
            _record: PhantomData,
        }
    }

    pub fn filter<FieldPath: UnionPath, Field, T>(
        self,
        filter: StrongFieldFilter<Field, T>,
    ) -> UpdateStmtBuilt<Db, Rec, P>
    where
        Field: FieldKindOfDatabase<Db> + IntoUnion<Rec::FieldKind, FieldPath>,
        T: Into<Datatype> + Clone,
    {
        let mut filters = SmallVec::new();
        filters.push(filter.to_weak());

        UpdateStmtBuilt::new(self.table_name, self.partial, filters)
    }
}

impl<Db, Rec, P> Mutation<Db> for UpdateStmtUnbuilt<Db, Rec, P>
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
                changed: self.partial.clone().into_set_datatypes(),
                filters: SmallVec::new(),
            },
        }
    }

    async fn execute<Adptr: Adapter>(self, db: &Notitia<Db, Adptr>) -> Result<(), Adptr::Error> {
        let built: UpdateStmtBuilt<Db, Rec, P> =
            UpdateStmtBuilt::new(self.table_name, self.partial, SmallVec::new());
        db.execute_update_stmt(built).await
    }
}
