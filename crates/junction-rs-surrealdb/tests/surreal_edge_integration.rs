use junction_rs_core::prelude::*;
use junction_rs_macros::*;
use junction_rs_surrealdb::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
#[junction(table = "user")]
struct User {
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Edge)]
#[junction(table = "follows")]
struct Follows {
    #[junction(source)]
    r#in: SimpleId<User>,
    #[junction(target)]
    r#out: SimpleId<User>,
    since: u32,
}

async fn setup_adapter() -> SurrealDbAdapter {
    use surrealdb::engine::any::connect;
    let db = connect("mem://").await.unwrap();
    db.use_ns("test").use_db("test").await.unwrap();
    SurrealDbAdapter::new(db)
}

#[tokio::test(flavor = "current_thread")]
async fn insert_and_query_edges() {
    let db = setup_adapter().await;

    // Seed users
    let _ = insert(vec![
        User {
            name: "Alice".into(),
        },
        User { name: "Bob".into() },
    ])
    .into()
    .return_many::<_>(db.clone())
    .await
    .unwrap();

    // Insert edges
    let edges = vec![
        Follows {
            r#in: User::create_simple_id(),
            r#out: User::create_simple_id(),
            since: 2020,
        },
        Follows {
            r#in: User::create_simple_id(),
            r#out: User::create_simple_id(),
            since: 2021,
        },
    ];
    let created = insert_into(<Follows as Edge>::table(), edges.clone())
        .into()
        .return_many::<_>(db.clone())
        .await
        .unwrap();
    assert_eq!(created.len(), 2);

    // Select edge with filter and limit
    let s = Follows::schema();
    let rows: Vec<Follows> = select::<Follows>(All)
        .from(Follows::table())
        .where_(expr!(s.since > json!(2020)))
        .limit(1)
        .return_many(db.clone())
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].since > 2020);
}
