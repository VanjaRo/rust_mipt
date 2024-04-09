#![forbid(unsafe_code)]

use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::hash::Hash;

#[derive(Debug)]
pub struct LRUCache<K, V> {
    capacity: usize,
    size: usize,
    key_to_val: HashMap<K, V>,
    key_to_time: HashMap<K, u64>,
    btree: BTreeMap<u64, K>,
    cur_time: u64,
}

impl<K: Clone + Hash + Ord, V> LRUCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            capacity,
            size: 0,
            key_to_val: HashMap::new(),
            key_to_time: HashMap::new(),
            btree: BTreeMap::new(),
            cur_time: 0,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        let val_opt = self.key_to_val.get(key);
        if let Some(val) = val_opt {
            self.cur_time += 1;
            self.key_to_time
                .insert(key.clone(), self.cur_time)
                .and_then(|time| self.btree.remove(&time));
            self.btree.insert(self.cur_time, key.clone());
            return Some(val);
        }
        None
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let ret_val = self.key_to_val.insert(key.clone(), value);
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
