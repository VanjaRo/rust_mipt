#![forbid(unsafe_code)]

use std::{borrow::Borrow, cmp::Ordering, fmt::Debug, mem::swap};

pub struct Node<K, V> {
    key: K,
    value: V,
    height: usize,
    sub_tree_size: usize,

    left: Option<Box<Node<K, V>>>,
    right: Option<Box<Node<K, V>>>,
}

impl<K: Ord + Debug, V> PartialEq for Node<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.get_key() == other.get_key()
    }
}

impl<K: Ord + Debug, V> Eq for Node<K, V> {}

impl<K: Ord + Debug, V> PartialOrd for Node<K, V> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.get_key().partial_cmp(other.get_key())
    }
}

impl<K: Ord + Debug, V> Ord for Node<K, V> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.get_key().cmp(other.get_key())
    }
}

impl<K: Ord + Debug, V> Node<K, V> {
    pub fn new(key: K, value: V, height: usize) -> Self {
        Self {
            key,
            value,
            height,
            left: None,
            right: None,
            sub_tree_size: 0,
        }
    }

    pub fn get_height(node_opt: Option<&Box<Node<K, V>>>) -> usize {
        node_opt.map_or(0, |node| node.height)
    }

    pub fn get_size(node_opt: Option<&Box<Node<K, V>>>) -> usize {
        node_opt.map_or(0, |node| node.sub_tree_size + 1)
    }

    fn get_disbalance(node_opt: Option<&Box<Node<K, V>>>) -> i32 {
        node_opt.map_or(0, |node| {
            let left_height = Node::get_height(node.left.as_ref());
            let right_height = Node::get_height(node.right.as_ref());
            if let Some(sub_res) = right_height.checked_sub(left_height) {
                return sub_res as i32;
            } else {
                return -((left_height - right_height) as i32);
            }
        })
    }

    pub fn get_left_ref(&self) -> Option<&Box<Node<K, V>>> {
        self.left.as_ref()
    }

    pub fn get_left_mut(&mut self) -> Option<&mut Box<Node<K, V>>> {
        self.left.as_mut()
    }

    pub fn get_right_ref(&self) -> Option<&Box<Node<K, V>>> {
        self.right.as_ref()
    }

    pub fn get_right_mut(&mut self) -> Option<&mut Box<Node<K, V>>> {
        self.right.as_mut()
    }

    pub fn get_key(&self) -> &K {
        &self.key
    }

    pub fn get_value(&self) -> &V {
        &self.value
    }

    pub fn set_value(&mut self, mut value: V) -> V {
        swap(&mut self.value, &mut value);
        value
    }

    pub fn take_left(&mut self) -> Option<Box<Node<K, V>>> {
        self.left.take()
    }

    pub fn take_right(&mut self) -> Option<Box<Node<K, V>>> {
        self.right.take()
    }

    pub fn set_left(&mut self, new_left: Option<Box<Node<K, V>>>) {
        self.left = new_left
    }

    pub fn set_right(&mut self, new_right: Option<Box<Node<K, V>>>) {
        self.right = new_right
    }

    pub fn sift_node_down(
        parent_node_opt: Option<Box<Node<K, V>>>,
        new_node: Box<Node<K, V>>,
    ) -> Option<Box<Node<K, V>>> {
        if let Some(mut parent_node) = parent_node_opt {
            if new_node <= parent_node {
                parent_node.left = Node::sift_node_down(parent_node.left, new_node);
            } else {
                parent_node.right = Node::sift_node_down(parent_node.right, new_node);
            }

            Node::update_size_height(&mut parent_node);
            let balanced = Node::balance(parent_node);

            Some(balanced)
        } else {
            return Some(new_node);
        }
    }

    pub fn remove_node_with_key<Q>(
        mut parent_node_opt: Option<Box<Node<K, V>>>,
        key: &Q,
    ) -> (Option<Box<Node<K, V>>>, Option<(K, V)>)
    where
        K: Borrow<Q> + Debug,
        Q: Ord + ?Sized,
    {
        if let Some(mut node) = parent_node_opt.take() {
            let tpl_res: Option<(K, V)>;
            match key.cmp(node.get_key().borrow()) {
                Ordering::Less => (node.left, tpl_res) = Node::remove_node_with_key(node.left, key),
                Ordering::Greater => {
                    (node.right, tpl_res) = Node::remove_node_with_key(node.right, key)
                }
                Ordering::Equal => {
                    let (node_opt, ret_opt) = Node::remove_node(node);
                    if let Some(mut node_new) = node_opt {
                        Node::update_size_height(&mut node_new);
                        node_new = Node::balance(node_new);

                        return (Some(node_new), ret_opt);
                    } else {
                        return (None, ret_opt);
                    }
                }
            };
            Node::update_size_height(&mut node);
            node = Node::balance(node);

            return (Some(node), tpl_res);
        }

        (None, None)
    }

    fn remove_node(mut node: Box<Node<K, V>>) -> (Option<Box<Node<K, V>>>, Option<(K, V)>) {
        if node.left.is_none() && node.right.is_none() {
            return (None, Some((node.key, node.value)));
        } else if node.right.is_none() {
            return (node.left, Some((node.key, node.value)));
        } else if node.left.is_none() {
            return (node.right, Some((node.key, node.value)));
        } else {
            let right = node.right.take();
            if let (new_right, Some((mut new_k, mut new_v))) = Node::remove_leftmost(right) {
                node.right = new_right;
                swap(&mut node.key, &mut new_k);
                swap(&mut node.value, &mut new_v);

                Node::update_size_height(&mut node);
                node = Node::balance(node);

                return (Some(node), Some((new_k, new_v)));
            } else {
                return (None, None);
            }
        }
    }

    fn remove_leftmost(
        node_opt: Option<Box<Node<K, V>>>,
    ) -> (Option<Box<Node<K, V>>>, Option<(K, V)>) {
        if let Some(mut node) = node_opt {
            if node.left.is_none() {
                return Node::remove_node(node);
            }
            let ret_res: Option<(K, V)>;
            (node.left, ret_res) = Node::remove_leftmost(node.left.take());
            Node::update_size_height(&mut node);
            node = Node::balance(node);

            return (Some(node), ret_res);
        }
        (None, None)
    }

    fn balance(mut parent_node: Box<Node<K, V>>) -> Box<Node<K, V>> {
        let diff: i32 = Node::get_disbalance(Some(&parent_node));
        match diff {
            2 => {
                if Node::get_disbalance(parent_node.right.as_ref()) < 0 {
                    parent_node.right = Some(Node::r_rotation(parent_node.right.take().unwrap()));
                }
                Node::l_rotation(parent_node)
            }
            -2 => {
                if Node::get_disbalance(parent_node.left.as_ref()) > 0 {
                    parent_node.left = Some(Node::l_rotation(parent_node.left.take().unwrap()));
                }
                Node::r_rotation(parent_node)
            }
            _ => parent_node,
        }
    }

    // while making any rotations ensure that height and size is consistent
    fn r_rotation(mut root: Box<Node<K, V>>) -> Box<Node<K, V>> {
        // Old tree:    Tree after rotation:
        //      C           L
        //    /  \         / \
        //   L   R        LL C
        //  / \             / \
        // LL LR           LR R
        let mut l_sibling = root.take_left().unwrap();
        root.set_left(l_sibling.take_right());
        Node::update_size_height(&mut root);

        l_sibling.set_right(Some(root));
        Node::update_size_height(&mut l_sibling);

        return l_sibling;
    }

    fn l_rotation(mut root: Box<Node<K, V>>) -> Box<Node<K, V>> {
        // Old tree:    Tree after rotation:
        //      C           R
        //    /  \         / \
        //   L   R        C  RR
        //      / \      / \
        //     RL RR    L  RL

        let mut r_sibling = root.take_right().unwrap();
        root.set_right(r_sibling.take_left());
        Node::update_size_height(&mut root);

        r_sibling.set_left(Some(root));
        Node::update_size_height(&mut r_sibling);

        return r_sibling;
    }
    fn update_size_height(node: &mut Box<Node<K, V>>) {
        node.sub_tree_size =
            Node::get_size(node.get_left_ref()) + Node::get_size(node.get_right_ref());
        node.height = usize::max(
            Node::get_height(node.get_left_ref()),
            Node::get_height(node.get_right_ref()),
        ) + 1;
    }
}
