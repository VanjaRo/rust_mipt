#![forbid(unsafe_code)]
use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
};

use rusqlite::{types::ToSqlOutput, ToSql};
// use rusqlite::ToSql;

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct ObjectId(i64);

impl ObjectId {
    pub fn new(i: i64) -> Self {
        Self(i)
    }
}

impl ToSql for ObjectId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

// impl From<ObjectId> for i64 {
//     fn from(id: ObjectId) -> i64 {
//         id.0
//     }
// }

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    String,
    Bytes,
    Int64,
    Float64,
    Bool,
}

impl Display for DataType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            DataType::String => "TEXT",
            DataType::Bytes => "BLOB",
            DataType::Int64 => "BIGINT",
            DataType::Float64 => "REAL",
            DataType::Bool => "TINYINT",
        };
        write!(f, "{}", name)
    }
}

////////////////////////////////////////////////////////////////////////////////

pub enum Value<'a> {
    String(Cow<'a, str>),
    Bytes(Cow<'a, [u8]>),
    Int64(i64),
    Float64(f64),
    Bool(bool),
}

impl<'a> ToSql for Value<'a> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            Value::String(s) => Ok(ToSqlOutput::from(s.as_ref())),
            Value::Bytes(b) => Ok(ToSqlOutput::from(b.as_ref())),
            Value::Int64(i) => Ok(ToSqlOutput::from(*i)),
            Value::Float64(f) => Ok(ToSqlOutput::from(*f)),
            Value::Bool(bl) => Ok(ToSqlOutput::from(*bl)),
        }
    }
}

// TODO: your code goes here.
