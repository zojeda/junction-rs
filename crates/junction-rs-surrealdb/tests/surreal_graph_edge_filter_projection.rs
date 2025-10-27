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
async fn edge_filter_and_projection_fields() {
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
    // Manually create edge with strength using SurrealQL RELATE (source = a, target = b)
    let (a_id, b_id) = (a.id.as_uuid_str(), b.id.as_uuid_str());
    let sql = format!(
        "LET $a = type::thing(\"person\", \"{}\"); LET $b = type::thing(\"person\", \"{}\"); RELATE $a->knows->$b SET strength = 5;",
        a_id, b_id
    );
    db.raw_query(&sql).await.unwrap();

    let s = Person::schema();
    let e = Knows::schema();
    // Apply edge filter (strength > 3) and project only name
    // Build graph query separately to allow debugging of compiled SQL
    let gq = junction_rs::start_at::<Person>(a.id.clone())
        .forward::<Knows>()
        .filter_edge(expr!(e.strength > 3))
        .filter_node(expr!(s.age > 10))
        .project_fields::<MinimalPerson>(&["name"]);

    let results: Vec<MinimalPerson> = gq.return_many(db.clone()).await.unwrap_or_else(|e| {
        panic!("graph exec failed: {:?}", e);
    });

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Bob");
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MinimalPerson {
    name: String,
}

// (No extra helpers required.)
