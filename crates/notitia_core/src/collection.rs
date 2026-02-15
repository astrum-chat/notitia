use std::{collections::BTreeMap, hash::Hash};

use crate::{Datatype, DatatypeConversionError, OrderKey, subscription::merge::SubscribableRow};

/// Base collection trait for query results.
pub trait Collection: Clone + PartialEq + Send + 'static {
    type Item: SubscribableRow;

    /// Construct from a `Vec` of items and their corresponding order keys.
    /// For unordered collections (e.g. `Vec`), `order_keys` is ignored.
    fn from_vec(items: Vec<Self::Item>, order_keys: Vec<OrderKey>) -> Self;

    /// Add an item to the collection with its order key.
    /// For unordered collections, this appends to the end and ignores the key.
    /// For ordered collections, this inserts in sorted position.
    fn push(&mut self, item: Self::Item, order_key: OrderKey);

    /// Iterate mutably over all items.
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Self::Item>;

    /// Retain only items for which the predicate returns `true`.
    fn retain(&mut self, f: impl FnMut(&Self::Item) -> bool);

    /// Update the order key for a given item. No-op for unordered collections.
    fn update_order(&mut self, _item: &Self::Item, _order_key: OrderKey) {}
}

/// Marker trait for ordered collections.
///
/// `push` must insert in sorted position (not append).
/// Required by queries that have ORDER BY clauses.
pub trait OrderedCollection: Collection {}

/// Trait for row types that have a unique key for deduplication.
pub trait KeyedRow {
    type Key: Eq + Hash + Clone + Send;
    fn key(&self) -> Self::Key;
}

// --- Blanket KeyedRow for single-value types ---

impl<T> KeyedRow for T
where
    T: Clone
        + Eq
        + Hash
        + PartialEq
        + Into<Datatype>
        + TryFrom<Datatype, Error = DatatypeConversionError>
        + Send
        + 'static,
{
    type Key = Self;
    fn key(&self) -> Self::Key {
        self.clone()
    }
}

// --- Tuple KeyedRow impls ---

macro_rules! impl_keyed_row_tuple {
    (@impl $($idx:tt: $T:ident),+) => {
        impl<$($T),+> KeyedRow for ($($T,)+)
        where
            $($T: Clone + Eq + Hash + PartialEq + Into<Datatype> + TryFrom<Datatype, Error = DatatypeConversionError> + Send + 'static,)+
        {
            type Key = Self;
            fn key(&self) -> Self::Key {
                self.clone()
            }
        }
    };

    (@build [$($acc:tt)*] $idx:tt: $T:ident) => {
        impl_keyed_row_tuple!(@impl $($acc)* $idx: $T);
    };

    (@build [$($acc:tt)*] $idx:tt: $T:ident, $($rest:tt)+) => {
        impl_keyed_row_tuple!(@impl $($acc)* $idx: $T);
        impl_keyed_row_tuple!(@build [$($acc)* $idx: $T,] $($rest)+);
    };

    ($($idx:tt: $T:ident),+ $(,)?) => {
        impl_keyed_row_tuple!(@build [] $($idx: $T),+);
    };
}

// Tier 1: 4 fields (extra_small_fields)
#[cfg(feature = "extra_small_fields")]
impl_keyed_row_tuple!(
    0: T0,
    1: T1,
    2: T2,
    3: T3,
);

// Tier 2: 12 fields (small_fields)
#[cfg(feature = "small_fields")]
impl_keyed_row_tuple!(
    0: T0, 1: T1, 2: T2, 3: T3,
    4: T4, 5: T5, 6: T6, 7: T7,
    8: T8, 9: T9, 10: T10, 11: T11,
);

// Tier 3: 22 fields (medium_fields)
#[cfg(feature = "medium_fields")]
impl_keyed_row_tuple!(
    0: T0, 1: T1, 2: T2, 3: T3,
    4: T4, 5: T5, 6: T6, 7: T7,
    8: T8, 9: T9, 10: T10, 11: T11,
    12: T12, 13: T13, 14: T14, 15: T15,
    16: T16, 17: T17, 18: T18, 19: T19,
    20: T20, 21: T21,
);

// Tier 4: 42 fields (large_fields)
#[cfg(feature = "large_fields")]
impl_keyed_row_tuple!(
    0: T0, 1: T1, 2: T2, 3: T3,
    4: T4, 5: T5, 6: T6, 7: T7,
    8: T8, 9: T9, 10: T10, 11: T11,
    12: T12, 13: T13, 14: T14, 15: T15,
    16: T16, 17: T17, 18: T18, 19: T19,
    20: T20, 21: T21, 22: T22, 23: T23,
    24: T24, 25: T25, 26: T26, 27: T27,
    28: T28, 29: T29, 30: T30, 31: T31,
    32: T32, 33: T33, 34: T34, 35: T35,
    36: T36, 37: T37, 38: T38, 39: T39,
    40: T40, 41: T41,
);

// Tier 5: 64 fields (extra_large_fields)
#[cfg(feature = "extra_large_fields")]
impl_keyed_row_tuple!(
    0: T0, 1: T1, 2: T2, 3: T3,
    4: T4, 5: T5, 6: T6, 7: T7,
    8: T8, 9: T9, 10: T10, 11: T11,
    12: T12, 13: T13, 14: T14, 15: T15,
    16: T16, 17: T17, 18: T18, 19: T19,
    20: T20, 21: T21, 22: T22, 23: T23,
    24: T24, 25: T25, 26: T26, 27: T27,
    28: T28, 29: T29, 30: T30, 31: T31,
    32: T32, 33: T33, 34: T34, 35: T35,
    36: T36, 37: T37, 38: T38, 39: T39,
    40: T40, 41: T41, 42: T42, 43: T43,
    44: T44, 45: T45, 46: T46, 47: T47,
    48: T48, 49: T49, 50: T50, 51: T51,
    52: T52, 53: T53, 54: T54, 55: T55,
    56: T56, 57: T57, 58: T58, 59: T59,
    60: T60, 61: T61, 62: T62, 63: T63,
);

// --- Vec implementation ---

impl<T: SubscribableRow> Collection for Vec<T> {
    type Item = T;

    fn from_vec(items: Vec<T>, _order_keys: Vec<OrderKey>) -> Self {
        items
    }

    fn push(&mut self, item: T, _order_key: OrderKey) {
        Vec::push(self, item);
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.as_mut_slice().iter_mut()
    }

    fn retain(&mut self, f: impl FnMut(&T) -> bool) {
        Vec::retain(self, f);
    }
}

// --- BTreeMap implementation ---

impl<T> Collection for BTreeMap<OrderKey, T>
where
    T: SubscribableRow,
{
    type Item = T;

    fn from_vec(items: Vec<T>, order_keys: Vec<OrderKey>) -> Self {
        items
            .into_iter()
            .zip(order_keys)
            .map(|(item, key)| (key, item))
            .collect()
    }

    fn push(&mut self, item: T, order_key: OrderKey) {
        self.insert(order_key, item);
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.values_mut()
    }

    fn retain(&mut self, mut f: impl FnMut(&T) -> bool) {
        self.retain(|_, v| f(v));
    }

    fn update_order(&mut self, item: &T, order_key: OrderKey) {
        // Find and remove the old entry, re-insert with new key.
        let old_key = self.iter().find_map(|(k, v)| {
            if v == item {
                Some(k.clone())
            } else {
                None
            }
        });
        if let Some(old_key) = old_key {
            if let Some(val) = self.remove(&old_key) {
                self.insert(order_key, val);
            }
        }
    }
}

impl<T> OrderedCollection for BTreeMap<OrderKey, T> where T: SubscribableRow {}
