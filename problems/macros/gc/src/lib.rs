#![forbid(unsafe_code)]

pub use gc_derive::Scan;

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    ops::Deref,
    rc::{Rc, Weak},
};

////////////////////////////////////////////////////////////////////////////////

pub struct Gc<T> {
    weak: Weak<T>,
}

impl<T> Clone for Gc<T> {
    fn clone(&self) -> Self {
        Self {
            weak: self.weak.clone(),
        }
    }
}

impl<T> Gc<T> {
    pub fn borrow(&self) -> GcRef<'_, T> {
        GcRef {
            rc: self.weak.upgrade().unwrap(),
            lifetime: PhantomData::<&'_ Gc<T>>,
        }
    }
}

pub struct GcRef<'a, T> {
    rc: Rc<T>,
    lifetime: PhantomData<&'a Gc<T>>,
}

impl<'a, T> Deref for GcRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.rc
    }
}

////////////////////////////////////////////////////////////////////////////////

pub trait Scan {
    fn get_children_ref_adrrs(&self) -> Vec<usize>;
}

impl<T> Scan for Gc<T> {
    fn get_children_ref_adrrs(&self) -> Vec<usize> {
        vec![self.weak.as_ptr() as usize]
    }
}

impl<T: Scan> Scan for Vec<T> {
    fn get_children_ref_adrrs(&self) -> Vec<usize> {
        self.iter().flat_map(Scan::get_children_ref_adrrs).collect()
    }
}

impl<T: Scan> Scan for Option<T> {
    fn get_children_ref_adrrs(&self) -> Vec<usize> {
        if let Some(el) = &self {
            return el.get_children_ref_adrrs();
        }
        Vec::new()
    }
}
impl<T: Scan> Scan for RefCell<T> {
    fn get_children_ref_adrrs(&self) -> Vec<usize> {
        self.borrow().get_children_ref_adrrs()
    }
}

impl Scan for i32 {
    fn get_children_ref_adrrs(&self) -> Vec<usize> {
        vec![]
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct Arena {
    addr_to_rc: HashMap<usize, Rc<dyn Scan>>,
}

impl Arena {
    pub fn new() -> Self {
        Self {
            addr_to_rc: HashMap::new(),
        }
    }

    pub fn allocation_count(&self) -> usize {
        self.addr_to_rc.len()
    }

    pub fn alloc<T: Scan + 'static>(&mut self, obj: T) -> Gc<T> {
        let rc_ref = Rc::new(obj);
        let new_gc = Gc {
            weak: Rc::downgrade(&rc_ref),
        };
        self.addr_to_rc.insert(Rc::as_ptr(&rc_ref) as usize, rc_ref);
        new_gc
    }

    pub fn sweep(&mut self) {
        // count the number of other objects
        // that refer to stored references to objects
        let mut addrs_ref_count = HashMap::<usize, usize>::new();
        self.addr_to_rc.iter().for_each(|(_, rc_scan)| {
            for gc_ref in rc_scan.get_children_ref_adrrs() {
                addrs_ref_count
                    .entry(gc_ref)
                    .and_modify(|count| *count += 1)
                    .or_insert(0);
            }
        });

        let mut marked_addrs = HashSet::<usize>::new();
        self.addr_to_rc.iter().for_each(|(addr, rc_scan)| {
            if Rc::weak_count(rc_scan) > *addrs_ref_count.get(addr).unwrap_or(&0) {
                self.spread_mark(*addr, &mut marked_addrs);
            }
        });

        // drop unmarked rcs
        self.addr_to_rc
            .clone()
            .into_iter()
            .filter(|(addr, _)| !marked_addrs.contains(addr))
            .for_each(|(addr, rc_scan)| {
                drop(rc_scan);
                self.addr_to_rc.remove(&addr);
            });
    }

    fn spread_mark(&self, root_addr: usize, marked: &mut HashSet<usize>) {
        if marked.insert(root_addr) {
            self.addr_to_rc
                .get(&root_addr)
                .unwrap()
                .get_children_ref_adrrs()
                .into_iter()
                .for_each(|addr| self.spread_mark(addr, marked))
        }
    }
}
