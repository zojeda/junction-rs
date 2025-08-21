use junction_rs::prelude::*;
use junction_rs_surrealdb::compiler::compile_to_surql;

#[derive(Node, serde::Serialize, serde::Deserialize, Debug, Clone)]
#[junction(table = "person")]
struct Person {
    id: SimpleId<Person>,
    name: String,
}

#[derive(Edge, serde::Serialize, serde::Deserialize, Debug, Clone)]
#[junction(table = "knows")]
struct Knows {
    id: SimpleId<Knows>,
    #[junction(source)]
    r#in: SimpleId<Person>,
    #[junction(target)]
    r#out: SimpleId<Person>,
}

#[derive(Edge, serde::Serialize, serde::Deserialize, Debug, Clone)]
#[junction(table = "follows")]
struct Follows {
    id: SimpleId<Follows>,
    #[junction(source)]
    r#in: SimpleId<Person>,
    #[junction(target)]
    r#out: SimpleId<Person>,
}

#[test]
fn multihop_union_edge_error() {
    // Build two-step traversal with UNION (two edges in first step) which should error in phase A path mode.
    use junction_rs::graph::{
        Direction, EdgeRef, EdgeSet, GraphTraversal, ReturnMode, StartAnchor, TraversalStep,
    };
    let start_id = SimpleId::<Person>::new();
    let mut gt = GraphTraversal {
        start: StartAnchor {
            table: Person::TABLE,
            id: Some(format!("person:{}", start_id.as_uuid_str())),
        },
        steps: vec![],
        return_mode: ReturnMode::Path,
    };
    // First step has two edges (violates constraint)
    gt.steps.push(TraversalStep::new(
        EdgeSet {
            edges: vec![
                EdgeRef {
                    table: Knows::TABLE,
                    direction: Direction::Out,
                },
                EdgeRef {
                    table: Follows::TABLE,
                    direction: Direction::Out,
                },
            ],
        },
        Some(Person::TABLE),
    ));
    // Second step single edge
    gt.steps.push(TraversalStep::new(
        EdgeSet::single(EdgeRef {
            table: Knows::TABLE,
            direction: Direction::Out,
        }),
        Some(Person::TABLE),
    ));
    let q = junction_rs::query::ast::Query::new(junction_rs::query::ast::Stmt::Graph(gt));
    let res = compile_to_surql(&q);
    assert!(res.is_err(), "expected union edge constraint error");
    let msg = format!("{}", res.err().unwrap());
    assert!(
        msg.contains("requires exactly one edge per step"),
        "unexpected error message: {}",
        msg
    );
}

#[test]
fn multihop_inward_edge_error() {
    use junction_rs::graph::{
        Direction, EdgeRef, EdgeSet, GraphTraversal, ReturnMode, StartAnchor, TraversalStep,
    };
    let start_id = SimpleId::<Person>::new();
    let mut gt = GraphTraversal {
        start: StartAnchor {
            table: Person::TABLE,
            id: Some(format!("person:{}", start_id.as_uuid_str())),
        },
        steps: vec![],
        return_mode: ReturnMode::Path,
    };
    gt.steps.push(TraversalStep::new(
        EdgeSet::single(EdgeRef {
            table: Knows::TABLE,
            direction: Direction::Out,
        }),
        Some(Person::TABLE),
    ));
    gt.steps.push(TraversalStep::new(
        EdgeSet::single(EdgeRef {
            table: Knows::TABLE,
            direction: Direction::In,
        }),
        Some(Person::TABLE),
    )); // inward disallowed
    let q = junction_rs::query::ast::Query::new(junction_rs::query::ast::Stmt::Graph(gt));
    let res = compile_to_surql(&q);
    assert!(res.is_err(), "expected inward edge constraint error");
    let msg = format!("{}", res.err().unwrap());
    assert!(
        msg.contains("supports only outward edges"),
        "unexpected error message: {}",
        msg
    );
}

#[test]
fn multihop_edge_filter_error() {
    use junction_rs::graph::{
        Direction, EdgeRef, EdgeSet, GraphTraversal, ReturnMode, StartAnchor, StepFilter,
        TraversalStep,
    };
    let start_id = SimpleId::<Person>::new();
    let mut gt = GraphTraversal {
        start: StartAnchor {
            table: Person::TABLE,
            id: Some(format!("person:{}", start_id.as_uuid_str())),
        },
        steps: vec![],
        return_mode: ReturnMode::Path,
    };
    let mut first = TraversalStep::new(
        EdgeSet::single(EdgeRef {
            table: Knows::TABLE,
            direction: Direction::Out,
        }),
        Some(Person::TABLE),
    );
    // Add edge-level filter (disallowed). Use a trivial TRUE expression placeholder.
    first.filters.push(StepFilter::Edge(junction_rs::expr::Expr(
        junction_rs::expr::ExprKind::True,
    )));
    gt.steps.push(first);
    gt.steps.push(TraversalStep::new(
        EdgeSet::single(EdgeRef {
            table: Knows::TABLE,
            direction: Direction::Out,
        }),
        Some(Person::TABLE),
    ));
    let q = junction_rs::query::ast::Query::new(junction_rs::query::ast::Stmt::Graph(gt));
    let res = compile_to_surql(&q);
    assert!(res.is_err(), "expected edge filter constraint error");
    let msg = format!("{}", res.err().unwrap());
    assert!(
        msg.contains("does not yet support edge filters"),
        "unexpected error message: {}",
        msg
    );
}
