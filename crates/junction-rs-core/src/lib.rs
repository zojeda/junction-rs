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

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::expr; // allow `expr!(...)` via junction_rs::prelude::* if referenced as `expr!(...)`
    pub use crate::expr::{cond, All, Expr};
    pub use crate::graph::{start, start_at, Path, PathSegment};
    pub use crate::id::SimpleId;
    pub use crate::query::select::IntoSelectColumn;
    pub use crate::query::{delete, insert, insert_into, select, select_edge, update};
    pub use crate::schema::{Col, Table};
    pub use crate::traits::{DbExecutor, Edge, HasTableName, Insertable, Node};
}
