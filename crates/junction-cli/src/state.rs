#![cfg(feature = "surreal")]

use anyhow::Result;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use serde::{Serialize, Deserialize};

const STATE_TABLE: &str = "__junction_migrations";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MigrationRecord {
    pub id: String,
    pub applied_at: String, // ISO8601
    pub checksum: String,
}

pub async fn ensure_state_table(db: &Surreal<Any>) -> Result<()> {
    // Surreal will create table implicitly on first insert; optional explicit definition could go here.
    Ok(())
}

pub async fn fetch_applied(db: &Surreal<Any>) -> Result<Vec<MigrationRecord>> {
    let sql = format!("SELECT * FROM {STATE_TABLE}");
    let res: Vec<MigrationRecord> = db.query(sql).await?.take(0)?;
    Ok(res)
}

pub async fn record_applied(db: &Surreal<Any>, rec: &MigrationRecord) -> Result<()> {
    let sql = format!("CREATE {STATE_TABLE}:{id} SET applied_at = $applied_at, checksum = $checksum", id = rec.id);
    db.query(sql)
        .bind(("applied_at", &rec.applied_at))
        .bind(("checksum", &rec.checksum))
        .await?;
    Ok(())
}

pub async fn delete_record(db: &Surreal<Any>, id: &str) -> Result<()> {
    let sql = format!("DELETE {STATE_TABLE}:{id}");
    db.query(sql).await?;
    Ok(())
}
