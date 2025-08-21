use junction_rs::prelude::*;
use junction_rs::SimpleId;
use junction_rs_surrealdb::SurrealDbAdapter;
use serde::{Deserialize, Serialize};

#[derive(Node, Serialize, Deserialize, Debug, Clone)]
#[junction(table = "person")]
struct Person {
    pub id: SimpleId<Person>,
    pub name: String,
}

#[derive(Edge, Serialize, Deserialize, Debug, Clone)]
#[junction(table = "knows")]
struct Knows {
    pub id: SimpleId<Knows>,
    #[junction(source)]
    pub r#in: SimpleId<Person>,
    #[junction(target)]
    pub r#out: SimpleId<Person>,
    pub strength: i32,
}

async fn setup() -> SurrealDbAdapter {
    let db = surrealdb::engine::any::connect("mem://").await.unwrap();
    db.use_ns("test").use_db("test").await.unwrap();
    db.query("DEFINE TABLE person SCHEMALESS; DEFINE TABLE knows SCHEMALESS;")
        .await
        .unwrap();
    let adapter = SurrealDbAdapter::new(db);
    let p1 = Person {
        id: SimpleId::default(),
        name: "Alice".into(),
    };
    let p2 = Person {
        id: SimpleId::default(),
        name: "Bob".into(),
    };
    let mut inserted: Vec<Person> = junction_rs::insert(vec![p1, p2])
        .return_many(adapter.clone())
        .await
        .unwrap();
    inserted.sort_by(|a, b| a.name.cmp(&b.name));
    let p1 = inserted.iter().find(|p| p.name == "Alice").unwrap().clone();
    let p2 = inserted.iter().find(|p| p.name == "Bob").unwrap().clone();
    junction_rs_surrealdb::relate_edges(
        &adapter,
        vec![Knows {
            id: SimpleId::new(),
            r#out: p1.id.clone(),
            r#in: p2.id.clone(),
            strength: 5,
        }],
    )
    .await
    .unwrap();
    // Debug: fetch inserted edge to inspect orientation
    if let Ok(val) = adapter.raw_query("SELECT * FROM knows;").await {
        let _ = val;
    }
    adapter
}

#[tokio::test]
async fn path_two_nodes() {
    let adapter = setup().await;
    // Fetch inserted people to get stable ids again (already have via insert but demonstrates pattern)
    let people: Vec<Person> = junction_rs::select::<Person>(junction_rs::All)
        .from(Person::table())
        .return_many(adapter.clone())
        .await
        .unwrap();
    assert_eq!(people.len(), 2);
    // Ordering from SELECT * may not be deterministic; choose Alice explicitly as source (edge orientation 'in' = source)
    let start = people
        .iter()
        .find(|p| p.name == "Alice")
        .expect("Alice inserted");
    // (debug edge selection removed – edge id format differs from SimpleId expectations)
    // Build path query: start_at + one outward step + return_path (supported narrow case)
    let query = junction_rs::graph::start_at::<Person>(start.id.clone())
        .forward::<Knows>()
        .return_path();
    // Compile should now succeed (single-hop supported)
    let compiled_res = junction_rs_surrealdb::compiler::compile_to_surql(&query.as_query_ref());
    if let Err(e) = &compiled_res {
        panic!("Compilation failed: {}", e);
    }
    let compiled = compiled_res.unwrap();
    eprintln!("Compiled SQL: {}", compiled.sql);
    // Execute and retrieve path arrays
    let rows: Vec<junction_rs::graph::PathSegment> =
        query.return_many(adapter.clone()).await.unwrap();
    assert_eq!(
        rows.len(),
        1,
        "expected single path row for single edge traversal"
    );
    let path = &rows[0];
    assert_eq!(
        path.nodes.len(),
        2,
        "path should contain two node ids (start, end)"
    );
    let edges = path
        .edges
        .as_ref()
        .expect("edges should be captured for single-hop path");
    assert_eq!(
        edges.len(),
        1,
        "single-hop path should have exactly one edge id"
    );
    assert!(!edges[0].is_empty(), "edge id should be non-empty");
}
