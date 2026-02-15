use std::{fmt::Debug, marker::PhantomData};

use derivative::Derivative;
use smallvec::{SmallVec, smallvec};
use unions::{IsUnion, Union};

use crate::{
    BuiltRecord, Database, DeleteStmtUnbuilt, FieldKindGroup, InsertStmtBuilt, IsTable,
    PartialRecord, Record, SelectStmtJoin, SelectStmtJoinable, SelectStmtSelectable,
    UpdateStmtUnbuilt,
};

pub trait TableKind: Debug {
    fn name(&self) -> &'static str;
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct StrongTableKind<Db, Tbl>
where
    Db: Database,
    Tbl: IsTable<Database = Db>,
{
    pub kind: Db::TableKind,
    #[doc(hidden)]
    #[derivative(Debug = "ignore")]
    _database: PhantomData<Db>,
    #[doc(hidden)]
    #[derivative(Debug = "ignore")]
    _table: PhantomData<Tbl>,
}

impl<Db, Tbl> StrongTableKind<Db, Tbl>
where
    Db: Database,
    Tbl: IsTable<Database = Db>,
{
    pub const fn new(kind: Db::TableKind) -> Self {
        Self {
            kind,
            _database: PhantomData,
            _table: PhantomData,
        }
    }
}

pub trait IsStrongTableKind {
    type Database: Database;
    type Table: IsTable;
}

impl<Db, Tbl> IsStrongTableKind for StrongTableKind<Db, Tbl>
where
    Db: Database,
    Tbl: IsTable<Database = Db>,
{
    type Database = Db;
    type Table = Tbl;
}

impl<Db, Tbl, Rec, FieldUnion, FieldPath, Fields>
    SelectStmtSelectable<Db, FieldUnion, FieldPath, Fields> for &StrongTableKind<Db, Tbl>
where
    Db: Database,
    Rec: Record<FieldKind = FieldUnion>,
    Tbl: IsTable<Record = Rec, Database = Db>,
    FieldUnion: IsUnion,
    Fields: FieldKindGroup<FieldUnion, FieldPath>,
{
}

impl<Db, Tbl, Rec> SelectStmtJoinable<Db, Rec::FieldKind> for &StrongTableKind<Db, Tbl>
where
    Db: Database,
    Rec: Record,
    Tbl: IsTable<Record = Rec, Database = Db>,
{
    fn join<InnerTbl: IsTable<Database = Db>>(
        self,
        table: StrongTableKind<Db, InnerTbl>,
    ) -> SelectStmtJoin<
        Db,
        Union<Rec::FieldKind, <<InnerTbl as IsTable>::Record as Record>::FieldKind>,
    > {
        SelectStmtJoin::new(SmallVec::from_buf([self.kind.name(), table.kind.name()]))
    }

    fn tables(self) -> SmallVec<[&'static str; 2]> {
        smallvec![self.kind.name()]
    }
}

impl<Db, Tbl, Rec> StrongTableKind<Db, Tbl>
where
    Db: Database,
    Rec: Record,
    Tbl: IsTable<Record = Rec, Database = Db>,
{
    pub fn insert<B: BuiltRecord<Record = Rec>>(&self, builder: B) -> InsertStmtBuilt<Db, Rec> {
        InsertStmtBuilt::new(self.kind.name(), builder.finish())
    }

    pub fn update<B: PartialRecord<FieldKind = Rec::FieldKind>>(
        &self,
        builder: B,
    ) -> UpdateStmtUnbuilt<Db, Rec, B> {
        UpdateStmtUnbuilt::new(self.kind.name(), builder)
    }

    pub fn delete(&self) -> DeleteStmtUnbuilt<Db, Rec> {
        DeleteStmtUnbuilt::new(self.kind.name())
    }
}
