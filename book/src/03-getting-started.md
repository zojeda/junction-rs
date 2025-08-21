# Getting Started

This chapter walks through setting up JunctionRS in a new project.

## 1. Add Dependency
In `Cargo.toml`:
```toml
[dependencies]
junction-rs = { version = "0.1.0", features = ["surreal"] }
serde = { version = "1", features = ["derive"] }
```
(Adjust the version to the latest published crate.)

## 2. Create a Node Model
```rust
use serde::{Serialize, Deserialize};
use junction_rs::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
#[junction(table = "person")]
struct Person { name: String, age: i32, marketing: bool }
```

## 3. Setup Adapter (In-Memory SurrealDB)
```rust
async fn setup_adapter() -> junction_rs::adapter::SurrealDbAdapter {
    use surrealdb::engine::any::connect;
    let db = connect("mem://").await.unwrap();
    db.use_ns("demo").use_db("demo").await.unwrap();
    junction_rs::adapter::SurrealDbAdapter::new(db)
}
```

## 4. Insert & Select
```rust
let db = setup_adapter().await;
let people = vec![
    Person { name: "Tom".into(), age: 42, marketing: true },
    Person { name: "Jaime".into(), age: 40, marketing: false },
];
insert(people.clone()).into().return_many::<_>(db.clone()).await?;
let s = Person::schema();
let selected: Vec<Person> = select::<Person>(All)
    .from(Person::table())
    .where_(expr!(s.age > 40))
    .return_many(db.clone())
    .await?;
```

## 5. Column Projection
```rust
let partial: Vec<Person> = select::<Person>(All)
    .from(Person::table())
    .column(Person::schema().name)
    .column(Person::schema().age)
    .return_many(db.clone())
    .await?;
```

## 6. Subset Struct Projection
```rust
#[derive(Debug, Deserialize)]
struct PersonNameAge { name: String, age: i32 }
let rows: Vec<PersonNameAge> = select::<PersonNameAge>(All)
    .from(Person::table())
    .return_many(db.clone())
    .await?;
```

## 7. Delete
```rust
delete(Person::table())
    .where_(expr!(Person::schema().age < 41))
    .run(db.clone())
    .await?;
```

## 8. Next Steps
Proceed to Defining Models and Expression DSL chapters for deeper understanding.
