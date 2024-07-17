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

impl From<ObjectId> for i64 {
    fn from(id: ObjectId) -> i64 {
        id.0
    }
}

impl From<i64> for ObjectId {
    fn from(i: i64) -> ObjectId {
        ObjectId(i)
    }
}

impl ObjectId {
    pub fn into_i64(&self) -> i64 {
        self.0
    }
}

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

pub trait DataTypeWrapper {
    const TYPE: DataType;
}

macro_rules! datatype_wrap {
    ($from_ty:ty, $enum_var:ident) => {
        impl DataTypeWrapper for $from_ty {
            const TYPE: DataType = DataType::$enum_var;
        }
    };
}

datatype_wrap!(String, String);
datatype_wrap!(Vec<u8>, Bytes);
datatype_wrap!(i64, Int64);
datatype_wrap!(f64, Float64);
datatype_wrap!(bool, Bool);

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
#[derive(Debug)]
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

// using ident for enum_var to use pattern matching semantics for the second method
macro_rules! impl_val_from_ty {
    ($from_ty:ty, $enum_var:ident) => {
        impl<'a> From<&'a $from_ty> for Value<'a> {
            fn from(ty: &'a $from_ty) -> Self {
                Value::$enum_var(*ty)
            }
        }
        impl<'a> From<Value<'a>> for $from_ty {
            fn from(val: Value<'a>) -> Self {
                if let Value::$enum_var(v) = val {
                    return v;
                }
                panic!("no matching value to unwrap");
            }
        }
    };
}

macro_rules! impl_val_from_ty_cow {
    ($from_ty:ty, $enum_var:ident) => {
        impl<'a> From<&'a $from_ty> for Value<'a> {
            fn from(ty: &'a $from_ty) -> Self {
                Value::$enum_var(Cow::Borrowed(ty))
            }
        }
        impl<'a> From<Value<'a>> for $from_ty {
            fn from(val: Value<'a>) -> Self {
                if let Value::$enum_var(v) = val {
                    return v.into_owned();
                }
                panic!("no matching value to unwrap")
            }
        }
    };
}
impl_val_from_ty_cow!(String, String);
impl_val_from_ty_cow!(Vec<u8>, Bytes);
impl_val_from_ty!(i64, Int64);
impl_val_from_ty!(f64, Float64);
impl_val_from_ty!(bool, Bool);
