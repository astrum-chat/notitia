use std::marker::PhantomData;

use derivative::Derivative;
use smallvec::SmallVec;
use unions::{IntoUnion, IsUnion, UnionPath};

use crate::{
    Collection, Database, Embedded, Embedding, FieldFilter, FieldKindGroup, FieldKindOfDatabase,
    InnerFieldType, SelectStmtBuilt, SelectStmtFetchFirst, SelectStmtFetchMany,
    SelectStmtFetchMode, SelectStmtFetchOne, StrongFieldKind,
};

// ---------------------------------------------------------------------------
// SimilaritySearch — parameters stored on SelectStmtBuilt
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct SimilaritySearch {
    pub table_name: &'static str,
    pub field_name: &'static str,
    pub query: Embedding,
    pub topk: usize,
}

// ---------------------------------------------------------------------------
// SelectStmtSearch — builder state after .search()
// ---------------------------------------------------------------------------

#[derive(Derivative)]
#[derivative(Debug)]
pub struct SelectStmtSearch<Db, FieldUnion, FieldPath, Fields>
where
    Db: Database,
    FieldUnion: IsUnion,
    Fields: FieldKindGroup<FieldUnion, FieldPath>,
{
    tables: SmallVec<[&'static str; 2]>,
    fields: Fields,
    filters: SmallVec<[FieldFilter; 1]>,
    search: SimilaritySearch,
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

impl<Db, FieldUnion, FieldPath, Fields> SelectStmtSearch<Db, FieldUnion, FieldPath, Fields>
where
    Db: Database,
    FieldUnion: IsUnion,
    Fields: FieldKindGroup<FieldUnion, FieldPath>,
{
    pub(crate) fn new(
        tables: SmallVec<[&'static str; 2]>,
        fields: Fields,
        filters: SmallVec<[FieldFilter; 1]>,
        search: SimilaritySearch,
    ) -> Self {
        Self {
            tables,
            fields,
            filters,
            search,
            _database: PhantomData,
            _path: PhantomData,
            _union: PhantomData,
        }
    }

    /// Fetches exactly one row. Errors if zero or more than one row is returned.
    pub fn fetch_one(
        self,
    ) -> SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, SelectStmtFetchOne> {
        let mut search = self.search;
        search.topk = 1;
        SelectStmtBuilt::new_searched(
            self.tables,
            self.fields,
            self.filters,
            search,
            SelectStmtFetchOne {},
        )
    }

    /// Fetches the first row found, or `None` if no rows match.
    pub fn fetch_first(
        self,
    ) -> SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, SelectStmtFetchFirst> {
        let mut search = self.search;
        search.topk = 1;
        SelectStmtBuilt::new_searched(
            self.tables,
            self.fields,
            self.filters,
            search,
            SelectStmtFetchFirst {},
        )
    }

    /// Fetches up to `max` matching rows into a collection, ranked by similarity.
    pub fn fetch_many<FetchAs: Collection>(
        self,
        max: usize,
    ) -> SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, SelectStmtFetchMany<FetchAs>>
    where
        SelectStmtFetchMany<FetchAs>: SelectStmtFetchMode<Fields::Type>,
    {
        let mut search = self.search;
        search.topk = max;
        SelectStmtBuilt::new_searched(
            self.tables,
            self.fields,
            self.filters,
            search,
            SelectStmtFetchMany::new(max),
        )
    }
}

// ---------------------------------------------------------------------------
// SelectStmtSearchable — trait for builder states that can call .search()
// ---------------------------------------------------------------------------

pub trait SelectStmtSearchable<Db, FieldUnion, FieldPath, Fields>: Sized
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
    );

    fn search<
        InnerFieldPath: UnionPath,
        InnerField: FieldKindOfDatabase<Db> + IntoUnion<FieldUnion, InnerFieldPath>,
        T: InnerFieldType,
    >(
        self,
        field: StrongFieldKind<InnerField, Embedded<T>>,
        query: impl Into<Embedding>,
    ) -> SelectStmtSearch<Db, FieldUnion, FieldPath, Fields> {
        let (tables, fields, filters) = self.tables_fields_and_filters_for_search();
        SelectStmtSearch::new(
            tables,
            fields,
            filters,
            SimilaritySearch {
                table_name: InnerField::table_name(),
                field_name: field.kind.name(),
                query: query.into(),
                topk: 0, // will be set by fetch_*()
            },
        )
    }
}
