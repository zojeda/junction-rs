mod person;

use junction_rs_core::prelude::*;
use person::{setup_adapter, Person};

#[tokio::main]
async fn main() {
    let db = setup_adapter().await;

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

    insert(people.clone())
        .into()
        .return_many(db.clone())
        .await
        .unwrap();

    let s = Person::schema();
    delete(Person::table())
        .where_(expr!(s.age < 41))
        .run(db.clone())
        .await
        .unwrap();

    let remaining: Vec<Person> = select::<Person>(All)
        .from(Person::table())
        .return_many(db.clone())
        .await
        .unwrap();

    println!("Remaining {} people", remaining.len());
    for person in remaining {
        println!("{:?}", person);
    }
}
