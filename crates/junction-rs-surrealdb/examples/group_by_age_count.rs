mod person;

use junction_rs_core::prelude::*;
use person::{setup_adapter, Person};

#[tokio::main]
async fn main() {
    let db = setup_adapter().await;

    // Seed sample data
    let people = vec![
        Person {
            id: Person::create_simple_id(),
            name: "A".into(),
            age: 30,
            marketing: true,
        },
        Person {
            id: Person::create_simple_id(),
            name: "B".into(),
            age: 30,
            marketing: false,
        },
        Person {
            id: Person::create_simple_id(),
            name: "C".into(),
            age: 42,
            marketing: true,
        },
        Person {
            id: Person::create_simple_id(),
            name: "D".into(),
            age: 42,
            marketing: false,
        },
        Person {
            id: Person::create_simple_id(),
            name: "E".into(),
            age: 42,
            marketing: true,
        },
    ];
    insert(people).into().return_many(db.clone()).await.unwrap();

    // Group by an age bucket expression (e.g., 30-40, 40-50) and count per bucket
    // Use numeric bucket lower bound: age - (age % 10)
    let rows: Vec<BucketCount> = select::<BucketCount>(All)
        .from(Person::table())
        .expr_as("age - (age % 10)", "bucket")
        .count_all_as("cnt")
        .group_by_expr("bucket")
        .return_many(db.clone())
        .await
        .unwrap();

    for row in rows {
        println!("range={}-{} count={}", row.bucket, row.bucket + 10, row.cnt);
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BucketCount {
    bucket: i64,
    cnt: u64,
}
