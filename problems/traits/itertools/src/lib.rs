#![forbid(unsafe_code)]

use std::{cell::RefCell, collections::VecDeque, rc::Rc, vec::IntoIter};

pub struct LazyCycle<I>
where
    I: Iterator,
{
    iter: I,
    loopback: Vec<I::Item>,
    curr_idx: usize,
    done: bool,
}
impl<I> LazyCycle<I>
where
    I: Iterator,
    I::Item: Clone,
{
    fn on_done(&mut self) -> Option<I::Item> {
        self.loopback.get(self.curr_idx).cloned().map(|val| {
            self.curr_idx = (self.curr_idx + 1) % self.loopback.len();
            val
        })
    }
}

impl<I> Iterator for LazyCycle<I>
where
    I: Iterator,
    I::Item: Clone,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return self.on_done();
        }
        match self.iter.next() {
            Some(item) => {
                self.loopback.push(item.clone());
                Some(item)
            }
            None => {
                self.done = true;
                self.on_done()
            }
        }
    }
}
////////////////////////////////////////////////////////////////////////////////

pub struct Extract<I: Iterator> {
    iter: I,
}

impl<I: Iterator> Iterator for Extract<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct Tee<I>
where
    I: Iterator,
    I::Item: Clone,
{
    tee_deq: Rc<RefCell<TeeDeq<I>>>,
    is_fst: bool,
}

struct TeeDeq<I>
where
    I: Iterator,
    I::Item: Clone,
{
    iter: I,
    deq: VecDeque<I::Item>,
    fst_iterates: bool,
    done: bool,
}

impl<I> Iterator for Tee<I>
where
    I: Iterator,
    I::Item: Clone,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let mut tee_deq = self.tee_deq.borrow_mut();
        if self.is_fst == tee_deq.fst_iterates && !tee_deq.deq.is_empty() {
            return tee_deq.deq.pop_front();
        }

        if !tee_deq.done {
            match tee_deq.iter.next() {
                Some(val) => {
                    tee_deq.deq.push_back(val.clone());
                    tee_deq.fst_iterates = !self.is_fst;
                    return Some(val);
                }
                None => {
                    tee_deq.done = true;
                    return None;
                }
            }
        }

        None
        // if self.done {
        //     return None;
        // }
        // let mut tee_deq = self.tee_deq.borrow_mut();
        // if tee_deq.fst_behind && self.is_fst && !tee_deq.deq.is_empty() {
        //     return tee_deq.deq.pop_front();
        // }

        // tee_deq.iter.next().map(|val| {
        //     tee_deq.deq.push_back(val.clone());
        //     val
        // })
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct GroupBy<I, F, V>
where
    I: Iterator,
    F: FnMut(&I::Item) -> V,
    V: Eq,
{
    iter: I,
    ret_iter: IntoIter<(V, Vec<I::Item>)>,
    precalculated: bool,
    f: F,
}

impl<I, F, V> GroupBy<I, F, V>
where
    I: Iterator,
    F: FnMut(&I::Item) -> V,
    V: Eq,
{
    fn precalculate_iter(&mut self) {
        let mut vec_of_tpls: Vec<(V, Vec<I::Item>)> = Vec::new();
        for val in self.iter.by_ref() {
            if vec_of_tpls.is_empty() {
                vec_of_tpls.push(((self.f)(&val), vec![val]));
                continue;
            }
            let (prev_v, last_vec) = vec_of_tpls.last_mut().unwrap();
            let curr_v = (self.f)(&val);
            if *prev_v == curr_v {
                last_vec.push(val);
            } else {
                vec_of_tpls.push((curr_v, vec![val]));
            }
        }
        self.precalculated = true;
        self.ret_iter = vec_of_tpls.into_iter();
    }
}

impl<I, F, V> Iterator for GroupBy<I, F, V>
where
    I: Iterator,
    F: FnMut(&I::Item) -> V,
    V: Eq,
{
    type Item = (V, Vec<I::Item>);

    fn next(&mut self) -> Option<Self::Item> {
        if !self.precalculated {
            self.precalculate_iter();
        }
        self.ret_iter.next()
    }
}

////////////////////////////////////////////////////////////////////////////////

pub trait ExtendedIterator: Iterator {
    fn lazy_cycle(self) -> LazyCycle<Self>
    where
        Self: Sized,
        Self::Item: Clone,
    {
        LazyCycle {
            iter: self,
            loopback: Vec::new(),
            curr_idx: 0,
            done: false,
        }
    }

    fn extract(
        mut self,
        index: usize,
    ) -> (
        Option<Self::Item>,
        Extract<impl Iterator<Item = Self::Item>>,
    )
    where
        Self: Sized,
    {
        let mut before_idx_vec = Vec::new();
        for _ in 0..index {
            match self.next() {
                None => break,
                Some(item) => before_idx_vec.push(item),
            }
        }
        (
            self.next(),
            Extract {
                iter: before_idx_vec.into_iter().chain(self),
            },
        )
    }

    fn tee(self) -> (Tee<Self>, Tee<Self>)
    where
        Self: Sized,
        Self::Item: Clone,
    {
        let tee_deq = Rc::new(RefCell::new(TeeDeq {
            deq: VecDeque::new(),
            iter: self,
            fst_iterates: true,
            done: false,
        }));

        (
            Tee {
                tee_deq: tee_deq.clone(),
                is_fst: true,
            },
            Tee {
                tee_deq,
                is_fst: false,
            },
        )
    }

    fn group_by<F, V>(self, func: F) -> GroupBy<Self, F, V>
    where
        Self: Sized,
        F: FnMut(&Self::Item) -> V,
        V: Eq,
    {
        GroupBy {
            iter: self,
            ret_iter: Vec::new().into_iter(),
            precalculated: false,
            f: func,
        }
    }
}

impl<I: Iterator> ExtendedIterator for I {}
// TODO: your code goes here.
