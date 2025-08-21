mod person;

use junction_rs_core::prelude::*;
use person::{setup_adapter, Person};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersonNameAge {
    name: String,
    age: i32,
}

#[tokio::main]
async fn main() {
    let db = setup_adapter().await;

    // Seed some data
    let people = vec![
        Person {
            id: Person::create_simple_id(),
            name: "Tom".into(),
            age: 42,
            marketing: true,
        },
        Person {
            id: Person::create_simple_id(),
            name: "Jaime".into(),
            age: 40,
            marketing: false,
        },
    ];
    insert(people).into().return_many(db.clone()).await.unwrap();

    // Select only a projection: name and age
    // Note: The backend returns full rows; serde will ignore extra fields when
    // deserializing into PersonNameAge.
    let rows: Vec<PersonNameAge> = select::<PersonNameAge>(All)
        .from(Person::table())
        .return_many(db.clone())
        .await
        .unwrap();

    println!("Got {} projected rows", rows.len());
    for row in rows {
        println!("{:?}", row);
    }
}
