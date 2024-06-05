#![forbid(unsafe_code)]

use std::{borrow::Borrow, iter::FromIterator, ops::Index, vec};
////////////////////////////////////////////////////////////////////////////////

#[derive(Default, Debug, PartialEq, Eq)]
pub struct FlatMap<K, V>(Vec<(K, V)>);

impl<K: Ord, V> FlatMap<K, V> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    pub fn as_slice(&self) -> &[(K, V)] {
        self.0.as_slice()
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let vec = &mut self.0;
        let srch_res = vec.binary_search_by(|(k, _v)| k.cmp(&key));
        match srch_res {
            Ok(idx) => Some(std::mem::replace(&mut vec[idx].1, value)),
            Err(idx) => {
                if idx > vec.len() {
                    vec.push((key, value));
                } else {
                    vec.insert(idx, (key, value))
                }
                None
            }
        }
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let vec = &self.0;
        vec.binary_search_by(|(k, _v)| k.borrow().cmp(key))
            .ok()
            .and_then(|idx| vec.get(idx).map(|(_k, v)| v))
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.remove_entry(key).map(|(_, v)| v)
    }

    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let vec = &mut self.0;
        vec.binary_search_by(|(k, _v)| k.borrow().cmp(key))
            .ok()
            .map(|idx| vec.remove(idx))
    }
}

////////////////////////////////////////////////////////////////////////////////

impl<K, Q, V> Index<&Q> for FlatMap<K, V>
where
    K: Ord + Borrow<Q>,
    Q: Ord + ?Sized,
{
    type Output = V;
    fn index(&self, index: &Q) -> &V {
        self.get(index).unwrap()
    }
}

impl<K: Ord, V> Extend<(K, V)> for FlatMap<K, V> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<K: Ord, V> From<Vec<(K, V)>> for FlatMap<K, V> {
    fn from(value: Vec<(K, V)>) -> Self {
        let mut val_mp = value;
        // to remove same elements up-to the last insertion
        val_mp.reverse();
        val_mp.dedup_by(|a, b| a.0 == b.0);
        val_mp.sort_by(|a, b| a.0.cmp(&b.0));
        Self(val_mp)
    }
}

impl<K: Ord, V> From<FlatMap<K, V>> for Vec<(K, V)> {
    fn from(value: FlatMap<K, V>) -> Self {
        value.0
    }
}

impl<K: Ord, V> FromIterator<(K, V)> for FlatMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut fl_map = FlatMap::new();
        fl_map.extend(iter);
        fl_map
    }
}

impl<K: Ord, V> IntoIterator for FlatMap<K, V> {
    type Item = (K, V);

    type IntoIter = vec::IntoIter<(K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
