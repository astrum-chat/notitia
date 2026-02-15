use std::marker::PhantomData;

use derivative::Derivative;
use smallvec::SmallVec;
use unions::{IsUnion, Union};

use crate::{
    Database, FieldKindGroup, IsTable, Record, SelectStmtSelectable, StrongTableKind, TableKind,
};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct SelectStmtJoin<Db, FieldsUnion>
where
    Db: Database,
    FieldsUnion: IsUnion,
{
    tables: SmallVec<[&'static str; 2]>,
    #[doc(hidden)]
    #[derivative(Debug = "ignore")]
    _database: PhantomData<Db>,
    #[doc(hidden)]
    #[derivative(Debug = "ignore")]
    _union: PhantomData<FieldsUnion>,
}

impl<Db, FieldsUnion> SelectStmtJoin<Db, FieldsUnion>
where
    Db: Database,
    FieldsUnion: IsUnion,
{
    #[allow(unused)]
    pub(crate) fn new(tables: SmallVec<[&'static str; 2]>) -> SelectStmtJoin<Db, FieldsUnion> {
        SelectStmtJoin {
            tables,
            _database: PhantomData,
            _union: PhantomData,
        }
    }
}

impl<Db, FieldsUnion, FieldPath, Fields> SelectStmtSelectable<Db, FieldsUnion, FieldPath, Fields>
    for SelectStmtJoin<Db, FieldsUnion>
where
    Db: Database,
    FieldsUnion: IsUnion,
    Fields: FieldKindGroup<FieldsUnion, FieldPath>,
{
}

impl<Db, FieldsUnion> SelectStmtJoinable<Db, FieldsUnion> for SelectStmtJoin<Db, FieldsUnion>
where
    Db: Database,
    FieldsUnion: IsUnion,
{
    fn join<Tbl: IsTable<Database = Db>>(
        mut self,
        table: StrongTableKind<Db, Tbl>,
    ) -> SelectStmtJoin<Db, Union<FieldsUnion, <<Tbl as IsTable>::Record as Record>::FieldKind>>
    {
        self.tables.push(table.kind.name());
        SelectStmtJoin::new(self.tables)
    }

    fn tables(self) -> SmallVec<[&'static str; 2]> {
        self.tables
    }
}

pub trait SelectStmtJoinable<Db, FieldsUnion>
where
    Db: Database,
    FieldsUnion: IsUnion,
{
    fn join<Tbl: IsTable<Database = Db>>(
        self,
        table: StrongTableKind<Db, Tbl>,
    ) -> SelectStmtJoin<Db, Union<FieldsUnion, <<Tbl as IsTable>::Record as Record>::FieldKind>>;

    fn tables(self) -> SmallVec<[&'static str; 2]>;
}
