#![forbid(unsafe_code)]
use crate::trie_key::ToKeyIter;
use std::{
    borrow::{Borrow, BorrowMut},
    collections::HashMap,
    hash::Hash,
    ops::Index,
};

struct TrieNode<K: Eq + Hash, V> {
    children: HashMap<K, TrieNode<K, V>>,
    val: Option<V>,
    words_count: usize,
}

impl<K, V> Default for Trie<K, V>
where
    K: ToKeyIter,
    K::Item: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> TrieNode<K, V>
where
    K: Eq + Hash,
{
    fn new() -> Self {
        Self {
            children: HashMap::new(),
            val: None,
            words_count: 0,
        }
    }

    fn get_children_mut(&mut self) -> &mut HashMap<K, TrieNode<K, V>> {
        self.children.borrow_mut()
    }

    fn insert(&mut self, key: K) -> Option<TrieNode<K, V>> {
        self.children.insert(key, TrieNode::new())
    }

    fn contains_key(&self, key: &K) -> bool {
        self.children.contains_key(key)
    }

    fn get_child(&self, key: &K) -> Option<&TrieNode<K, V>> {
        self.children.get(key)
    }

    fn get_child_mut(&mut self, key: &K) -> Option<&mut TrieNode<K, V>> {
        self.children.get_mut(key)
    }

    fn set_val(&mut self, value: V) -> Option<V> {
        self.val.replace(value)
    }

    fn get_val_ref(&self) -> Option<&V> {
        self.val.as_ref()
    }

    fn get_val_mut(&mut self) -> Option<&mut V> {
        self.val.as_mut()
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct Trie<K, V>
where
    K: ToKeyIter,
    K::Item: Eq + Hash,
{
    root: Option<TrieNode<K::Item, V>>,
}
impl<K, V> Trie<K, V>
where
    K: ToKeyIter,
    K::Item: Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            root: Some(TrieNode::new()),
        }
    }

    pub fn len(&self) -> usize {
        self.root.as_ref().unwrap().words_count
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert<Q>(&mut self, key: &Q, value: V) -> Option<V>
    where
        K: Borrow<Q>,
        Q: ToKeyIter<Item = K::Item> + ?Sized,
    {
        let mut diff = 1;
        if self.contains(key) {
            diff = 0;
        }
        let mut cur_node = self.root.as_mut().unwrap();
        for iter in key.key_iter() {
            cur_node.words_count += diff;
            if !cur_node.contains_key(&iter) {
                cur_node.insert(iter.clone());
            }

            cur_node = cur_node.get_child_mut(&iter).unwrap();
        }
        cur_node.words_count += diff;
        cur_node.set_val(value)
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: ToKeyIter<Item = K::Item> + ?Sized,
    {
        let mut cur_node = self.root.as_ref().unwrap();
        for iter in key.key_iter() {
            if !cur_node.contains_key(&iter) {
                return None;
            }
            cur_node = cur_node.get_child(&iter).unwrap();
        }
        cur_node.get_val_ref()
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: ToKeyIter<Item = K::Item> + ?Sized,
    {
        let mut cur_node = self.root.as_mut().unwrap();
        for iter in key.key_iter() {
            if !cur_node.contains_key(&iter) {
                return None;
            }
            cur_node = cur_node.get_child_mut(&iter).unwrap();
        }
        cur_node.get_val_mut()
    }

    pub fn contains<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: ToKeyIter<Item = K::Item> + ?Sized,
    {
        self.get(key).is_some()
    }

    pub fn starts_with<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: ToKeyIter<Item = K::Item> + ?Sized,
    {
        let mut cur_node = self.root.as_ref().unwrap();
        for iter in key.key_iter() {
            if !cur_node.contains_key(&iter) {
                return false;
            }
            cur_node = cur_node.get_child(&iter).unwrap();
        }
        true
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: ToKeyIter<Item = K::Item> + ?Sized,
    {
        if self.contains(key) {
            let (ret_val, new_root) = Self::remove_recur(self.root.take().unwrap(), key.key_iter());
            self.root = Some(new_root);
            ret_val
        } else {
            None
        }
    }

    fn remove_recur<Q>(
        mut node: TrieNode<Q::Item, V>,
        mut key_iter: Q::KeyIter<'_>,
    ) -> (Option<V>, TrieNode<Q::Item, V>)
    where
        K: Borrow<Q>,
        Q: ToKeyIter<Item = K::Item> + ?Sized,
    {
        node.words_count -= 1;
        // let key_to_check = key_iter.next();
        if let Some(key) = key_iter.next() {
            let child = node.get_children_mut().remove(&key).unwrap();
            let (ret_val, child) = Self::remove_recur(child, key_iter);
            if child.words_count != 0 {
                node.get_children_mut().insert(key, child);
            }
            return (ret_val, node);
        }
        (node.val.take(), node)
    }
}

////////////////////////////////////////////////////////////////////////////////

impl<K, V, Q> Index<&Q> for Trie<K, V>
where
    K: ToKeyIter + Borrow<Q>,
    Q: ToKeyIter<Item = K::Item> + ?Sized,
    K::Item: Eq + Hash,
{
    type Output = V;

    fn index(&self, index: &Q) -> &Self::Output {
        self.get(index).unwrap()
    }
}
