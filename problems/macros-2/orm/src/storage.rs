#![forbid(unsafe_code)]
use crate::{
    data::{DataType, Value},
    error::{Error, NotFoundError, Result},
    object::Schema,
    ObjectId,
};
use rusqlite::ToSql;
use std::{borrow::Cow, fmt::Write};

////////////////////////////////////////////////////////////////////////////////

pub type Row<'a> = Vec<Value<'a>>;
pub type RowSlice<'a> = [Value<'a>];

////////////////////////////////////////////////////////////////////////////////

pub trait StorageTransaction {
    fn table_exists(&self, table: &str) -> Result<bool>;
    fn create_table(&self, schema: &Schema) -> Result<()>;

    fn insert_row(&self, schema: &Schema, row: &RowSlice) -> Result<ObjectId>;
    fn update_row(&self, id: ObjectId, schema: &Schema, row: &RowSlice) -> Result<()>;
    fn select_row(&self, id: ObjectId, schema: &Schema) -> Result<Row<'static>>;
    fn delete_row(&self, id: ObjectId, schema: &Schema) -> Result<()>;

    fn commit(&self) -> Result<()>;
    fn rollback(&self) -> Result<()>;
}

impl<'a> StorageTransaction for rusqlite::Transaction<'a> {
    fn table_exists(&self, table: &str) -> Result<bool> {
        Ok(self
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE name = ?",
                [table],
                |_| Ok(()),
            )
            .is_ok())
    }

    fn create_table(&self, schema: &Schema) -> Result<()> {
        let mut query = format!(
            "CREATE TABLE {}(
        id INTEGER PRIMARY KEY AUTOINCREMENT",
            schema.table_name
        );
        if !schema.obj_fields.is_empty() {
            write!(&mut query, ",").unwrap();
            write!(
                &mut query,
                "{}",
                schema
                    .obj_fields
                    .iter()
                    .map(|field| format!("{} {}", field.column, field.data_ty))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
            .unwrap();
        }
        write!(&mut query, ")").unwrap();
        println!("{}", query);

        self.execute(&query, [])
            .map_err(|e| Error::with_schema(e, schema))?;
        Ok(())
    }

    fn insert_row(&self, schema: &Schema, row: &RowSlice) -> Result<ObjectId> {
        let mut sql_query = format!("INSERT INTO {} ", schema.table_name);

        if row.is_empty() {
            write!(&mut sql_query, "DEFAULT VALUES").unwrap();
        } else {
            write!(&mut sql_query, "({}) ", schema.columns_to_sql()).unwrap();
            write!(
                &mut sql_query,
                "VALUES ({}) ",
                vec!["?"; row.len()].join(", ")
            )
            .unwrap();
        }
        let params = row.iter().map(|val| val as &dyn ToSql).collect::<Vec<_>>();
        self.execute(&sql_query, params.as_slice()).map_err(|e| {
            Error::with_schema_obj_id(e, schema, ObjectId::new(self.last_insert_rowid()))
        })?;
        Ok(ObjectId::new(self.last_insert_rowid()))
    }

    fn update_row(&self, id: ObjectId, schema: &Schema, row: &RowSlice) -> Result<()> {
        if schema.obj_fields.is_empty() {
            return Ok(());
        }
        let mut sql_query = format!("UPDATE {} SET ", schema.table_name);
        let updated_vals = schema
            .obj_fields
            .iter()
            .map(|field| format!("{} = ?", field.name))
            .collect::<Vec<_>>()
            .join(",");
        write!(&mut sql_query, "{}", updated_vals).unwrap();
        write!(&mut sql_query, "WHERE id = {}", id).unwrap();

        let params = row.iter().map(|val| val as &dyn ToSql).collect::<Vec<_>>();
        self.execute(&sql_query, params.as_slice()).map_err(|e| {
            Error::with_schema_obj_id(e, schema, ObjectId::new(self.last_insert_rowid()))
        })?;
        Ok(())
    }

    fn select_row(&self, id: ObjectId, schema: &Schema) -> Result<Row<'static>> {
        // SELECT co1, col2 FROM table WHERE id = 123
        // may add self.prepare_cached(sql)

        if !self.table_exists(schema.table_name).unwrap() {
            return Err(Error::NotFound(Box::new(NotFoundError {
                object_id: id,
                type_name: schema.obj_ty,
            })));
        }

        let sql_query = format!(
            "SELECT {} FROM {} WHERE id = ?",
            schema.columns_to_sql(),
            schema.table_name
        );

        let ret_val = self.query_row(&sql_query, [&id], |row| {
            let columns_count = schema.obj_fields.len();
            let mut obj_fields = Vec::with_capacity(columns_count);
            for i in 0..columns_count {
                let d_type = schema.obj_fields[i].data_ty;
                let value = match d_type {
                    DataType::Bytes => Value::Bytes(Cow::Owned(row.get(i)?)),
                    DataType::Int64 => Value::Int64(row.get(i)?),
                    DataType::String => Value::String(Cow::Owned(row.get(i)?)),
                    DataType::Float64 => Value::Float64(row.get(i)?),
                    DataType::Bool => Value::Bool(row.get(i)?),
                };
                obj_fields.push(value);
            }
            Ok(obj_fields)
        });
        ret_val.map_err(|e| Error::with_schema_obj_id(e, schema, id))
    }

    fn delete_row(&self, id: ObjectId, schema: &Schema) -> Result<()> {
        let sql = format!("DELETE FROM {} WHERE id = ?", schema.table_name);
        self.execute(&sql, [&id])
            .map_err(|e| Error::with_schema_obj_id(e, schema, id))?;
        Ok(())
    }

    fn commit(&self) -> Result<()> {
        self.execute("COMMIT", []).map_err(Error::from)?;
        Ok(())
    }

    fn rollback(&self) -> Result<()> {
        self.execute("ROLLBACK", []).map_err(Error::from)?;
        Ok(())
    }
}
