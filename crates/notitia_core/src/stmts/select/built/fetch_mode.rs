use std::marker::PhantomData;

use derivative::Derivative;
use unions::IsUnion;

use crate::{
    Adapter, Database, DatatypeConversionError, FieldKindGroup, MutationEvent, MutationEventKind,
    Notitia, SelectStmtBuilt, SubscribableCollection, SubscribableRow, SubscriptionDescriptor,
    merge_event_into_data,
    subscription::merge::{merge_update_single_row, row_from_insert},
};

pub(crate) trait SelectStmtFetchModeSealed {}

#[allow(private_bounds)] // `SelectStmtFetchModeSealed` is an internal helper.
pub trait SelectStmtFetchMode<Ty: Send>: SelectStmtFetchModeSealed + Sized {
    type Output: Send;

    fn from_rows(&self, rows: Vec<Ty>) -> Result<Self::Output, DatatypeConversionError>;

    /// Apply a mutation event to the output data in place.
    /// Returns `true` if the data was changed.
    fn merge_event(
        &self,
        output: &mut Self::Output,
        descriptor: &SubscriptionDescriptor,
        event: &MutationEvent,
    ) -> bool
    where
        Ty: SubscribableRow;

    fn execute<Db, Adptr, FieldUnion, FieldPath, Fields>(
        &self,
        db: &Notitia<Db, Adptr>,
        stmt: &SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, Self>,
    ) -> impl Future<Output = Result<Self::Output, Adptr::Error>> + Send
    where
        Db: Database,
        Adptr: Adapter,
        FieldUnion: IsUnion + Send + Sync,
        FieldPath: Send + Sync,
        Fields: FieldKindGroup<FieldUnion, FieldPath, Type = Ty> + Send + Sync;
}

#[derive(Debug)]
pub struct SelectStmtFetchOne {}

impl<Ty: Send> SelectStmtFetchMode<Ty> for SelectStmtFetchOne {
    type Output = Ty;

    fn from_rows(&self, rows: Vec<Ty>) -> Result<Self::Output, DatatypeConversionError> {
        if rows.len() != 1 {
            return Err(DatatypeConversionError::WrongNumberOfValues {
                expected: 1,
                got: rows.len(),
            });
        }
        Ok(rows.into_iter().next().unwrap())
    }

    fn merge_event(
        &self,
        output: &mut Ty,
        descriptor: &SubscriptionDescriptor,
        event: &MutationEvent,
    ) -> bool
    where
        Ty: SubscribableRow,
    {
        match &event.kind {
            MutationEventKind::Insert { values } => {
                if let Some(row) = row_from_insert::<Ty>(descriptor, values) {
                    if *output != row {
                        *output = row;
                        return true;
                    }
                }
                false
            }
            MutationEventKind::Update {
                changed,
                filters: mutation_filters,
            } => merge_update_single_row(output, descriptor, changed, mutation_filters),
            MutationEventKind::Delete { .. } => {
                // Cannot remove a single-row output; no-op.
                false
            }
        }
    }

    async fn execute<Db, Adptr, FieldUnion, FieldPath, Fields>(
        &self,
        db: &Notitia<Db, Adptr>,
        stmt: &SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, Self>,
    ) -> Result<Ty, Adptr::Error>
    where
        Db: Database,
        Adptr: Adapter,
        FieldUnion: IsUnion + Send + Sync,
        FieldPath: Send + Sync,
        Fields: FieldKindGroup<FieldUnion, FieldPath, Type = Ty> + Send + Sync,
    {
        db.execute_select_stmt(stmt).await
    }
}

impl SelectStmtFetchModeSealed for SelectStmtFetchOne {}

#[derive(Debug)]
pub struct SelectStmtFetchFirst {}

impl<Ty: Send> SelectStmtFetchMode<Ty> for SelectStmtFetchFirst {
    type Output = Ty;

    fn from_rows(&self, rows: Vec<Ty>) -> Result<Self::Output, DatatypeConversionError> {
        rows.into_iter()
            .next()
            .ok_or(DatatypeConversionError::WrongNumberOfValues {
                expected: 1,
                got: 0,
            })
    }

    fn merge_event(
        &self,
        output: &mut Ty,
        descriptor: &SubscriptionDescriptor,
        event: &MutationEvent,
    ) -> bool
    where
        Ty: SubscribableRow,
    {
        match &event.kind {
            MutationEventKind::Insert { values } => {
                if let Some(row) = row_from_insert::<Ty>(descriptor, values) {
                    if *output != row {
                        *output = row;
                        return true;
                    }
                }
                false
            }
            MutationEventKind::Update {
                changed,
                filters: mutation_filters,
            } => merge_update_single_row(output, descriptor, changed, mutation_filters),
            MutationEventKind::Delete { .. } => {
                // Cannot remove a single-row output; no-op.
                false
            }
        }
    }

    async fn execute<Db, Adptr, FieldUnion, FieldPath, Fields>(
        &self,
        db: &Notitia<Db, Adptr>,
        stmt: &SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, Self>,
    ) -> Result<Ty, Adptr::Error>
    where
        Db: Database,
        Adptr: Adapter,
        FieldUnion: IsUnion + Send + Sync,
        FieldPath: Send + Sync,
        Fields: FieldKindGroup<FieldUnion, FieldPath, Type = Ty> + Send + Sync,
    {
        db.execute_select_stmt(stmt).await
    }
}

impl SelectStmtFetchModeSealed for SelectStmtFetchFirst {}

#[allow(private_bounds)] // `FetchCollection` is an internal helper.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct SelectStmtFetchAll<FetchAs: FetchCollection + Send> {
    #[doc(hidden)]
    #[derivative(Debug = "ignore")]
    _fetch_group: PhantomData<FetchAs>,
}

#[allow(private_bounds)] // `FetchCollection` is an internal helper.
impl<FetchAs: FetchCollection + Send> SelectStmtFetchAll<FetchAs> {
    pub(crate) fn new() -> Self {
        Self {
            _fetch_group: PhantomData,
        }
    }
}

impl<T, FetchAs> SelectStmtFetchMode<T> for SelectStmtFetchAll<FetchAs>
where
    T: Send,
    FetchAs: FetchCollection<Item = T, Output = FetchAs>
        + SubscribableCollection<Item = T>
        + Send
        + Sync,
{
    type Output = FetchAs;

    #[allow(private_interfaces)] // `FetchCollection` is an internal helper.
    fn from_rows(&self, rows: Vec<T>) -> Result<Self::Output, DatatypeConversionError> {
        Ok(FetchAs::from_vec(rows))
    }

    fn merge_event(
        &self,
        output: &mut FetchAs,
        descriptor: &SubscriptionDescriptor,
        event: &MutationEvent,
    ) -> bool
    where
        T: SubscribableRow,
    {
        let old = output.clone();
        merge_event_into_data(output, descriptor, event);
        *output != old
    }

    #[allow(private_bounds)] // `FetchCollection` is an internal helper.
    async fn execute<Db, Adptr, FieldUnion, FieldPath, Fields>(
        &self,
        db: &Notitia<Db, Adptr>,
        stmt: &SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, Self>,
    ) -> Result<FetchAs, Adptr::Error>
    where
        Db: Database,
        Adptr: Adapter,
        FieldUnion: IsUnion + Send + Sync,
        FieldPath: Send + Sync,
        Fields: FieldKindGroup<FieldUnion, FieldPath, Type = T> + Send + Sync,
    {
        db.execute_select_stmt(stmt).await
    }
}

impl<FetchAs: FetchCollection + Send> SelectStmtFetchModeSealed for SelectStmtFetchAll<FetchAs> {}

#[allow(private_bounds)] // `FetchCollection` is an internal helper.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct SelectStmtFetchMany<FetchAs: FetchCollection + Send> {
    max: usize,
    #[doc(hidden)]
    #[derivative(Debug = "ignore")]
    _fetch_group: PhantomData<FetchAs>,
}

#[allow(private_bounds)] // `FetchCollection` is an internal helper.
impl<FetchAs: FetchCollection + Send> SelectStmtFetchMany<FetchAs> {
    pub(crate) fn new(max: usize) -> Self {
        Self {
            max,
            _fetch_group: PhantomData,
        }
    }
}

impl<T, FetchAs> SelectStmtFetchMode<T> for SelectStmtFetchMany<FetchAs>
where
    T: Send,
    FetchAs: FetchCollection<Item = T, Output = FetchAs>
        + SubscribableCollection<Item = T>
        + Send
        + Sync,
{
    type Output = FetchAs;

    #[allow(private_interfaces)] // `FetchCollection` is an internal helper.
    fn from_rows(&self, rows: Vec<T>) -> Result<Self::Output, DatatypeConversionError> {
        let truncated: Vec<_> = rows.into_iter().take(self.max).collect();
        Ok(FetchAs::from_vec(truncated))
    }

    fn merge_event(
        &self,
        output: &mut FetchAs,
        descriptor: &SubscriptionDescriptor,
        event: &MutationEvent,
    ) -> bool
    where
        T: SubscribableRow,
    {
        let old = output.clone();
        merge_event_into_data(output, descriptor, event);
        *output != old
    }

    #[allow(private_bounds)] // `FetchCollection` is an internal helper.
    async fn execute<Db, Adptr, FieldUnion, FieldPath, Fields>(
        &self,
        db: &Notitia<Db, Adptr>,
        stmt: &SelectStmtBuilt<Db, FieldUnion, FieldPath, Fields, Self>,
    ) -> Result<FetchAs, Adptr::Error>
    where
        Db: Database,
        Adptr: Adapter,
        FieldUnion: IsUnion + Send + Sync,
        FieldPath: Send + Sync,
        Fields: FieldKindGroup<FieldUnion, FieldPath, Type = T> + Send + Sync,
    {
        db.execute_select_stmt(stmt).await
    }
}

impl<FetchAs: FetchCollection + Send> SelectStmtFetchModeSealed for SelectStmtFetchMany<FetchAs> {}

pub(crate) trait FetchCollection {
    type Item: Send;
    type Output;

    fn from_vec(items: Vec<Self::Item>) -> Self::Output;
}

impl<T: Send> FetchCollection for Vec<T> {
    type Item = T;
    type Output = Vec<T>;

    fn from_vec(items: Vec<T>) -> Vec<T> {
        items
    }
}
