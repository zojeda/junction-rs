mod person;

use junction_rs::prelude::*;
use person::setup_adapter;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Node, PartialEq)]
#[junction(table = "person2")]
struct Person2 {
    id: SimpleId<Person2>,
    name: String,
    age: i32,
    marketing: bool,
}

#[tokio::main]
async fn main() {
    let db = setup_adapter().await;

    let people = vec![
        Person2 {
            id: Person2::create_simple_id(),
            name: "Tom".into(),
            age: 42,
            marketing: true,
        },
        Person2 {
            id: Person2::create_simple_id(),
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

    println!("Created {} people", created.len());
    for person in created {
        println!("{:?}", person);
    }
}
