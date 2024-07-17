#![forbid(unsafe_code)]
use crate::{data::DataType, storage::Row};
use std::any::Any;

////////////////////////////////////////////////////////////////////////////////

pub trait Object: Any + Sized {
    fn to_row(&self) -> Row;
    fn from_row(row: Row) -> Self;
    fn get_schema() -> &'static Schema;
}

pub trait Store {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn get_schema(&self) -> &'static Schema;
    fn to_row(&self) -> Row;
}

impl<T: Object> Store for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_schema(&self) -> &'static Schema {
        T::get_schema()
    }

    fn to_row(&self) -> Row {
        self.to_row()
    }
}
////////////////////////////////////////////////////////////////////////////////

pub struct Schema {
    pub obj_ty: &'static str,
    pub table_name: &'static str,
    pub obj_fields: &'static [ObjectField],
}

impl Schema {
    pub fn columns_to_sql(&self) -> String {
        self.obj_fields
            .iter()
            .map(|field| field.column.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub struct ObjectField {
    pub name: &'static str,
    pub column: &'static str,
    pub data_ty: DataType,
}
