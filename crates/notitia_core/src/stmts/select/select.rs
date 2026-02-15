use std::marker::PhantomData;

use derivative::Derivative;
use smallvec::{SmallVec, smallvec};
use unions::IsUnion;

use crate::{
    Database, FieldFilter, FieldKindGroup, SelectStmtBuildable, SelectStmtFilterable,
    SelectStmtJoinable,
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
