use serde_json::Value;
use std::collections::BTreeMap;

pub type Bindings = BTreeMap<String, Value>;

/// Backend-agnostic query with bound parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub stmt: Stmt,
    pub binds: Bindings,
}

impl Query {
    pub fn new(stmt: Stmt) -> Self {
        Self {
            stmt,
            binds: BTreeMap::new(),
        }
    }
    pub fn bind(mut self, key: impl Into<String>, val: Value) -> Self {
        self.binds.insert(key.into(), val);
        self
    }
}

/// Statement kinds supported by the high-level DSL.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Select(Select),
    Insert(Insert),
    Update(Update),
    Delete(Delete),
    Graph(crate::graph::GraphTraversal),
}

/// Select field specification
#[derive(Debug, Clone, PartialEq)]
pub enum SelectField {
    /// `*`
    Star,
    /// A simple column name
    Column(&'static str),
    /// A raw expression with optional alias
    Expr {
        expr: String,
        alias: Option<&'static str>,
    },
    /// `count()` with alias
    CountAll { alias: &'static str },
    /// `count(col)` with alias
    CountCol {
        col: &'static str,
        alias: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Distinct {
    All,
    On(Vec<&'static str>),
}

/// A `SELECT` statement.
#[derive(Debug, Clone, PartialEq)]
pub struct Select {
    pub table: &'static str,
    /// Optional record id to target a specific record like `table:id`.
    pub record_id: Option<String>,
    /// Projection list. When empty, `*" is selected.
    pub fields: Vec<SelectField>,
    pub filter: Option<PredExpr>,
    pub order_by: Vec<Ordering>,
    pub limit: Option<u64>,
    /// Group by specs
    pub group_by: Vec<GroupBySpec>,
    /// Traversal segments applied after base FROM target (backend-specific serialization for SurrealQL path navigation).
    pub traversal: Vec<crate::query::select::TraversalSeg>,
    pub distinct: Option<Distinct>,
}

/// Group-by specification
#[derive(Debug, Clone, PartialEq)]
pub enum GroupBySpec {
    /// Group by a simple column name (no table prefix)
    Column(&'static str),
    /// Group by a raw expression
    Expr(String),
}

/// An `INSERT` statement.
#[derive(Debug, Clone, PartialEq)]
pub struct Insert {
    pub table: &'static str,
    pub values: Value,
    /// Optional (source_field_name, target_field_name) when inserting an Edge type with
    /// custom endpoint field identifiers. Backends like SurrealDB can use this to
    /// synthesize their required internal relation endpoint keys (`in` / `out`).
    pub endpoint_fields: Option<(&'static str, &'static str)>,
}

/// An `UPDATE` statement.
#[derive(Debug, Clone, PartialEq)]
pub struct Update {
    pub table: &'static str,
    pub merge: Option<Value>,
    pub filter: Option<PredExpr>,
}

/// A `DELETE` statement.
#[derive(Debug, Clone, PartialEq)]
pub struct Delete {
    pub table: &'static str,
    pub filter: Option<PredExpr>,
}

/// Predicate expression for statements after lowering from the high-level `Expr`.
#[derive(Debug, Clone, PartialEq)]
pub enum PredExpr {
    True,
    Binary {
        col: &'static str,
        op: &'static str,
        param: Param,
    },
    And(Box<PredExpr>, Box<PredExpr>),
    Or(Box<PredExpr>, Box<PredExpr>),
    Not(Box<PredExpr>),
}

/// Parameter reference. Either inline or named.
#[derive(Debug, Clone, PartialEq)]
pub enum Param {
    Inline(Value),
    Named(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ordering {
    Asc(&'static str),
    Desc(&'static str),
}
