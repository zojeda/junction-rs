use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use junction_rs_core::traits::{DbExecutor, Query};
use serde_json::Value;

use crate::config::DatabaseConfig;

pub type ConnectFuture<'cfg> =
    Pin<Box<dyn Future<Output = Result<Arc<dyn DynExecutor>>> + Send + 'cfg>>;

pub struct AdapterRegistration {
    pub name: &'static str,
    pub connect: for<'cfg> fn(&'cfg DatabaseConfig) -> ConnectFuture<'cfg>,
}

inventory::collect!(AdapterRegistration);

pub fn all_adapters() -> Vec<&'static AdapterRegistration> {
    inventory::iter::<AdapterRegistration>.into_iter().collect()
}

pub fn find_adapter(name: &str) -> Result<&'static AdapterRegistration> {
    all_adapters()
        .into_iter()
        .find(|entry| entry.name == name)
        .ok_or_else(|| anyhow!("adapter '{}' not registered", name))
}

pub async fn connect_from_config(cfg: &DatabaseConfig) -> Result<Arc<dyn DynExecutor>> {
    let reg = find_adapter(&cfg.adapter)?;
    (reg.connect)(cfg).await
}

#[async_trait]
pub trait DynExecutor: Send + Sync {
    async fn execute(&self, query: Query) -> Result<Vec<Value>>;
}

#[async_trait]
impl<T> DynExecutor for T
where
    T: DbExecutor,
{
    async fn execute(&self, query: Query) -> Result<Vec<Value>> {
        DbExecutor::execute::<Value>(self, query)
            .await
            .map_err(|err| anyhow!(err))
    }
}

pub fn wrap_executor<E>(executor: E) -> Arc<dyn DynExecutor>
where
    E: DbExecutor,
{
    let exec: Arc<dyn DynExecutor> = Arc::new(executor);
    exec
}

#[macro_export]
macro_rules! register_adapter {
    ($name:expr, $ctor:path) => {
        inventory::submit! {
            $crate::adapter::AdapterRegistration {
                name: $name,
                connect: |cfg: &$crate::config::DatabaseConfig| -> $crate::adapter::ConnectFuture<'_> {
                    Box::pin(async move {
                        let exec = $ctor(cfg).await?;
                        Ok($crate::adapter::wrap_executor(exec))
                    })
                },
            }
        }
    };
}
