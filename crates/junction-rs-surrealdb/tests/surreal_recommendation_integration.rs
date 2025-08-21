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
    marketing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
#[junction(table = "product")]
struct Product {
    id: SimpleId<Product>,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Edge)]
#[junction(table = "bought")]
struct Bought {
    #[junction(source)]
    r#in: SimpleId<Person>,
    #[junction(target)]
    r#out: SimpleId<Product>,
    qty: u32,
}

async fn setup_adapter() -> SurrealDbAdapter {
    use surrealdb::engine::any::connect;
    let db = connect("mem://").await.unwrap();
    db.use_ns("test").use_db("test").await.unwrap();
    SurrealDbAdapter::new(db)
}

#[tokio::test(flavor = "current_thread")]
async fn recommend_high_score_selection() {
    let db = setup_adapter().await;

    let alice = Person {
        id: SimpleId::new(),
        name: "Alice".into(),
        age: 30,
        marketing: true,
    };
    let bob = Person {
        id: SimpleId::new(),
        name: "Bob".into(),
        age: 28,
        marketing: false,
    };
    // Seed nodes
    let _ = insert(vec![alice.clone(), bob.clone()])
        .into()
        .return_many::<_>(db.clone())
        .await
        .unwrap();

    let laptop = Product {
        id: SimpleId::new(),
        name: "Laptop".into(),
    };
    let phone = Product {
        id: SimpleId::new(),
        name: "Phone".into(),
    };
    let _ = insert(vec![laptop.clone(), phone.clone()])
        .into()
        .return_many::<_>(db.clone())
        .await
        .unwrap();

    // Insert some edges
    let _ = insert_into(
        <Bought as Edge>::table(),
        vec![
            Bought {
                r#in: alice.id,
                r#out: laptop.id,
                qty: 1,
            },
            Bought {
                r#in: bob.id,
                r#out: phone.id,
                qty: 2,
            },
        ],
    )
    .into()
    .return_many::<_>(db.clone())
    .await
    .unwrap();
}
