mod person;

use junction_rs::prelude::*; // façade prelude for convenience
use person::{setup_adapter, Person};

#[tokio::main]
async fn main() -> Result<(), DbError> {
    let db = setup_adapter().await;

    // Insert a person.
    let p = Person { id: Person::create_simple_id(), name: "Ada".into(), age: 42, marketing: true };
    let pid = p.id.clone(); // clone id to reuse without moving
    insert(vec![p.clone()])
        .into()
        .return_many(db.clone())
        .await?;

    // Fetch person by typed id.
    if let Some(found) = get_by_id::<Person, _>(pid.clone(), db.clone()).await? {
        println!("Found person: {} (age {})", found.name, found.age);
    } else {
        println!("Person not found");
    }

    // Projection variant: only name using record_id + projection struct.
    #[derive(serde::Deserialize)]
    struct PersonName { name: String }
    if let Some(name_only) = select::<PersonName>(All)
        .record_id(&pid) // sets table + id
        .return_one(db.clone())
        .await? {
        println!("Name-only projection: {}", name_only.name);
    }

    Ok(())
}
