use anyhow::Result;
use junction_rs_core::schema_ops::SchemaOp;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct MigrationFile {
    pub id: String,
    pub description: String,
    pub checksum: String,
    pub path: PathBuf,
    pub ops_forward: Vec<SchemaOp>,
    pub ops_reverse: Vec<SchemaOp>,
}

pub fn discover_migrations(dir: &Path) -> Result<Vec<MigrationFile>> {
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut entries: Vec<MigrationFile> = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            if let Some((id, desc)) = split_name(name) {
                let content = fs::read_to_string(&path)?;
                let checksum = sha256_hex(&content);
                let (ops_forward, ops_reverse) = parse_ops(&content);
                entries.push(MigrationFile {
                    id: id.to_string(),
                    description: desc.to_string(),
                    checksum,
                    path: path.clone(),
                    ops_forward,
                    ops_reverse,
                });
            }
        }
    }
    // order by id (timestamp or incremental assumes lexicographic order)
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(entries)
}

fn split_name(filename: &str) -> Option<(&str, &str)> {
    // pattern: <id>_<desc>.rs
    if let Some(rest) = filename.strip_suffix(".rs") {
        if let Some(idx) = rest.find('_') {
            let (id, desc) = rest.split_at(idx);
            let desc = &desc[1..];
            if !id.is_empty() && !desc.is_empty() {
                return Some((id, desc));
            }
        } else if rest.chars().all(|c| c.is_ascii_digit()) {
            // fallback: id only
            return Some((rest, "(no_description)"));
        }
    }
    None
}

fn sha256_hex(content: &str) -> String {
    let mut h = Sha256::new();
    h.update(content.as_bytes());
    let out = h.finalize();
    hex::encode(out)
}

fn parse_ops(content: &str) -> (Vec<SchemaOp>, Vec<SchemaOp>) {
    // Heuristic extraction: look for Ok(vec![ ... ]) inside up() and down().
    // For now we don't attempt to fully parse Rust; later can move to syn.
    let re_up = Regex::new(r"up\([^)]*\)\s*->[^\{]*\{[^O]*Ok\(vec!\[(?P<body>[^;]*)\]\)").unwrap();
    let re_down =
        Regex::new(r"down\([^)]*\)\s*->[^\{]*\{[^O]*Ok\(vec!\[(?P<body>[^;]*)\]\)").unwrap();
    let forward = re_up
        .captures(content)
        .map(|c| c.name("body").map(|m| m.as_str()).unwrap_or(""))
        .unwrap_or("");
    let reverse = re_down
        .captures(content)
        .map(|c| c.name("body").map(|m| m.as_str()).unwrap_or(""))
        .unwrap_or("");
    (parse_op_list(forward), parse_op_list(reverse))
}

fn parse_op_list(body: &str) -> Vec<SchemaOp> {
    // Extremely naive split on 'SchemaOp::'
    let mut ops = Vec::new();
    for part in body.split("SchemaOp::").skip(1) {
        // skip text before first occurrence
        let trimmed = part.trim();
        if trimmed.starts_with("CreateTable") {
            // We will not reconstruct full field list for now (TODO). Leaving empty.
            if let Some(name) = extract_string_arg(trimmed) {
                ops.push(SchemaOp::CreateTable {
                    name,
                    fields: vec![],
                });
            }
        } else if trimmed.starts_with("DropTable") {
            if let Some(name) = extract_string_arg(trimmed) {
                ops.push(SchemaOp::DropTable { name });
            }
        } else if trimmed.starts_with("AddField") {
            // Not reconstructing field details
            if let Some(_table) = extract_table_field_names(trimmed).0 { /* placeholder for future field capture */
            }
        } else if trimmed.starts_with("DropField") {
            // ignore for now
        } else if trimmed.starts_with("CreateEdge") {
            if let Some(name) = extract_string_arg(trimmed) {
                ops.push(SchemaOp::CreateEdge {
                    name,
                    from: String::new(),
                    to: String::new(),
                    fields: vec![],
                });
            }
        } else if trimmed.starts_with("DropEdge") {
            if let Some(name) = extract_string_arg(trimmed) {
                ops.push(SchemaOp::DropEdge { name });
            }
        }
    }
    ops
}

fn extract_string_arg(part: &str) -> Option<String> {
    // look for name: "..."
    let re = Regex::new(r#"name:\s*\"([A-Za-z0-9_]+)\""#).ok()?;
    re.captures(part)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

fn extract_table_field_names(_part: &str) -> (Option<String>, Option<String>) {
    (None, None)
}
