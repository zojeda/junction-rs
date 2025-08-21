use junction_rs_core::prelude::*;
use junction_rs_macros::*;
use junction_rs_surrealdb::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize, Node, PartialEq)]
#[junction(table = "person")]
struct Person {
    name: String,
    age: i32,
    marketing: bool,
}

async fn setup_adapter() -> SurrealDbAdapter {
    use surrealdb::engine::any::connect;
    let db = connect("mem://").await.unwrap();
    db.use_ns("test").use_db("test").await.unwrap();
    SurrealDbAdapter::new(db)
}

#[tokio::test(flavor = "current_thread")]
async fn insert_and_select_many() {
    let db = setup_adapter().await;

    // Insert
    let people = vec![
        Person {
            name: "Tom".into(),
            age: 42,
            marketing: true,
        },
        Person {
            name: "Jaime".into(),
            age: 40,
            marketing: false,
        },
    ];
    let created = insert(people.clone())
        .into()
        .return_many::<_>(db.clone())
        .await
        .unwrap();
    assert_eq!(created.len(), 2);

    // Select all
    let selected: Vec<Person> = select::<Person>(All)
        .from(Person::table())
        .return_many(db.clone())
        .await
        .unwrap();
    assert_eq!(selected.len(), 2);

    // Select with filter + limit
    let s = Person::schema();
    let filtered: Vec<Person> = select::<Person>(All)
        .from(Person::table())
        .where_(expr!(s.age > json!(40)))
        .limit(1)
        .return_many(db.clone())
        .await
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert!(filtered[0].age > 40);
}

#[tokio::test(flavor = "current_thread")]
async fn update_with_filter_and_return_after() {
    let db = setup_adapter().await;

    // Seed
    let _ = insert(vec![
        Person {
            name: "A".into(),
            age: 20,
            marketing: true,
        },
        Person {
            name: "B".into(),
            age: 30,
            marketing: true,
        },
    ])
    .into()
    .return_many::<_>(db.clone())
    .await
    .unwrap();

    let s = Person::schema();
    // Update marketing=false where age >= 25
    let updated: Vec<Person> = update(Person::table())
        .content(json!({"marketing": false}))
        .where_(expr!(s.age >= json!(25)))
        .return_many::<_, Person>(db.clone())
        .await
        .unwrap();
    // Only the 30-year-old should be updated
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].name, "B");
    assert!(!updated[0].marketing);
}

#[tokio::test(flavor = "current_thread")]
async fn delete_with_filter() {
    let db = setup_adapter().await;

    // Seed
    let _ = insert(vec![
        Person {
            name: "C".into(),
            age: 10,
            marketing: false,
        },
        Person {
            name: "D".into(),
            age: 50,
            marketing: true,
        },
    ])
    .into()
    .return_many::<_>(db.clone())
    .await
    .unwrap();

    let s = Person::schema();
    // Delete where age < 20
    delete(Person::table())
        .where_(expr!(s.age < json!(20)))
        .run(db.clone())
        .await
        .unwrap();

    // Ensure only the 50-year-old remains
    let remaining: Vec<Person> = select::<Person>(All)
        .from(Person::table())
        .return_many(db.clone())
        .await
        .unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].name, "D");
}
