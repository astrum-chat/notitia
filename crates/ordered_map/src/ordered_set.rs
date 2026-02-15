use std::{fmt, hash::Hash};

use crate::OrderedMap;

/// An ordered set backed by an `OrderedMap<K, K, O>`.
///
/// Provides O(1) membership lookup and sorted iteration.
pub struct OrderedSet<K, O>
where
    K: Eq + Hash + Clone,
    O: Ord,
{
    inner: OrderedMap<K, K, O>,
}

impl<K, O> fmt::Debug for OrderedSet<K, O>
where
    K: Eq + Hash + Clone + fmt::Debug,
    O: Ord + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            writeln!(f, "{{")?;
            for (i, (order, key)) in self.inner.order_map.iter().enumerate() {
                let (_, value) = &self.inner.lookup_map[key];
                write!(f, "    {value:#?} [{order:#?}]")?;
                if i + 1 < self.inner.order_map.len() {
                    writeln!(f, ",")?;
                } else {
                    writeln!(f)?;
                }
            }
            write!(f, "}}")
        } else {
            f.write_str("{")?;
            for (i, (order, key)) in self.inner.order_map.iter().enumerate() {
                if i > 0 {
                    f.write_str(", ")?;
                }
                let (_, value) = &self.inner.lookup_map[key];
                write!(f, "{value:?} [{order:?}]")?;
            }
            f.write_str("}")
        }
    }
}

impl<K, O> Default for OrderedSet<K, O>
where
    K: Eq + Hash + Clone,
    O: Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, O> Clone for OrderedSet<K, O>
where
    K: Eq + Hash + Clone,
    O: Ord + Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<K, O> PartialEq for OrderedSet<K, O>
where
    K: Eq + Hash + Clone,
    O: Ord,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<K, O> OrderedSet<K, O>
where
    K: Eq + Hash + Clone,
    O: Ord,
{
    pub fn new() -> Self {
        Self {
            inner: OrderedMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn insert(&mut self, key: K, order_key: O) {
        let value = key.clone();
        self.inner.insert(key, value, order_key);
    }

    pub fn contains(&self, key: &K) -> bool {
        self.inner.get(key).is_some()
    }

    pub fn remove(&mut self, key: &K) -> Option<K> {
        self.inner.remove(key)
    }

    pub fn update_order_for_key(&mut self, key: &K, new_order_key: O) -> Option<()> {
        self.inner.update_order_for_key(key, new_order_key)
    }

    /// Iterate over values in sorted order.
    pub fn iter(&self) -> impl Iterator<Item = &K> {
        self.inner.values()
    }

    /// Iterate mutably over values (arbitrary order).
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut K> {
        self.inner.values_mut()
    }

    /// Retain only entries for which the predicate returns `true`.
    pub fn retain(&mut self, f: impl FnMut(&K) -> bool) {
        self.inner.retain(f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_contains() {
        let mut set = OrderedSet::new();
        set.insert("a", 10);
        set.insert("b", 5);

        assert!(set.contains(&"a"));
        assert!(set.contains(&"b"));
        assert!(!set.contains(&"c"));
    }

    #[test]
    fn iter_sorted_order() {
        let mut set = OrderedSet::new();
        set.insert("c", 30);
        set.insert("a", 10);
        set.insert("b", 20);

        let values: Vec<&&str> = set.iter().collect();
        assert_eq!(values, vec![&"a", &"b", &"c"]);
    }

    #[test]
    fn remove() {
        let mut set = OrderedSet::new();
        set.insert("a", 10);
        set.insert("b", 20);

        assert_eq!(set.remove(&"a"), Some("a"));
        assert!(!set.contains(&"a"));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn retain() {
        let mut set = OrderedSet::new();
        set.insert("a", 10);
        set.insert("b", 20);
        set.insert("c", 30);

        set.retain(|k| *k != "a");

        assert_eq!(set.len(), 2);
        assert!(!set.contains(&"a"));
        assert!(set.contains(&"b"));
        assert!(set.contains(&"c"));
    }

    #[test]
    fn debug_format() {
        let mut set = OrderedSet::new();
        set.insert("a", 10);
        set.insert("b", 20);

        let debug = format!("{:?}", set);
        assert_eq!(debug, r#"{"a" [10], "b" [20]}"#);
    }
}
