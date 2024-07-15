#![forbid(unsafe_code)]
use std::marker::PhantomData;

use crate::{data::DataType, object::Schema, ObjectId};
use thiserror::Error;

////////////////////////////////////////////////////////////////////////////////

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    NotFound(Box<NotFoundError>),
    #[error(transparent)]
    UnexpectedType(Box<UnexpectedTypeError>),
    #[error(transparent)]
    MissingColumn(Box<MissingColumnError>),
    #[error("database is locked")]
    LockConflict,
    #[error("storage error: {0}")]
    Storage(#[source] Box<dyn std::error::Error>),
}
impl Error {
    pub fn with_schema(e: rusqlite::Error, schema: &Schema) -> Error {
        Error::from(ErrorWithCtx::new(e, ErrorCtx::new(Some(schema), None)))
    }

    pub fn with_schema_obj_id(e: rusqlite::Error, schema: &Schema, object_id: ObjectId) -> Error {
        Error::from(ErrorWithCtx::new(
            e,
            ErrorCtx::new(Some(schema), Some(object_id)),
        ))
    }
}

// Used to pass the schema context into the error creation
#[derive(Default)]
pub struct ErrorCtx<'a> {
    pub schema: Option<&'a Schema>,
    pub object_id: Option<ObjectId>,
    // pub obj_ty_name: Option<&'static str>,
    // pub table_name: Option<&'static str>,
    // pub field_name: Option<&'static str>,
    // pub column_name: Option<&'static str>,

    // pub expected_type: Option<DataType>,
    // pub got_type: Option<String>,
}

impl<'a> ErrorCtx<'a> {
    pub fn new(schema: Option<&Schema>, object_id: Option<ObjectId>) -> Self {
        Self { schema, object_id }
    }
}

pub struct ErrorWithCtx<'a, Err> {
    err: Err,
    ctx: ErrorCtx<'a>,
    lifetime: PhantomData<&'a Err>,
}

impl<'a, Err> ErrorWithCtx<'a, Err> {
    pub fn new(err: Err, ctx: ErrorCtx) -> Self {
        Self {
            err,
            ctx,
            lifetime: PhantomData,
        }
    }
}

impl<'a> From<ErrorWithCtx<'a, rusqlite::Error>> for Error {
    fn from(err: ErrorWithCtx<rusqlite::Error>) -> Self {
        let ctx_schema = err.ctx.schema.unwrap();
        match &err.err {
            rusqlite::Error::QueryReturnedNoRows => Error::NotFound(Box::new(NotFoundError {
                object_id: err.ctx.object_id.unwrap(),
                type_name: &ctx_schema.obj_ty,
            })),

            rusqlite::Error::InvalidColumnType(field_idx, column, ty_got) => {
                Error::UnexpectedType(Box::new(UnexpectedTypeError {
                    type_name: ctx_schema.obj_ty,
                    attr_name: ctx_schema.obj_fields[*field_idx].name,
                    table_name: ctx_schema.table_name,
                    column_name: column,
                    expected_type: ctx_schema.obj_fields[*field_idx].data_ty,
                    got_type: ty_got.to_string(),
                }))
            }
            rusqlite::Error::SqliteFailure(err_sql, msg) => {
                if err_sql.code == rusqlite::ErrorCode::DatabaseBusy {
                    return Error::LockConflict;
                }
                let column_name_opt = msg.as_ref().and_then(|txt| {
                    if txt.contains("no such column:") {
                        return txt.split("no such column:").last();
                    } else if txt.contains("has no column named") {
                        return txt.split("has no column named").last();
                    }
                    None
                });

                match column_name_opt {
                    None => Error::Storage(Box::new(err.err)),
                    Some(column_name) => {
                        let attr_name = ctx_schema
                            .obj_fields
                            .iter()
                            .find(|&&field| field.column == column_name)
                            .map(|field| field.name)
                            .unwrap();
                        Error::MissingColumn(Box::new(MissingColumnError {
                            type_name: ctx_schema.obj_ty,
                            attr_name: attr_name,
                            table_name: ctx_schema.table_name,
                            column_name: &column_name,
                        }))
                    }
                }
            }
            _ => Error::Storage(Box::new(err.err)),
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Self::from(ErrorWithCtx::new(err, ErrorCtx::default()))
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Error, Debug)]
#[error("object is not found: type '{type_name}', id {object_id}")]
pub struct NotFoundError {
    pub object_id: ObjectId,
    pub type_name: &'static str,
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Error, Debug)]
#[error(
    "invalid type for {type_name}::{attr_name}: expected equivalent of {expected_type:?}, \
    got {got_type} (table: {table_name}, column: {column_name})"
)]
pub struct UnexpectedTypeError {
    pub type_name: &'static str,
    pub attr_name: &'static str,
    pub table_name: &'static str,
    pub column_name: &'static str,
    pub expected_type: DataType,
    pub got_type: String,
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Error, Debug)]
#[error(
    "missing a column for {type_name}::{attr_name} \
    (table: {table_name}, column: {column_name})"
)]
pub struct MissingColumnError {
    pub type_name: &'static str,
    pub attr_name: &'static str,
    pub table_name: &'static str,
    pub column_name: &'static str,
}

////////////////////////////////////////////////////////////////////////////////

pub type Result<T> = std::result::Result<T, Error>;
