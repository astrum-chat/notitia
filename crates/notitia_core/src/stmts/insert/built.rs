use std::marker::PhantomData;

use crate::{Adapter, Database, Mutation, MutationEvent, MutationEventKind, Notitia, Record};

pub struct InsertStmtBuilt<Db: Database, R: Record> {
    pub table_name: &'static str,
    pub record: R,
    _database: PhantomData<Db>,
}

impl<Db: Database, R: Record> InsertStmtBuilt<Db, R> {
    pub(crate) fn new(table_name: &'static str, record: R) -> Self {
        Self {
            table_name,
            record,
            _database: PhantomData,
        }
    }

    pub async fn execute<Adptr: Adapter>(self, db: &Notitia<Db, Adptr>) -> Result<(), Adptr::Error>
    where
        R: Send,
    {
        db.execute_insert_stmt(self).await
    }
}

impl<Db, R> Mutation<Db> for InsertStmtBuilt<Db, R>
where
    Db: Database,
    R: Record + Send,
{
    type Output = ();

    fn to_mutation_event(&self) -> MutationEvent {
        MutationEvent {
            table_name: self.table_name,
            kind: MutationEventKind::Insert {
                values: self.record.clone().into_datatypes(),
            },
        }
    }

    async fn execute<Adptr: Adapter>(self, db: &Notitia<Db, Adptr>) -> Result<(), Adptr::Error> {
        db.execute_insert_stmt(self).await
    }
}
