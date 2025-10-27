// This file previously asserted multi-hop path returned an error. After Phase A implementation
// it now simply ensures compilation of a two-hop path query succeeds (endpoints only) by building
// the AST and compiling to SurrealQL.
use junction_rs::prelude::*;

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

#[test]
fn multi_hop_path_compiles_phase_a() {
    use junction_rs::graph::{
        Direction, EdgeRef, EdgeSet, GraphTraversal, ReturnMode, StartAnchor, TraversalStep,
    };
    let start_id = Person::create_simple_id();
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
            direction: Direction::Out,
        }),
        Some(Person::TABLE),
    ));
    let q = junction_rs::query::ast::Query::new(junction_rs::query::ast::Stmt::Graph(gt));
    let compiled = junction_rs_surrealdb::compiler::compile_to_surql(&q)
        .expect("multi-hop path should compile in phase A");
    assert!(
        compiled.sql.contains("nodes"),
        "expected path projection in SQL"
    );
}
