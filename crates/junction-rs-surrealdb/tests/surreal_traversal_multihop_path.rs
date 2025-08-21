use junction_rs::prelude::*;
use junction_rs_surrealdb::edge_utils::relate_edges;

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

#[tokio::test]
async fn multihop_path_two_edges() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_adapter().await;

    // Insert three people
    let a = Person {
        id: SimpleId::default(),
        name: "Alice".into(),
        age: 30,
    };
    let b = Person {
        id: SimpleId::default(),
        name: "Bob".into(),
        age: 32,
    };
    let c = Person {
        id: SimpleId::default(),
        name: "Carol".into(),
        age: 34,
    };
    insert(vec![a.clone(), b.clone(), c.clone()])
        .return_many(db.clone())
        .await?;

    // Create edges A->B, B->C
    // Create edges using helper – provide actual edge structs so required fields exist
    relate_edges::<Knows>(
        &db,
        vec![
            Knows {
                id: SimpleId::default(),
                r#out: a.id.clone(),
                r#in: b.id.clone(),
            },
            Knows {
                id: SimpleId::default(),
                r#out: b.id.clone(),
                r#in: c.id.clone(),
            },
        ],
    )
    .await?;

    // Build multi-hop path query A -> B -> C
    let path: Vec<PathSegment> = start_at(a.id.clone())
        .forward::<Knows>()
        .forward::<Knows>()
        .return_path()
        .return_many(db.clone())
        .await?;

    assert_eq!(path.len(), 1, "expected one endpoint path row");
    let seg = &path[0];
    // Phase C: full node sequence (start, intermediate, terminal) and edges captured
    assert_eq!(
        seg.nodes.len(),
        3,
        "phase C multi-hop returns full node sequence"
    );
    let edges = seg.edges.as_ref().expect("edges list expected in phase C");
    assert_eq!(
        edges.len(),
        seg.nodes.len() - 1,
        "edges len should be nodes.len()-1"
    );
    Ok(())
}
