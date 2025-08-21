use regex::Regex;
use std::{fs, path::Path};
use walkdir::WalkDir;

use crate::schema::{Edge, Field, FieldType, Model, SchemaSnapshot};

#[derive(Debug)]
pub struct ScanOptions<'a> {
    pub root: &'a Path,
    pub paths: &'a [String],
    pub include: &'a [String],
    pub exclude: &'a [String],
}

pub fn scan_schema(opts: &ScanOptions) -> anyhow::Result<SchemaSnapshot> {
    let mut snapshot = SchemaSnapshot::new();

    // Precompile regexes
    let re_derive_node = Regex::new(
        r"(?s)#\s*\[derive\([^]]*Node[^]]*\)\](?:\s*#\s*\[[^]]*\])*\s*pub\s+struct\s+([A-Za-z0-9_]+)",
    )
    .unwrap();
    let re_derive_edge = Regex::new(
        r"(?s)#\s*\[derive\([^]]*Edge[^]]*\)\](?:\s*#\s*\[[^]]*\])*\s*pub\s+struct\s+([A-Za-z0-9_]+)",
    )
    .unwrap();
    // Simplified table attribute matcher: #[ogm(table = "name")]
    // NOTE: Simplified pattern: capture table name inside #[ogm(... table = "name" ...)]
    let re_table_attr =
        Regex::new(r#"#\s*\[ogm\([^\]]*table\s*=\s*"([A-Za-z0-9_]+)"[^\[\]]*\)\]"#).unwrap();
    // Edge attribute capturing from/to: #[ogm(table = "t", from = FromType, to = ToType)]
    let re_edge_attr = Regex::new(
        r#"#\s*\[ogm\([^\]]*from\s*=\s*([A-Za-z0-9_]+)\s*,\s*to\s*=\s*([A-Za-z0-9_]+)[^\]]*\)\]"#,
    )
    .unwrap();
    let re_field = Regex::new(r"pub\s+([A-Za-z0-9_]+)\s*:\s*([^,]+),").unwrap();
    let re_edge_endpoint = Regex::new(r"(?:pub\s+)?r#?(in|out)\s*:\s*([^,]+),").unwrap();

    // Simplistic include/exclude filtering compiled
    let include_res: Vec<Regex> = opts
        .include
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect();
    let exclude_res: Vec<Regex> = opts
        .exclude
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect();

    for entry in WalkDir::new(opts.root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        // Basic path pattern filtering (substring match for now)
        if !opts
            .paths
            .iter()
            .any(|p| path.to_string_lossy().contains(p.trim_matches('*')))
        {
            continue;
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Node structs
        for cap in re_derive_node.captures_iter(&content) {
            let struct_name = cap.get(1).unwrap().as_str();
            if !include_res.iter().all(|r| r.is_match(struct_name)) {
                continue;
            }
            if exclude_res.iter().any(|r| r.is_match(struct_name)) {
                continue;
            }

            // Find table attribute preceding struct
            let table = re_table_attr
                .captures(&content)
                .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
                .unwrap_or_else(|| struct_name.to_lowercase());

            // Extract fields inside struct block (simplistic)
            let fields_block = extract_struct_block(&content, struct_name).unwrap_or_default();
            let mut fields = vec![];
            for fcap in re_field.captures_iter(&fields_block) {
                let name = fcap.get(1).unwrap().as_str().to_string();
                let ty_raw = fcap.get(2).unwrap().as_str().trim();
                let ty = map_type(ty_raw);
                let primary = name == "id"; // heuristic
                fields.push(Field { name, ty, primary });
            }

            snapshot.models.push(Model { table, fields });
        }

        // Edge structs
        for cap in re_derive_edge.captures_iter(&content) {
            let struct_name = cap.get(1).unwrap().as_str();
            if !include_res.iter().all(|r| r.is_match(struct_name)) {
                continue;
            }
            if exclude_res.iter().any(|r| r.is_match(struct_name)) {
                continue;
            }

            let table = re_table_attr
                .captures(&content)
                .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
                .unwrap_or_else(|| struct_name.to_lowercase());
            let attr_endpoints = re_edge_attr.captures(&content).and_then(|c| {
                Some((
                    c.get(1)?.as_str().to_string(),
                    c.get(2)?.as_str().to_string(),
                ))
            });

            let mut inferred_from: Option<String> = None;
            let mut inferred_to: Option<String> = None;
            let fields_block = extract_struct_block(&content, struct_name).unwrap_or_default();
            for cap_endpoint in re_edge_endpoint.captures_iter(&fields_block) {
                let kind = cap_endpoint.get(1).unwrap().as_str();
                let ty_raw = cap_endpoint.get(2).unwrap().as_str().trim();
                match kind {
                    "in" => {
                        if inferred_from.is_none() {
                            inferred_from = infer_endpoint_from_type(ty_raw);
                        }
                    }
                    "out" => {
                        if inferred_to.is_none() {
                            inferred_to = infer_endpoint_from_type(ty_raw);
                        }
                    }
                    _ => {}
                }
            }
            let mut fields = vec![];
            for fcap in re_field.captures_iter(&fields_block) {
                let name = fcap.get(1).unwrap().as_str().to_string();
                let ty_raw = fcap.get(2).unwrap().as_str().trim();
                let ty = map_type(ty_raw);
                let primary = name == "id";
                fields.push(Field { name, ty, primary });
            }

            let (from, to) = if let Some((attr_from, attr_to)) = attr_endpoints {
                (attr_from, attr_to)
            } else {
                (
                    inferred_from.unwrap_or_else(|| "unknown_from".into()),
                    inferred_to.unwrap_or_else(|| "unknown_to".into()),
                )
            };

            snapshot.edges.push(Edge {
                table,
                from,
                to,
                fields,
            });
        }
    }

    Ok(snapshot)
}

fn extract_struct_block(content: &str, struct_name: &str) -> Option<String> {
    let pattern = format!("struct {}", struct_name);
    let start = content.find(&pattern)?;
    let after = &content[start..];
    let brace_pos = after.find('{')?;
    let mut depth = 0usize;
    let mut collected = String::new();
    for ch in after[brace_pos + 1..].chars() {
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            if depth == 0 {
                break;
            } else {
                depth -= 1;
            }
        }
        collected.push(ch);
    }
    Some(collected)
}

fn infer_endpoint_from_type(raw: &str) -> Option<String> {
    let mut ty = raw.trim();
    while let Some(start) = ty.find('<') {
        let end = ty.rfind('>')?;
        ty = ty[start + 1..end].trim();
    }
    let ty = ty.trim_start_matches('&').trim();
    if ty.is_empty() {
        return None;
    }
    ty.split("::").last().map(|s| s.trim().to_string())
}

fn map_type(raw: &str) -> FieldType {
    match raw {
        "String" | "&str" => FieldType::String,
        "i64" | "i32" | "i16" | "i8" | "isize" => FieldType::Int64,
        "u64" | "u32" | "u16" | "u8" | "usize" => FieldType::Int64,
        "f64" | "f32" => FieldType::Float64,
        "bool" => FieldType::Bool,
        "DateTime<Utc>" | "DateTime" => FieldType::DateTime,
        ty if ty.contains("Uuid") => FieldType::Uuid,
        ty if ty.contains("serde_json::Value") => FieldType::Json,
        other => FieldType::Other(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    fn unique_tmp_dir() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!("junction_scan_test_{}", nanos));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn write_rs(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let p = dir.join(name);
        fs::write(&p, content).unwrap();
        p
    }

    fn opts<'a>(
        root: &'a Path,
        paths: &'a [String],
        include: &'a [String],
        exclude: &'a [String],
    ) -> ScanOptions<'a> {
        ScanOptions {
            root,
            paths,
            include,
            exclude,
        }
    }

    #[test]
    fn scan_single_node_basic() {
        let dir = unique_tmp_dir();
        write_rs(
            &dir,
            "person.rs",
            r#"
      #[derive(Node)]
      #[ogm(table = "people")]
      pub struct Person {
        pub id: String,
        pub name: String,
        pub age: i32,
      }
    "#,
        );
        let paths = vec!["person.rs".to_string()];
        let snap = scan_schema(&opts(&dir, &paths, &[], &[])).unwrap();
        assert_eq!(snap.models.len(), 1);
        let m = &snap.models[0];
        assert_eq!(m.table, "people");
        assert_eq!(m.fields.len(), 3);
        let mut map = HashMap::new();
        for f in &m.fields {
            map.insert(f.name.clone(), (&f.ty, f.primary));
        }
        assert_eq!(map.get("id").unwrap().1, true);
        assert!(matches!(map.get("age").unwrap().0, FieldType::Int64));
        assert!(matches!(map.get("name").unwrap().0, FieldType::String));
    }

    #[test]
    fn scan_edge_with_from_to_and_fields() {
        let dir = unique_tmp_dir();
        write_rs(
            &dir,
            "edges.rs",
            r#"
      #[derive(Edge)]
      #[ogm(table = "friend_edge")]
      pub struct Friend {
        pub id: String,
        pub strength: i32,
        r#in: SimpleId<Person>,
        r#out: SimpleId<Person>,
      }
    "#,
        );
        let p = vec!["edges.rs".to_string()];
        let snap = scan_schema(&opts(&dir, &p, &[], &[])).unwrap();
        assert_eq!(snap.edges.len(), 1);
        let e = &snap.edges[0];
        assert_eq!(e.table, "friend_edge");
        assert_eq!(e.from, "Person");
        assert_eq!(e.to, "Person");
        assert_eq!(e.fields.len(), 2);
        assert!(e
            .fields
            .iter()
            .any(|f| f.name == "strength" && matches!(f.ty, FieldType::Int64)));
    }

    #[test]
    fn include_and_exclude_patterns() {
        let dir = unique_tmp_dir();
        write_rs(
            &dir,
            "a.rs",
            r#"
      #[derive(Node)]
      pub struct Alpha { pub id: String, }
    "#,
        );
        write_rs(
            &dir,
            "b.rs",
            r#"
      #[derive(Node)]
      pub struct BetaIgnore { pub id: String, }
    "#,
        );
        let paths = vec![".rs".to_string()];
        let include = vec!["A.*".to_string()];
        let exclude = vec![".*Ignore".to_string()];
        let snap = scan_schema(&opts(&dir, &paths, &include, &exclude)).unwrap();
        assert_eq!(snap.models.len(), 1);
        assert_eq!(snap.models[0].table, "alpha");
    }

    #[test]
    fn path_filtering_only_matches_subset() {
        let dir = unique_tmp_dir();
        write_rs(
            &dir,
            "one.rs",
            r#"
      #[derive(Node)]
      pub struct One { pub id: String, }
    "#,
        );
        write_rs(
            &dir,
            "two.rs",
            r#"
      #[derive(Node)]
      pub struct Two { pub id: String, }
    "#,
        );
        let paths = vec!["two.rs".to_string()];
        let snap = scan_schema(&opts(&dir, &paths, &[], &[])).unwrap();
        assert_eq!(snap.models.len(), 1);
        assert_eq!(snap.models[0].table, "two");
    }

    #[test]
    fn table_and_edge_fallbacks() {
        let dir = unique_tmp_dir();
        write_rs(
            &dir,
            "fallback.rs",
            r#"
      #[derive(Node)]
      pub struct PlainNode { pub id: String, }

      #[derive(Edge)]
      pub struct PlainEdge { pub id: String, }
    "#,
        );
        let paths = vec!["fallback.rs".to_string()];
        let snap = scan_schema(&opts(&dir, &paths, &[], &[])).unwrap();
        assert_eq!(snap.models.len(), 1);
        assert_eq!(snap.models[0].table, "plainnode");
        assert_eq!(snap.edges.len(), 1);
        let e = &snap.edges[0];
        assert_eq!(e.table, "plainedge");
        assert_eq!(e.from, "unknown_from");
        assert_eq!(e.to, "unknown_to");
    }

    #[test]
    fn does_not_leak_fields_from_following_struct() {
        let dir = unique_tmp_dir();
        write_rs(
            &dir,
            "multi.rs",
            r#"
      #[derive(Node)]
      pub struct First {
        pub id: String,
        pub x: i32,
      }

      pub struct Second {
        pub unrelated: bool,
      }
    "#,
        );
        let paths = vec!["multi.rs".to_string()];
        let snap = scan_schema(&opts(&dir, &paths, &[], &[])).unwrap();
        assert_eq!(snap.models.len(), 1);
        let m = &snap.models[0];
        assert_eq!(m.fields.len(), 2);
        assert!(m.fields.iter().any(|f| f.name == "x"));
        assert!(!m.fields.iter().any(|f| f.name == "unrelated"));
    }

    #[test]
    fn comprehensive_type_mapping() {
        let dir = unique_tmp_dir();
        write_rs(
            &dir,
            "types.rs",
            r#"
      #[derive(Node)]
      pub struct TypeNode {
        pub id: String,
        pub a: i32,
        pub b: u64,
        pub c: f32,
        pub d: bool,
        pub e: DateTime<Utc>,
        pub f: uuid::Uuid,
        pub g: serde_json::Value,
        pub h: CustomType,
      }
    "#,
        );
        let paths = vec!["types.rs".to_string()];
        let snap = scan_schema(&opts(&dir, &paths, &[], &[])).unwrap();
        let m = &snap.models[0];
        let mut map: HashMap<String, &FieldType> = HashMap::new();
        for f in &m.fields {
            map.insert(f.name.clone(), &f.ty);
        }
        assert!(matches!(map.get("a").unwrap(), FieldType::Int64));
        assert!(matches!(map.get("b").unwrap(), FieldType::Int64));
        assert!(matches!(map.get("c").unwrap(), FieldType::Float64));
        assert!(matches!(map.get("d").unwrap(), FieldType::Bool));
        assert!(matches!(map.get("e").unwrap(), FieldType::DateTime));
        assert!(matches!(map.get("f").unwrap(), FieldType::Uuid));
        assert!(matches!(map.get("g").unwrap(), FieldType::Json));
        assert!(matches!(map.get("h").unwrap(), FieldType::Other(s) if s == "CustomType"));
    }
}
