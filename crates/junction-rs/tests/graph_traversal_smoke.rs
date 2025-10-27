use junction_rs::prelude::*;
use junction_rs::{start, start_at, SimpleId};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Node)]
#[junction(table = "person")]
struct Person {
    id: SimpleId<Person>,
    name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Edge)]
#[junction(table = "knows")]
struct Knows {
    #[junction(source)]
    r#in: SimpleId<Person>,
    #[junction(target)]
    r#out: SimpleId<Person>,
    strength: f32,
}

#[test]
fn build_traversal_ast_forward_backward() {
    let alice = Person::create_simple_id();
    let t = start_at(alice.clone())
        .forward::<Knows>()
        .backward::<Knows>()
        .into_traversal();
    assert_eq!(t.steps.len(), 2);
    assert_eq!(t.start.table, Person::TABLE);
}

#[test]
fn build_traversal_from_all() {
    let t = start::<Person>().forward::<Knows>().into_traversal();
    assert_eq!(t.steps.len(), 1);
}

#[test]
fn compile_inbound_arrow_direction() {
    // Build a traversal person->knows->person<-knows<-person (out then in) and ensure
    // the intermediate inbound node hop uses <-person not ->person.
    let alice = Person::create_simple_id();
    let traversal = start_at(alice).forward::<Knows>().backward::<Knows>();
    // Convert to query and compile using Surreal adapter compiler directly.
    let q = traversal.into_traversal();
    let query =
        junction_rs_core::query::ast::Query::new(junction_rs_core::query::ast::Stmt::Graph(q));
    // Import compile_to_surql from the surrealdb adapter crate (re-export path not available in facade yet).
    use junction_rs_surrealdb::compiler::compile_to_surql;
    let compiled = compile_to_surql(&query).expect("compile");
    let sql = compiled.sql;
    // Expect pattern person:<id>->knows->person<-knows<-person
    // We don't know the concrete UUID, so replace it with a wildcard capture for assertion.
    assert!(
        sql.contains("->knows->person<-knows<-person"),
        "sql did not contain expected inbound pattern: {}",
        sql
    );
}
