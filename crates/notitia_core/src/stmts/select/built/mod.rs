mod fetch_mode;
pub use fetch_mode::*;

mod buildable;
pub use buildable::*;

mod query_executor;
pub use query_executor::*;

use std::marker::PhantomData;

use derivative::Derivative;
use smallvec::SmallVec;
use unions::IsUnion;

use crate::{Adapter, Database, FieldFilter, FieldKindGroup, Notitia, OrderBy};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, Mode>
where
    Db: Database,
    FieldUnion: IsUnion,
    Fields: FieldKindGroup<FieldUnion, FieldPath>,
    Mode: SelectStmtFetchMode<Fields::Type>,
{
    pub tables: SmallVec<[&'static str; 2]>,
    pub fields: Fields,
    pub filters: SmallVec<[FieldFilter; 1]>,
    pub order_by: SmallVec<[OrderBy; 1]>,
    pub mode: Mode,
    #[doc(hidden)]
    #[derivative(Debug = "ignore")]
    _database: PhantomData<Db>,
    #[doc(hidden)]
    #[derivative(Debug = "ignore")]
    _path: PhantomData<FieldPath>,
    #[doc(hidden)]
    #[derivative(Debug = "ignore")]
    _union: PhantomData<FieldUnion>,
}

impl<Db, FieldUnion, FieldPath, Fields, Mode>
    SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, Mode>
where
    Db: Database,
    FieldUnion: IsUnion,
    Fields: FieldKindGroup<FieldUnion, FieldPath>,
    Mode: SelectStmtFetchMode<Fields::Type>,
{
    pub(crate) fn new(
        tables: SmallVec<[&'static str; 2]>,
        fields: Fields,
        filters: SmallVec<[FieldFilter; 1]>,
        mode: Mode,
    ) -> Self {
        Self {
            tables,
            fields,
            filters,
            order_by: SmallVec::new(),
            mode,
            _database: PhantomData,
            _path: PhantomData,
            _union: PhantomData,
        }
    }

    pub(crate) fn new_ordered(
        tables: SmallVec<[&'static str; 2]>,
        fields: Fields,
        filters: SmallVec<[FieldFilter; 1]>,
        order_by: SmallVec<[OrderBy; 1]>,
        mode: Mode,
    ) -> Self {
        Self {
            tables,
            fields,
            filters,
            order_by,
            mode,
            _database: PhantomData,
            _path: PhantomData,
            _union: PhantomData,
        }
    }

    pub fn sql(schema_builder: impl sea_query::SchemaBuilder) -> String {
        sea_query::Query::select().to_string(schema_builder)
    }

    pub fn execute_blocking(
        &self,
        _db: &Db,
    ) -> <Mode as SelectStmtFetchMode<Fields::Type>>::Output {
        todo!()
    }
}

impl<Db, FieldUnion, FieldPath, Fields, Mode>
    SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, Mode>
where
    Db: Database,
    FieldUnion: IsUnion + Send + Sync,
    FieldPath: Send + Sync,
    Fields: FieldKindGroup<FieldUnion, FieldPath> + Send + Sync,
    Mode: SelectStmtFetchMode<Fields::Type> + Sync,
{
    pub async fn execute<Adptr: Adapter>(
        &self,
        db: &Notitia<Db, Adptr>,
    ) -> Result<<Mode as SelectStmtFetchMode<Fields::Type>>::Output, Adptr::Error> {
        self.mode.execute(db, &self).await
    }
}
