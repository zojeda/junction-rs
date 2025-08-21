use serde::{de::DeserializeOwned, Serialize};
use std::collections::BTreeMap;

/// Parameters used in rendered queries
pub type Params = BTreeMap<String, serde_json::Value>;

/// DB-agnostic logical query. Backends must compile this AST to their native form.
pub type Query = crate::query::ast::Query;

/// Database-agnostic executor abstraction. Implement this for each backend.
///
/// Contract:
/// - Accepts a backend-agnostic [`Query`], returns a list of `T` decoded from the DB response.
/// - Should return `Ok(Vec::new())` for statements that don't produce rows (e.g. delete), not panic.
/// - Map backend errors to [`DbError::Adapter`].
///
/// Example (SurrealDB adapter implementation lives behind the `surreal` feature):
/// ```ignore
/// impl DbExecutor for SurrealDbAdapter {
///     async fn execute<T: DeserializeOwned + Send>(&self, query: Query) -> Result<Vec<T>, DbError> {
///         // compile AST -> native SQL, bind params, run, deserialize Vec<T>
///         # unimplemented!()
///     }
/// }
/// ```
pub trait DbExecutor: Clone + Send + Sync + 'static {
    fn execute<T: DeserializeOwned + Send>(
        &self,
        query: Query,
    ) -> impl std::future::Future<Output = Result<Vec<T>, DbError>> + Send;
}

#[derive(thiserror::Error, Debug)]
pub enum DbError {
    #[error("adapter error: {0}")]
    Adapter(String),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Common base trait to expose a model's table name.
///
/// Implemented automatically for types that derive `Node` or `Edge`.
pub trait HasTableName: Sized + Send + Sync + 'static {
    /// Returns the static table name associated with this entity.
    fn table_name() -> &'static str;
}

pub trait Node:
    HasTableName + Serialize + DeserializeOwned + Sized + Send + Sync + 'static
{
    const TABLE: &'static str;
    type Schema;
    fn table() -> crate::schema::Table<Self> {
        crate::schema::Table::new(Self::TABLE)
    }
    fn schema() -> Self::Schema;
    fn table_name() -> &'static str {
        Self::TABLE
    }
}

/// Marker trait for Edge entities. Generic over endpoints for static typing.
pub trait Edge:
    HasTableName + Serialize + DeserializeOwned + Sized + Send + Sync + 'static
{
    const TABLE: &'static str;
    type Schema;
    type From: Node;
    type To: Node;
    /// Name of the struct field annotated with #[junction(source)] (user-level identifier)
    const SOURCE_FIELD_NAME: &'static str = "in"; // default placeholder; derive overrides
    /// Name of the struct field annotated with #[junction(target)] (user-level identifier)
    const TARGET_FIELD_NAME: &'static str = "out"; // default placeholder; derive overrides

    fn table() -> crate::schema::EdgeTable<Self, (), ()> {
        crate::schema::EdgeTable::new(Self::TABLE)
    }
    fn schema() -> Self::Schema;
    fn table_name() -> &'static str {
        Self::TABLE
    }
}

/// Types that can be inserted via the unified `insert` API. Implemented by the derive macros
/// for `Node` and `Edge` so we avoid overlapping blanket impl coherence issues.
pub trait Insertable: Serialize + DeserializeOwned + Sized + Send + Sync + 'static {
    /// Table name target for insertion.
    fn table_name() -> &'static str;
    /// For edges, returns the (source_field_name, target_field_name); nodes return None.
    fn endpoint_fields() -> Option<(&'static str, &'static str)> {
        None
    }
}
