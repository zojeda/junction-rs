use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

const DEFAULT_CONFIG_FILE: &str = "junction.toml";

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProjectConfig {
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelsConfig {
    #[serde(default = "default_paths")]
    pub paths: Vec<String>,
    #[serde(default = "default_include")]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

fn default_paths() -> Vec<String> {
    vec!["crates/**/*.rs".into()]
}
fn default_include() -> Vec<String> {
    vec![".*".into()]
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MigrationsConfig {
    #[serde(default = "default_migrations_dir")]
    pub dir: String,
    #[serde(default = "default_naming")]
    pub naming: String,
}
fn default_migrations_dir() -> String {
    "migrations".into()
}
fn default_naming() -> String {
    "timestamp".into()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub adapter: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub database: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SchemaSectionConfig {
    #[serde(default = "default_true")]
    pub capture_fields: bool,
    #[serde(default = "default_true")]
    pub capture_edge_relationships: bool,
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RootConfig {
    #[serde(default)]
    pub project: ProjectConfig,
    #[serde(default)]
    pub models: ModelsConfig,
    #[serde(default)]
    pub migrations: MigrationsConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub schema: SchemaSectionConfig,
}

impl Default for ModelsConfig {
    fn default() -> Self {
        Self {
            paths: default_paths(),
            include: default_include(),
            exclude: vec![],
        }
    }
}
impl Default for MigrationsConfig {
    fn default() -> Self {
        Self {
            dir: default_migrations_dir(),
            naming: default_naming(),
        }
    }
}
impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            adapter: "surreal".into(),
            url: Some("memory".into()),
            namespace: Some("test".into()),
            database: Some("test".into()),
        }
    }
}
impl Default for SchemaSectionConfig {
    fn default() -> Self {
        Self {
            capture_fields: true,
            capture_edge_relationships: true,
        }
    }
}

impl RootConfig {
    pub fn load(explicit: Option<&Path>) -> Result<(Self, PathBuf)> {
        let path = if let Some(p) = explicit {
            p.to_path_buf()
        } else {
            PathBuf::from(DEFAULT_CONFIG_FILE)
        };
        if path.exists() {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("reading config file {}", path.display()))?;
            let cfg: Self = toml::from_str(&content).with_context(|| "parsing config toml")?;
            Ok((cfg, path))
        } else {
            // Return defaults if absent
            Ok((Self::default(), path))
        }
    }

    pub fn to_pretty_toml(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }
}
