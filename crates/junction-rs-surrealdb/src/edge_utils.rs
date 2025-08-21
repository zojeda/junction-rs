use crate::adapter::SurrealDbAdapter;
use junction_rs_core::{traits::DbError, Edge};
use serde::Serialize;
use serde_json::Value;

// Lightweight internal logging macro (no external dependency) gated behind tests or optional feature.
// Usage: debug_log!("message: {}", value);
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {{
        #[cfg(test)]
        eprintln!("[junction-rs-surrealdb][debug] {}", format!($($arg)*));
    }};
}

// Re-export for local (module) use without full path.
use debug_log;

/// Create graph edges using SurrealDB RELATE statements from a list of edge structs.
/// This bypasses the generic INSERT mechanism so traversals (`->edge->node`) work.
///
/// Conventions:
///   * Edge struct fields follow SurrealDB naming: `in` = destination (incoming), `out` = source (outgoing)
///   * We therefore generate: `RELATE $out_var->edge_table->$in_var`.
pub async fn relate_edges<E: Edge + Serialize>(
    db: &SurrealDbAdapter,
    edges: Vec<E>,
) -> Result<(), DbError> {
    if edges.is_empty() {
        return Ok(());
    }
    use std::collections::BTreeMap;
    // Collect unique record ids (person:uuid, product:uuid ...)
    let mut ids: BTreeMap<String, (String, String)> = BTreeMap::new();
    for edge in &edges {
        let v = serde_json::to_value(edge).map_err(|e| DbError::Adapter(e.to_string()))?;
        let obj = v
            .as_object()
            .ok_or_else(|| DbError::Adapter("edge not an object".into()))?;
        let src = obj
            .get("out")
            .and_then(Value::as_str)
            .ok_or_else(|| DbError::Adapter("missing out".into()))?; // source
        let dst = obj
            .get("in")
            .and_then(Value::as_str)
            .ok_or_else(|| DbError::Adapter("missing in".into()))?; // destination
        let parse_id = |s: &str| -> Result<(String, String), DbError> {
            if let Some((table, rest)) = s.split_once(':') {
                let cleaned = rest
                    .trim()
                    .trim_start_matches('⟨')
                    .trim_end_matches('⟩')
                    .trim_start_matches('<')
                    .trim_end_matches('>');
                Ok((table.to_string(), cleaned.to_string()))
            } else {
                Err(DbError::Adapter(format!("bad record id: {}", s)))
            }
        };
        let (src_table, src_id) = parse_id(src)?;
        let (dst_table, dst_id) = parse_id(dst)?;
        ids.entry(src.to_string())
            .or_insert_with(|| (src_table.clone(), src_id.clone()));
        ids.entry(dst.to_string())
            .or_insert_with(|| (dst_table.clone(), dst_id.clone()));
    }
    // Map ids to variable names
    let mut var_names: BTreeMap<String, String> = BTreeMap::new();
    let mut sql = String::new();
    for (var_index, (orig, (table, idpart))) in ids.iter().enumerate() {
        let var = format!("v{}", var_index);
        var_names.insert(orig.clone(), var.clone());
        sql.push_str(&format!(
            "LET ${} = type::thing(\"{}\", \"{}\");",
            var, table, idpart
        ));
    }
    // Build RELATE statements (source = out, dest = in) including any additional fields beyond endpoints.
    for edge in &edges {
        let v = serde_json::to_value(edge).map_err(|e| DbError::Adapter(e.to_string()))?;
        let obj = v.as_object().unwrap();
        let dst = obj
            .get("in")
            .and_then(Value::as_str)
            .ok_or_else(|| DbError::Adapter("missing in".into()))?;
        let src = obj
            .get("out")
            .and_then(Value::as_str)
            .ok_or_else(|| DbError::Adapter("missing out".into()))?;
        let src_var = var_names.get(src).unwrap();
        let dst_var = var_names.get(dst).unwrap();
        // Collect arbitrary field assignments (skip in/out/id)
        let mut set_parts: Vec<String> = Vec::new();
        for (k, val) in obj.iter() {
            if k == "in" || k == "out" || k == "id" {
                continue;
            }
            // Render value as JSON literal (SurrealQL accepts JSON style for primitives/objects/arrays)
            let lit = serde_json::to_string(val).map_err(|e| DbError::Adapter(e.to_string()))?;
            set_parts.push(format!("{} = {}", k, lit));
        }
        if set_parts.is_empty() {
            sql.push_str(&format!("RELATE ${}->{}->${};", src_var, E::TABLE, dst_var));
        } else {
            sql.push_str(&format!(
                "RELATE ${}->{}->${} SET {} ;",
                src_var,
                E::TABLE,
                dst_var,
                set_parts.join(", ")
            ));
        }
    }
    debug_log!("edges SQL: {}", sql);
    let q = db.client.query(&sql);
    q.await.map_err(|e| DbError::Adapter(e.to_string()))?;
    // Temporary debug: fetch created edge rows (first 5) to inspect shape during tests.
    #[cfg(test)]
    {
        if let Ok(mut dbg_res) = db
            .client
            .query(&format!("SELECT * FROM {} LIMIT 5", E::TABLE))
            .await
        {
            if let Ok(val) = dbg_res.take::<surrealdb::Value>(0usize) {
                let json_val = serde_json::to_value(&val).unwrap_or(serde_json::Value::Null);
                debug_log!("edges sample rows: {}", json_val);
            }
        }
    }
    Ok(())
}
