use crate::{Adapter, Database, Mutation, Notitia};
use tracing::error;

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
        let result = self.stmt.execute(&self.db).await;
        if let Err(ref err) = result {
            error!("notitia mutation failed: {}", err);
        }
        let output = result?;
        self.db.notify_subscribers(&event);
        Ok(output)
    }
}
