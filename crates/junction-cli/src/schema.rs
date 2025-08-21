use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaSnapshot {
    pub version: u32,
    pub generated_at: DateTime<Utc>,
    pub models: Vec<Model>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub table: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub table: String,
    pub from: String,
    pub to: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: FieldType,
    #[serde(default)]
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "data")]
pub enum FieldType {
    String,
    Int64,
    Float64,
    Bool,
    DateTime,
    Uuid,
    Json,
    Other(String),
}

impl Default for SchemaSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaSnapshot {
    pub fn new() -> Self {
        Self {
            version: 1,
            generated_at: Utc::now(),
            models: vec![],
            edges: vec![],
        }
    }
}
