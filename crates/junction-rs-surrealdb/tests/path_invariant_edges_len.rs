use junction_rs::prelude::*;

mod common;
use common::setup_adapter;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Node)]
#[junction(table = "person")]
struct Person {
    id: SimpleId<Person>,
    name: String,
    age: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Edge)]
#[junction(table = "knows")]
struct Knows {
    id: SimpleId<Knows>,
    #[junction(source)]
    r#out: SimpleId<Person>,
    #[junction(target)]
    r#in: SimpleId<Person>,
}

// Helper to create a linear chain of N persons connected by Knows edges.
async fn make_chain(db: &junction_rs_surrealdb::SurrealDbAdapter, count: usize) -> Vec<Person> {
    assert!(count >= 2);
    let mut people = Vec::new();
    for i in 0..count {
        people.push(Person {
            id: SimpleId::default(),
            name: format!("P{i}"),
            age: 20 + i as i32,
        });
    }
    insert(people.clone())
        .return_many(db.clone())
        .await
        .unwrap();
    // Create edges in sequence
    let mut edges = Vec::new();
    for w in people.windows(2) {
        edges.push(Knows {
            id: SimpleId::default(),
            r#out: w[0].id.clone(),
            r#in: w[1].id.clone(),
        });
    }
    junction_rs_surrealdb::edge_utils::relate_edges::<Knows>(db, edges)
        .await
        .unwrap();
    people
}

#[tokio::test]
async fn path_edges_len_invariant_chain_of_four() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_adapter().await;
    let people = make_chain(&db, 4).await; // 4 nodes -> 3 edges
    let start_id = people[0].id.clone();
    let path: Vec<PathSegment> = start_at(start_id)
        .forward::<Knows>()
        .forward::<Knows>()
        .forward::<Knows>()
        .return_path()
        .return_many(db.clone())
        .await?;
    assert_eq!(path.len(), 1, "single simple linear path expected");
    let seg = &path[0];
    assert_eq!(seg.nodes.len(), 4, "should have 4 nodes in sequence");
    let edges = seg.edges.as_ref().expect("edges should be present phase C");
    assert_eq!(
        edges.len(),
        seg.nodes.len() - 1,
        "edges length invariant failed"
    );
    Ok(())
}
