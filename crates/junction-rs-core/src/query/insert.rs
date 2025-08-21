use super::ast;
use crate::schema::IntoTableName;
use crate::traits::{DbExecutor, Insertable};
/// Builder for inserting a homogeneous set of records (nodes or edges).
pub struct InsertBuilder<T: Insertable> {
    values: Vec<T>,
    table: &'static str,
    endpoint_fields: Option<(&'static str, &'static str)>,
}

pub fn insert<T: Insertable>(values: Vec<T>) -> InsertBuilder<T> {
    InsertBuilder {
        values,
        table: T::table_name(),
        endpoint_fields: T::endpoint_fields(),
    }
}

pub fn insert_into<T: Insertable>(table: impl IntoTableName, values: Vec<T>) -> InsertBuilder<T> {
    InsertBuilder {
        values,
        table: table.table_name(),
        endpoint_fields: T::endpoint_fields(),
    }
}

impl<T: Insertable> InsertBuilder<T> {
    pub fn into(self) -> Self {
        self
    }

    pub async fn return_many<DB>(self, db: DB) -> Result<Vec<T>, crate::traits::DbError>
    where
        DB: DbExecutor,
    {
        let arr = serde_json::to_value(&self.values)?;
        let mut q = ast::Query::new(ast::Stmt::Insert(ast::Insert {
            table: self.table,
            values: arr,
            endpoint_fields: self.endpoint_fields,
        }));
        if let Some((src, tgt)) = self.endpoint_fields {
            q = q
                .bind("__jxn_edge_src", serde_json::Value::String(src.to_string()))
                .bind("__jxn_edge_tgt", serde_json::Value::String(tgt.to_string()));
        }
        db.execute::<T>(q).await
    }
}
