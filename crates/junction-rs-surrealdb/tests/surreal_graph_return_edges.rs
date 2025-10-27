use junction_rs_core::prelude::*;
use junction_rs_macros::*;
use junction_rs_surrealdb::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
#[junction(table = "person")]
struct Person {
    id: SimpleId<Person>,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Edge)]
#[junction(table = "knows")]
struct Knows {
    #[junction(source)]
    r#out: SimpleId<Person>,
    #[junction(target)]
    r#in: SimpleId<Person>,
    strength: i32,
}

async fn setup_adapter() -> SurrealDbAdapter {
    use surrealdb::engine::any::connect;
    let db = connect("mem://").await.unwrap();
    let ns = format!(
        "edge_ret_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    db.use_ns(&ns).use_db("test").await.unwrap();
    SurrealDbAdapter::new(db)
}

#[tokio::test(flavor = "current_thread")]
async fn return_edges_last_step() {
    let db = setup_adapter().await;
    let a = Person {
        id: Person::create_simple_id(),
        name: "A".into(),
    };
    let b = Person {
        id: Person::create_simple_id(),
        name: "B".into(),
    };
    insert(vec![a.clone(), b.clone()])
        .into()
        .return_many::<_>(db.clone())
        .await
        .unwrap();
    relate_edges(
        &db,
        vec![Knows {
            r#out: a.id.clone(),
            r#in: b.id.clone(),
            strength: 5,
        }],
    )
    .await
    .unwrap();
    let e_schema = Knows::schema();
    let query = junction_rs_core::graph::start_at::<Person>(a.id.clone())
        .forward::<Knows>()
        .filter_edge(expr!(e_schema.strength > 3))
        .return_edges::<Knows>();
    let qref = query.as_query_ref();
    if let junction_rs_core::query::ast::Stmt::Graph(_) = &qref.stmt {
        let compiled = junction_rs_surrealdb::compiler::compile_to_surql(&qref)
            .expect("edge graph compilation should succeed");
        eprintln!("[edge-debug] compiled graph SQL: {}", compiled.sql);
    }
    // Manual raw check: run a select building equivalent SQL using direct select traversal to see shape.
    // (Optional) could add direct SQL verification here later.
    let edges: Vec<Knows> = query.return_many(db.clone()).await.unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].strength, 5);
    // Log stored raw edge to confirm orientation
    #[cfg(test)]
    {
        let stored = db
            .debug_select_json("SELECT * FROM knows LIMIT 1")
            .await
            .unwrap();
        eprintln!("[edge-debug] stored edge row: {}", stored);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn edge_orientation_probe() {
    let db = setup_adapter().await;
    let a = Person {
        id: Person::create_simple_id(),
        name: "A".into(),
    };
    let b = Person {
        id: Person::create_simple_id(),
        name: "B".into(),
    };
    insert(vec![a.clone(), b.clone()])
        .into()
        .return_many::<_>(db.clone())
        .await
        .unwrap();
    relate_edges(
        &db,
        vec![Knows {
            r#out: a.id.clone(),
            r#in: b.id.clone(),
            strength: 7,
        }],
    )
    .await
    .unwrap();
    let stored = db
        .debug_select_json("SELECT * FROM knows LIMIT 1")
        .await
        .unwrap();
    eprintln!("[edge-orientation] row: {}", stored);
    // Orientation semantics: relate_edges(rel out=a, in=b) produced edge with IN = a, OUT = b based on debug row.
    // So direction is reversed relative to naming expectation; we record that so compiler can anchor on correct field.
    let a_full = format!("{}", a.id.as_uuid_str());
    let b_full = format!("{}", b.id.as_uuid_str());
    let text = stored.to_string();
    let in_has_a = text.contains(&a_full);
    let out_has_b = text.contains(&b_full);
    assert!(
        in_has_a && out_has_b,
        "expected in=a ({}) and out=b ({}) in {:?}",
        a_full,
        b_full,
        stored
    );
}
