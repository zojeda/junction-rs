use junction_rs_core::prelude::*;
use junction_rs_macros::*;
use junction_rs_surrealdb::{relate_edges, SurrealDbAdapter};
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

async fn setup() -> SurrealDbAdapter {
    let db = surrealdb::engine::any::connect("mem://").await.unwrap();
    db.use_ns("relate_edges_smoke")
        .use_db("test")
        .await
        .unwrap();
    SurrealDbAdapter::new(db)
}

#[tokio::test(flavor = "current_thread")]
async fn relate_edges_creates_edge() {
    let db = setup().await;
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
            strength: 9,
        }],
    )
    .await
    .unwrap();
    let row = db
        .debug_select_json("SELECT * FROM knows LIMIT 1")
        .await
        .unwrap();
    let text = row.to_string();
    assert!(
        text.contains(&a.id.as_uuid_str()),
        "expected edge to store source id"
    );
    assert!(
        text.contains(&b.id.as_uuid_str()),
        "expected edge to store destination id"
    );
}
