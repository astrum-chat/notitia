use std::marker::PhantomData;

use derivative::Derivative;
use smallvec::SmallVec;
use unions::{IntoUnion, IsUnion, UnionPath};

use crate::{
    Database, Datatype, FieldKind, FieldKindGroup, FieldKindOfDatabase, OrderBy,
    SelectStmtBuildable, SelectStmtOrderable, StrongFieldKind,
};

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct SelectStmtFilter<Db, FieldUnion, FieldPath, Fields>
where
    Db: Database,
    FieldUnion: IsUnion,
    Fields: FieldKindGroup<FieldUnion, FieldPath>,
{
    tables: SmallVec<[&'static str; 2]>,
    fields: Fields,
    filters: SmallVec<[FieldFilter; 1]>,
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

impl<Db, FieldUnion, FieldPath, Fields> SelectStmtFilter<Db, FieldUnion, FieldPath, Fields>
where
    Db: Database,
    FieldUnion: IsUnion,
    Fields: FieldKindGroup<FieldUnion, FieldPath>,
{
    pub(crate) fn new(
        tables: SmallVec<[&'static str; 2]>,
        fields: Fields,
        filters: SmallVec<[FieldFilter; 1]>,
    ) -> Self {
        Self {
            tables,
            fields,
            filters,
            _database: PhantomData,
            _path: PhantomData,
            _union: PhantomData,
        }
    }
}

impl<Db, FieldUnion, FieldPath, Fields> SelectStmtBuildable<Db, FieldUnion, FieldPath, Fields>
    for SelectStmtFilter<Db, FieldUnion, FieldPath, Fields>
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
        (self.tables, self.fields, self.filters)
    }
}

impl<Db, FieldUnion, FieldPath, Fields> SelectStmtFilterable<Db, FieldUnion, FieldPath, Fields>
    for SelectStmtFilter<Db, FieldUnion, FieldPath, Fields>
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
        (self.tables, self.fields, self.filters)
    }
}

impl<Db, FieldUnion, FieldPath, Fields> SelectStmtOrderable<Db, FieldUnion, FieldPath, Fields>
    for SelectStmtFilter<Db, FieldUnion, FieldPath, Fields>
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
        (
            self.tables,
            self.fields,
            self.filters,
            SmallVec::new(),
        )
    }
}

pub trait SelectStmtFilterable<Db, FieldUnion, FieldPath, Fields>: Sized
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
    );

    fn filter<
        InnerFieldPath: UnionPath,
        InnerField: FieldKindOfDatabase<Db> + IntoUnion<FieldUnion, InnerFieldPath>,
        T: Into<Datatype> + Clone,
    >(
        self,
        filter: StrongFieldFilter<InnerField, T>,
    ) -> SelectStmtFilter<Db, FieldUnion, FieldPath, Fields> {
        let (tables, fields, mut filters) = self.tables_fields_and_filters();
        filters.push(filter.to_weak());

        SelectStmtFilter::new(tables, fields, filters)
    }
}

#[derive(Clone, Debug)]
pub enum StrongFieldFilter<F: FieldKind, T: Into<Datatype> + Clone> {
    Eq(StrongFieldKind<F, T>, Datatype),
    Gt(StrongFieldKind<F, T>, Datatype),
    Lt(StrongFieldKind<F, T>, Datatype),
    Gte(StrongFieldKind<F, T>, Datatype),
    Lte(StrongFieldKind<F, T>, Datatype),
    Ne(StrongFieldKind<F, T>, Datatype),
}

impl<F: FieldKind, T: Into<Datatype> + Clone> StrongFieldFilter<F, T> {
    pub(crate) fn to_weak<D: Database>(self) -> FieldFilter
    where
        F: FieldKindOfDatabase<D>,
    {
        match self {
            Self::Eq(strong_field, datatype) => FieldFilter::Eq(FieldFilterMetadata::new(
                TableFieldPair::new(F::table_name(), strong_field.kind.name()),
                datatype,
            )),
            Self::Gt(strong_field, datatype) => FieldFilter::Gt(FieldFilterMetadata::new(
                TableFieldPair::new(F::table_name(), strong_field.kind.name()),
                datatype,
            )),
            Self::Lt(strong_field, datatype) => FieldFilter::Lt(FieldFilterMetadata::new(
                TableFieldPair::new(F::table_name(), strong_field.kind.name()),
                datatype,
            )),
            Self::Gte(strong_field, datatype) => FieldFilter::Gte(FieldFilterMetadata::new(
                TableFieldPair::new(F::table_name(), strong_field.kind.name()),
                datatype,
            )),
            Self::Lte(strong_field, datatype) => FieldFilter::Lte(FieldFilterMetadata::new(
                TableFieldPair::new(F::table_name(), strong_field.kind.name()),
                datatype,
            )),
            Self::Ne(strong_field, datatype) => FieldFilter::Ne(FieldFilterMetadata::new(
                TableFieldPair::new(F::table_name(), strong_field.kind.name()),
                datatype,
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FieldFilter {
    Eq(FieldFilterMetadata),
    Gt(FieldFilterMetadata),
    Lt(FieldFilterMetadata),
    Gte(FieldFilterMetadata),
    Lte(FieldFilterMetadata),
    Ne(FieldFilterMetadata),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldFilterMetadata {
    pub left: TableFieldPair,
    pub right: Datatype,
}

impl FieldFilterMetadata {
    fn new(left: TableFieldPair, right: Datatype) -> Self {
        Self { left, right }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TableFieldPair {
    pub table_name: &'static str,
    pub field_name: &'static str,
}

impl TableFieldPair {
    fn new(table_name: &'static str, field_name: &'static str) -> Self {
        Self {
            table_name,
            field_name,
        }
    }
}

impl FieldFilter {
    pub fn metadata(&self) -> &FieldFilterMetadata {
        match self {
            Self::Eq(m) | Self::Gt(m) | Self::Lt(m) | Self::Gte(m) | Self::Lte(m) | Self::Ne(m) => {
                m
            }
        }
    }
}

pub enum TableFieldOrDatatype {
    TableField(TableFieldPair),
    Datatype(Datatype),
}
