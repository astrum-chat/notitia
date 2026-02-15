mod ordered_set;
pub use ordered_set::*;

use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    hash::Hash,
    sync::Arc,
};

/// A dual-map data structure providing O(1) key lookup and sorted iteration.
///
/// Values are owned by `lookup_map`. The `order_map` stores only the lookup
/// key, pointing back to `lookup_map` for the value.
pub struct OrderedMap<K, V, O>
where
    K: Eq + Hash + Clone,
    O: Ord,
{
    pub(crate) lookup_map: HashMap<K, (Arc<O>, V)>,
    pub(crate) order_map: BTreeMap<Arc<O>, K>,
}

impl<K, V, O> fmt::Debug for OrderedMap<K, V, O>
where
    K: Eq + Hash + Clone + fmt::Debug,
    V: fmt::Debug,
    O: Ord + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            writeln!(f, "{{")?;
            for (i, (order, key)) in self.order_map.iter().enumerate() {
                let (_, value) = &self.lookup_map[key];
                write!(f, "    {key:#?} [{order:#?}]: {value:#?}")?;
                if i + 1 < self.order_map.len() {
                    writeln!(f, ",")?;
                } else {
                    writeln!(f)?;
                }
            }
            write!(f, "}}")
        } else {
            f.write_str("{")?;
            for (i, (order, key)) in self.order_map.iter().enumerate() {
                if i > 0 {
                    f.write_str(", ")?;
                }
                let (_, value) = &self.lookup_map[key];
                write!(f, "{key:?} [{order:?}]: {value:?}")?;
            }
            f.write_str("}")
        }
    }
}

impl<K, V, O> Default for OrderedMap<K, V, O>
where
    K: Eq + Hash + Clone,
    O: Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V, O> Clone for OrderedMap<K, V, O>
where
    K: Eq + Hash + Clone,
    V: Clone,
    O: Ord + Clone,
{
    fn clone(&self) -> Self {
        let mut new = Self::new();
        // Rebuild from order_map to preserve Arc sharing within each entry.
        for (order_key, lookup_key) in &self.order_map {
            let (_, value) = &self.lookup_map[lookup_key];
            let new_order = Arc::new((**order_key).clone());
            new.lookup_map
                .insert(lookup_key.clone(), (new_order.clone(), value.clone()));
            new.order_map.insert(new_order, lookup_key.clone());
        }
        new
    }
}

impl<K, V, O> PartialEq for OrderedMap<K, V, O>
where
    K: Eq + Hash + Clone,
    V: PartialEq,
    O: Ord,
{
    fn eq(&self, other: &Self) -> bool {
        if self.lookup_map.len() != other.lookup_map.len() {
            return false;
        }
        // Compare both values and iteration order.
        for ((order_a, key_a), (order_b, key_b)) in self.order_map.iter().zip(other.order_map.iter()) {
            if key_a != key_b || order_a != order_b {
                return false;
            }
        }
        for (key, (_, value)) in &self.lookup_map {
            match other.lookup_map.get(key) {
                Some((_, other_value)) if value == other_value => continue,
                _ => return false,
            }
        }
        true
    }
}

impl<K, V, O> OrderedMap<K, V, O>
where
    K: Eq + Hash + Clone,
    O: Ord,
{
    pub fn new() -> Self {
        Self {
            lookup_map: HashMap::new(),
            order_map: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.lookup_map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lookup_map.is_empty()
    }

    pub fn insert(&mut self, key: K, value: V, order_key: O) {
        // Remove old entry if the key already exists.
        if let Some((existing_order_key, _)) = self.lookup_map.remove(&key) {
            self.order_map.remove(&existing_order_key);
        }

        let order_key = Arc::new(order_key);
        self.lookup_map
            .insert(key.clone(), (order_key.clone(), value));
        self.order_map.insert(order_key, key);
    }

    pub fn update_order_for_key(&mut self, key: &K, new_order_key: O) -> Option<()> {
        let (key, (old_order_key, value)) = self.lookup_map.remove_entry(key)?;
        self.order_map.remove(&old_order_key);

        let new_order_key = Arc::new(new_order_key);
        self.lookup_map
            .insert(key.clone(), (new_order_key.clone(), value));
        self.order_map.insert(new_order_key, key);

        Some(())
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.lookup_map.get(key).map(|(_, value)| value)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.lookup_map.get_mut(key).map(|(_, value)| value)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let (order_key, value) = self.lookup_map.remove(key)?;
        self.order_map.remove(&order_key);
        Some(value)
    }

    /// Iterate over values in sorted order.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.order_map.values().map(|key| &self.lookup_map[key].1)
    }

    /// Iterate mutably over values (arbitrary order — HashMap iteration).
    /// This is fine for merge operations that need to visit all rows.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.lookup_map.values_mut().map(|(_, value)| value)
    }

    /// Iterate mutably over all values (arbitrary order).
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.values_mut()
    }

    /// Retain only entries for which the predicate returns `true`.
    pub fn retain(&mut self, mut f: impl FnMut(&V) -> bool) {
        let keys_to_remove: Vec<K> = self
            .lookup_map
            .iter()
            .filter(|(_, (_, value))| !f(value))
            .map(|(key, _)| key.clone())
            .collect();

        for key in keys_to_remove {
            self.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get() {
        let mut map = OrderedMap::new();
        map.insert("a", 1, 10);
        map.insert("b", 2, 5);

        assert_eq!(map.get(&"a"), Some(&1));
        assert_eq!(map.get(&"b"), Some(&2));
        assert_eq!(map.get(&"c"), None);
    }

    #[test]
    fn get_mut() {
        let mut map = OrderedMap::new();
        map.insert("a", 1, 10);

        *map.get_mut(&"a").unwrap() = 42;
        assert_eq!(map.get(&"a"), Some(&42));
    }

    #[test]
    fn values_sorted_order() {
        let mut map = OrderedMap::new();
        map.insert("c", 3, 30);
        map.insert("a", 1, 10);
        map.insert("b", 2, 20);

        let values: Vec<&i32> = map.values().collect();
        assert_eq!(values, vec![&1, &2, &3]);
    }

    #[test]
    fn remove() {
        let mut map = OrderedMap::new();
        map.insert("a", 1, 10);
        map.insert("b", 2, 20);

        assert_eq!(map.remove(&"a"), Some(1));
        assert_eq!(map.get(&"a"), None);
        assert_eq!(map.len(), 1);

        let values: Vec<&i32> = map.values().collect();
        assert_eq!(values, vec![&2]);
    }

    #[test]
    fn retain() {
        let mut map = OrderedMap::new();
        map.insert("a", 1, 10);
        map.insert("b", 2, 20);
        map.insert("c", 3, 30);

        map.retain(|v| *v > 1);

        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&"a"), None);
        assert_eq!(map.get(&"b"), Some(&2));
        assert_eq!(map.get(&"c"), Some(&3));
    }

    #[test]
    fn update_order_for_key() {
        let mut map = OrderedMap::new();
        map.insert("a", 1, 10);
        map.insert("b", 2, 20);
        map.insert("c", 3, 30);

        // Move "a" to the end.
        map.update_order_for_key(&"a", 100);

        let values: Vec<&i32> = map.values().collect();
        assert_eq!(values, vec![&2, &3, &1]);
    }

    #[test]
    fn insert_overwrites_existing() {
        let mut map = OrderedMap::new();
        map.insert("a", 1, 10);
        map.insert("a", 99, 5);

        assert_eq!(map.get(&"a"), Some(&99));
        assert_eq!(map.len(), 1);

        let values: Vec<&i32> = map.values().collect();
        assert_eq!(values, vec![&99]);
    }

    #[test]
    fn clone_and_partial_eq() {
        let mut map = OrderedMap::new();
        map.insert("a", 1, 10);
        map.insert("b", 2, 20);

        let cloned = map.clone();
        assert_eq!(map, cloned);

        let mut different = map.clone();
        *different.get_mut(&"a").unwrap() = 42;
        assert_ne!(map, different);
    }

    #[test]
    fn empty_map() {
        let map: OrderedMap<String, i32, i32> = OrderedMap::new();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
        assert_eq!(map.values().count(), 0);
    }
}
