#![forbid(unsafe_code)]

#[derive(Debug, PartialEq, Eq)]
pub struct HNil;

#[derive(Debug, PartialEq, Eq)]
pub struct HCons<H, T> {
    pub head: H,
    pub tail: T,
}

impl HNil {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HNil {
    fn default() -> Self {
        Self::new()
    }
}

impl<H, T> HCons<H, T> {
    pub fn new(head: H, tail: T) -> Self {
        Self { head, tail }
    }
}

////////////////////////////////////////////////////////////////////////////////
#[macro_export]
macro_rules! HList {
    [] => {$crate::hlist::HNil};
    [$head:ty $(, $tail:ty)* $(,)?] => {$crate::hlist::HCons<$head, $crate::HList![$($tail),*]>}
}

#[macro_export]
macro_rules! hlist {
    [] => {$crate::hlist::HNil};
    [$head:expr $(, $tail:expr)* $(,)?] => {$crate::hlist::HCons::new($head, $crate::hlist![$($tail),*])}
}

#[macro_export]
macro_rules! hlist_pat {
    [] => {$crate::hlist::HNil};
    [$head:ident $(, $tail:ident)* $(,)?] => {$crate::hlist::HCons{head: $head, tail: $crate::hlist_pat![$($tail),*]}}
}

// TODO: your code goes here.
