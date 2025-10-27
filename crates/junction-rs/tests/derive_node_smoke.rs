use junction_rs::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::ops::Not;
use std::sync::{Arc, Mutex};
#[cfg(feature = "surreal")]
#[derive(Clone, Default)]
struct DummyDb {
    last: Arc<Mutex<Option<(String, serde_json::Value)>>>,
}

#[cfg(feature = "surreal")]
impl DbExecutor for DummyDb {
    async fn execute<T: serde::de::DeserializeOwned + Send>(
        &self,
        query: junction_rs_core::query::ast::Query,
    ) -> Result<Vec<T>, junction_rs_core::traits::DbError> {
        // Compile using Surreal compiler to preserve existing assertions on text
        let compiled = junction_rs_surrealdb::compiler::compile_to_surql(&query)
            .expect("compile_to_surql should succeed for non-path queries");
        *self.last.lock().unwrap() =
            Some((compiled.sql, serde_json::to_value(compiled.params).unwrap()));
        Ok(Vec::new())
    }
}

#[derive(Node, Serialize, Deserialize, Debug, Clone)]
#[junction(table = "space_ship")]
#[serde(rename_all = "camelCase")]
pub struct SpaceShip {
    pub id: SimpleId<SpaceShip>,
    pub name: String,
    pub age: u8,
}

#[derive(Node, Serialize, Deserialize, Debug, Clone)]
#[junction(table = "planet")]
#[serde(rename_all = "camelCase")]
pub struct Planet {
    pub id: SimpleId<Planet>,
    pub name: String,
}

fn sample_spaceships() -> Vec<SpaceShip> {
    vec![
        SpaceShip {
            id: SpaceShip::create_simple_id(),
            name: "Millennium Falcon".into(),
            age: 79,
        },
        SpaceShip {
            id: SpaceShip::create_simple_id(),
            name: "Starship Enterprise".into(),
            age: 15,
        },
    ]
}

// Edge-related types and tests moved to `derive_edge_smoke.rs`.

#[cfg(feature = "surreal")]
#[cfg(feature = "surreal")]
#[tokio::test]
async fn test_insert_statement() {
    let db = DummyDb::default();
    let ships = sample_spaceships();

    let _ = insert(ships)
        .into()
        .return_many::<DummyDb>(db.clone())
        .await
        .unwrap();
    let (sql, params) = db.last.lock().unwrap().clone().unwrap();
    assert!(sql.starts_with("INSERT INTO space_ship $values"), "{}", sql);
    let params_obj = params.as_object().unwrap();
    assert_eq!(params_obj.len(), 1);
    assert!(params_obj.contains_key("values"));
}

#[cfg(feature = "surreal")]
#[cfg(feature = "surreal")]
#[tokio::test]
async fn test_select_statement_basic() {
    let db = DummyDb::default();
    let schema = <SpaceShip as Node>::schema();
    let table = <SpaceShip as Node>::table();

    let _ = select::<SpaceShip>(expr::All)
        .from(table)
        .where_(expr!(schema.name == "Millennium Falcon"))
        .order_by(schema.age.desc())
        .limit(10)
        .return_many::<DummyDb>(db.clone())
        .await
        .unwrap();

    let (sql, params) = db.last.lock().unwrap().clone().unwrap();
    assert!(sql.contains("SELECT * FROM space_ship"), "{}", sql);
    assert!(
        sql.contains("WHERE name = $p1") || sql.contains("WHERE name = $p1"),
        "{}",
        sql
    );
    assert_eq!(params.as_object().unwrap().len(), 1);
    assert!(sql.contains("ORDER BY age DESC"));
    assert!(sql.contains("LIMIT 10"));
}

#[cfg(feature = "surreal")]
#[tokio::test]
async fn test_select_minimal_from_only() {
    let db = DummyDb::default();
    let table = <SpaceShip as Node>::table();

    let _ = select::<SpaceShip>(expr::All)
        .from(table)
        .return_many::<DummyDb>(db.clone())
        .await
        .unwrap();

    let (sql, params) = db.last.lock().unwrap().clone().unwrap();
    assert_eq!(sql, "SELECT * FROM space_ship");
    assert!(params.as_object().unwrap().is_empty());
}

#[cfg(feature = "surreal")]
#[tokio::test]
async fn test_select_with_complex_filter_and_multi_order() {
    let db = DummyDb::default();
    let schema = <SpaceShip as Node>::schema();
    let table = <SpaceShip as Node>::table();

    let filter =
        expr!(schema.age.clone() > 10).and(expr!(schema.name.clone() == "Millennium Falcon").not());

    let _ = select::<SpaceShip>(expr::All)
        .from(table)
        .where_(filter)
        .order_by(schema.age.clone().asc())
        .order_by(schema.name.clone().desc())
        .return_many::<DummyDb>(db.clone())
        .await
        .unwrap();

    let (sql, params) = db.last.lock().unwrap().clone().unwrap();
    assert!(sql.contains("SELECT * FROM space_ship"));
    assert!(
        sql.contains("WHERE (age > $p1 AND (NOT name = $p2))"),
        "{}",
        sql
    );
    assert_eq!(params.as_object().unwrap().len(), 2);
    assert!(sql.contains("ORDER BY age ASC, name DESC"), "{}", sql);
    assert!(!sql.contains("LIMIT "));
}

#[cfg(feature = "surreal")]
#[tokio::test]
async fn test_update_with_content_and_where() {
    let db = DummyDb::default();
    let schema = <SpaceShip as Node>::schema();
    let table = <SpaceShip as Node>::table();

    let _ = update(table)
        .content(json!({"age": 99}))
        .where_(expr!(schema.name == "Millennium Falcon"))
        .return_many::<DummyDb, SpaceShip>(db.clone())
        .await
        .unwrap();

    let (sql, params) = db.last.lock().unwrap().clone().unwrap();
    assert!(
        sql.starts_with("UPDATE space_ship MERGE $content WHERE name = $p1 RETURN AFTER"),
        "{}",
        sql
    );
    let params_obj = params.as_object().unwrap();
    assert!(params_obj.contains_key("content"));
    assert_eq!(params_obj.len(), 2, "params: {:?}", params_obj);
}

#[cfg(feature = "surreal")]
#[tokio::test]
async fn test_update_without_content_just_filter() {
    let db = DummyDb::default();
    let schema = <SpaceShip as Node>::schema();
    let table = <SpaceShip as Node>::table();

    let _ = update(table)
        .where_(schema.age.gt(5))
        .return_many::<DummyDb, SpaceShip>(db.clone())
        .await
        .unwrap();

    let (sql, params) = db.last.lock().unwrap().clone().unwrap();
    assert!(
        sql.starts_with("UPDATE space_ship WHERE age > $p1 RETURN AFTER"),
        "{}",
        sql
    );
    let params_obj = params.as_object().unwrap();
    assert!(!params_obj.contains_key("content"));
    assert_eq!(params_obj.len(), 1);
}

#[cfg(feature = "surreal")]
#[tokio::test]
async fn test_update_without_content_or_where() {
    let db = DummyDb::default();
    let table = <SpaceShip as Node>::table();

    let _ = update(table)
        .return_many::<DummyDb, SpaceShip>(db.clone())
        .await
        .unwrap();

    let (sql, params) = db.last.lock().unwrap().clone().unwrap();
    assert_eq!(sql, "UPDATE space_ship RETURN AFTER");
    assert!(params.as_object().unwrap().is_empty());
}

#[cfg(feature = "surreal")]
#[tokio::test]
async fn test_delete_with_where() {
    let db = DummyDb::default();
    let schema = <SpaceShip as Node>::schema();
    let table = <SpaceShip as Node>::table();

    delete(table)
        .where_(expr!(schema.age <= 10))
        .run(db.clone())
        .await
        .unwrap();

    let (sql, params) = db.last.lock().unwrap().clone().unwrap();
    assert!(
        sql.starts_with("DELETE space_ship WHERE age <= $p1"),
        "{}",
        sql
    );
    assert_eq!(params.as_object().unwrap().len(), 1);
}

#[cfg(feature = "surreal")]
#[tokio::test]
async fn test_delete_all_no_where() {
    let db = DummyDb::default();
    let table = <SpaceShip as Node>::table();

    delete(table).run(db.clone()).await.unwrap();

    let (sql, params) = db.last.lock().unwrap().clone().unwrap();
    assert_eq!(sql, "DELETE space_ship");
    assert!(params.as_object().unwrap().is_empty());
}

#[cfg(feature = "surreal")]
#[tokio::test]
async fn test_select_with_or_and_grouping() {
    let db = DummyDb::default();
    let schema = <SpaceShip as Node>::schema();
    let table = <SpaceShip as Node>::table();

    // (age >= 18 OR age < 5) AND name = "Foo"
    let filter = expr!(schema.age.clone() >= 18)
        .or(expr!(schema.age.clone() < 5))
        .and(expr!(schema.name.clone() == "Foo"));

    let _ = select::<SpaceShip>(expr::All)
        .from(table)
        .where_(filter)
        .return_many::<DummyDb>(db.clone())
        .await
        .unwrap();

    let (sql, params) = db.last.lock().unwrap().clone().unwrap();
    assert!(sql.contains("SELECT * FROM space_ship"));
    assert!(
        sql.contains("WHERE ((age >= $p1 OR age < $p2) AND name = $p3)"),
        "{}",
        sql
    );
    assert_eq!(params.as_object().unwrap().len(), 3);
}
