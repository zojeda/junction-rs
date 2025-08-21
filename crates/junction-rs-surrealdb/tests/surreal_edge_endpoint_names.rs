use junction_rs_core::prelude::*;
use junction_rs_macros::*;
use junction_rs_surrealdb::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
#[junction(table = "person_edge_name_demo")] // distinct table to avoid collisions
struct PersonDemo {
    id: SimpleId<PersonDemo>,
    name: String,
}

// Edge with semantic endpoint field names instead of in/out
#[derive(Debug, Clone, Serialize, Deserialize, Edge)]
#[junction(table = "knows_demo")]
struct KnowsDemo {
    #[junction(source)]
    from_person: SimpleId<PersonDemo>,
    #[junction(target)]
    to_person: SimpleId<PersonDemo>,
    since: u32,
}

async fn setup_adapter() -> SurrealDbAdapter {
    use surrealdb::engine::any::connect;
    let db = connect("mem://").await.unwrap();
    db.use_ns("test").use_db("test").await.unwrap();
    SurrealDbAdapter::new(db)
}

#[tokio::test(flavor = "current_thread")]
async fn edge_endpoint_field_names_constants() {
    // Validate macro exposed constants
    assert_eq!(KnowsDemo::SOURCE_FIELD_NAME, "from_person");
    assert_eq!(KnowsDemo::TARGET_FIELD_NAME, "to_person");
    assert_eq!(
        KnowsDemo::endpoint_field_names(),
        ("from_person", "to_person")
    );
}

#[tokio::test(flavor = "current_thread")]
async fn insert_with_custom_endpoint_names() {
    let db = setup_adapter().await;

    // Seed two persons
    let p1 = PersonDemo {
        id: SimpleId::new(),
        name: "Alice".into(),
    };
    let p2 = PersonDemo {
        id: SimpleId::new(),
        name: "Bob".into(),
    };
    let _ = insert(vec![p1.clone(), p2.clone()])
        .into()
        .return_many::<_>(db.clone())
        .await
        .unwrap();

    // Insert edge using custom endpoint field names via unified insert API
    let edges = vec![KnowsDemo {
        from_person: p1.id.clone(),
        to_person: p2.id.clone(),
        since: 2024,
    }];
    let created: Vec<KnowsDemo> = insert(edges.clone())
        .return_many::<_>(db.clone())
        .await
        .unwrap();
    assert_eq!(created.len(), 1);

    // Simple select to ensure deserialization still works
    let rows: Vec<KnowsDemo> = select_edge::<KnowsDemo>(All)
        .from(KnowsDemo::table())
        .return_many(db.clone())
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].from_person.as_uuid_str().len(), 36);
}
