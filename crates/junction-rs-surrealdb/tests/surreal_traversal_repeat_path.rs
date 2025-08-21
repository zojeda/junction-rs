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

// Build linear chain length = 5 nodes (4 edges) so repeat variants have enough room.
async fn seed(db: &junction_rs_surrealdb::SurrealDbAdapter) -> Vec<Person> {
    let mut people = Vec::new();
    for i in 0..5 {
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
async fn repeat_tail_min2_max4_variants() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_adapter().await;
    let people = seed(&db).await; // P0 -> P1 -> P2 -> P3 -> P4
    let start = people[0].id.clone();
    // Base pattern: two explicit .forward::<Knows>() steps, then we mark second repeated with repeat(2,4)
    // Approach: construct with one step, then .repeat to enforce min copies (builder duplicates), then rely on extra for enumeration.
    let path: Vec<PathSegment> = start_at(start)
        .forward::<Knows>() // first hop (mandatory)
        .forward::<Knows>() // second hop (basis for repetition)
        .repeat(2, 4) // require at least 2 occurrences total of the last hop (already 1 present) and allow up to +2 more (so 2..=4 occurrences of that hop)
        .return_path()
        .return_many(db.clone())
        .await?;
    // Expected variants (occurrences of last hop): 2,3,4 -> path hop lengths: first hop + those => total hops: 1 + (2..=4)
    // So node counts: (1+2)+1 = 4 nodes, (1+3)+1 = 5 nodes, (1+4)+1 = 6 nodes (but we seeded only 5 nodes, so final variant capped by data availability => expect first two variants)
    // Because we only have P0..P4 (5 nodes) max hops realistic = 4 -> nodes=5 so 6-node variant can't materialize.
    assert!(
        path.len() >= 2,
        "should have at least two realized variants"
    );
    // Ensure ordering ascending by nodes length
    let lens: Vec<usize> = path.iter().map(|s| s.nodes.len()).collect();
    let mut sorted = lens.clone();
    sorted.sort();
    assert_eq!(lens, sorted, "variants should be in ascending length order");
    for seg in &path {
        let edges = seg.edges.as_ref().expect("edges present");
        assert_eq!(edges.len(), seg.nodes.len() - 1, "invariant");
    }
    Ok(())
}
