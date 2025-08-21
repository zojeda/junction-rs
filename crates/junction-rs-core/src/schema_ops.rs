//! Schema operation primitives used for migrations and diff generation.
//! These are backend-agnostic operations that can be compiled by adapters into
//! engine-specific DDL or graph schema statements.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldDef {
    pub name: String,
    pub ty: FieldType,
    #[serde(default)]
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SchemaOp {
    CreateTable {
        name: String,
        fields: Vec<FieldDef>,
    },
    DropTable {
        name: String,
    },
    AddField {
        table: String,
        field: FieldDef,
    },
    DropField {
        table: String,
        field: String,
    },
    CreateEdge {
        name: String,
        from: String,
        to: String,
        fields: Vec<FieldDef>,
    },
    DropEdge {
        name: String,
    },
}
