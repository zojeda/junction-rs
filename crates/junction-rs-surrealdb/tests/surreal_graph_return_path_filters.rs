use junction_rs::prelude::*;
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
    // Align with other path tests: explicitly set namespace + database so subsequent queries see data.
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
    let p3 = Person {
        id: SimpleId::default(),
        name: "Charlie".into(),
    };
    let mut inserted: Vec<Person> = junction_rs::insert(vec![p1, p2, p3])
        .return_many(adapter.clone())
        .await
        .unwrap();
    inserted.sort_by(|a, b| a.name.cmp(&b.name));
    let alice = inserted.iter().find(|p| p.name == "Alice").unwrap().clone();
    let bob = inserted.iter().find(|p| p.name == "Bob").unwrap().clone();
    let charlie = inserted
        .iter()
        .find(|p| p.name == "Charlie")
        .unwrap()
        .clone();
    // Use type::thing form to avoid Surreal parse ambiguity with raw hyphenated UUID record ids in RELATE source/target
    let relate_ab = format!(
        "LET $_a = type::thing(\"person\", \"{}\"); LET $_b = type::thing(\"person\", \"{}\"); RELATE $_a->knows->$_b SET strength = 5;",
        alice.id.as_uuid_str(), bob.id.as_uuid_str());
    let relate_ac = format!(
        "LET $_a = type::thing(\"person\", \"{}\"); LET $_c = type::thing(\"person\", \"{}\"); RELATE $_a->knows->$_c SET strength = 1;",
        alice.id.as_uuid_str(), charlie.id.as_uuid_str());
    adapter.raw_query(&relate_ab).await.unwrap();
    adapter.raw_query(&relate_ac).await.unwrap();
    adapter
}

#[tokio::test]
async fn path_with_filters() {
    let adapter = setup().await;
    let people: Vec<Person> = junction_rs::select::<Person>(junction_rs::All)
        .from(Person::table())
        .return_many(adapter.clone())
        .await
        .unwrap();
    let alice = people.iter().find(|p| p.name == "Alice").unwrap();
    // Build path with edge filter (strength > 3) should include Alice->Bob but exclude Alice->Charlie
    let e = Knows::schema();
    let query = junction_rs::graph::start_at::<Person>(alice.id.clone())
        .forward::<Knows>()
        .filter_edge(expr!(e.strength > 3))
        .return_path();
    let rows: Vec<junction_rs::graph::PathSegment> =
        query.return_many(adapter.clone()).await.unwrap();
    assert_eq!(rows.len(), 1, "expected exactly one qualifying path");
    assert_eq!(rows[0].nodes.len(), 2);
    let edges = rows[0]
        .edges
        .as_ref()
        .expect("single-hop path now captures edge id vector");
    assert_eq!(edges.len(), 1, "expected one edge id captured");
    assert!(!edges[0].is_empty(), "edge id should be non-empty");
}
