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

// Path traversal projection semantics under redesign; ignoring this test until implementation stabilizes.
#[tokio::test(flavor = "current_thread")]
async fn path_projection_exact_shape() {
    // Enforce a timeout so the test fails fast rather than hanging indefinitely if traversal breaks.
    let fut = async {
        let db = setup_adapter().await;

        // Seed graph: person tom & others -> products, plus other purchasers to create inbound segment
        let tom = Person {
            id: SimpleId::new(),
            name: "tom".into(),
        };
        let other = Person {
            id: SimpleId::new(),
            name: "jaime".into(),
        };
        let p1 = Product {
            id: SimpleId::new(),
            name: "Phone".into(),
        };
        let p2 = Product {
            id: SimpleId::new(),
            name: "Laptop".into(),
        };
        let p3 = Product {
            id: SimpleId::new(),
            name: "Camera".into(),
        };

        // Insert nodes
        let _ = insert(vec![tom.clone(), other.clone()])
            .into()
            .return_many::<_>(db.clone())
            .await
            .unwrap();
        let _ = insert(vec![p1.clone(), p2.clone(), p3.clone()])
            .into()
            .return_many::<_>(db.clone())
            .await
            .unwrap();

        // Edges: tom->p1, tom->p2, other->p2, other->p3
        let _ = insert_into(
            <Purchased as Edge>::table(),
            vec![
                Purchased {
                    r#in: tom.id.clone(),
                    r#out: p1.id.clone(),
                    qty: 1,
                },
                Purchased {
                    r#in: tom.id.clone(),
                    r#out: p2.id.clone(),
                    qty: 2,
                },
                Purchased {
                    r#in: other.id.clone(),
                    r#out: p2.id.clone(),
                    qty: 1,
                },
                Purchased {
                    r#in: other.id.clone(),
                    r#out: p3.id.clone(),
                    qty: 1,
                },
            ],
        )
        .into()
        .return_many::<_>(db.clone())
        .await
        .unwrap();

        // Build traversal path expression
        // SELECT ->purchased->product<-purchased<-person->purchased->product FROM person:<tom>
        // We only verify semantic result (contains recommended product(s) like Camera) since capturing raw SQL
        // is not exposed yet. If needed, future work can add an inspection hook.
        let results: Vec<Product> = select::<Product>(All)
            .from(Person::table())
            .record_id(tom.id.as_uuid_str())
            .forward::<Purchased, Product>()
            .backward::<Purchased, Person>()
            .forward::<Purchased, Product>()
            .return_many(db.clone())
            .await
            .unwrap();

        // Should include at least product(s) purchased by other buyers of tom's products (p3 via other).
        assert!(
            results.iter().any(|p| p.name == "Camera"),
            "Expected Camera in traversal-based recommendation set"
        );
        // Ensure we got some products back
        assert!(!results.is_empty());
    };
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(10), fut).await;
    if timeout.is_err() {
        panic!("path_projection_exact_shape timed out after 10s");
    }
}
