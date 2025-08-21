use junction_rs::prelude::*;

#[cfg(feature = "surreal")]
#[derive(Node, serde::Serialize, serde::Deserialize, Debug, Clone)]
#[junction(table = "proj_person")]
pub struct ProjPerson {
    pub id: SimpleId<ProjPerson>,
    pub name: String,
    pub age: u8,
}

#[cfg(feature = "surreal")]
#[derive(Clone, Default)]
struct DummyDb(dummy_support::LastQuery);

#[cfg(feature = "surreal")]
impl DbExecutor for DummyDb {
    async fn execute<T: serde::de::DeserializeOwned + Send>(
        &self,
        query: junction_rs_core::query::ast::Query,
    ) -> Result<Vec<T>, junction_rs_core::traits::DbError> {
        let compiled =
            junction_rs_surrealdb::compiler::compile_to_surql(&query).expect("compile to surql");
        (self.0)
            .0
            .lock()
            .unwrap()
            .replace((compiled.sql, serde_json::to_value(compiled.params).unwrap()));
        Ok(Vec::new())
    }
}

#[cfg(feature = "surreal")]
mod dummy_support {
    use std::sync::{Arc, Mutex};
    #[derive(Default, Clone)]
    pub struct LastQuery(pub Arc<Mutex<Option<(String, serde_json::Value)>>>);
}

#[cfg(feature = "surreal")]
#[tokio::test]
async fn projection_with_schema_columns() {
    let db = DummyDb::default();
    let s = ProjPerson::schema();
    let _ = select::<ProjPerson>(expr::All)
        .from(ProjPerson::table())
        .column(s.name)
        .column(s.age)
        .return_many::<DummyDb>(db.clone())
        .await
        .unwrap();
    let (sql, _params) = (db.0).0.lock().unwrap().clone().unwrap();
    assert_eq!(sql, "SELECT name, age FROM proj_person");
}
