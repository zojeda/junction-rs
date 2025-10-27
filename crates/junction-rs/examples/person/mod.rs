use junction_rs::prelude::*;
use junction_rs::SurrealDbAdapter;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Node, PartialEq)]
#[junction(table = "person_example")]
pub struct Person {
    pub id: SimpleId<Person>,
    pub name: String,
    pub age: i32,
    pub marketing: bool,
}

pub async fn setup_adapter() -> SurrealDbAdapter {
    use surrealdb::engine::any::connect; // available behind feature
    let db = connect("mem://").await.unwrap();
    db.use_ns("test").use_db("test").await.unwrap();
    SurrealDbAdapter::new(db)
}
