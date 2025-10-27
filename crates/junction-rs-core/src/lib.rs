pub mod expr;
pub mod graph;
pub mod id;
pub mod query;
pub mod schema;
pub mod schema_ops;
pub mod traits;

pub use expr::{cond, All, Expr};
pub use graph::{start, start_at, Path, PathSegment};
pub use id::SimpleId;
pub use query::ast as qast;
pub use schema::{Col, EdgeTable, Table};
pub use traits::{DbExecutor, Edge, HasTableName, Insertable, Node, Params};
// Intentionally do not re-export `expr` module by name to avoid macro namespace conflicts.
pub use query::select::IntoSelectColumn;
pub use query::{delete, insert, insert_into, select, select_edge, update};
pub use query_get::get_by_id;

mod query_get {
    use crate::{id::SimpleId, query::select::SelectBuilder, traits::DbExecutor, All, Node};
    use serde::de::DeserializeOwned;

    /// Convenience helper to fetch a single entity by its `SimpleId<T>`.
    /// Returns `Ok(None)` if the row does not exist.
    pub async fn get_by_id<T, DB>(id: SimpleId<T>, db: DB) -> Result<Option<T>, crate::traits::DbError>
    where
        T: Node + DeserializeOwned + Send,
        DB: DbExecutor,
    {
        // Start with default star projection.
        SelectBuilder::<T>::from(crate::query::select::select::<T>(All), T::table())
            .by_id(id)
            .return_one(db)
            .await
    }
}

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::expr; // allow `expr!(...)` via junction_rs::prelude::* if referenced as `expr!(...)`
    pub use crate::expr::{cond, All, Expr};
    pub use crate::graph::{start, start_at, Path, PathSegment};
    pub use crate::id::SimpleId;
    pub use crate::query::select::IntoSelectColumn;
    pub use crate::query::{delete, insert, insert_into, select, select_edge, update};
    pub use crate::get_by_id; // re-export helper
    pub use crate::schema::{Col, Table};
    pub use crate::traits::{DbExecutor, Edge, HasTableName, Insertable, Node};
}
