mod person;

use junction_rs::prelude::*;
use person::{setup_adapter, Person};

#[tokio::main]
async fn main() -> Result<(), DbError> {
    let db = setup_adapter().await;
    let p = Person { id: Person::create_simple_id(), name: "Ada".into(), age: 42, marketing: true };
    let pid = p.id.clone(); // clone id to reuse
    insert(vec![p.clone()]).into().return_many(db.clone()).await?;

    if let Some(found) = get_by_id::<Person, _>(pid.clone(), db.clone()).await? {
        println!("Found person: {} (age {})", found.name, found.age);
    }

    #[derive(serde::Deserialize)]
    struct PersonName { name: String }
    let uuid_part = pid.as_uuid_str().to_string();
    if let Some(name_only) = select::<PersonName>(All)
        .from(Person::table())
        .record_id(uuid_part)
        .return_one(db.clone())
        .await? {
        println!("Name-only projection: {}", name_only.name);
    }

    Ok(())
}
