#![forbid(unsafe_code)]

use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

pub struct Context {
    hmap_k_v: HashMap<String, Box<dyn Any>>,
    hmap_t_v: HashMap<TypeId, Box<dyn Any>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            hmap_k_v: HashMap::new(),
            hmap_t_v: HashMap::new(),
        }
    }

    pub fn insert<K, V>(&mut self, key: K, obj: V)
    where
        K: AsRef<str> + 'static,
        V: Any + 'static,
    {
        self.hmap_k_v
            .insert(String::from(key.as_ref()), Box::new(obj));
    }

    pub fn get<V>(&self, key: impl AsRef<str>) -> &V
    where
        V: 'static,
    {
        self.hmap_k_v
            .get(key.as_ref())
            .and_then(|boxed_val| boxed_val.downcast_ref())
            .unwrap()
    }

    pub fn insert_singletone<T: Any>(&mut self, obj: T) {
        self.hmap_t_v.insert(obj.type_id(), Box::new(obj));
    }

    pub fn get_singletone<T: Any>(&self) -> &T {
        self.hmap_t_v
            .get(&TypeId::of::<T>())
            .and_then(|boxed_val| boxed_val.downcast_ref())
            .unwrap()
    }
}
