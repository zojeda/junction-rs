use crate::config::RootConfig;
use crate::{diff::diff_schemas, scanner, schema::SchemaSnapshot};
use anyhow::{Context, Result};
use junction_rs_core::schema_ops::{FieldDef, FieldType, SchemaOp};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const SNAPSHOT_FILE: &str = "schema.snapshot.json";

/// Simple migration file generator placeholder.
/// Future: integrate diff + snapshot update.
pub fn generate_new_migration(name: &str, cfg: &RootConfig) -> Result<()> {
    // Load old snapshot (or empty)
    let old_snapshot: SchemaSnapshot = if PathBuf::from(SNAPSHOT_FILE).exists() {
        serde_json::from_str(&fs::read_to_string(SNAPSHOT_FILE)?)?
    } else {
        SchemaSnapshot::new()
    };

    // Scan new snapshot
    let root = std::env::current_dir()?;
    let new_snapshot = scanner::scan_schema(&scanner::ScanOptions {
        root: &root,
        paths: &cfg.models.paths,
        include: &cfg.models.include,
        exclude: &cfg.models.exclude,
    })?;

    // Diff
    let diff = diff_schemas(&old_snapshot, &new_snapshot);
    if diff.ops_forward.is_empty() {
        println!("No schema changes detected. Migration not created.");
        return Ok(());
    }

    // Build code vectors
    let forward_code = render_ops(&diff.ops_forward);
    let reverse_code = render_ops(&diff.ops_reverse);

    let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let id = format!("{ts}");
    let snake = name.replace(' ', "_").to_lowercase();
    let fname = format!("migrations/{}_{}.rs", id, snake);
    if !PathBuf::from("migrations").exists() {
        fs::create_dir_all("migrations")?;
    }
    let template = format!(
        r#"// Auto-generated migration based on schema diff
use async_trait::async_trait;
use junction_rs_core::schema_ops::{{SchemaOp, FieldDef, FieldType}};

#[async_trait]
pub trait Migration {{
    fn id(&self) -> &'static str;
    fn description(&self) -> &'static str;
    async fn up(&self) -> anyhow::Result<Vec<SchemaOp>>;
    async fn down(&self) -> anyhow::Result<Vec<SchemaOp>>;
}}

pub struct Migration{id};

#[async_trait]
impl Migration for Migration{id} {{
    fn id(&self) -> &'static str {{ "{id}" }}
    fn description(&self) -> &'static str {{ "{snake}" }}
    async fn up(&self) -> anyhow::Result<Vec<SchemaOp>> {{
        Ok(vec!{forward_code})
    }}
    async fn down(&self) -> anyhow::Result<Vec<SchemaOp>> {{
        Ok(vec!{reverse_code})
    }}
}}
"#
    );
    fs::write(&fname, template).with_context(|| format!("writing migration file {fname}"))?;
    println!("created migration {fname} ({} ops)", diff.ops_forward.len());

    // Write new snapshot
    fs::write(SNAPSHOT_FILE, serde_json::to_string_pretty(&new_snapshot)?)?;
    println!("updated {SNAPSHOT_FILE}");
    Ok(())
}

fn render_ops(ops: &[SchemaOp]) -> String {
    let mut parts = Vec::new();
    for op in ops {
        let s = match op {
            SchemaOp::CreateTable { name, fields } => format!("SchemaOp::CreateTable {{ name: \"{name}\".into(), fields: vec![{}] }}", render_fields(fields)),
            SchemaOp::DropTable { name } => format!("SchemaOp::DropTable {{ name: \"{name}\".into() }}"),
            SchemaOp::AddField { table, field } => format!("SchemaOp::AddField {{ table: \"{table}\".into(), field: {} }}", render_field(field)),
            SchemaOp::DropField { table, field } => format!("SchemaOp::DropField {{ table: \"{table}\".into(), field: \"{field}\".into() }}"),
            SchemaOp::CreateEdge { name, from, to, fields } => format!("SchemaOp::CreateEdge {{ name: \"{name}\".into(), from: \"{from}\".into(), to: \"{to}\".into(), fields: vec![{}] }}", render_fields(fields)),
            SchemaOp::DropEdge { name } => format!("SchemaOp::DropEdge {{ name: \"{name}\".into() }}"),
        };
        parts.push(s);
    }
    format!("[{}]", parts.join(", "))
}

fn render_fields(fields: &[FieldDef]) -> String {
    fields
        .iter()
        .map(render_field)
        .collect::<Vec<_>>()
        .join(", ")
}
fn render_field(f: &FieldDef) -> String {
    format!(
        "FieldDef {{ name: \"{}\".into(), ty: {}, primary: {} }}",
        f.name,
        render_field_type(&f.ty),
        f.primary
    )
}
fn render_field_type(ft: &FieldType) -> String {
    match ft {
        FieldType::String => "FieldType::String".into(),
        FieldType::Int64 => "FieldType::Int64".into(),
        FieldType::Float64 => "FieldType::Float64".into(),
        FieldType::Bool => "FieldType::Bool".into(),
        FieldType::DateTime => "FieldType::DateTime".into(),
        FieldType::Uuid => "FieldType::Uuid".into(),
        FieldType::Json => "FieldType::Json".into(),
        FieldType::Other(s) => format!("FieldType::Other(\"{}\".into())", s),
    }
}
