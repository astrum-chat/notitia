use std::marker::PhantomData;

use derivative::Derivative;
use smallvec::{SmallVec, smallvec};
use unions::IsUnion;

use crate::{
    Database, FieldFilter, FieldKindGroup, OrderBy, SelectStmtBuildable, SelectStmtFilterable,
    SelectStmtJoinable, SelectStmtOrderable,
};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct SelectStmtSelect<Db, FieldUnion, FieldPath, Fields>
where
    Db: Database,
    FieldUnion: IsUnion,
    Fields: FieldKindGroup<FieldUnion, FieldPath>,
{
    tables: SmallVec<[&'static str; 2]>,
    fields: Fields,
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

impl<Db, FieldUnion, FieldPath, Fields> SelectStmtSelect<Db, FieldUnion, FieldPath, Fields>
where
    Db: Database,
    FieldUnion: IsUnion,
    Fields: FieldKindGroup<FieldUnion, FieldPath>,
{
    pub(crate) fn new(tables: SmallVec<[&'static str; 2]>, fields: Fields) -> Self {
        Self {
            tables,
            fields,
            _database: PhantomData,
            _path: PhantomData,
            _union: PhantomData,
        }
    }
}

impl<Db, FieldUnion, FieldPath, Fields> SelectStmtFilterable<Db, FieldUnion, FieldPath, Fields>
    for SelectStmtSelect<Db, FieldUnion, FieldPath, Fields>
where
    Db: Database,
    FieldUnion: IsUnion,
    Fields: FieldKindGroup<FieldUnion, FieldPath>,
{
    fn tables_fields_and_filters(
        self,
    ) -> (
        SmallVec<[&'static str; 2]>,
        Fields,
        SmallVec<[FieldFilter; 1]>,
    ) {
        (self.tables, self.fields, smallvec![])
    }
}

impl<Db, FieldUnion, FieldPath, Fields> SelectStmtBuildable<Db, FieldUnion, FieldPath, Fields>
    for SelectStmtSelect<Db, FieldUnion, FieldPath, Fields>
where
    Db: Database,
    FieldUnion: IsUnion,
    Fields: FieldKindGroup<FieldUnion, FieldPath>,
{
    fn tables_fields_and_filters(
        self,
    ) -> (
        SmallVec<[&'static str; 2]>,
        Fields,
        SmallVec<[FieldFilter; 1]>,
    ) {
        (self.tables, self.fields, smallvec![])
    }
}

impl<Db, FieldUnion, FieldPath, Fields> SelectStmtOrderable<Db, FieldUnion, FieldPath, Fields>
    for SelectStmtSelect<Db, FieldUnion, FieldPath, Fields>
where
    Db: Database,
    FieldUnion: IsUnion,
    Fields: FieldKindGroup<FieldUnion, FieldPath>,
{
    fn tables_fields_filters_and_orders(
        self,
    ) -> (
        SmallVec<[&'static str; 2]>,
        Fields,
        SmallVec<[FieldFilter; 1]>,
        SmallVec<[OrderBy; 1]>,
    ) {
        (self.tables, self.fields, smallvec![], smallvec![])
    }
}

#[cfg(feature = "embeddings")]
impl<Db, FieldUnion, FieldPath, Fields>
    crate::SelectStmtSearchable<Db, FieldUnion, FieldPath, Fields>
    for SelectStmtSelect<Db, FieldUnion, FieldPath, Fields>
where
    Db: Database,
    FieldUnion: IsUnion,
    Fields: FieldKindGroup<FieldUnion, FieldPath>,
{
    fn tables_fields_and_filters_for_search(
        self,
    ) -> (
        SmallVec<[&'static str; 2]>,
        Fields,
        SmallVec<[FieldFilter; 1]>,
    ) {
        (self.tables, self.fields, smallvec![])
    }
}

pub trait SelectStmtSelectable<Db, FieldUnion, FieldPath, Fields>:
    SelectStmtJoinable<Db, FieldUnion> + Sized
where
    Db: Database,
    FieldUnion: IsUnion,
    Fields: FieldKindGroup<FieldUnion, FieldPath>,
{
    fn select(self, fields: Fields) -> SelectStmtSelect<Db, FieldUnion, FieldPath, Fields> {
        SelectStmtSelect::new(self.tables(), fields)
    }
}
