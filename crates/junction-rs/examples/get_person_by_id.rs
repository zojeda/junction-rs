mod person; // reuse shared person definition & adapter helper like other examples.
use junction_rs::prelude::*;
use person::{setup_adapter, Person};

#[tokio::main]
async fn main() -> Result<(), DbError> {
    let db = setup_adapter().await;

    // Insert a person.
    let p = Person { id: Person::create_simple_id(), name: "Ada".into(), age: 42, marketing: true };
    insert(vec![p.clone()])
        .into()
        .return_many(db.clone())
        .await?;

    // Fetch person by typed id.
    let pid = p.id.clone();
    if let Some(found) = get_by_id::<Person, _>(pid.clone(), db.clone()).await? {
        println!("Found person: {} (age {})", found.name, found.age);
    } else {
        println!("Person not found");
    }

    // Projection variant: only name using builder + record_id (projection struct isn't a Node).
    #[derive(Deserialize)]
    struct PersonName { name: String }
    if let Some(name_only) = select::<PersonName>(All)
        .record_id(&pid)
        .return_one(db.clone())
        .await? {
        println!("Name-only projection: {}", name_only.name);
    }
    Ok(())
}
