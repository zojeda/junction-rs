use junction_rs_core::prelude::*;
use junction_rs_macros::Node;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
#[junction(table = "person")]
struct Person {
    id: SimpleId<Person>,
    name: String,
}

async fn setup() -> junction_rs_surrealdb::SurrealDbAdapter {
    use surrealdb::engine::any::connect;
    let db = connect("mem://").await.unwrap();
    db.use_ns("test").use_db("test").await.unwrap();
    junction_rs_surrealdb::SurrealDbAdapter::new(db)
}

#[tokio::test(flavor = "current_thread")]
async fn inspect_ids() {
    let db = setup().await;
    let expected_id = Person::create_simple_id();
    let p = Person {
        id: expected_id.clone(),
        name: "tom".into(),
    };
    let _ = insert(vec![p.clone()])
        .into()
        .return_many::<_>(db.clone())
        .await
        .unwrap();
    // Fetch all persons raw via surreal client direct query to see shape
    use serde_json::Value;
    use surrealdb::opt::Resource;
    let vals = db
        .client_ref()
        .select(Resource::from("person"))
        .await
        .unwrap();
    let vals = serde_json::to_value(&vals).unwrap();
    let v = if let Some(Value::Array(v)) = vals.get("Array") {
        v
    } else {
        panic!("Expected array of person rows");
    };
    let saved_id = v[0]["Object"]["id"]["Thing"]["id"]["String"]
        .as_str()
        .unwrap();
    assert_eq!(saved_id, expected_id.to_string());

    // assert!(!vals.is_empty());
}
