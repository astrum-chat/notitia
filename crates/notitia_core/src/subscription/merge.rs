use crate::{Collection, Datatype, DatatypeConversionError, FieldExpr, FieldFilter, OrderDirection, OrderKey};

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

/// Merge a mutation event into the subscription's local data.
pub fn merge_event_into_data<C: Collection>(
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
fn merge_insert<C: Collection>(
    data: &mut C,
    descriptor: &SubscriptionDescriptor,
    inserted_values: &[(&'static str, Datatype)],
) {
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
        let order_key = order_key_from_values(&descriptor.order_by_field_names, &descriptor.order_by_directions, inserted_values);
        data.push(row, order_key);
    }
}

/// Extract an `OrderKey` from named values using the descriptor's order_by field names and directions.
fn order_key_from_values(
    order_by_field_names: &[&'static str],
    order_by_directions: &[OrderDirection],
    values: &[(&'static str, Datatype)],
) -> OrderKey {
    let vals = order_by_field_names
        .iter()
        .map(|name| {
            values
                .iter()
                .find_map(
                    |(col, val)| {
                        if col == name { Some(val.clone()) } else { None }
                    },
                )
                .unwrap_or(Datatype::Null)
        })
        .collect();
    let reversed = order_by_directions
        .iter()
        .map(|d| matches!(d, OrderDirection::Desc))
        .collect();
    OrderKey::new(vals, reversed)
}

/// For an update: find rows that match the mutation's filters and apply the changes.
/// Uses `FieldExpr::resolve` to evaluate expressions against the current row values.
fn merge_update<C: Collection>(
    data: &mut C,
    descriptor: &SubscriptionDescriptor,
    changed: &[(&'static str, FieldExpr)],
    mutation_filters: &[FieldFilter],
) {
    // Check if any ORDER BY field was changed.
    let order_changed = descriptor
        .order_by_field_names
        .iter()
        .any(|name| changed.iter().any(|(col, _)| col == name));

    // Collect deferred order updates to apply after the mutable iteration.
    let mut deferred_order_updates: Vec<(C::Item, OrderKey)> = Vec::new();

    for row in data.iter_mut() {
        let row_values = row.to_datatypes(&descriptor.field_names);

        if !row_matches_mutation_filters(&row_values, mutation_filters) {
            continue;
        }

        // Apply the changed values using FieldExpr::resolve.
        let updated_values: Vec<Datatype> = descriptor
            .field_names
            .iter()
            .map(|field_name| {
                if let Some((_, expr)) = changed.iter().find(|(col, _)| col == field_name) {
                    return expr.resolve(&row_values);
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

        // Compute order key before consuming updated_values.
        let new_order_key = if order_changed {
            let all_values: Vec<(&'static str, Datatype)> = descriptor
                .field_names
                .iter()
                .zip(updated_values.iter())
                .map(|(name, val)| (*name, val.clone()))
                .chain(
                    changed
                        .iter()
                        .map(|(name, expr)| (*name, expr.resolve(&row_values))),
                )
                .collect();
            Some(order_key_from_values(
                &descriptor.order_by_field_names,
                &descriptor.order_by_directions,
                &all_values,
            ))
        } else {
            None
        };

        if let Ok(updated_row) = C::Item::from_datatypes(&mut updated_values.into_iter()) {
            if let Some(ref order_key) = new_order_key {
                deferred_order_updates.push((updated_row.clone(), order_key.clone()));
            }
            *row = updated_row;
        }
    }

    // Apply deferred order updates.
    for (item, order_key) in deferred_order_updates {
        data.update_order(&item, order_key);
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
    changed: &[(&'static str, FieldExpr)],
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
            if let Some((_, expr)) = changed.iter().find(|(col, _)| col == field_name) {
                return expr.resolve(&row_values);
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
fn merge_delete<C: Collection>(
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
            continue;
        };

        if !super::overlap::filter_satisfied_by_value(filter, value) {
            return false;
        }
    }

    true
}
