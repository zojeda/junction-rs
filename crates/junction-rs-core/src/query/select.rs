use serde::de::DeserializeOwned;

use super::ast;
use super::ast::{Distinct, GroupBySpec, SelectField};
use super::ast::{Param, PredExpr};
use crate::expr::{All, Expr, Ordering};
use crate::schema::Col;
use crate::schema::IntoTableName;
use crate::traits::DbExecutor;

pub struct SelectBuilder<T> {
    table: Option<&'static str>,
    fields: Vec<SelectField>,
    filter: Option<Expr>,
    ordering: Vec<Ordering>,
    limit: Option<u64>,
    group_by: Vec<GroupBySpec>,
    /// Optional id for selecting from a specific record `table:id`.
    record_id: Option<String>,
    /// Traversal segments to append after the base FROM target.
    traversal: Vec<TraversalSeg>,
    distinct: Option<Distinct>,
    _p: std::marker::PhantomData<T>,
}

/// Trait allowing projection-related methods (`columns`, `column`, `group_by`, `distinct_on`,
/// `count_col_as`) to accept either a raw `&'static str` or a typed schema column `Col<T>`.
/// This keeps the selection DSL symmetric with expression building (`expr!(schema.age > 10)`),
/// letting the same schema handles be reused for projection without spelling field names twice.
///
/// Users typically won't implement this manually; use generated schema columns via
/// `MyModel::schema().field` or raw strings for ad-hoc projections.
pub trait IntoSelectColumn {
    fn name(&self) -> &'static str;
}

impl IntoSelectColumn for &'static str {
    fn name(&self) -> &'static str {
        self
    }
}

impl<T> IntoSelectColumn for Col<T> {
    fn name(&self) -> &'static str {
        self.name
    }
}

/// Direction of a traversal relative to the starting node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraversalDir {
    Out,
    In,
}

/// A traversal segment (edge table + direction + optional target node table for type guidance).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraversalSeg {
    pub edge: &'static str,
    pub dir: TraversalDir,
    pub node: Option<&'static str>,
}

pub fn select<T>(_all: All) -> SelectBuilder<T> {
    SelectBuilder {
        table: None,
        // Implicit default projection: star (cleared when explicit columns/expr added).
        fields: vec![SelectField::Star],
        filter: None,
        ordering: vec![],
        limit: None,
        group_by: vec![],
        record_id: None,
        traversal: vec![],
        distinct: None,
        _p: Default::default(),
    }
}

/// Edge-aware select convenience. Provides same builder as `select` but will inject
/// opaque metadata binds so backend adapters (e.g. Surreal) can restore custom endpoint
/// field names for edge types with semantic source/target identifiers.
pub fn select_edge<E>(_all: All) -> EdgeSelectBuilder<E>
where
    E: crate::traits::Edge,
{
    EdgeSelectBuilder {
        inner: select::<E>(_all),
        _p: std::marker::PhantomData,
    }
}

pub struct EdgeSelectBuilder<E: crate::traits::Edge> {
    inner: SelectBuilder<E>,
    _p: std::marker::PhantomData<E>,
}

impl<E: crate::traits::Edge> EdgeSelectBuilder<E> {
    // Mirror builder methods by forwarding; only implement the subset we actually use in tests/examples.
    pub fn from(mut self, table: impl IntoTableName) -> Self {
        self.inner = self.inner.from(table);
        self
    }
    pub fn where_(mut self, expr: Expr) -> Self {
        self.inner = self.inner.where_(expr);
        self
    }
    pub fn order_by(mut self, ord: Ordering) -> Self {
        self.inner = self.inner.order_by(ord);
        self
    }
    pub fn limit(mut self, n: u64) -> Self {
        self.inner = self.inner.limit(n);
        self
    }
    pub fn columns<C: IntoSelectColumn>(mut self, cols: &[C]) -> Self {
        self.inner = self.inner.columns(cols);
        self
    }
    pub fn star(mut self) -> Self {
        self.inner = self.inner.star();
        self
    }
    pub fn record_id(mut self, id: impl Into<String>) -> Self {
        self.inner = self.inner.record_id(id);
        self
    }
    pub fn forward<EE: crate::traits::Edge, N: crate::traits::Node>(mut self) -> Self {
        self.inner = self.inner.forward::<EE, N>();
        self
    }
    pub fn backward<EE: crate::traits::Edge, N: crate::traits::Node>(mut self) -> Self {
        self.inner = self.inner.backward::<EE, N>();
        self
    }
    pub fn project_path(mut self) -> Self {
        self.inner = self.inner.project_path();
        self
    }
    pub async fn return_many<DB>(self, db: DB) -> Result<Vec<E>, crate::traits::DbError>
    where
        DB: DbExecutor,
        E: serde::de::DeserializeOwned + Send + 'static,
    {
        // Build query, then tack on metadata binds for adapter.
        let table = self
            .inner
            .table
            .expect("select_edge: .from(...) must be provided");
        let filter = self.inner.filter.map(expr_to_pred);
        let order_by: Vec<ast::Ordering> = self
            .inner
            .ordering
            .into_iter()
            .map(|o| match o {
                Ordering::Asc(_, c) => ast::Ordering::Asc(c),
                Ordering::Desc(_, c) => ast::Ordering::Desc(c),
            })
            .collect();
        let mut q = ast::Query::new(ast::Stmt::Select(ast::Select {
            table,
            record_id: self.inner.record_id,
            fields: self.inner.fields,
            filter,
            order_by,
            limit: self.inner.limit,
            group_by: self.inner.group_by,
            traversal: self.inner.traversal,
            distinct: self.inner.distinct,
        }));
        q = q
            .bind(
                "__jxn_edge_src",
                serde_json::Value::String(E::SOURCE_FIELD_NAME.to_string()),
            )
            .bind(
                "__jxn_edge_tgt",
                serde_json::Value::String(E::TARGET_FIELD_NAME.to_string()),
            );
        db.execute::<E>(q).await
    }
}

impl<T> SelectBuilder<T> {
    /// Internal helper trait so projection & grouping methods can take either a raw
    /// column `&'static str` or a generated schema `Col<T>` maintaining parity with
    /// expression construction which already uses `Col<T>`.
    /// This is intentionally not public API for now – users interact via the blanket impls.
    fn col_name<C: IntoSelectColumn + ?Sized>(c: &C) -> &'static str {
        c.name()
    }

    pub fn from(mut self, table: impl IntoTableName) -> Self {
        self.table = Some(table.table_name());
        self
    }
    /// Target a specific record id (appends as `table:id`). Caller must ensure id validity for backend.
    pub fn record_id(mut self, id: impl Into<String>) -> Self {
        self.record_id = Some(id.into());
        self
    }
    /// Select a single row by a typed `SimpleId<T>`. Sets the table, internal record id and a LIMIT 1.
    /// Keeps existing projection (defaults to `*`). Additional filters/order/columns after this call are allowed
    /// but typically unnecessary. Intended for ergonomic single-entity retrieval.
    pub fn by_id(mut self, id: crate::id::SimpleId<T>) -> Self
    where
        T: crate::traits::Node,
    {
        self.table = Some(T::TABLE);
        // Use raw UUID part only; backend compiler prepends table name if needed.
        self.record_id = Some(id.as_uuid_str().to_string());
        // Defensive limit to ensure adapter cannot return multiple rows.
        self.limit = Some(1);
        self
    }
    /// Traverse forward along an edge table ( `->edge->` ).
    pub fn forward<E: crate::traits::Edge, N: crate::traits::Node>(mut self) -> Self {
        self.traversal.push(TraversalSeg {
            edge: E::TABLE,
            dir: TraversalDir::Out,
            node: Some(N::TABLE),
        });
        self
    }
    /// Traverse backward along an edge table ( `<-edge-` ). Node generic supplies compile-time guidance but only table name used for now.
    pub fn backward<E: crate::traits::Edge, N: crate::traits::Node>(mut self) -> Self {
        self.traversal.push(TraversalSeg {
            edge: E::TABLE,
            dir: TraversalDir::In,
            node: Some(N::TABLE),
        });
        self
    }
    /// Low-level traversal segment allowing explicit direction and no node type.
    pub fn traverse_edge(mut self, edge: &'static str, dir: TraversalDir) -> Self {
        self.traversal.push(TraversalSeg {
            edge,
            dir,
            node: None,
        });
        self
    }
    /// Project the currently accumulated traversal chain as a raw expression like
    /// `->purchased->product<-purchased<-person->purchased->product` so that the
    /// generated query matches SurrealQL forms such as:
    /// `SELECT ->purchased->product FROM person:tom;`
    /// After calling this the internal traversal list is cleared so it is not
    /// also appended to the FROM target.
    pub fn project_path(mut self) -> Self {
        if !self.traversal.is_empty() {
            let mut expr = String::new();
            for seg in &self.traversal {
                match seg.dir {
                    TraversalDir::Out => {
                        expr.push_str(&format!("->{}", seg.edge));
                        if let Some(node) = seg.node {
                            expr.push_str(&format!("->{}", node));
                        }
                    }
                    TraversalDir::In => {
                        expr.push_str(&format!("<-{}", seg.edge));
                        if let Some(node) = seg.node {
                            expr.push_str(&format!("<-{}", node));
                        }
                    }
                }
            }
            self.fields.push(SelectField::Expr { expr, alias: None });
            self.traversal.clear();
        }
        self
    }
    /// Project all columns (default)
    pub fn star(mut self) -> Self {
        // Explicit star request appends only if not already sole implicit projection.
        if !(self.fields.len() == 1 && matches!(self.fields[0], SelectField::Star)) {
            self.fields.push(SelectField::Star);
        }
        self
    }
    /// Clear implicit initial star so explicit projection fully replaces it.
    fn clear_implicit_star(&mut self) {
        if self.fields.len() == 1 && matches!(self.fields[0], SelectField::Star) {
            self.fields.clear();
        }
    }
    /// Project a sequence of simple columns provided either as raw `&'static str`
    /// names or typed schema columns (`Col<T>`). This keeps the DSL consistent so
    /// the same schema values used in `expr!(...)` can be reused here.
    pub fn columns<C: IntoSelectColumn>(mut self, cols: &[C]) -> Self {
        self.clear_implicit_star();
        for col in cols {
            self.fields.push(SelectField::Column(Self::col_name(col)));
        }
        self
    }
    /// Alias for `columns` to mirror graph projection naming.
    pub fn project_fields<C: IntoSelectColumn>(self, cols: &[C]) -> Self {
        self.columns(cols)
    }
    /// Project a simple column by name or schema column reference.
    pub fn column<C: IntoSelectColumn>(mut self, col: C) -> Self {
        self.clear_implicit_star();
        self.fields.push(SelectField::Column(Self::col_name(&col)));
        self
    }
    /// Project a raw expression with optional alias
    pub fn expr(mut self, expr: impl Into<String>) -> Self {
        self.clear_implicit_star();
        self.fields.push(SelectField::Expr {
            expr: expr.into(),
            alias: None,
        });
        self
    }
    /// Project a raw expression aliased as the provided name
    pub fn expr_as(mut self, expr: impl Into<String>, alias: &'static str) -> Self {
        self.clear_implicit_star();
        self.fields.push(SelectField::Expr {
            expr: expr.into(),
            alias: Some(alias),
        });
        self
    }
    /// Project count(*) as alias
    pub fn count_all_as(mut self, alias: &'static str) -> Self {
        self.clear_implicit_star();
        self.fields.push(SelectField::CountAll { alias });
        self
    }
    /// Project count(col) as alias
    pub fn count_col_as<C: IntoSelectColumn>(mut self, col: C, alias: &'static str) -> Self {
        self.clear_implicit_star();
        self.fields.push(SelectField::CountCol {
            col: Self::col_name(&col),
            alias,
        });
        self
    }
    pub fn where_(mut self, expr: Expr) -> Self {
        self.filter = Some(expr);
        self
    }
    pub fn order_by(mut self, ord: Ordering) -> Self {
        self.ordering.push(ord);
        self
    }
    pub fn limit(mut self, n: u64) -> Self {
        self.limit = Some(n);
        self
    }
    pub fn group_by<C: IntoSelectColumn>(mut self, col: C) -> Self {
        self.group_by
            .push(GroupBySpec::Column(Self::col_name(&col)));
        self
    }
    /// Group by a raw expression
    pub fn group_by_expr(mut self, expr: impl Into<String>) -> Self {
        self.group_by.push(GroupBySpec::Expr(expr.into()));
        self
    }

    /// Request row-level DISTINCT across the projected columns.
    pub fn distinct(mut self) -> Self {
        if self.distinct.is_some() {
            panic!("select(): distinct already specified");
        }
        self.distinct = Some(Distinct::All);
        self
    }

    /// Request DISTINCT on a subset of columns. Currently requires all projected fields to be
    /// simple columns matching the provided order.
    pub fn distinct_on<C: IntoSelectColumn>(mut self, cols: &[C]) -> Self {
        if self.distinct.is_some() {
            panic!("select(): distinct already specified");
        }
        if cols.is_empty() {
            panic!("select(): distinct_on requires at least one column");
        }
        // Collect &'static str names (schema columns carry &'static str for field names)
        let mut names: Vec<&'static str> = Vec::with_capacity(cols.len());
        for c in cols {
            names.push(Self::col_name(c));
        }
        self.distinct = Some(Distinct::On(names));
        self
    }

    pub async fn return_many<DB>(self, db: DB) -> Result<Vec<T>, crate::traits::DbError>
    where
        DB: DbExecutor,
        T: DeserializeOwned + Send,
    {
        let table = self.table.expect("select: .from(...) must be provided");
        // Map old Expr to PredExpr AST minimally
        let filter = self.filter.map(expr_to_pred);
        let order_by: Vec<ast::Ordering> = self
            .ordering
            .into_iter()
            .map(|o| match o {
                Ordering::Asc(_, c) => ast::Ordering::Asc(c),
                Ordering::Desc(_, c) => ast::Ordering::Desc(c),
            })
            .collect();
        // If T implements Insertable we can attempt to capture edge endpoint metadata (returns None for Nodes)
        let q = ast::Query::new(ast::Stmt::Select(ast::Select {
            table,
            record_id: self.record_id,
            fields: self.fields,
            filter,
            order_by,
            limit: self.limit,
            group_by: self.group_by,
            traversal: self.traversal,
            distinct: self.distinct,
        }));
        db.execute::<T>(q).await
    }

    /// Execute and return at most one row as `Option<T>`.
    /// Returns `Ok(None)` when no row matches; does NOT error on absence.
    /// If more than one row is returned (should not happen with `by_id`), the first is taken.
    pub async fn return_one<DB>(self, db: DB) -> Result<Option<T>, crate::traits::DbError>
    where
        DB: DbExecutor,
        T: DeserializeOwned + Send,
    {
        let rows = self.return_many(db).await?;
        Ok(rows.into_iter().next())
    }
}

fn expr_to_pred(e: Expr) -> PredExpr {
    use crate::expr::ExprKind as K;
    match e.0 {
        K::True => PredExpr::True,
        K::Binary {
            col,
            table: _,
            op,
            val,
            ..
        } => PredExpr::Binary {
            col,
            op,
            param: Param::Inline(val),
        },
        K::And(a, b) => PredExpr::And(Box::new(expr_to_pred(*a)), Box::new(expr_to_pred(*b))),
        K::Or(a, b) => PredExpr::Or(Box::new(expr_to_pred(*a)), Box::new(expr_to_pred(*b))),
        K::Not(a) => PredExpr::Not(Box::new(expr_to_pred(*a))),
    }
}
