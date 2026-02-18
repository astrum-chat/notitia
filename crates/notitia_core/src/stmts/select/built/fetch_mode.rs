use std::marker::PhantomData;

use derivative::Derivative;
use unions::IsUnion;

use crate::{
    Adapter, Collection, Database, DatatypeConversionError, FieldKindGroup, MutationEvent,
    MutationEventKind, Notitia, OrderKey, SelectStmtBuilt, SubscribableRow, SubscriptionDescriptor,
    merge_event_into_data,
    subscription::merge::{merge_update_single_row, row_from_insert},
};

pub(crate) trait SelectStmtFetchModeSealed {}

#[allow(private_bounds)] // `SelectStmtFetchModeSealed` is an internal helper.
pub trait SelectStmtFetchMode<Ty: Send>: SelectStmtFetchModeSealed + Sized {
    type Output: Send;

    /// Whether the fetch mode needs order keys extracted from query results.
    fn needs_order_keys(&self) -> bool;

    fn from_rows(
        &self,
        rows: Vec<Ty>,
        order_keys: Vec<OrderKey>,
    ) -> Result<Self::Output, DatatypeConversionError>;

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

    fn needs_order_keys(&self) -> bool {
        false
    }

    fn from_rows(
        &self,
        rows: Vec<Ty>,
        _order_keys: Vec<OrderKey>,
    ) -> Result<Self::Output, DatatypeConversionError> {
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

    fn needs_order_keys(&self) -> bool {
        false
    }

    fn from_rows(
        &self,
        rows: Vec<Ty>,
        _order_keys: Vec<OrderKey>,
    ) -> Result<Self::Output, DatatypeConversionError> {
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

#[derive(Derivative)]
#[derivative(Debug)]
pub struct SelectStmtFetchAll<FetchAs: Collection> {
    #[doc(hidden)]
    #[derivative(Debug = "ignore")]
    _fetch_group: PhantomData<FetchAs>,
}

impl<FetchAs: Collection> SelectStmtFetchAll<FetchAs> {
    pub(crate) fn new() -> Self {
        Self {
            _fetch_group: PhantomData,
        }
    }
}

impl<T, FetchAs> SelectStmtFetchMode<T> for SelectStmtFetchAll<FetchAs>
where
    T: Send,
    FetchAs: Collection<Item = T> + Send + Sync,
{
    type Output = FetchAs;

    fn needs_order_keys(&self) -> bool {
        true
    }

    fn from_rows(
        &self,
        rows: Vec<T>,
        order_keys: Vec<OrderKey>,
    ) -> Result<Self::Output, DatatypeConversionError> {
        Ok(FetchAs::from_vec(rows, order_keys))
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

impl<FetchAs: Collection> SelectStmtFetchModeSealed for SelectStmtFetchAll<FetchAs> {}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct SelectStmtFetchMany<FetchAs: Collection> {
    max: usize,
    #[doc(hidden)]
    #[derivative(Debug = "ignore")]
    _fetch_group: PhantomData<FetchAs>,
}

impl<FetchAs: Collection> SelectStmtFetchMany<FetchAs> {
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
    FetchAs: Collection<Item = T> + Send + Sync,
{
    type Output = FetchAs;

    fn needs_order_keys(&self) -> bool {
        true
    }

    fn from_rows(
        &self,
        rows: Vec<T>,
        order_keys: Vec<OrderKey>,
    ) -> Result<Self::Output, DatatypeConversionError> {
        let truncated_keys: Vec<_> = order_keys.into_iter().take(self.max).collect();
        let truncated: Vec<_> = rows.into_iter().take(self.max).collect();
        Ok(FetchAs::from_vec(truncated, truncated_keys))
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

impl<FetchAs: Collection> SelectStmtFetchModeSealed for SelectStmtFetchMany<FetchAs> {}
