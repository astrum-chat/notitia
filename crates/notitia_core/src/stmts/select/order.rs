use std::marker::PhantomData;

use derivative::Derivative;
use smallvec::SmallVec;
use unions::{IntoUnion, IsUnion, UnionPath};

use crate::{
    Database, FieldFilter, FieldKindGroup, FieldKindOfDatabase, InnerFieldType, OrderedCollection,
    SelectStmtBuilt, SelectStmtFetchAll, SelectStmtFetchFirst, SelectStmtFetchMany,
    SelectStmtFetchMode, SelectStmtFetchOne, StrongFieldKind,
};

#[derive(Clone, Debug, PartialEq)]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Clone, Debug)]
pub struct OrderBy {
    pub field: &'static str,
    pub table: &'static str,
    pub direction: OrderDirection,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct SelectStmtOrder<Db, FieldUnion, FieldPath, Fields>
where
    Db: Database,
    FieldUnion: IsUnion,
    Fields: FieldKindGroup<FieldUnion, FieldPath>,
{
    tables: SmallVec<[&'static str; 2]>,
    fields: Fields,
    filters: SmallVec<[FieldFilter; 1]>,
    order_by: SmallVec<[OrderBy; 1]>,
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

/// Trait for builder states that can add ORDER BY clauses.
pub trait SelectStmtOrderable<Db, FieldUnion, FieldPath, Fields>: Sized
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
    );

    fn order_by<
        InnerFieldPath: UnionPath,
        InnerField: FieldKindOfDatabase<Db> + IntoUnion<FieldUnion, InnerFieldPath>,
        T: InnerFieldType,
    >(
        self,
        field: StrongFieldKind<InnerField, T>,
        direction: OrderDirection,
    ) -> SelectStmtOrder<Db, FieldUnion, FieldPath, Fields> {
        let (tables, fields, filters, mut order_by) = self.tables_fields_filters_and_orders();
        order_by.push(OrderBy {
            field: field.kind.name(),
            table: InnerField::table_name(),
            direction,
        });
        SelectStmtOrder {
            tables,
            fields,
            filters,
            order_by,
            _database: PhantomData,
            _path: PhantomData,
            _union: PhantomData,
        }
    }
}

// SelectStmtOrder can chain more order_by calls.
impl<Db, FieldUnion, FieldPath, Fields> SelectStmtOrderable<Db, FieldUnion, FieldPath, Fields>
    for SelectStmtOrder<Db, FieldUnion, FieldPath, Fields>
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
        (self.tables, self.fields, self.filters, self.order_by)
    }
}

// SelectStmtOrder implements a buildable-like interface for ordered queries.
impl<Db, FieldUnion, FieldPath, Fields> SelectStmtOrder<Db, FieldUnion, FieldPath, Fields>
where
    Db: Database,
    FieldUnion: IsUnion,
    Fields: FieldKindGroup<FieldUnion, FieldPath>,
{
    /// Fetches exactly one row. Errors if zero or more than one row is returned.
    pub fn fetch_one(
        self,
    ) -> SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, SelectStmtFetchOne> {
        SelectStmtBuilt::new_ordered(
            self.tables,
            self.fields,
            self.filters,
            self.order_by,
            SelectStmtFetchOne {},
        )
    }

    /// Fetches the first row found, or `None` if no rows match.
    pub fn fetch_first(
        self,
    ) -> SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, SelectStmtFetchFirst> {
        SelectStmtBuilt::new_ordered(
            self.tables,
            self.fields,
            self.filters,
            self.order_by,
            SelectStmtFetchFirst {},
        )
    }

    /// Fetches all matching rows into an ordered collection.
    pub fn fetch_all<FetchAs: OrderedCollection>(
        self,
    ) -> SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, SelectStmtFetchAll<FetchAs>>
    where
        SelectStmtFetchAll<FetchAs>: SelectStmtFetchMode<Fields::Type>,
    {
        SelectStmtBuilt::new_ordered(
            self.tables,
            self.fields,
            self.filters,
            self.order_by,
            SelectStmtFetchAll::new(),
        )
    }

    /// Fetches up to `max` matching rows into an ordered collection.
    pub fn fetch_many<FetchAs: OrderedCollection>(
        self,
        max: usize,
    ) -> SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, SelectStmtFetchMany<FetchAs>>
    where
        SelectStmtFetchMany<FetchAs>: SelectStmtFetchMode<Fields::Type>,
    {
        SelectStmtBuilt::new_ordered(
            self.tables,
            self.fields,
            self.filters,
            self.order_by,
            SelectStmtFetchMany::new(max),
        )
    }
}
