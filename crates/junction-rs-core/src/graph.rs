//! Graph traversal intermediate AST and type-safe builder (initial skeleton).
//!
//! This module defines backend-agnostic structures describing a graph traversal
//! separate from the existing relational query AST. Adapters can opt-in to
//! support it by translating [`GraphTraversal`] into their native graph query
//! representation (e.g. SurrealQL arrow paths, Cypher patterns, recursive CTEs).

use crate::{
    id::SimpleId,
    traits::{DbError, DbExecutor, Edge, Node},
    Expr,
};
use std::marker::PhantomData;

/// Direction of an edge traversal relative to the current frontier node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Out,
    In,
}

/// A single edge reference (table + direction).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeRef {
    pub table: &'static str,
    pub direction: Direction,
}

/// Set of edges (used for union traversal). Initially constrained to same From/To.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeSet {
    pub edges: Vec<EdgeRef>,
}

impl EdgeSet {
    pub fn single(edge: EdgeRef) -> Self {
        Self { edges: vec![edge] }
    }
}

/// Trait implemented for tuples of edges to represent a union traversal (all edges share From/To types).
pub trait EdgeUnion<From: Node, To: Node> {
    fn to_edges() -> Vec<EdgeRef>;
}

impl<From: Node, To: Node, E1> EdgeUnion<From, To> for (E1,)
where
    E1: Edge<From = From, To = To>,
    To: Node,
{
    fn to_edges() -> Vec<EdgeRef> {
        vec![EdgeRef {
            table: E1::TABLE,
            direction: Direction::Out,
        }]
    }
}

impl<From: Node, To: Node, E1, E2> EdgeUnion<From, To> for (E1, E2)
where
    E1: Edge<From = From, To = To>,
    E2: Edge<From = From, To = To>,
    To: Node,
{
    fn to_edges() -> Vec<EdgeRef> {
        vec![
            EdgeRef {
                table: E1::TABLE,
                direction: Direction::Out,
            },
            EdgeRef {
                table: E2::TABLE,
                direction: Direction::Out,
            },
        ]
    }
}

impl<From: Node, To: Node, E1, E2, E3> EdgeUnion<From, To> for (E1, E2, E3)
where
    E1: Edge<From = From, To = To>,
    E2: Edge<From = From, To = To>,
    E3: Edge<From = From, To = To>,
    To: Node,
{
    fn to_edges() -> Vec<EdgeRef> {
        vec![
            EdgeRef {
                table: E1::TABLE,
                direction: Direction::Out,
            },
            EdgeRef {
                table: E2::TABLE,
                direction: Direction::Out,
            },
            EdgeRef {
                table: E3::TABLE,
                direction: Direction::Out,
            },
        ]
    }
}

impl<From: Node, To: Node, E1, E2, E3, E4> EdgeUnion<From, To> for (E1, E2, E3, E4)
where
    E1: Edge<From = From, To = To>,
    E2: Edge<From = From, To = To>,
    E3: Edge<From = From, To = To>,
    E4: Edge<From = From, To = To>,
    To: Node,
{
    fn to_edges() -> Vec<EdgeRef> {
        vec![
            EdgeRef {
                table: E1::TABLE,
                direction: Direction::Out,
            },
            EdgeRef {
                table: E2::TABLE,
                direction: Direction::Out,
            },
            EdgeRef {
                table: E3::TABLE,
                direction: Direction::Out,
            },
            EdgeRef {
                table: E4::TABLE,
                direction: Direction::Out,
            },
        ]
    }
}

impl<From: Node, To: Node, E1, E2, E3, E4, E5> EdgeUnion<From, To> for (E1, E2, E3, E4, E5)
where
    E1: Edge<From = From, To = To>,
    E2: Edge<From = From, To = To>,
    E3: Edge<From = From, To = To>,
    E4: Edge<From = From, To = To>,
    E5: Edge<From = From, To = To>,
    To: Node,
{
    fn to_edges() -> Vec<EdgeRef> {
        vec![
            EdgeRef {
                table: E1::TABLE,
                direction: Direction::Out,
            },
            EdgeRef {
                table: E2::TABLE,
                direction: Direction::Out,
            },
            EdgeRef {
                table: E3::TABLE,
                direction: Direction::Out,
            },
            EdgeRef {
                table: E4::TABLE,
                direction: Direction::Out,
            },
            EdgeRef {
                table: E5::TABLE,
                direction: Direction::Out,
            },
        ]
    }
}

/// Filter applied at a step (distinguish whether it targets edge or node).
#[derive(Debug, Clone, PartialEq)]
pub enum StepFilter {
    Node(Expr),
    Edge(Expr),
}

/// A single traversal step (no repetition) from current node frontier across one edge set.
#[derive(Debug, Clone, PartialEq)]
pub struct TraversalStep {
    pub edge_set: EdgeSet,
    pub filters: Vec<StepFilter>,
    pub node_alias: Option<String>,
    /// Destination node table for this step (E::To for out, E::From for in_). Stored so
    /// adapters can emit intermediate node hops when the backend requires explicit node table
    /// segments (e.g. person->knows->person->follows->person) instead of collapsing edge chains.
    pub dest_node_table: Option<&'static str>,
    /// Optional variable-length repeat extension: number of additional optional occurrences beyond those materialized.
    pub repeat_extra: Option<usize>,
}

impl TraversalStep {
    pub fn new(edge_set: EdgeSet, dest_node_table: Option<&'static str>) -> Self {
        Self {
            edge_set,
            filters: vec![],
            node_alias: None,
            dest_node_table,
            repeat_extra: None,
        }
    }
}

/// Start anchor (table + optional id literal) where traversal originates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartAnchor {
    pub table: &'static str,
    pub id: Option<String>,
}

/// Return mode for a graph traversal query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnMode {
    Nodes,
    Project(&'static str),
    /// Project a concrete list of fields (columns) from the terminal node.
    ProjectFields(Vec<&'static str>),
    /// Return edge documents of the final traversal step instead of terminal nodes.
    Edges,
    /// Return full path node id sequence(s) (experimental). Adapter chooses shape; core represents intent only.
    Path,
}

/// Placeholder structure for future `ReturnMode::Path` results.
///
/// Planned shape: nodes in traversal order plus optional edge identifiers.
/// Not currently produced by any adapter; using `return_path()` will yield an adapter error until implemented.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PathSegment {
    /// Ordered node record ids ("table:uuid")
    pub nodes: Vec<String>,
    /// Optional edge record ids parallel to node transitions (len = nodes.len() - 1)
    pub edges: Option<Vec<String>>,
}

/// High-level traversal AST.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphTraversal {
    pub start: StartAnchor,
    pub steps: Vec<TraversalStep>,
    pub return_mode: ReturnMode,
}

impl GraphTraversal {
    pub fn new(start: StartAnchor) -> Self {
        Self {
            start,
            steps: vec![],
            return_mode: ReturnMode::Nodes,
        }
    }
}

/// Type-safe path builder capturing Root and Current node phantom types.
pub struct Path<Root: Node, Cur: Node> {
    ast: GraphTraversal,
    _p: PhantomData<(Root, Cur)>,
}

/// Begin traversal from all records of a node type.
pub fn start<N: Node>() -> Path<N, N> {
    Path {
        ast: GraphTraversal::new(StartAnchor {
            table: N::TABLE,
            id: None,
        }),
        _p: PhantomData,
    }
}

/// Begin traversal from a specific record id.
pub fn start_at<N: Node>(id: SimpleId<N>) -> Path<N, N> {
    // Store full table-qualified id (e.g. person:uuid) to avoid later escaping that
    // can introduce angle bracket wrappers and mismatch Surreal's native id form.
    let full = format!("{}:{}", N::TABLE, id.as_uuid_str());
    Path {
        ast: GraphTraversal::new(StartAnchor {
            table: N::TABLE,
            id: Some(full),
        }),
        _p: PhantomData,
    }
}

impl<R: Node, C: Node> Path<R, C> {
    /// Traverse forward along an edge whose `From` matches the current node type.
    pub fn forward<E: Edge<From = C>>(mut self) -> Path<R, E::To>
    where
        E::To: Node,
    {
        let step = TraversalStep::new(
            EdgeSet::single(EdgeRef {
                table: E::TABLE,
                direction: Direction::Out,
            }),
            Some(<E::To as Node>::TABLE),
        );
        self.ast.steps.push(step);
        Path {
            ast: self.ast,
            _p: PhantomData,
        }
    }

    /// Traverse backward (reverse) along an edge whose `To` matches the current node type.
    pub fn backward<E: Edge<To = C>>(mut self) -> Path<R, E::From>
    where
        E::From: Node,
    {
        let step = TraversalStep::new(
            EdgeSet::single(EdgeRef {
                table: E::TABLE,
                direction: Direction::In,
            }),
            Some(<E::From as Node>::TABLE),
        );
        self.ast.steps.push(step);
        Path {
            ast: self.ast,
            _p: PhantomData,
        }
    }

    /// Repeat the previous outward step `min..=max` times (basic homogenous repetition). For now we duplicate steps in AST.
    pub fn repeat(mut self, min: usize, _max: usize) -> Self {
        // Current simplified semantics: ensure the last step occurs at least `min` times.
        // We DO NOT yet model variable length (max) – that will require backend specific
        // pattern expansion (e.g. SurrealQL range repetition or recursive query). For now
        // we only materialize (min-1) additional copies (the original step already exists).
        let max = _max;
        let extra = if max > min { Some(max - min) } else { None };
        if self.ast.steps.is_empty() {
            return self;
        }
        if min <= 1 {
            return self;
        }
        let last = self.ast.steps.last().cloned();
        if let Some(step) = last {
            for _ in 1..min {
                self.ast.steps.push(step.clone());
            }
        }
        if let Some(extra_count) = extra {
            if let Some(last_step) = self.ast.steps.last_mut() {
                last_step.repeat_extra = Some(extra_count);
            }
        }
        self
    }

    /// Traverse forward across a union of multiple edge tables sharing the same From/To.
    pub fn out_union<U, T>(mut self) -> Path<R, T>
    where
        C: Node,
        T: Node,
        U: EdgeUnion<C, T>,
    {
        let edges = U::to_edges();
        let step = TraversalStep::new(EdgeSet { edges }, Some(<T as Node>::TABLE));
        self.ast.steps.push(step);
        Path {
            ast: self.ast,
            _p: PhantomData,
        }
    }

    /// Alias the current frontier node (for adapter-specific naming / debugging).
    pub fn alias(mut self, alias: &str) -> Self {
        if let Some(last) = self.ast.steps.last_mut() {
            last.node_alias = Some(alias.to_string());
        }
        self
    }

    /// Apply a node-level filter on current frontier.
    pub fn filter_node(mut self, f: Expr) -> Self {
        if let Some(last) = self.ast.steps.last_mut() {
            last.filters.push(StepFilter::Node(f));
        }
        self
    }

    /// Apply an edge-level filter on the most recently traversed edge.
    pub fn filter_edge(mut self, f: Expr) -> Self {
        if let Some(last) = self.ast.steps.last_mut() {
            last.filters.push(StepFilter::Edge(f));
        }
        self
    }

    /// Switch to projection return mode, specifying a synthetic alias key used by adapter.
    pub fn project<T: serde::de::DeserializeOwned + 'static>(mut self) -> GraphQuery<T> {
        // Use type name as projection key marker.
        self.ast.return_mode = ReturnMode::Project(std::any::type_name::<T>());
        GraphQuery {
            ast: self.ast,
            _p: PhantomData,
        }
    }

    /// Explicitly project a list of node fields (columns) at terminal nodes.
    pub fn project_fields<T: serde::de::DeserializeOwned + 'static>(
        mut self,
        fields: &[&'static str],
    ) -> GraphQuery<T> {
        self.ast.return_mode = ReturnMode::ProjectFields(fields.to_vec());
        GraphQuery {
            ast: self.ast,
            _p: PhantomData,
        }
    }

    /// Return edge documents of the final step instead of terminal nodes.
    pub fn return_edges<E: Edge<From = C> + serde::de::DeserializeOwned + 'static>(
        mut self,
    ) -> GraphQuery<E>
    where
        E::To: Node,
    {
        // Set mode to Edges. Compiler will truncate final node hop.
        self.ast.return_mode = ReturnMode::Edges;
        GraphQuery {
            ast: self.ast,
            _p: PhantomData,
        }
    }

    /// Return path sequences as structured `PathSegment` values.
    ///
    /// SurrealDB adapter behaviour today:
    /// * Single-hop traversals return `nodes` (start + end) and a single edge id.
    /// * Multi-hop traversals are supported when you call `start_at(id)` and only use outward
    ///   edges without unions or edge-level filters. Results include the full node sequence plus
    ///   parallel edge ids (`edges.len() == nodes.len() - 1`).
    /// * `.repeat(min, max)` enumerates tail variants up to a safety cap (10 extra occurrences).
    ///
    /// Roadmap items include union support, edge-level filters for multi-hop path capture, and
    /// richer projection helpers for including edge properties alongside ids.
    pub fn return_path(mut self) -> GraphQuery<PathSegment> {
        self.ast.return_mode = ReturnMode::Path;
        GraphQuery {
            ast: self.ast,
            _p: PhantomData,
        }
    }

    /// Finish building and return the raw traversal AST.
    pub fn into_traversal(self) -> GraphTraversal {
        self.ast
    }

    /// Convenience: directly execute expecting `Vec<C>` nodes.
    pub async fn return_many<E>(self, exec: E) -> Result<Vec<C>, DbError>
    where
        E: DbExecutor,
        C: serde::de::DeserializeOwned + Send + 'static,
    {
        GraphQuery::<C> {
            ast: self.ast,
            _p: PhantomData,
        }
        .return_many(exec)
        .await
    }
}

/// Executable graph query wrapper.
pub struct GraphQuery<T> {
    pub(crate) ast: GraphTraversal,
    _p: PhantomData<T>,
}

impl<T: serde::de::DeserializeOwned + Send + 'static> GraphQuery<T> {
    pub fn into_query(self) -> crate::query::ast::Query {
        crate::query::ast::Query::new(crate::query::ast::Stmt::Graph(self.ast))
    }
    /// Borrowing conversion used for debugging / compilation without consuming the query.
    pub fn as_query_ref(&self) -> crate::query::ast::Query {
        crate::query::ast::Query::new(crate::query::ast::Stmt::Graph(self.ast.clone()))
    }
    pub async fn return_many<E: DbExecutor>(self, exec: E) -> Result<Vec<T>, DbError> {
        exec.execute(self.into_query()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct Person {
        id: crate::id::SimpleId<Person>,
    }
    impl crate::traits::HasTableName for Person {
        fn table_name() -> &'static str {
            "person"
        }
    }
    impl crate::traits::Node for Person {
        const TABLE: &'static str = "person";
        type Schema = ();
        fn schema() -> Self::Schema {
            ()
        }
    }

    #[derive(Serialize, Deserialize)]
    struct Knows {
        id: crate::id::SimpleId<Knows>,
        r#in: crate::id::SimpleId<Person>,
        r#out: crate::id::SimpleId<Person>,
    }
    impl crate::traits::HasTableName for Knows {
        fn table_name() -> &'static str {
            "knows"
        }
    }

    impl crate::traits::Edge for Knows {
        const TABLE: &'static str = "knows";
        type Schema = ();
        type From = Person;
        type To = Person;
        fn schema() -> Self::Schema {
            ()
        }
    }

    #[test]
    fn build_simple_forward_traversal() {
        let p = start::<Person>().forward::<Knows>().into_traversal();
        assert_eq!(p.steps.len(), 1);
        assert_eq!(p.start.table, "person");
        assert_eq!(p.steps[0].edge_set.edges[0].table, "knows");
    }
}
