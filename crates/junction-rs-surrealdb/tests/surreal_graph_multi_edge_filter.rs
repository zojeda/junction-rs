use junction_rs_core::prelude::*;
use junction_rs_macros::*;
use junction_rs_surrealdb::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
#[junction(table = "person")]
struct Person {
    id: SimpleId<Person>,
    name: String,
    age: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Edge)]
#[junction(table = "knows")]
struct Knows {
    #[junction(source)]
    r#in: SimpleId<Person>,
    #[junction(target)]
    r#out: SimpleId<Person>,
    strength: i32,
    since: i32,
}

async fn setup_adapter() -> SurrealDbAdapter {
    use surrealdb::engine::any::connect;
    let db = connect("mem://").await.unwrap();
    let ns = format!(
        "test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    db.use_ns(&ns).use_db("test").await.unwrap();
    SurrealDbAdapter::new(db)
}

#[tokio::test(flavor = "current_thread")]
async fn multi_edge_filters_and_projection() {
    let db = setup_adapter().await;
    let a = Person {
        id: Person::create_simple_id(),
        name: "Alice".into(),
        age: 40,
    };
    let b = Person {
        id: Person::create_simple_id(),
        name: "Bob".into(),
        age: 35,
    };
    insert(vec![a.clone(), b.clone()])
        .into()
        .return_many(db.clone())
        .await
        .unwrap();

    // Create edge with multiple properties
    let sql = format!(
        "LET $a = type::thing(\"person\", \"{}\"); LET $b = type::thing(\"person\", \"{}\"); RELATE $a->knows->$b SET strength = 7, since = 2020;",
        a.id.as_uuid_str(), b.id.as_uuid_str()
    );
    db.raw_query(&sql).await.unwrap();

    #[derive(Deserialize)]
    struct Name {
        name: String,
    }
    let s = Person::schema();
    let e = Knows::schema();
    let results: Vec<Name> = junction_rs_core::graph::start_at::<Person>(a.id.clone())
        .forward::<Knows>()
        .filter_edge(expr!(e.strength > 5))
        .filter_edge(expr!(e.since >= 2020))
        .filter_node(expr!(s.age > 25))
        .project_fields::<Name>(&["name"])
        .return_many(db.clone())
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Bob");
}
