use crate::{Adapter, Database, Mutation, Notitia};

pub struct MutateExecutor<Db, Adptr, M>
where
    Db: Database,
    Adptr: Adapter,
    M: Mutation<Db>,
{
    pub(crate) db: Notitia<Db, Adptr>,
    pub(crate) stmt: M,
}

impl<Db, Adptr, M> MutateExecutor<Db, Adptr, M>
where
    Db: Database,
    Adptr: Adapter,
    M: Mutation<Db>,
{
    pub async fn execute(self) -> Result<M::Output, Adptr::Error> {
        let event = self.stmt.to_mutation_event();
        let result = self.stmt.execute(&self.db).await?;
        self.db.notify_subscribers(&event);
        Ok(result)
    }
}
