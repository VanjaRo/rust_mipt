#![forbid(unsafe_code)]

use std::collections::VecDeque;

#[derive(Default)]
pub struct MinQueue<T> {
    q: VecDeque<T>,
    min_q: VecDeque<T>,
}

impl<T: Clone + Ord> MinQueue<T> {
    pub fn new() -> Self {
        Self {
            q: VecDeque::new(),
            min_q: VecDeque::new(),
        }
    }

    pub fn push(&mut self, val: T) {
        self.q.push_back(val.clone());
        // was 0 before 1
        if self.q.len() == 1 {
            self.min_q.push_back(val);
        } else {
            while !self.min_q.is_empty() && *self.min_q.back().unwrap() > val {
                self.min_q.pop_back();
            }
            self.min_q.push_back(val);
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if let Some(val) = self.q.pop_front() {
            if val == *self.min_q.front().unwrap() {
                self.min_q.pop_front();
            }
            return Some(val);
        }
        None
    }

    pub fn front(&self) -> Option<&T> {
        self.q.front()
    }

    pub fn min(&self) -> Option<&T> {
        self.min_q.front()
    }

    pub fn len(&self) -> usize {
        self.q.len()
    }

    pub fn is_empty(&self) -> bool {
        self.q.is_empty()
    }
}
