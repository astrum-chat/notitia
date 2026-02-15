mod select;
pub use select::*;

mod insert;
pub use insert::*;

mod update;
pub use update::*;

mod delete;
pub use delete::*;

use crate::{Adapter, Database, MutationEvent, Notitia};

pub trait Mutation<Db: Database> {
    type Output;

    fn to_mutation_event(&self) -> MutationEvent;

    fn execute<Adptr: Adapter>(
        self,
        db: &Notitia<Db, Adptr>,
    ) -> impl Future<Output = Result<Self::Output, Adptr::Error>> + Send;
}
