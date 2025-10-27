mod person;

use junction_rs_core::prelude::*;
use junction_rs_macros::*;
use person::{setup_adapter, Person};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
#[junction(table = "product")]
struct Product {
    id: SimpleId<Product>,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Edge)]
struct Bought {
    #[junction(source)]
    person: SimpleId<Person>,
    #[junction(target)]
    product: SimpleId<Product>,
    qty: u32,
}

#[tokio::main]
async fn main() {
    let db = setup_adapter().await;

    let alice = Person {
        id: Person::create_simple_id(),
        name: "Alice".into(),
        age: 30,
        marketing: true,
    };
    let bob = Person {
        id: Person::create_simple_id(),
        name: "Bob".into(),
        age: 32,
        marketing: false,
    };
    let carol = Person {
        id: Person::create_simple_id(),
        name: "Carol".into(),
        age: 35,
        marketing: true,
    };
    let dave = Person {
        id: Person::create_simple_id(),
        name: "Dave".into(),
        age: 28,
        marketing: false,
    };
    // Seed people and products
    let people = vec![alice.clone(), bob.clone(), carol.clone(), dave.clone()];

    let laptop = Product {
        id: Product::create_simple_id(),
        name: "Laptop".into(),
    };
    let phone = Product {
        id: Product::create_simple_id(),
        name: "Phone".into(),
    };
    let headphones = Product {
        id: Product::create_simple_id(),
        name: "Headphones".into(),
    };

    let products = vec![laptop.clone(), phone.clone(), headphones.clone()];

    let _created_people = insert(people)
        .into()
        .return_many::<_>(db.clone())
        .await
        .unwrap();
    let _created_products = insert(products)
        .into()
        .return_many::<_>(db.clone())
        .await
        .unwrap();

    // Record purchases (edges)

    let buys = vec![
        Bought {
            person: alice.id.clone(),
            product: laptop.id.clone(),
            qty: 1,
        },
        Bought {
            person: alice.id.clone(),
            product: phone.id.clone(),
            qty: 1,
        },
        Bought {
            person: bob.id.clone(),
            product: phone.id.clone(),
            qty: 2,
        },
        Bought {
            person: bob.id.clone(),
            product: headphones.id.clone(),
            qty: 2,
        },
    ];
    let _created_buys: Vec<Bought> = insert(buys).return_many::<_>(db.clone()).await.unwrap();

    // Simple recommendation: products bought by people who bought the same as Alice
    // Path: person -> bought -> product <- bought <- person -> bought -> product
    // DSL:           forward              backward            forward
    let recommended = select::<Product>(All)
        .record_id(&alice.id)
        .forward::<Bought, Product>()
        .backward::<Bought, Person>()
        .forward::<Bought, Product>()
        .distinct()
        .return_many(db.clone())
        .await
        .unwrap();

    println!("High-score recommendations: {}", recommended.len());
    for product in recommended {
        println!("- {}", product.name);
    }
}
