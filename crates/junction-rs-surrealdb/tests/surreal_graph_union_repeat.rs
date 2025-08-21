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
}

#[derive(Debug, Clone, Serialize, Deserialize, Edge)]
#[junction(table = "follows")]
struct Follows {
    #[junction(source)]
    r#in: SimpleId<Person>,
    #[junction(target)]
    r#out: SimpleId<Person>,
}

async fn setup_adapter() -> SurrealDbAdapter {
    use surrealdb::engine::any::connect;
    let db = connect("mem://").await.unwrap();
    let ns_name = format!(
        "test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    db.use_ns(&ns_name).use_db("test").await.unwrap();
    SurrealDbAdapter::new(db)
}

#[tokio::test(flavor = "current_thread")]
async fn union_and_repeat_traversal_executes() {
    let db = setup_adapter().await;
    let a = Person {
        id: SimpleId::new(),
        name: "Alice".into(),
        age: 30,
    };
    let b = Person {
        id: SimpleId::new(),
        name: "Bob".into(),
        age: 31,
    };
    let c = Person {
        id: SimpleId::new(),
        name: "Carol".into(),
        age: 32,
    };
    let _ = insert(vec![a.clone(), b.clone(), c.clone()])
        .into()
        .return_many(db.clone())
        .await
        .unwrap();
    // Edge structs use in=destination, out=source consistent with Surreal's <from>-edge-><to> pattern (out -> in)
    relate_edges(
        &db,
        vec![Knows {
            r#in: b.id.clone(),
            r#out: a.id.clone(),
        }],
    )
    .await
    .unwrap(); // a -knows-> b (out=a, in=b)
    relate_edges(
        &db,
        vec![Follows {
            r#in: c.id.clone(),
            r#out: b.id.clone(),
        }],
    )
    .await
    .unwrap(); // b -follows-> c (out=b, in=c)

    // Traverse 2 hops using union edges and repeat duplication to reach c from a
    let s = Person::schema();
    let results: Vec<Person> = junction_rs_core::graph::start_at::<Person>(a.id.clone())
        .forward::<Knows>()
        .forward::<Follows>()
        .filter_node(expr!(s.age > 20))
        .return_many(db.clone())
        .await
        .unwrap();

    assert!(results.len() >= 1);
}
