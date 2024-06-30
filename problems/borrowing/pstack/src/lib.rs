#![forbid(unsafe_code)]
use std::rc::Rc;

pub struct PRef<T> {
    value: Rc<Node<T>>,
}

struct Node<T> {
    value: T,
    next: Option<Rc<Node<T>>>,
}

impl<T> std::ops::Deref for PRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value.as_ref().value
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct PStack<T> {
    head: Option<Rc<Node<T>>>,
    len: usize,
}

impl<T> Default for PStack<T> {
    fn default() -> Self {
        Self { head: None, len: 0 }
    }
}

impl<T> Clone for PStack<T> {
    fn clone(&self) -> Self {
        Self {
            head: self.head.clone(),
            len: self.len,
        }
    }
}

impl<T> Iterator for PStack<T> {
    type Item = PRef<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((next_node, new_stack)) = self.pop() {
            self.head = new_stack.head;
            return Some(next_node);
        }
        None
    }
}

impl<T> PStack<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&self, value: T) -> Self {
        Self {
            head: Some(Rc::new(Node {
                value,
                next: self.head.clone(),
            })),
            len: self.len + 1,
        }
    }

    pub fn pop(&self) -> Option<(PRef<T>, Self)> {
        match self.head.clone() {
            Some(node_ref) => {
                let new_stack = Self {
                    head: node_ref.next.clone(),
                    len: self.len - 1,
                };
                Some((PRef { value: node_ref }, new_stack))
            }
            None => None,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = PRef<T>> {
        self.clone()
    }
}
