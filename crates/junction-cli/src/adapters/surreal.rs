use anyhow::{Context, Result};
use junction_rs_surrealdb::SurrealDbAdapter;
use surrealdb::engine::any::connect;

use crate::{config::DatabaseConfig, register_adapter};

async fn connect_surreal(cfg: &DatabaseConfig) -> Result<SurrealDbAdapter> {
    let url = cfg.url.as_deref().unwrap_or("memory");
    let client = connect(url)
        .await
        .with_context(|| format!("connecting to surrealdb at {}", url))?;
    if let (Some(ns), Some(db)) = (cfg.namespace.as_deref(), cfg.database.as_deref()) {
        client
            .use_ns(ns)
            .use_db(db)
            .await
            .with_context(|| format!("selecting namespace={} db={}", ns, db))?;
    }
    Ok(SurrealDbAdapter::new(client))
}

register_adapter!("surreal", connect_surreal);
