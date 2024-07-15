#![forbid(unsafe_code)]
use crate::{data::DataType, storage::Row};
use std::any::Any;

////////////////////////////////////////////////////////////////////////////////

pub trait Object: Any + Sized {
    fn to_row(&self) -> Row;
    fn from_row(row: &Row) -> Self;
    fn get_schema() -> &'static Schema;
}

pub trait Store {}

impl<T: Object> Store for T {}
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
