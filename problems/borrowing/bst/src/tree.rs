#![forbid(unsafe_code)]
use std::{borrow::Borrow, cmp::Ordering, fmt::Debug};

use crate::node::Node;

pub struct AVLTreeMap<K, V> {
    root: Option<Box<Node<K, V>>>,
    len: usize,
}

impl<K: Ord + Debug, V> Default for AVLTreeMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Ord + Debug, V> AVLTreeMap<K, V> {
    pub fn new() -> Self {
        Self { root: None, len: 0 }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    fn get_closest<Q>(&self, key: &Q) -> Option<&Box<Node<K, V>>>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let mut curr_node = self.root.as_ref();
        while let Some(node) = curr_node {
            let next_node: Option<&Box<Node<K, V>>>;
            match key.cmp(node.get_key().borrow()) {
                Ordering::Equal => return Some(node),
                Ordering::Less => next_node = node.get_left_ref(),
                Ordering::Greater => next_node = node.get_right_ref(),
            }
            if next_node.is_some() {
                curr_node = next_node;
            } else {
                break;
            }
        }
        curr_node
    }

    fn get_closest_mut<Q>(&mut self, key: &Q) -> Option<&mut Box<Node<K, V>>>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let mut curr_node = self.root.as_mut();
        while let Some(node) = curr_node.take() {
            let next_node: Option<&mut Box<Node<K, V>>>;
            match key.cmp(node.get_key().borrow()) {
                Ordering::Equal => return Some(node),
                Ordering::Less => next_node = node.get_left_mut(),
                Ordering::Greater => next_node = node.get_right_mut(),
            }
            if next_node.is_none() {
                break;
            }
            curr_node = next_node;
        }
        curr_node
    }

    fn get_node<Q>(&self, key: &Q) -> Option<&Box<Node<K, V>>>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.get_closest(key).and_then(|node| {
            if node.get_key().borrow().eq(key) {
                Some(node)
            } else {
                None
            }
        })
    }

    fn get_node_mut<Q>(&mut self, key: &Q) -> Option<&mut Box<Node<K, V>>>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.get_closest_mut(key).and_then(|node| {
            if node.get_key().borrow().eq(key) {
                Some(node)
            } else {
                None
            }
        })
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.get_node(key).and_then(|node| Some(node.get_value()))
    }

    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.get_node(key)
            .and_then(|node| Some((node.get_key(), node.get_value())))
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.get_node(key).is_some()
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.is_empty() {
            self.root = Some(Box::new(Node::new(key, value, 1)));
            self.len = 1;
            return None;
        }
        if let Some(node) = self.get_node_mut(&key) {
            Some(node.set_value(value))
        } else {
            let new_node = Box::new(Node::new(key, value, 1));
            self.root = Node::sift_node_down(self.root.take(), new_node);
            self.len += 1;
            None
        }
    }
    pub fn nth_key_value(&self, k: usize) -> Option<(&K, &V)> {
        if self.is_empty() || k >= Node::get_size(self.root.as_ref()) {
            return None;
        }
        let mut current_idx = k;
        let mut curr_node_opt = self.root.as_ref();
        while let Some(node) = curr_node_opt {
            let left_br_size = Node::get_size(node.get_left_ref());
            if current_idx < left_br_size {
                curr_node_opt = node.get_left_ref();
            } else if current_idx == left_br_size {
                return Some((node.get_key(), node.get_value()));
            } else {
                curr_node_opt = node.get_right_ref();
                // + 1 for root node
                current_idx -= left_br_size + 1
            }
        }

        None
    }
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let ret_pair_opt: Option<(K, V)>;
        (self.root, ret_pair_opt) = Node::remove_node_with_key(self.root.take(), key);
        if let Some((_, ret_val)) = ret_pair_opt {
            self.len -= 1;
            Some(ret_val)
        } else {
            None
        }
    }

    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let ret_pair_opt: Option<(K, V)>;
        (self.root, ret_pair_opt) = Node::remove_node_with_key(self.root.take(), key);
        if ret_pair_opt.is_some() {
            self.len -= 1;
            ret_pair_opt
        } else {
            None
        }
    }
}
