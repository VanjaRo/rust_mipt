#![forbid(unsafe_code)]

use crate::{
    data::ObjectId,
    error::{Error, NotFoundError, Result},
    object::{Object, Store},
    storage::StorageTransaction,
};
use std::ops::Deref;
use std::{
    any::Any,
    cell::{Cell, Ref, RefCell, RefMut},
    collections::HashMap,
    marker::PhantomData,
    rc::Rc,
};

////////////////////////////////////////////////////////////////////////////////

pub struct Transaction<'a> {
    inner: Box<dyn StorageTransaction + 'a>,
    obj_cash: RefCell<HashMap<ObjectId, Rc<DataCell>>>,
    state_cash: RefCell<StateMap>,
}

pub struct DataCell(RefCell<Box<dyn Store>>);

pub type StateMap = HashMap<ObjectId, Rc<Cell<ObjectState>>>;

impl<'a> Transaction<'a> {
    pub(crate) fn new(inner: Box<dyn StorageTransaction + 'a>) -> Self {
        Self {
            inner,
            obj_cash: RefCell::default(),
            state_cash: RefCell::default(),
        }
    }

    fn ensure_table<T: Object>(&self) -> Result<()> {
        let table_exists = self.inner.table_exists(T::get_schema().table_name)?;
        if table_exists {
            return Ok(());
        }
        self.inner.create_table(T::get_schema())
    }

    pub fn create<T: Object>(&self, src_obj: T) -> Result<Tx<'_, T>> {
        self.ensure_table::<T>()?;
        let obj_id = self.inner.insert_row(T::get_schema(), &src_obj.to_row())?;
        let cell = Rc::new(DataCell(RefCell::new(Box::new(src_obj))));
        let state = Rc::new(Cell::new(ObjectState::Clean));

        self.obj_cash.borrow_mut().insert(obj_id, cell.clone());
        self.state_cash.borrow_mut().insert(obj_id, state.clone());

        Ok(Tx {
            id: obj_id,
            state,
            obj: cell,
            lifetime: PhantomData,
        })
    }

    pub fn get<T: Object>(&self, id: ObjectId) -> Result<Tx<'_, T>> {
        // self.ensure_table::<T>()?;

        // check the cache
        if let Some(state) = self.state_cash.borrow_mut().get(&id).cloned() {
            if state.get() == ObjectState::Removed {
                return Err(Error::NotFound(Box::new(NotFoundError {
                    object_id: id,
                    type_name: T::get_schema().obj_ty,
                })));
            }
            if let Some(obj) = self.obj_cash.borrow_mut().get(&id).cloned() {
                return Ok(Tx {
                    id,
                    state,
                    obj,
                    lifetime: PhantomData,
                });
            }
        }

        // go to db
        let obj = Rc::new(DataCell(RefCell::new(Box::new(T::from_row(
            self.inner.select_row(id, T::get_schema())?,
        )))));
        let state = Rc::new(Cell::new(ObjectState::Clean));

        self.obj_cash.borrow_mut().insert(id, obj.clone());
        self.state_cash.borrow_mut().insert(id, state.clone());

        Ok(Tx {
            id,
            state,
            obj,
            lifetime: PhantomData,
        })
    }

    fn try_apply(&self) -> Result<()> {
        for (key, value) in self.obj_cash.borrow().iter() {
            let object = value.0.borrow();
            let state = self.state_cash.borrow().get(key).cloned().unwrap();
            match state.deref().get() {
                ObjectState::Removed => self.inner.delete_row(*key, object.get_schema())?,
                ObjectState::Modified => {
                    self.inner
                        .update_row(*key, object.get_schema(), &object.to_row())?
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn commit(self) -> Result<()> {
        self.try_apply()?;
        self.inner.commit()
    }

    pub fn rollback(self) -> Result<()> {
        self.inner.rollback()
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ObjectState {
    Clean,
    Modified,
    Removed,
}

#[derive(Clone)]
pub struct Tx<'a, T> {
    id: ObjectId,
    state: Rc<Cell<ObjectState>>,
    obj: Rc<DataCell>,
    lifetime: PhantomData<&'a T>,
}

impl<'a, T: Any> Tx<'a, T> {
    pub fn id(&self) -> ObjectId {
        self.id
    }

    pub fn state(&self) -> ObjectState {
        self.state.get()
    }

    pub fn borrow(&self) -> Ref<'_, T> {
        if self.state.get() == ObjectState::Removed {
            panic!("cannot borrow a removed object")
        }
        Ref::map(self.obj.0.borrow(), |store| {
            store.as_any().downcast_ref().unwrap()
        })
    }

    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        if self.state.get() == ObjectState::Removed {
            panic!("cannot borrow a removed object")
        }
        self.state.replace(ObjectState::Modified);
        RefMut::map(self.obj.0.borrow_mut(), |store| {
            store.as_any_mut().downcast_mut().unwrap()
        })
    }

    pub fn delete(self) {
        if self.obj.0.try_borrow_mut().is_err() {
            panic!("cannot delete a borrowed object");
        }
        self.state.replace(ObjectState::Removed);
    }
}
