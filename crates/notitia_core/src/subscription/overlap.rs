use std::cmp::Ordering;

use crate::{Datatype, FieldFilter};

use super::{MutationEvent, MutationEventKind, SubscriptionDescriptor};

/// Check if a mutation event could affect a subscription.
pub fn event_matches_descriptor(event: &MutationEvent, desc: &SubscriptionDescriptor) -> bool {
    // The mutation must affect a table the subscription depends on.
    if !desc.tables.contains(&event.table_name) {
        return false;
    }

    match &event.kind {
        MutationEventKind::Insert { values } => insert_matches_filters(values, &desc.filters),
        MutationEventKind::Update {
            changed,
            filters: mutation_filters,
        } => {
            // The mutation must touch at least one column the subscription selects.
            let touches_selected_column = changed
                .iter()
                .any(|(col, _)| desc.field_names.contains(col));

            if !touches_selected_column {
                // Even if it doesn't touch selected columns, the mutation could affect
                // which rows match the subscription's filters (e.g., updating a filtered column
                // could move a row in or out of the result set). Check if the mutation changes
                // any column that the subscription filters on.
                let touches_filtered_column = changed.iter().any(|(col, _)| {
                    desc.filters
                        .iter()
                        .any(|f| f.table_field_pair().field_name == *col)
                });

                // Also check if the mutation changes an ORDER BY column, which affects
                // the sort position even if it's not a selected column.
                let touches_order_column = changed
                    .iter()
                    .any(|(col, _)| desc.order_by_field_names.contains(col));

                if !touches_filtered_column && !touches_order_column {
                    return false;
                }
            }

            // Check if the mutation's target rows could overlap with the subscription's rows.
            !filters_provably_disjoint(&desc.filters, mutation_filters)
        }
        MutationEventKind::Delete {
            filters: mutation_filters,
        } => {
            // Check if the delete's target rows could overlap with the subscription's rows.
            !filters_provably_disjoint(&desc.filters, mutation_filters)
        }
    }
}

/// Check if an inserted row satisfies all of the subscription's filters.
fn insert_matches_filters(
    values: &[(&'static str, Datatype)],
    sub_filters: &[FieldFilter],
) -> bool {
    for filter in sub_filters {
        let column = filter.table_field_pair().field_name;

        // Find the inserted value for this column.
        let Some(value) = values
            .iter()
            .find_map(|(col, val)| if *col == column { Some(val) } else { None })
        else {
            // Column not present in insert — can't confirm match, be conservative.
            return true;
        };

        if !filter_satisfied_by_value(filter, value) {
            return false;
        }
    }

    true
}

/// Check if a single filter condition is satisfied by a given value.
pub(crate) fn filter_satisfied_by_value(filter: &FieldFilter, value: &Datatype) -> bool {
    match filter {
        FieldFilter::In(m) => m.right.contains(value),
        _ => {
            let expected = &filter.metadata().right;
            match filter {
                FieldFilter::Eq(_) => value == expected,
                FieldFilter::Ne(_) => value != expected,
                FieldFilter::Gt(_) => {
                    matches!(value.partial_cmp(expected), Some(Ordering::Greater))
                }
                FieldFilter::Lt(_) => matches!(value.partial_cmp(expected), Some(Ordering::Less)),
                FieldFilter::Gte(_) => matches!(
                    value.partial_cmp(expected),
                    Some(Ordering::Greater | Ordering::Equal)
                ),
                FieldFilter::Lte(_) => matches!(
                    value.partial_cmp(expected),
                    Some(Ordering::Less | Ordering::Equal)
                ),
                FieldFilter::In(_) => unreachable!(),
            }
        }
    }
}

/// Returns true if the two filter sets are provably disjoint (no row can match both).
/// Conservative: returns false (not disjoint) when uncertain.
fn filters_provably_disjoint(
    sub_filters: &[FieldFilter],
    mutation_filters: &[FieldFilter],
) -> bool {
    // For each pair of filters on the same (table, column), check if they're contradictory.
    for sf in sub_filters {
        let s_pair = sf.table_field_pair();

        for mf in mutation_filters {
            let m_pair = mf.table_field_pair();

            // Only compare filters on the same table and column.
            if s_pair.table_name != m_pair.table_name || s_pair.field_name != m_pair.field_name {
                continue;
            }

            if pair_provably_disjoint(sf, mf) {
                return true;
            }
        }
    }

    false
}

/// Check if two filters on the same column are provably disjoint.
fn pair_provably_disjoint(a: &FieldFilter, b: &FieldFilter) -> bool {
    // In filters need special handling — be conservative.
    if matches!(a, FieldFilter::In(_)) || matches!(b, FieldFilter::In(_)) {
        return false;
    }

    let a_val = &a.metadata().right;
    let b_val = &b.metadata().right;

    match (a, b) {
        // Eq(x) vs Eq(y) where x != y
        (FieldFilter::Eq(_), FieldFilter::Eq(_)) => a_val != b_val,

        // Eq(x) vs Ne(x) — always disjoint
        (FieldFilter::Eq(_), FieldFilter::Ne(_)) | (FieldFilter::Ne(_), FieldFilter::Eq(_)) => {
            a_val == b_val
        }

        // Eq(x) vs Gt(y) — disjoint if x <= y
        (FieldFilter::Eq(_), FieldFilter::Gt(_)) | (FieldFilter::Gt(_), FieldFilter::Eq(_)) => {
            let (eq_val, gt_val) = if matches!(a, FieldFilter::Eq(_)) {
                (a_val, b_val)
            } else {
                (b_val, a_val)
            };
            matches!(
                eq_val.partial_cmp(gt_val),
                Some(Ordering::Less | Ordering::Equal)
            )
        }

        // Eq(x) vs Lt(y) — disjoint if x >= y
        (FieldFilter::Eq(_), FieldFilter::Lt(_)) | (FieldFilter::Lt(_), FieldFilter::Eq(_)) => {
            let (eq_val, lt_val) = if matches!(a, FieldFilter::Eq(_)) {
                (a_val, b_val)
            } else {
                (b_val, a_val)
            };
            matches!(
                eq_val.partial_cmp(lt_val),
                Some(Ordering::Greater | Ordering::Equal)
            )
        }

        // Gt(x) vs Lt(y) — disjoint if x >= y
        (FieldFilter::Gt(_), FieldFilter::Lt(_)) | (FieldFilter::Lt(_), FieldFilter::Gt(_)) => {
            let (gt_val, lt_val) = if matches!(a, FieldFilter::Gt(_)) {
                (a_val, b_val)
            } else {
                (b_val, a_val)
            };
            matches!(
                gt_val.partial_cmp(lt_val),
                Some(Ordering::Greater | Ordering::Equal)
            )
        }

        // For other combinations, be conservative.
        _ => false,
    }
}
