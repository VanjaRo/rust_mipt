#![no_std]

use core::{
    mem::MaybeUninit,
    ops::{Index, IndexMut},
    ptr, slice,
};

pub struct SmolVec<T, const CAP: usize> {
    len: usize,
    arr: [MaybeUninit<T>; CAP],
}

impl<T, const CAP: usize> SmolVec<T, CAP> {
    const CAPACITY: usize = CAP;

    pub fn new() -> Self {
        unsafe {
            Self {
                len: 0,
                arr: MaybeUninit::uninit().assume_init(),
            }
        }
    }

    fn as_mut_ptr(&mut self) -> *mut T {
        self.arr.as_mut_ptr() as _
    }

    fn as_ptr(&mut self) -> *const T {
        self.arr.as_ptr() as _
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&mut self, obj: T) -> Option<T> {
        if self.len() == Self::CAPACITY {
            return Some(obj);
        }
        unsafe {
            self.push_unchecked(obj);
        }
        None
    }

    unsafe fn push_unchecked(&mut self, obj: T) {
        ptr::write(self.as_mut_ptr().add(self.len), obj);
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        unsafe {
            self.len -= 1;
            Some(ptr::read(self.as_ptr().add(self.len)))
        }
    }
}

impl<T, const CAP: usize> Default for SmolVec<T, CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const CAP: usize> Index<usize> for SmolVec<T, CAP> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.len(), "index out of range");
        unsafe { self.arr[index].assume_init_ref() }
    }
}

impl<T, const CAP: usize> IndexMut<usize> for SmolVec<T, CAP> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.len(), "index out of range");
        unsafe { self.arr[index].assume_init_mut() }
    }
}

impl<T, const CAP: usize> Drop for SmolVec<T, CAP> {
    fn drop(&mut self) {
        unsafe {
            let raw_slice = slice::from_raw_parts_mut(self.as_mut_ptr(), self.len());
            ptr::drop_in_place(raw_slice);
        }
    }
}
