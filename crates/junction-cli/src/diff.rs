use crate::schema::{Edge, Field, FieldType, Model, SchemaSnapshot};
use junction_rs_core::schema_ops::{FieldDef, SchemaOp};
use std::collections::BTreeMap;

pub struct DiffResult {
    pub ops_forward: Vec<SchemaOp>, // apply current -> target
    pub ops_reverse: Vec<SchemaOp>, // reverse (target -> current)
}

pub fn diff_schemas(current: &SchemaSnapshot, target: &SchemaSnapshot) -> DiffResult {
    let mut forward = Vec::new();
    let mut reverse = Vec::new();

    // Index models by table
    let cur_models = index_models(current);
    let tgt_models = index_models(target);

    // Table additions & drops
    for (table, model) in tgt_models.iter() {
        if !cur_models.contains_key(table) {
            forward.push(SchemaOp::CreateTable {
                name: table.clone(),
                fields: model.fields.iter().map(field_def).collect(),
            });
            reverse.push(SchemaOp::DropTable {
                name: table.clone(),
            });
        }
    }
    for (table, _model) in cur_models.iter() {
        if !tgt_models.contains_key(table) {
            forward.push(SchemaOp::DropTable {
                name: table.clone(),
            });
            // For reversibility we would need original model – we have it here
            let orig = cur_models.get(table).unwrap();
            reverse.push(SchemaOp::CreateTable {
                name: table.clone(),
                fields: orig.fields.iter().map(field_def).collect(),
            });
        }
    }

    // Field-level diffs for tables present in both
    for (table, tgt_model) in tgt_models.iter() {
        if let Some(cur_model) = cur_models.get(table) {
            let cur_fields: BTreeMap<_, _> =
                cur_model.fields.iter().map(|f| (&f.name, f)).collect();
            let tgt_fields: BTreeMap<_, _> =
                tgt_model.fields.iter().map(|f| (&f.name, f)).collect();

            // Additions
            for (fname, f) in tgt_fields.iter() {
                if !cur_fields.contains_key(fname) {
                    let fd = field_def(f);
                    forward.push(SchemaOp::AddField {
                        table: table.clone(),
                        field: fd.clone(),
                    });
                    reverse.push(SchemaOp::DropField {
                        table: table.clone(),
                        field: (*fname).to_string(),
                    });
                }
            }
            // Removals
            for (fname, f) in cur_fields.iter() {
                if !tgt_fields.contains_key(fname) {
                    let fd = field_def(f);
                    forward.push(SchemaOp::DropField {
                        table: table.clone(),
                        field: (*fname).to_string(),
                    });
                    reverse.push(SchemaOp::AddField {
                        table: table.clone(),
                        field: fd,
                    });
                }
            }
            // Type changes (simple comparison)
            for (fname, tgt_field) in tgt_fields.iter() {
                if let Some(cur_field) = cur_fields.get(fname) {
                    if tgt_field.ty != cur_field.ty {
                        // Represent as drop/add (may lossy). Future: AlterField op.
                        let fd_new = field_def(tgt_field);
                        let fd_old = field_def(cur_field);
                        forward.push(SchemaOp::DropField {
                            table: table.clone(),
                            field: (*fname).to_string(),
                        });
                        forward.push(SchemaOp::AddField {
                            table: table.clone(),
                            field: fd_new.clone(),
                        });
                        reverse.push(SchemaOp::DropField {
                            table: table.clone(),
                            field: (*fname).to_string(),
                        });
                        reverse.push(SchemaOp::AddField {
                            table: table.clone(),
                            field: fd_old,
                        });
                    }
                }
            }
        }
    }

    // Edges (treat similar to tables)
    let cur_edges = index_edges(current);
    let tgt_edges = index_edges(target);
    for (ename, edge) in tgt_edges.iter() {
        if !cur_edges.contains_key(ename) {
            forward.push(SchemaOp::CreateEdge {
                name: ename.clone(),
                from: edge.from.clone(),
                to: edge.to.clone(),
                fields: edge.fields.iter().map(field_def).collect(),
            });
            reverse.push(SchemaOp::DropEdge {
                name: ename.clone(),
            });
        }
    }
    for (ename, edge) in cur_edges.iter() {
        if !tgt_edges.contains_key(ename) {
            forward.push(SchemaOp::DropEdge {
                name: ename.clone(),
            });
            reverse.push(SchemaOp::CreateEdge {
                name: ename.clone(),
                from: edge.from.clone(),
                to: edge.to.clone(),
                fields: edge.fields.iter().map(field_def).collect(),
            });
        }
    }

    DiffResult {
        ops_forward: forward,
        ops_reverse: reverse,
    }
}

fn index_models(s: &SchemaSnapshot) -> BTreeMap<String, &Model> {
    s.models.iter().map(|m| (m.table.clone(), m)).collect()
}
fn index_edges(s: &SchemaSnapshot) -> BTreeMap<String, &Edge> {
    s.edges.iter().map(|e| (e.table.clone(), e)).collect()
}

fn field_def(f: &Field) -> FieldDef {
    FieldDef {
        name: f.name.clone(),
        ty: map_field_type(&f.ty),
        primary: f.primary,
    }
}

fn map_field_type(ft: &FieldType) -> junction_rs_core::schema_ops::FieldType {
    use junction_rs_core::schema_ops::FieldType as CFT;
    use FieldType as FT;
    match ft {
        FT::String => CFT::String,
        FT::Int64 => CFT::Int64,
        FT::Float64 => CFT::Float64,
        FT::Bool => CFT::Bool,
        FT::DateTime => CFT::DateTime,
        FT::Uuid => CFT::Uuid,
        FT::Json => CFT::Json,
        FT::Other(s) => CFT::Other(s.clone()),
    }
}
