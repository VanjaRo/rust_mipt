#![forbid(unsafe_code)]

use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;

#[derive(Debug)]
pub struct LRUCache<K, V> {
    capacity: usize,
    size: usize,
    key_to_val: HashMap<u64, V>,
    key_to_time: HashMap<u64, u64>,
    btree: BTreeMap<u64, u64>,
    cur_time: u64,
    _tmp: PhantomData<K>,
}

impl<K: Hash + Ord, V> LRUCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            capacity,
            size: 0,
            key_to_val: HashMap::new(),
            key_to_time: HashMap::new(),
            btree: BTreeMap::new(),
            cur_time: 0,
            _tmp: PhantomData,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hashed_key = hasher.finish();
        let val_opt = self.key_to_val.get(&hashed_key);
        if let Some(val) = val_opt {
            self.cur_time += 1;

            self.key_to_time
                .insert(hashed_key, self.cur_time)
                .and_then(|time| self.btree.remove(&time));
            self.btree.insert(self.cur_time, hashed_key);
            return Some(val);
        }
        None
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hashed_key = hasher.finish();
        let ret_val = self.key_to_val.insert(hashed_key, value);
        self.get(&key);
        self.size += match &ret_val {
            Some(_) => 0,
            _ => 1,
        };
        if self.size > self.capacity {
            // remove least recently used key
            let (_, key_to_delete) = self.btree.pop_first().unwrap();
            self.key_to_time.remove(&key_to_delete);
            self.key_to_val.remove(&key_to_delete);
            self.size -= 1;
        }
        ret_val
    }
}
