//! Typed handles (newtype indices) and `IdMap` — a typed-index vector.

use std::fmt;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

/// Generate a newtype handle around `u32` with `Display`, `From<u32>`, and index traits.
macro_rules! define_handle {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(u32);

        impl $name {
            pub fn raw(self) -> u32 {
                self.0
            }
        }

        impl From<u32> for $name {
            fn from(v: u32) -> Self {
                Self(v)
            }
        }

        impl From<$name> for u32 {
            fn from(v: $name) -> u32 {
                v.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

define_handle!(ComponentId);
define_handle!(NetId);
define_handle!(PadId);
define_handle!(RuleId);
define_handle!(PolygonId);
define_handle!(LayerId);

/// A `Vec<V>` indexed by a typed handle `K`.
///
/// Provides type-safe indexing: you can only index an `IdMap<ComponentId, _>`
/// with a `ComponentId`, never a `NetId`.
pub struct IdMap<K, V> {
    entries: Vec<V>,
    _marker: PhantomData<K>,
}

impl<K, V> IdMap<K, V>
where
    K: From<u32> + Copy,
{
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            entries: Vec::with_capacity(cap),
            _marker: PhantomData,
        }
    }

    /// Push a value and return its handle.
    pub fn push(&mut self, value: V) -> K {
        let idx = self.entries.len() as u32;
        self.entries.push(value);
        K::from(idx)
    }

    pub fn get(&self, key: K) -> Option<&V>
    where
        K: Into<u32>,
    {
        self.entries.get(key.into() as usize)
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V>
    where
        K: Into<u32>,
    {
        self.entries.get_mut(key.into() as usize)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (K, &V)> {
        self.entries
            .iter()
            .enumerate()
            .map(|(i, v)| (K::from(i as u32), v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (K, &mut V)> {
        self.entries
            .iter_mut()
            .enumerate()
            .map(|(i, v)| (K::from(i as u32), v))
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter()
    }
}

impl<K, V> Default for IdMap<K, V>
where
    K: From<u32> + Copy,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Index<K> for IdMap<K, V>
where
    K: Into<u32> + Copy,
{
    type Output = V;

    fn index(&self, key: K) -> &V {
        &self.entries[key.into() as usize]
    }
}

impl<K, V> IndexMut<K> for IdMap<K, V>
where
    K: Into<u32> + Copy,
{
    fn index_mut(&mut self, key: K) -> &mut V {
        &mut self.entries[key.into() as usize]
    }
}

impl<K, V: fmt::Debug> fmt::Debug for IdMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.entries.iter()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idmap_push_and_get() {
        let mut map = IdMap::<ComponentId, String>::new();
        let a = map.push("alpha".into());
        let b = map.push("beta".into());
        assert_eq!(a.raw(), 0);
        assert_eq!(b.raw(), 1);
        assert_eq!(map[a], "alpha");
        assert_eq!(map[b], "beta");
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn idmap_iter() {
        let mut map = IdMap::<NetId, i32>::new();
        map.push(10);
        map.push(20);
        map.push(30);
        let collected: Vec<(u32, &i32)> = map.iter().map(|(k, v)| (k.raw(), v)).collect();
        assert_eq!(collected, vec![(0, &10), (1, &20), (2, &30)]);
    }

    #[test]
    fn idmap_get_out_of_range() {
        let map = IdMap::<PadId, u8>::new();
        assert!(map.get(PadId::from(0)).is_none());
    }
}
