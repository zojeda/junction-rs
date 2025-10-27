mod person;

use junction_rs::prelude::*; // façade prelude for convenience
use person::{setup_adapter, Person};

#[tokio::main]
async fn main() -> Result<(), DbError> {
    let db = setup_adapter().await;

    // Insert a person.
    let p = Person { id: Person::create_simple_id(), name: "Ada".into(), age: 42, marketing: true };
    let pid = p.id; // save before moves
    insert(vec![p.clone()])
        .into()
        .return_many(db.clone())
        .await?;

    // Fetch person by typed id.
    if let Some(found) = get_by_id::<Person, _>(pid, db.clone()).await? {
        println!("Found person: {} (age {})", found.name, found.age);
    } else {
        println!("Person not found");
    }

    // Projection variant: only name using record_id + projection struct.
    #[derive(serde::Deserialize)]
    struct PersonName { name: String }
    let person_id = pid.as_uuid_str().to_string();
    if let Some(name_only) = select::<PersonName>(All)
        .from(Person::table())
        .record_id(person_id)
        .return_one(db.clone())
        .await? {
        println!("Name-only projection: {}", name_only.name);
    }

    Ok(())
}
