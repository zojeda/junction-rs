use junction_rs_core::prelude::*;
use junction_rs_macros::*;
use junction_rs_surrealdb::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
#[junction(table = "cg_person")]
struct PersonCg {
    id: SimpleId<PersonCg>,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Edge)]
#[junction(table = "cg_knows")]
struct KnowsCg {
    #[junction(source)]
    src: SimpleId<PersonCg>,
    #[junction(target)]
    dst: SimpleId<PersonCg>,
    strength: i32,
}

async fn setup_adapter() -> SurrealDbAdapter {
    use surrealdb::engine::any::connect;
    let db = connect("mem://").await.unwrap();
    db.use_ns("test").use_db("test").await.unwrap();
    SurrealDbAdapter::new(db)
}

#[tokio::test(flavor = "current_thread")]
async fn traversal_with_custom_endpoint_names() {
    let db = setup_adapter().await;
    let a = PersonCg {
        id: PersonCg::create_simple_id(),
        name: "Alice".into(),
    };
    let b = PersonCg {
        id: PersonCg::create_simple_id(),
        name: "Bob".into(),
    };
    let c = PersonCg {
        id: PersonCg::create_simple_id(),
        name: "Carol".into(),
    };
    let _ = insert(vec![a.clone(), b.clone(), c.clone()])
        .into()
        .return_many::<_>(db.clone())
        .await
        .unwrap();

    // Insert edges using new helper; ensures src/dst remapped to out/in
    let _ = insert(vec![
        KnowsCg {
            src: a.id.clone(),
            dst: b.id.clone(),
            strength: 10,
        },
        KnowsCg {
            src: b.id.clone(),
            dst: c.id.clone(),
            strength: 20,
        },
    ])
    .return_many::<_>(db.clone())
    .await
    .unwrap();

    // Traverse a -> b -> (edge) -> c (two hops) expecting terminal nodes with Carol included.
    let rows: Vec<PersonCg> = select::<PersonCg>(All)
        .record_id(&a.id)
        .forward::<KnowsCg, PersonCg>()
        .forward::<KnowsCg, PersonCg>()
        .distinct()
        .return_many(db.clone())
        .await
        .unwrap();
    assert!(rows.iter().any(|p| p.name == "Carol"));
}
