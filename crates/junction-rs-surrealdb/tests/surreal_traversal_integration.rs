use junction_rs_core::prelude::*;
use junction_rs_macros::*;
use junction_rs_surrealdb::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
#[junction(table = "person")]
struct Person {
    id: SimpleId<Person>,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
#[junction(table = "product")]
struct Product {
    id: SimpleId<Product>,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Edge)]
#[junction(table = "purchased")]
struct Purchased {
    #[junction(source)]
    r#out: SimpleId<Person>,
    #[junction(target)]
    r#in: SimpleId<Product>,
    qty: u32,
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
async fn traverse_products_from_person_record() {
    let db = setup_adapter().await;

    let tom = Person {
        id: Person::create_simple_id(),
        name: "tom".into(),
    };
    let _ = insert(vec![tom.clone()])
        .into()
        .return_many::<_>(db.clone())
        .await
        .unwrap();
    let phone_seed = Product {
        id: Product::create_simple_id(),
        name: "Phone".into(),
    };
    let laptop_seed = Product {
        id: Product::create_simple_id(),
        name: "Laptop".into(),
    };
    let created_products: Vec<Product> = insert(vec![phone_seed.clone(), laptop_seed.clone()])
        .into()
        .return_many::<_>(db.clone())
        .await
        .unwrap();
    let phone = created_products
        .iter()
        .find(|p| p.name == "Phone")
        .unwrap()
        .clone();
    let laptop = created_products
        .iter()
        .find(|p| p.name == "Laptop")
        .unwrap()
        .clone();
    relate_edges(
        &db,
        vec![
            Purchased {
                r#out: tom.id.clone(),
                r#in: phone.id.clone(),
                qty: 1,
            },
            Purchased {
                r#out: tom.id.clone(),
                r#in: laptop.id.clone(),
                qty: 2,
            },
        ],
    )
    .await
    .unwrap();

    let rows: Vec<Product> = select::<Product>(All)
        .record_id(&tom.id)
        .forward::<Purchased, Product>()
        .return_many(db.clone())
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn multi_hop_recommendation_like_traversal() {
    let db = setup_adapter().await;

    let tom = Person {
        id: Person::create_simple_id(),
        name: "tom".into(),
    };
    let bob = Person {
        id: Person::create_simple_id(),
        name: "bob".into(),
    };
    let _ = insert(vec![tom.clone(), bob.clone()])
        .into()
        .return_many::<_>(db.clone())
        .await
        .unwrap();
    let a_seed = Product {
        id: Product::create_simple_id(),
        name: "A".into(),
    };
    let b_seed = Product {
        id: Product::create_simple_id(),
        name: "B".into(),
    };
    let created_products: Vec<Product> = insert(vec![a_seed.clone(), b_seed.clone()])
        .into()
        .return_many::<_>(db.clone())
        .await
        .unwrap();
    let a = created_products
        .iter()
        .find(|p| p.name == "A")
        .unwrap()
        .clone();
    let b = created_products
        .iter()
        .find(|p| p.name == "B")
        .unwrap()
        .clone();
    // Edges: tom->A, bob->A, bob->B
    relate_edges(
        &db,
        vec![
            Purchased {
                r#out: tom.id.clone(),
                r#in: a.id.clone(),
                qty: 1,
            },
            Purchased {
                r#out: bob.id.clone(),
                r#in: a.id.clone(),
                qty: 1,
            },
            Purchased {
                r#out: bob.id.clone(),
                r#in: b.id.clone(),
                qty: 1,
            },
        ],
    )
    .await
    .unwrap();

    let rows: Vec<Product> = select::<Product>(All)
        .record_id(&tom.id)
        .forward::<Purchased, Product>()
        .backward::<Purchased, Person>()
        .forward::<Purchased, Product>()
        .return_many(db.clone())
        .await
        .unwrap();
    // Should include at least product B (recommended) and maybe others.
    assert!(rows.iter().any(|p| p.name == "B"));
}
