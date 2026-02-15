use smallvec::SmallVec;
use unions::IsUnion;

use crate::{Collection, Database, FieldFilter, FieldKindGroup};

use super::{
    SelectStmtBuilt, SelectStmtFetchAll, SelectStmtFetchFirst,
    SelectStmtFetchMany, SelectStmtFetchMode, SelectStmtFetchOne,
};

pub trait SelectStmtBuildable<Db, FieldUnion, FieldPath, Fields>: Sized
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

    /// Fetches exactly one row. Errors if zero or more than one row is returned.
    fn fetch_one(self) -> SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, SelectStmtFetchOne> {
        let (tables, fields, filters) = self.tables_fields_and_filters();
        SelectStmtBuilt::new(tables, fields, filters, SelectStmtFetchOne {})
    }

    /// Fetches the first row found, or `None` if no rows match.
    fn fetch_first(
        self,
    ) -> SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, SelectStmtFetchFirst> {
        let (tables, fields, filters) = self.tables_fields_and_filters();
        SelectStmtBuilt::new(tables, fields, filters, SelectStmtFetchFirst {})
    }

    /// Fetches all matching rows into a collection.
    fn fetch_all<FetchAs: Collection>(
        self,
    ) -> SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, SelectStmtFetchAll<FetchAs>>
    where
        SelectStmtFetchAll<FetchAs>: SelectStmtFetchMode<Fields::Type>,
    {
        let (tables, fields, filters) = self.tables_fields_and_filters();
        SelectStmtBuilt::new(tables, fields, filters, SelectStmtFetchAll::new())
    }

    /// Fetches up to `max` matching rows into a collection.
    fn fetch_many<FetchAs: Collection>(
        self,
        max: usize,
    ) -> SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, SelectStmtFetchMany<FetchAs>>
    where
        SelectStmtFetchMany<FetchAs>: SelectStmtFetchMode<Fields::Type>,
    {
        let (tables, fields, filters) = self.tables_fields_and_filters();
        SelectStmtBuilt::new(tables, fields, filters, SelectStmtFetchMany::new(max))
    }
}
