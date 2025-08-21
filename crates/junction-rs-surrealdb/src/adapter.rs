use serde::de::DeserializeOwned;

use crate::compiler::compile_to_surql;
use junction_rs_core::traits::{DbError, DbExecutor};

/// `DbExecutor` implementation backed by a `surrealdb` client.
#[derive(Clone)]
pub struct SurrealDbAdapter {
    pub(crate) client: surrealdb::Surreal<surrealdb::engine::any::Any>,
}

impl SurrealDbAdapter {
    pub fn new(client: surrealdb::Surreal<surrealdb::engine::any::Any>) -> Self {
        Self { client }
    }
    /// Expose a reference to the underlying Surreal client (primarily for tests / debugging).
    pub fn client_ref(&self) -> &surrealdb::Surreal<surrealdb::engine::any::Any> {
        &self.client
    }
    /// Execute a raw SurrealQL statement (test utility). Returns adapter-style DbError on failure.
    pub async fn raw_query(&self, sql: &str) -> Result<(), junction_rs_core::traits::DbError> {
        self.client
            .query(sql)
            .await
            .map_err(|e| junction_rs_core::traits::DbError::Adapter(e.to_string()))?;
        Ok(())
    }

    pub async fn debug_select_json(
        &self,
        sql: &str,
    ) -> Result<serde_json::Value, junction_rs_core::traits::DbError> {
        let mut res = self
            .client
            .query(sql)
            .await
            .map_err(|e| junction_rs_core::traits::DbError::Adapter(e.to_string()))?;
        // Use take::<surrealdb::Value>(0)
        let v: surrealdb::Value = res
            .take(0usize)
            .map_err(|e| junction_rs_core::traits::DbError::Adapter(e.to_string()))?;
        serde_json::to_value(&v)
            .map_err(|e| junction_rs_core::traits::DbError::Adapter(e.to_string()))
    }
}

fn flatten_surreal_json(value: &serde_json::Value) -> serde_json::Value {
    // Surreal often encodes explicit nulls inside object projections as the literal string "Null".
    // We normalize that sentinel to an actual JSON null so downstream serde Option<> decoding works.
    if let serde_json::Value::String(s) = value {
        if s == "Null" {
            return serde_json::Value::Null;
        }
    }
    // Fast path: primitives and simple arrays of primitives are returned unchanged.
    match value {
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => return value.clone(),
        serde_json::Value::Array(a)
            if a.iter().all(|v| {
                matches!(
                    v,
                    serde_json::Value::Null
                        | serde_json::Value::Bool(_)
                        | serde_json::Value::Number(_)
                        | serde_json::Value::String(_)
                )
            }) =>
        {
            return value.clone();
        }
        _ => {}
    }
    if let Some(obj) = value.as_object() {
        if let Some(num) = obj.get("Number") {
            if let Some(int_val) = num.get("Int").and_then(|v| v.as_i64()) {
                return serde_json::Value::Number(int_val.into());
            } else if let Some(float_val) = num.get("Float").and_then(|v| v.as_f64()) {
                return serde_json::Number::from_f64(float_val)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null);
            } else if let Some(dec) = num.get("Decimal") {
                return dec.clone();
            } else if let Some(num) = num.get("Number") {
                return num.clone();
            }
        } else if let Some(strand) = obj.get("Strand") {
            return strand.clone();
        } else if let Some(bool_val) = obj.get("Bool") {
            return bool_val.clone();
        } else if let Some(thing) = obj.get("Thing") {
            if let Some(thing_obj) = thing.as_object() {
                if let Some(id) = thing_obj.get("id") {
                    if let Some(id_str) = id.get("String").and_then(|v| v.as_str()) {
                        return serde_json::Value::String(id_str.to_string());
                    }
                }
            }
            return serde_json::Value::String(thing.to_string());
        } else if let Some(arr) = obj.get("Array") {
            if let Some(arr_val) = arr.as_array() {
                return serde_json::Value::Array(
                    arr_val.iter().map(flatten_surreal_json).collect(),
                );
            }
        } else if let Some(obj_val) = obj.get("Object") {
            if let Some(obj_map) = obj_val.as_object() {
                return serde_json::Value::Object(
                    obj_map
                        .iter()
                        .map(|(k, v)| (k.clone(), flatten_surreal_json(v)))
                        .collect(),
                );
            }
        } else {
            // Already flat object or unknown
            return serde_json::Value::Object(
                obj.iter()
                    .map(|(k, v)| (k.clone(), flatten_surreal_json(v)))
                    .collect(),
            );
        }
    }
    value.clone()
}

impl DbExecutor for SurrealDbAdapter {
    async fn execute<T: DeserializeOwned + Send>(
        &self,
        query: junction_rs_core::query::ast::Query,
    ) -> Result<Vec<T>, DbError> {
        let compiled = compile_to_surql(&query)?;
        let debug_enabled = std::env::var("JUNCTION_DEBUG_SQL").is_ok();
        if debug_enabled {
            println!(
                "[junction-rs-surrealdb][adapter] executing SQL: {}",
                compiled.sql
            );
        }
        if debug_enabled {
            println!(
                "[junction-rs-surrealdb][adapter] params: {:?}",
                compiled.params
            );
        }
        let mut q = self.client.query(&compiled.sql);
        for (k, v) in compiled.params.into_iter() {
            q = q.bind((k, v));
        }
        let mut res = q.await.map_err(|e| DbError::Adapter(e.to_string()))?;
        // Attempt index-based extraction: Surreal supports taking each statement result by its index.
        let mut value_opt: Option<surrealdb::Value> = None;
        let scan_debug = std::env::var("JUNCTION_DEBUG_SCAN").is_ok();
        for idx in 0..128 {
            // reasonable upper bound
            match res.take(idx) {
                Ok(v) => {
                    if scan_debug {
                        let dbg_json = serde_json::to_string(
                            &serde_json::to_value(&v).unwrap_or(serde_json::Value::Null),
                        )
                        .unwrap_or_default();
                        eprintln!(
                            "[junction-rs-surrealdb][adapter][take {}] {}",
                            idx, dbg_json
                        );
                    }
                    let json_v = serde_json::to_value(&v).unwrap_or(serde_json::Value::Null);
                    let is_placeholder = matches!(json_v, serde_json::Value::Null)
                        || (json_v.as_str() == Some("None"));
                    if scan_debug {
                        println!(
                            "[junction-rs-surrealdb][adapter][scan] idx={} placeholder={} json={}",
                            idx, is_placeholder, json_v
                        );
                    }
                    if !is_placeholder {
                        value_opt = Some(v);
                    }
                }
                Err(e) => {
                    if scan_debug {
                        println!(
                            "[junction-rs-surrealdb][adapter][scan] idx={} take error: {}",
                            idx, e
                        );
                    }
                    break;
                }
            }
        }
        let value = match value_opt {
            Some(v) => v,
            None => {
                // Fallback: if this was an INSERT, Surreal may have returned no row set (unexpected). Attempt to reconstruct from bound values.
                if let junction_rs_core::query::ast::Stmt::Insert(_ins) = &query.stmt {
                    if let Some(val) = query.binds.get("values") {
                        println!("[junction-rs-surrealdb][adapter][fallback] reconstructing insert result from bound values");
                        let json = serde_json::json!([val]);
                        let values: Vec<T> = serde_json::from_value(json).map_err(|e| {
                            DbError::Adapter(format!("Serialization error fallback: {}", e))
                        })?;
                        return Ok(values);
                    }
                }
                #[cfg(test)]
                panic!(
                    "[junction-rs-surrealdb][adapter] No result returned for SQL: {}",
                    compiled.sql
                );
                #[allow(unreachable_code)]
                return Err(DbError::Adapter("No result returned".to_string()));
            }
        };
        let nested_json: serde_json::Value = serde_json::to_value(&value)
            .map_err(|e| DbError::Adapter(format!("JSON conversion error: {}", e)))?;
        let json_value = flatten_surreal_json(&nested_json);
        if debug_enabled {
            eprintln!(
                "[junction-rs-surrealdb][adapter][debug] raw flattened json: {}",
                json_value
            );
        }
        if json_value.is_string() && json_value.as_str() == Some("None") {
            // Treat Surreal's None sentinel as an empty set of rows.
            return Ok(Vec::new());
        }
        let mut json_value = json_value; // make mutable for post-processing
                                         // Restore custom endpoint field names for edges (applies to INSERT and SELECT) using bind markers.
        let has_edge_binds = query.binds.contains_key("__jxn_edge_src")
            && query.binds.contains_key("__jxn_edge_tgt");
        if has_edge_binds {
            if let (Some(src_name), Some(tgt_name)) = (
                query.binds.get("__jxn_edge_src"),
                query.binds.get("__jxn_edge_tgt"),
            ) {
                if let (Some(src_str), Some(tgt_str)) = (src_name.as_str(), tgt_name.as_str()) {
                    match json_value {
                        serde_json::Value::Array(ref mut arr) => {
                            for obj in arr.iter_mut() {
                                if let Some(map) = obj.as_object_mut() {
                                    if !map.contains_key(src_str) {
                                        if let Some(v) = map.get("in").cloned() {
                                            map.insert(src_str.to_string(), v);
                                        }
                                    }
                                    if !map.contains_key(tgt_str) {
                                        if let Some(v) = map.get("out").cloned() {
                                            map.insert(tgt_str.to_string(), v);
                                        }
                                    }
                                }
                            }
                        }
                        serde_json::Value::Object(ref mut map) => {
                            if !map.contains_key(src_str) {
                                if let Some(v) = map.get("in").cloned() {
                                    map.insert(src_str.to_string(), v);
                                }
                            }
                            if !map.contains_key(tgt_str) {
                                if let Some(v) = map.get("out").cloned() {
                                    map.insert(tgt_str.to_string(), v);
                                }
                            }
                            // Wrap object in array to standardize downstream handling
                            json_value = serde_json::Value::Array(vec![serde_json::Value::Object(
                                map.clone(),
                            )]);
                        }
                        _ => {}
                    }
                }
            }
        }
        let arr = json_value
            .as_array()
            .ok_or_else(|| DbError::Adapter("Expected array in result".to_string()))?;
        let mut json_objects = Vec::new();
        // Helper to recursively unwrap nested single-element arrays (common in Surreal path + projection results)
        fn unwrap_nested(v: &serde_json::Value) -> serde_json::Value {
            match v {
                serde_json::Value::Array(a) if a.len() == 1 => {
                    let first = &a[0];
                    // Only unwrap wrapper arrays that are clearly Surreal nesting artifacts (array of array / array of {Object:...} / array of {Array:...}).
                    // Preserve semantic single-element arrays (e.g., edges: [edge_id]) so they still deserialize as Vec<_>.
                    let unwrap = matches!(first, serde_json::Value::Array(_))
                        || first
                            .as_object()
                            .is_some_and(|m| m.contains_key("Object") || m.contains_key("Array"));
                    if unwrap {
                        unwrap_nested(first)
                    } else {
                        serde_json::Value::Array(vec![unwrap_nested(first)])
                    }
                }
                serde_json::Value::Array(a) => {
                    serde_json::Value::Array(a.iter().map(unwrap_nested).collect())
                }
                serde_json::Value::Object(map) => {
                    let mut new_map = serde_json::Map::new();
                    for (k, val) in map.iter() {
                        new_map.insert(k.clone(), unwrap_nested(val));
                    }
                    serde_json::Value::Object(new_map)
                }
                _ => v.clone(),
            }
        }
        for item in arr {
            if item.is_null() {
                continue;
            }
            if let Some(obj) = item.as_object() {
                json_objects.push(unwrap_nested(&serde_json::Value::Object(obj.clone())));
            } else if let Some(inner_arr) = item.as_array() {
                // Fallback: an array of primitives (legacy path form) -> wrap into PathSegment shape if strings
                if inner_arr.iter().all(|v| v.is_string()) {
                    let nodes: Vec<serde_json::Value> = inner_arr.to_vec();
                    let mut map = serde_json::Map::new();
                    map.insert("nodes".to_string(), serde_json::Value::Array(nodes));
                    map.insert("edges".to_string(), serde_json::Value::Null);
                    json_objects.push(serde_json::Value::Object(map));
                } else {
                    for inner_item in inner_arr {
                        if let Some(obj) = inner_item.as_object() {
                            json_objects
                                .push(unwrap_nested(&serde_json::Value::Object(obj.clone())));
                        } else {
                            return Err(DbError::Adapter(
                                "Expected object in inner array item".to_string(),
                            ));
                        }
                    }
                }
            } else {
                return Err(DbError::Adapter(
                    "Expected object or array in array item".to_string(),
                ));
            }
        }
        // If this is a plain SELECT over an edge table (Stmt::Select) we won't have endpoint bind metadata.
        // We attempt heuristic restoration of custom endpoint field names if the target type expects them.
        // For SELECT queries we rely solely on the presence of the same bind markers used for INSERT to restore custom endpoint names (handled above for INSERT; here we reuse the logic via binds check during result flattening below).
        // Post-process: For non-path graph queries, unwrap single-element primitive arrays produced by Surreal nested projection quirks
        // (e.g. SELECT name FROM (subquery) can yield [{ "name": ["Bob"] }]). We preserve semantic single-element arrays only for Path mode.
        let is_non_path_graph = matches!(query.stmt, junction_rs_core::query::ast::Stmt::Graph(ref g) if !matches!(g.return_mode, junction_rs_core::graph::ReturnMode::Path));
        let is_path_graph = !is_non_path_graph
            && matches!(query.stmt, junction_rs_core::query::ast::Stmt::Graph(_));
        if is_path_graph {
            // Normalize PathSegment shape: Surreal can wrap each projected id in its own single-element array
            // when using SELECT VALUE with nested path expressions. We want `nodes: ["person:uuid", "person:uuid2"]`.
            for val in json_objects.iter_mut() {
                if let serde_json::Value::Object(obj_map) = val {
                    if let Some(serde_json::Value::Array(nodes_arr)) = obj_map.get_mut("nodes") {
                        for node in nodes_arr.iter_mut() {
                            if let serde_json::Value::Array(inner) = node {
                                if inner.len() == 1 && inner[0].is_string() {
                                    *node = inner[0].clone();
                                }
                            }
                        }
                    }
                    // Phase C: perform the same normalization for edges; each edge id is currently wrapped in a single-element array
                    if let Some(serde_json::Value::Array(edges_arr)) = obj_map.get_mut("edges") {
                        for edge in edges_arr.iter_mut() {
                            if let serde_json::Value::Array(inner) = edge {
                                if inner.len() == 1 && inner[0].is_string() {
                                    *edge = inner[0].clone();
                                }
                            }
                        }
                    }
                }
            }
        }
        if is_non_path_graph {
            for val in json_objects.iter_mut() {
                if let serde_json::Value::Object(obj_map) = val {
                    // Collect keys first to avoid borrow issues
                    let keys: Vec<String> = obj_map.keys().cloned().collect();
                    for k in keys {
                        if let Some(serde_json::Value::Array(a)) = obj_map.get(&k) {
                            if a.len() == 1
                                && matches!(
                                    a[0],
                                    serde_json::Value::Null
                                        | serde_json::Value::Bool(_)
                                        | serde_json::Value::Number(_)
                                        | serde_json::Value::String(_)
                                )
                            {
                                obj_map.insert(k.clone(), a[0].clone());
                            }
                        }
                    }
                }
            }
        }
        // Client-side distinct for implicit star distinct queries (no GROUP BY) signaled by sentinel.
        let array_value = if compiled.sql.contains("jxn_distinct_star") {
            let mut seen = std::collections::HashSet::new();
            let mut filtered: Vec<serde_json::Value> = Vec::new();
            for obj in json_objects.into_iter() {
                if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                    if seen.insert(id.to_string()) {
                        filtered.push(obj);
                    }
                } else {
                    filtered.push(obj);
                }
            }
            serde_json::Value::Array(filtered)
        } else {
            serde_json::Value::Array(json_objects)
        };
        let values: Vec<T> = serde_json::from_value(array_value)
            .map_err(|e| DbError::Adapter(format!("Serialization error: {}", e)))?;
        Ok(values)
    }
}
