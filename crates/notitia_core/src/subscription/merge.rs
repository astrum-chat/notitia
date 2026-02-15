use crate::{Datatype, DatatypeConversionError, FieldFilter};

use super::{MutationEvent, MutationEventKind, SubscriptionDescriptor};

/// Trait for row types that can be decomposed and recomposed for patch merging.
///
/// Implemented for single values and tuples via the `impl_field_group!` macro
/// in `field_group.rs`.
pub trait SubscribableRow: Clone + PartialEq + Send + Sized + 'static {
    /// Decompose this row into named `(column_name, value)` pairs.
    fn to_datatypes(&self, field_names: &[&'static str]) -> Vec<(&'static str, Datatype)>;

    /// Construct a row from an iterator of `Datatype` values (in field order).
    fn from_datatypes(
        values: &mut impl Iterator<Item = Datatype>,
    ) -> Result<Self, DatatypeConversionError>;
}

/// Trait for collection types that can be used with subscriptions.
///
/// This allows `merge_event_into_data` to work with any collection type
/// (e.g. `Vec<T>`, `SmallVec<[T; N]>`, etc.) rather than being hardcoded to `Vec<T>`.
pub trait SubscribableCollection: Clone + PartialEq + Send + 'static {
    type Item: SubscribableRow;

    fn push(&mut self, item: Self::Item);
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Self::Item>;
    fn retain(&mut self, f: impl FnMut(&Self::Item) -> bool);
}

impl<T: SubscribableRow> SubscribableCollection for Vec<T> {
    type Item = T;

    fn push(&mut self, item: T) {
        Vec::push(self, item);
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        <[T]>::iter_mut(self)
    }

    fn retain(&mut self, f: impl FnMut(&T) -> bool) {
        Vec::retain(self, f);
    }
}

/// Merge a mutation event into the subscription's local data.
pub fn merge_event_into_data<C: SubscribableCollection>(
    data: &mut C,
    descriptor: &SubscriptionDescriptor,
    event: &MutationEvent,
) {
    match &event.kind {
        MutationEventKind::Insert { values } => {
            merge_insert(data, descriptor, values);
        }
        MutationEventKind::Update {
            changed,
            filters: mutation_filters,
        } => {
            merge_update(data, descriptor, changed, mutation_filters);
        }
        MutationEventKind::Delete {
            filters: mutation_filters,
        } => {
            merge_delete(data, descriptor, mutation_filters);
        }
    }
}

/// For an insert: extract the subscription's selected fields from the inserted row,
/// construct a new row, and push it into the data.
fn merge_insert<C: SubscribableCollection>(
    data: &mut C,
    descriptor: &SubscriptionDescriptor,
    inserted_values: &[(&'static str, Datatype)],
) {
    // Build an iterator of Datatype values in the order of the subscription's field_names.
    let ordered_values: Vec<Datatype> = descriptor
        .field_names
        .iter()
        .map(|field_name| {
            inserted_values
                .iter()
                .find_map(|(col, val)| {
                    if col == field_name {
                        Some(val.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or(Datatype::Null)
        })
        .collect();

    if let Ok(row) = C::Item::from_datatypes(&mut ordered_values.into_iter()) {
        data.push(row);
    }
}

/// For an update: find rows that match the mutation's filters and apply the changes.
fn merge_update<C: SubscribableCollection>(
    data: &mut C,
    descriptor: &SubscriptionDescriptor,
    changed: &[(&'static str, Datatype)],
    mutation_filters: &[FieldFilter],
) {
    for row in data.iter_mut() {
        // Check if this row matches the mutation's filters.
        let row_values = row.to_datatypes(&descriptor.field_names);

        if !row_matches_mutation_filters(&row_values, mutation_filters) {
            continue;
        }

        // Apply the changed values: reconstruct the row with updated fields.
        let updated_values: Vec<Datatype> = descriptor
            .field_names
            .iter()
            .map(|field_name| {
                // If this field was changed, use the new value.
                if let Some((_, new_val)) = changed.iter().find(|(col, _)| col == field_name) {
                    return new_val.clone();
                }
                // Otherwise, keep the existing value.
                row_values
                    .iter()
                    .find_map(|(col, val)| {
                        if col == field_name {
                            Some(val.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or(Datatype::Null)
            })
            .collect();

        if let Ok(updated_row) = C::Item::from_datatypes(&mut updated_values.into_iter()) {
            *row = updated_row;
        }
    }
}

/// Construct a row from inserted values, using the subscription's field ordering.
pub(crate) fn row_from_insert<T: SubscribableRow>(
    descriptor: &SubscriptionDescriptor,
    inserted_values: &[(&'static str, Datatype)],
) -> Option<T> {
    let ordered_values: Vec<Datatype> = descriptor
        .field_names
        .iter()
        .map(|field_name| {
            inserted_values
                .iter()
                .find_map(|(col, val)| {
                    if col == field_name {
                        Some(val.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or(Datatype::Null)
        })
        .collect();

    T::from_datatypes(&mut ordered_values.into_iter()).ok()
}

/// Apply changed values to a single row if it matches the mutation's filters.
/// Returns `true` if the row was modified.
pub(crate) fn merge_update_single_row<T: SubscribableRow>(
    row: &mut T,
    descriptor: &SubscriptionDescriptor,
    changed: &[(&'static str, Datatype)],
    mutation_filters: &[FieldFilter],
) -> bool {
    let row_values = row.to_datatypes(&descriptor.field_names);

    if !row_matches_mutation_filters(&row_values, mutation_filters) {
        return false;
    }

    let updated_values: Vec<Datatype> = descriptor
        .field_names
        .iter()
        .map(|field_name| {
            if let Some((_, new_val)) = changed.iter().find(|(col, _)| col == field_name) {
                return new_val.clone();
            }
            row_values
                .iter()
                .find_map(|(col, val)| {
                    if col == field_name {
                        Some(val.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or(Datatype::Null)
        })
        .collect();

    if let Ok(updated_row) = T::from_datatypes(&mut updated_values.into_iter()) {
        if *row != updated_row {
            *row = updated_row;
            return true;
        }
    }

    false
}

/// For a delete: remove rows that match the mutation's filters.
fn merge_delete<C: SubscribableCollection>(
    data: &mut C,
    descriptor: &SubscriptionDescriptor,
    mutation_filters: &[FieldFilter],
) {
    data.retain(|row| {
        let row_values = row.to_datatypes(&descriptor.field_names);
        !row_matches_mutation_filters(&row_values, mutation_filters)
    });
}

/// Check if a row's values satisfy all of the mutation's filters.
pub(crate) fn row_matches_mutation_filters(
    row_values: &[(&'static str, Datatype)],
    mutation_filters: &[FieldFilter],
) -> bool {
    for filter in mutation_filters {
        let meta = filter.metadata();
        let column = meta.left.field_name;

        let Some(value) = row_values
            .iter()
            .find_map(|(col, val)| if *col == column { Some(val) } else { None })
        else {
            // Row doesn't have this column — can't confirm match, be conservative.
            continue;
        };

        if !super::overlap::filter_satisfied_by_value(filter, value) {
            return false;
        }
    }

    true
}
