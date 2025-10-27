//! Example demonstrating custom edge endpoint field names.
//!
//! Run with:
//!   cargo run -p junction-rs-surrealdb --example custom_edge_endpoints
//!
//! This shows how an edge struct can rename its endpoint fields away from SurrealDB's
//! native `in` / `out` while the adapter maps them transparently.

use junction_rs::prelude::*;
use serde::{Deserialize, Serialize};
mod person; // reuse existing person module for Person + setup_adapter
use person::{setup_adapter, Person};

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
#[junction(table = "product")] // separate simple product node
struct Product {
    id: SimpleId<Product>,
    title: String,
}

// Custom endpoint field names instead of `in` / `out`.
#[derive(Debug, Clone, Serialize, Deserialize, Edge)]
#[junction(table = "bought")]
struct Bought {
    #[junction(source)]
    from_person: SimpleId<Person>,
    #[junction(target)]
    to_product: SimpleId<Product>,
    quantity: i32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_adapter().await; // in-memory Surreal

    // Insert some nodes
    let people: Vec<Person> = insert(vec![
        Person {
            id: Person::create_simple_id(),
            name: "Alice".into(),
            age: 30,
            marketing: false,
        },
        Person {
            id: Person::create_simple_id(),
            name: "Bob".into(),
            age: 40,
            marketing: true,
        },
    ])
    .return_many(db.clone())
    .await?;

    let products: Vec<Product> = insert(vec![
        Product {
            id: Product::create_simple_id(),
            title: "Gadget".into(),
        },
        Product {
            id: Product::create_simple_id(),
            title: "Widget".into(),
        },
    ])
    .return_many(db.clone())
    .await?;

    // Create edges using unified insert API (no special edge insert function needed).
    let edges: Vec<Bought> = insert(vec![
        Bought {
            from_person: people[0].id.clone(),
            to_product: products[0].id.clone(),
            quantity: 2,
        },
        Bought {
            from_person: people[1].id.clone(),
            to_product: products[1].id.clone(),
            quantity: 1,
        },
    ])
    .return_many(db.clone())
    .await?;

    println!("Inserted edges (custom endpoints preserved):");
    for e in &edges {
        println!("  {:?}", e);
    }

    // Selecting edges: use select_edge so adapter restores custom endpoint names for deserialization.
    let selected: Vec<Bought> = select_edge::<Bought>(All)
        .from(Bought::table())
        .return_many(db.clone())
        .await?;
    println!("Selected edges:");
    for e in &selected {
        println!("  {:?}", e);
    }

    // (Optional) Show we can filter edges by quantity using expression DSL
    let s_b = Bought::schema();
    let filtered: Vec<Bought> = select_edge::<Bought>(All)
        .from(Bought::table())
        .where_(expr!(s_b.quantity >= 2))
        .return_many(db.clone())
        .await?;
    println!("Edges with quantity >= 2: {:?}", filtered);

    Ok(())
}
