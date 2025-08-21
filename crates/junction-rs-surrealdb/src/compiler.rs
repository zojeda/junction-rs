use std::collections::BTreeMap;

use junction_rs_core::traits::DbError;
use serde_json::Value;
use uuid::Uuid;

use junction_rs_core::query::ast::{
    Distinct, GroupBySpec, Ordering, PredExpr, Query, SelectField, Stmt,
};

/// Result of compiling a backend-agnostic AST into SurrealQL text and bound params.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledText {
    pub sql: String,
    pub params: BTreeMap<String, Value>,
}
/// Compile a [`junction_rs_core::query::ast::Query`] into SurrealQL plus parameters.
pub fn compile_to_surql(q: &Query) -> Result<CompiledText, DbError> {
    let mut params: BTreeMap<String, Value> = BTreeMap::new();
    let mut pc = 0usize;
    let sql = match &q.stmt {
        Stmt::Select(s) => {
            // Render projection
            let proj = if s.fields.is_empty() {
                "*".to_string()
            } else {
                let parts: Vec<String> = s
                    .fields
                    .iter()
                    .map(|f| match f {
                        SelectField::Star => "*".to_string(),
                        SelectField::Column(c) => c.to_string(),
                        SelectField::Expr { expr, alias } => match alias {
                            Some(a) => format!("{} AS {}", expr, a),
                            None => expr.clone(),
                        },
                        SelectField::CountAll { alias } => format!("count() AS {}", alias),
                        SelectField::CountCol { col, alias } => {
                            format!("count({}) AS {}", col, alias)
                        }
                    })
                    .collect();
                parts.join(", ")
            };
            let mut group_parts: Vec<String> = s
                .group_by
                .iter()
                .map(|g| match g {
                    GroupBySpec::Column(c) => c.to_string(),
                    GroupBySpec::Expr(e) => e.clone(),
                })
                .collect();
            let mut star_distinct = false; // implicit/star distinct -> client side dedupe
            let select_kw = match &s.distinct {
                None => "SELECT".to_string(),
                Some(Distinct::All) => {
                    if !group_parts.is_empty() {
                        return Err(DbError::Adapter(
                            "distinct with additional group_by not yet supported".into(),
                        ));
                    }
                    // Detect implicit or explicit sole star projection for client-side dedupe.
                    if s.fields.is_empty()
                        || (s.fields.len() == 1 && matches!(s.fields[0], SelectField::Star))
                    {
                        star_distinct = true; // no GROUP BY injection; adapter will dedupe on id
                    } else {
                        let cols = simple_projection_columns(&s.fields)?;
                        for col in cols {
                            if !group_parts.iter().any(|g| g == col) {
                                group_parts.push(col.to_string());
                            }
                        }
                    }
                    "SELECT".to_string()
                }
                Some(Distinct::On(cols)) => {
                    if cols.is_empty() {
                        return Err(DbError::Adapter(
                            "distinct_on requires at least one column".into(),
                        ));
                    }
                    if !group_parts.is_empty() {
                        return Err(DbError::Adapter(
                            "distinct_on with additional group_by not yet supported".into(),
                        ));
                    }
                    let projected = simple_projection_columns(&s.fields)?;
                    if projected != *cols {
                        return Err(DbError::Adapter(
                            "distinct_on columns must match projected columns and order".into(),
                        ));
                    }
                    for col in cols {
                        if !group_parts.iter().any(|g| g == col) {
                            group_parts.push(col.to_string());
                        }
                    }
                    "SELECT".to_string()
                }
            };
            // FROM target: table or specific record reference. SurrealQL requires wrapping
            // record ids containing non-identifier characters (hyphenated UUIDs, etc.) in
            // angle brackets otherwise the parser errors on the first non-alphanumeric
            // token. We normalize here so the caller can pass either raw UUID strings or
            // already-formatted record ids.
            let mut base = if let Some(id) = &s.record_id {
                normalize_record_target(s.table, id)
            } else {
                s.table.to_string()
            };
            // Append traversal segments directly to the FROM target so queries like
            // `SELECT * FROM person:<id>->purchased->product` return terminal node rows.
            if !s.traversal.is_empty() {
                for seg in &s.traversal {
                    match seg.dir {
                        junction_rs_core::query::select::TraversalDir::Out => {
                            base.push_str(&format!("->{}", seg.edge));
                            if let Some(node) = seg.node {
                                base.push_str(&format!("->{}", node));
                            }
                        }
                        junction_rs_core::query::select::TraversalDir::In => {
                            base.push_str(&format!("<-{}", seg.edge));
                            if let Some(node) = seg.node {
                                base.push_str(&format!("<-{}", node));
                            }
                        }
                    }
                }
            }
            let mut out = format!("{} {} FROM {}", select_kw, proj, base);
            if let Some(pred) = &s.filter {
                out.push_str(&format!(
                    " WHERE {}",
                    render_pred(pred, &mut pc, &mut params)
                ));
            }
            if !group_parts.is_empty() {
                out.push_str(&format!(" GROUP BY {}", group_parts.join(", ")));
            }
            if star_distinct {
                // Append sentinel comment so adapter can apply client-side dedupe.
                out.push_str(" -- jxn_distinct_star");
            }
            if !s.order_by.is_empty() {
                let parts: Vec<_> = s
                    .order_by
                    .iter()
                    .map(|o| match o {
                        Ordering::Asc(c) => format!("{} ASC", c),
                        Ordering::Desc(c) => format!("{} DESC", c),
                    })
                    .collect();
                out.push_str(&format!(" ORDER BY {}", parts.join(", ")));
            }
            if let Some(n) = s.limit {
                out.push_str(&format!(" LIMIT {}", n));
            }
            out
        }
        Stmt::Insert(i) => {
            let mut values = if let Some(obj) = q.binds.get("values") {
                obj.clone()
            } else {
                i.values.clone()
            };
            // If endpoint metadata present, synthesize Surreal relation endpoint keys (`in`/`out`) when missing.
            if let Some((src_field, tgt_field)) = i.endpoint_fields {
                match &mut values {
                    Value::Array(arr) => {
                        for item in arr.iter_mut() {
                            maybe_inject_relation_keys(item, src_field, tgt_field);
                        }
                    }
                    Value::Object(_) => {
                        maybe_inject_relation_keys(&mut values, src_field, tgt_field);
                    }
                    _ => {}
                }
            }
            normalize_insert_values(i.table, &mut values);
            if let Some(sql) = try_compile_relational_insert(i.table, &values, &mut params) {
                return Ok(CompiledText { sql, params });
            }
            params.insert("values".into(), values);
            format!("INSERT INTO {} $values RETURN *", i.table)
        }
        Stmt::Update(u) => {
            let mut out = match &u.merge {
                Some(v) => {
                    params.insert("content".into(), v.clone());
                    format!("UPDATE {} MERGE $content", u.table)
                }
                None => format!("UPDATE {}", u.table),
            };
            if let Some(pred) = &u.filter {
                out.push_str(&format!(
                    " WHERE {}",
                    render_pred(pred, &mut pc, &mut params)
                ));
            }
            out.push_str(" RETURN AFTER");
            out
        }
        Stmt::Delete(d) => {
            let mut out = format!("DELETE {}", d.table);
            if let Some(pred) = &d.filter {
                out.push_str(&format!(
                    " WHERE {}",
                    render_pred(pred, &mut pc, &mut params)
                ));
            }
            out
        }
        Stmt::Graph(g) => {
            // We may need to materialize multiple path length variants due to variable-length repeat.
            // Strategy: build a vector of step sequences representing alternative expansions.
            // We only support variability at the tail (last step with repeat_extra). Enumerate base_min..=base_min+extra.
            const REPEAT_MAX_CAP: usize = 10; // soft safety cap; revisit once we have perf data
            let mut variants: Vec<Vec<junction_rs_core::graph::TraversalStep>> = Vec::new();
            if let Some((last_idx, last_step)) = g
                .steps
                .iter()
                .enumerate()
                .rfind(|(_, s)| s.repeat_extra.is_some())
            {
                let extra = last_step.repeat_extra.unwrap();
                if extra > REPEAT_MAX_CAP {
                    return Err(DbError::Adapter(format!(
                        "ReturnMode::Path repeat range too large (extra {} > cap {})",
                        extra, REPEAT_MAX_CAP
                    )));
                }

                // Count how many occurrences of the repeated step are already materialized (minimum bound).
                let mut min_occurrences: usize = 1;
                let mut cursor = last_idx;
                let mut template = last_step.clone();
                template.repeat_extra = None;
                while cursor > 0 {
                    let mut prev = g.steps[cursor - 1].clone();
                    prev.repeat_extra = None;
                    if prev == template {
                        min_occurrences += 1;
                        cursor -= 1;
                    } else {
                        break;
                    }
                }
                let repeat_start = last_idx + 1 - min_occurrences;
                let prefix: Vec<junction_rs_core::graph::TraversalStep> =
                    g.steps[..repeat_start].to_vec();
                for total in min_occurrences..=min_occurrences + extra {
                    let mut variant = prefix.clone();
                    for _ in 0..total {
                        variant.push(template.clone());
                    }
                    variants.push(variant);
                }
            } else {
                variants.push(g.steps.clone());
            }

            let mut variant_sql_parts: Vec<String> = Vec::new();
            for steps in &variants {
                let mut base = if let Some(id) = &g.start.id {
                    normalize_record_target(g.start.table, id)
                } else {
                    g.start.table.to_string()
                };
                // Track node base BEFORE final step (needed for edge return) and also reference to final step.
                let mut node_bases: Vec<String> = Vec::new();
                for step in steps.iter() {
                    // Before applying this step's edge traversal, capture current node base.
                    node_bases.push(base.clone());
                    // Collect edge-level predicates for this step.
                    let mut edge_preds: Vec<String> = Vec::new();
                    for f in &step.filters {
                        if let junction_rs_core::graph::StepFilter::Edge(expr) = f {
                            edge_preds.push(render_graph_expr(expr, &mut pc, &mut params));
                        }
                    }
                    let has_edge_filters = !edge_preds.is_empty();
                    // Build edge arrow chain (used when returning nodes)
                    let mut edge_chain = String::new();
                    for edge in &step.edge_set.edges {
                        let arrow = match edge.direction {
                            junction_rs_core::graph::Direction::Out => format!("->{}", edge.table),
                            junction_rs_core::graph::Direction::In => format!("<-{}", edge.table),
                        };
                        edge_chain.push_str(&arrow);
                    }
                    if has_edge_filters {
                        let pred = if edge_preds.len() == 1 {
                            edge_preds[0].clone()
                        } else {
                            format!("({})", edge_preds.join(" AND "))
                        };
                        base = format!("(SELECT * FROM {}{} WHERE {})", base, edge_chain, pred);
                    } else {
                        base.push_str(&edge_chain);
                    }
                    if let Some(node_tbl) = step.dest_node_table {
                        // Preserve traversal direction when emitting the implicit hop to the destination node table.
                        // Previously this always used an outward arrow (->node) which produced invalid inbound chains like
                        //   ...<-edge->node
                        // SurrealQL expects the node hop arrow to match the edge direction for inbound traversals:
                        //   ...<-edge<-node
                        // We inspect the first edge in the set (all share direction by construction) to choose arrow.
                        if let Some(first_edge) = step.edge_set.edges.first() {
                            match first_edge.direction {
                                junction_rs_core::graph::Direction::Out => {
                                    base.push_str(&format!("->{}", node_tbl));
                                }
                                junction_rs_core::graph::Direction::In => {
                                    base.push_str(&format!("<-{}", node_tbl));
                                }
                            }
                        } else {
                            // Fallback (should not happen): default to outward to avoid silent omission.
                            base.push_str(&format!("->{}", node_tbl));
                        }
                    }
                    // step_index no longer used after refactor
                }
                // Build select fragment depending on return mode.
                let select_fragment = match &g.return_mode {
                    junction_rs_core::graph::ReturnMode::Nodes => format!("SELECT * FROM {}", base),
                    junction_rs_core::graph::ReturnMode::Project(_) => {
                        format!("SELECT * FROM {}", base)
                    }
                    junction_rs_core::graph::ReturnMode::ProjectFields(cols) => {
                        if cols.is_empty() {
                            format!("SELECT * FROM {}", base)
                        } else {
                            format!("SELECT {} FROM {}", cols.join(", "), base)
                        }
                    }
                    junction_rs_core::graph::ReturnMode::Edges => {
                        // Use the last step of this variant to construct edge selection.
                        if steps.is_empty() {
                            format!("SELECT * FROM {}", base)
                        } else {
                            let last_step = steps.last().unwrap();
                            // Node base before last step traversal
                            let prev_node_base =
                                node_bases.last().cloned().unwrap_or_else(|| base.clone());
                            // Collect edge filters for last step
                            let mut edge_pred_parts: Vec<String> = Vec::new();
                            for f in &last_step.filters {
                                if let junction_rs_core::graph::StepFilter::Edge(expr) = f {
                                    edge_pred_parts.push(render_graph_expr(
                                        expr,
                                        &mut pc,
                                        &mut params,
                                    ));
                                }
                            }
                            // Build one SELECT per edge in union
                            let mut edge_selects: Vec<String> = Vec::new();
                            for edge in &last_step.edge_set.edges {
                                // Surreal stores relation orientation with `in` pointing to the source (left side of RELATE) and `out` to the destination.
                                // Therefore for an outward traversal (Direction::Out) we anchor on `in` (start node id), and for inward traversal on `out`.
                                let endpoint_field = match edge.direction {
                                    junction_rs_core::graph::Direction::Out => "in",
                                    junction_rs_core::graph::Direction::In => "out",
                                };
                                let simple_base = !prev_node_base.contains(' ')
                                    && !prev_node_base.contains('(')
                                    && !prev_node_base.contains(')');
                                let anchor_pred = if simple_base {
                                    if let Some((tbl, rest)) = prev_node_base.split_once(':') {
                                        let idseg = rest
                                            .trim()
                                            .trim_start_matches('<')
                                            .trim_end_matches('>')
                                            .trim_start_matches('⟨')
                                            .trim_end_matches('⟩');
                                        format!(
                                            "{} = type::thing(\"{}\", \"{}\")",
                                            endpoint_field, tbl, idseg
                                        )
                                    } else {
                                        prev_node_base.clone()
                                    }
                                } else {
                                    format!(
                                        "{} IN (SELECT id FROM {})",
                                        endpoint_field, prev_node_base
                                    )
                                };
                                let mut preds: Vec<String> = vec![anchor_pred];
                                if !edge_pred_parts.is_empty() {
                                    preds.extend(edge_pred_parts.clone());
                                }
                                let where_clause = if preds.is_empty() {
                                    String::new()
                                } else {
                                    format!(" WHERE {}", preds.join(" AND "))
                                };
                                edge_selects
                                    .push(format!("SELECT * FROM {}{}", edge.table, where_clause));
                            }
                            if edge_selects.len() == 1 {
                                edge_selects.remove(0)
                            } else {
                                edge_selects.join(" UNION ALL ")
                            }
                        }
                    }
                    junction_rs_core::graph::ReturnMode::Path => {
                        // Single-hop: capture both node ids and single edge id.
                        if steps.len() == 1 {
                            let last_step = steps.last().unwrap();
                            let prev_node_base =
                                node_bases.last().cloned().unwrap_or_else(|| base.clone());
                            let mut path_selects: Vec<String> = Vec::new();
                            for edge in &last_step.edge_set.edges {
                                let anchor_field = match edge.direction {
                                    junction_rs_core::graph::Direction::Out => "in",
                                    junction_rs_core::graph::Direction::In => "out",
                                };
                                let simple_base = !prev_node_base.contains(' ')
                                    && !prev_node_base.contains('(')
                                    && !prev_node_base.contains(')');
                                let anchor_pred = if simple_base {
                                    if let Some((tbl, rest)) = prev_node_base.split_once(':') {
                                        let idseg = rest
                                            .trim()
                                            .trim_start_matches('<')
                                            .trim_end_matches('>')
                                            .trim_start_matches('⟨')
                                            .trim_end_matches('⟩');
                                        format!(
                                            "{} = type::thing(\"{}\", \"{}\")",
                                            anchor_field, tbl, idseg
                                        )
                                    } else {
                                        prev_node_base.clone()
                                    }
                                } else {
                                    format!(
                                        "{} IN (SELECT id FROM {})",
                                        anchor_field, prev_node_base
                                    )
                                };
                                let mut edge_pred_parts: Vec<String> = Vec::new();
                                for f in &last_step.filters {
                                    if let junction_rs_core::graph::StepFilter::Edge(expr) = f {
                                        edge_pred_parts.push(render_graph_expr(
                                            expr,
                                            &mut pc,
                                            &mut params,
                                        ));
                                    }
                                }
                                let mut preds: Vec<String> = vec![anchor_pred];
                                if !edge_pred_parts.is_empty() {
                                    preds.extend(edge_pred_parts);
                                }
                                let where_clause = format!(" WHERE {}", preds.join(" AND "));
                                let (src_field, dst_field) = match edge.direction {
                                    junction_rs_core::graph::Direction::Out => ("in", "out"),
                                    junction_rs_core::graph::Direction::In => ("out", "in"),
                                };
                                path_selects.push(format!("SELECT VALUE {{ nodes: [{} .id, {} .id], edges: [id] }} FROM {}{}", src_field, dst_field, edge.table, where_clause));
                            }
                            if path_selects.len() == 1 {
                                path_selects.remove(0)
                            } else {
                                path_selects.join(" UNION ALL ")
                            }
                        } else {
                            // Multi-hop Phase C: full node sequence capture + edge id list (len = nodes.len()-1).
                            // Constraints (still enforced): single outward edge per step, no edge-level filters.
                            for step in steps {
                                if step.edge_set.edges.len() != 1 {
                                    return Err(DbError::Adapter("ReturnMode::Path multi-hop (phase C) requires exactly one edge per step (no unions)".into()));
                                }
                                for e in &step.edge_set.edges {
                                    if matches!(e.direction, junction_rs_core::graph::Direction::In)
                                    {
                                        return Err(DbError::Adapter("ReturnMode::Path multi-hop (phase C) supports only outward edges".into()));
                                    }
                                }
                                for f in &step.filters {
                                    if matches!(f, junction_rs_core::graph::StepFilter::Edge(_)) {
                                        return Err(DbError::Adapter("ReturnMode::Path multi-hop (phase C) does not yet support edge filters".into()));
                                    }
                                }
                            }
                            // Require explicit concrete start id for now (enumerating from all nodes would explode path counts).
                            let start_id_raw = if let Some(id) = &g.start.id {
                                id
                            } else {
                                return Err(DbError::Adapter("ReturnMode::Path multi-hop (phase C) currently requires an explicit start id".into()));
                            };
                            let start_norm = normalize_record_target(g.start.table, start_id_raw);
                            // Rebuild simple outward chain bases after each step (since constraints guarantee no filters / unions / inward edges).
                            let mut bases: Vec<String> = Vec::new(); // node frontier chains after each step
                            let mut edge_chains: Vec<String> = Vec::new(); // chain at edge table (before optional dest node table)
                            let mut current = start_norm.clone();
                            for step in steps {
                                let edge = &step.edge_set.edges[0];
                                // Edge chain reference (before adding dest node table)
                                let mut edge_chain = current.clone();
                                edge_chain.push_str(&format!("->{}", edge.table));
                                edge_chains.push(edge_chain.clone());
                                current.push_str(&format!("->{}", edge.table));
                                if let Some(node_tbl) = step.dest_node_table {
                                    current.push_str(&format!("->{}", node_tbl));
                                }
                                bases.push(current.clone()); // path after this step
                            }
                            // Create node id expressions: first element is the start id, followed by id of each base in order.
                            let mut node_exprs: Vec<String> = Vec::with_capacity(bases.len() + 1);
                            // start id extraction: ensure we always refer to the .id to keep consistency with other entries.
                            if let Some((_, id_seg)) = start_norm.split_once(':') {
                                let plain_id = id_seg
                                    .trim()
                                    .trim_start_matches('⟨')
                                    .trim_end_matches('⟩')
                                    .trim_start_matches('<')
                                    .trim_end_matches('>');
                                node_exprs.push(format!(
                                    "type::thing(\"{}\", \"{}\").id",
                                    g.start.table, plain_id
                                ));
                            } else {
                                node_exprs.push(format!("{}.id", start_norm));
                            }
                            for b in &bases {
                                // b is a simple chain (no spaces/parentheses) so we can append .id directly.
                                node_exprs.push(format!("{}.id", b));
                            }
                            // Build edge id expressions in traversal order using subselects over each edge_chain.
                            let mut edge_exprs: Vec<String> = Vec::with_capacity(edge_chains.len());
                            for ec in &edge_chains {
                                // SELECT VALUE id FROM <chain> LIMIT 1 returns array with single id; rely on LIMIT 1 to constrain.
                                // Some Surreal versions return single value (non-array) for SELECT VALUE with LIMIT 1; avoid indexing to stay compatible.
                                edge_exprs.push(format!("(SELECT VALUE id FROM {} LIMIT 1)", ec));
                            }
                            // Provide a FROM clause anchored at the last base (terminal frontier) to satisfy SurrealQL parser.
                            // Using LIMIT 1 prevents cartesian expansion; bases already deterministically follow a single chain under constraints.
                            let terminal_base = bases.last().cloned().unwrap_or(start_norm.clone());
                            format!(
                                "SELECT VALUE {{ nodes: [{}], edges: [{}] }} FROM {} LIMIT 1",
                                node_exprs.join(", "),
                                edge_exprs.join(", "),
                                terminal_base
                            )
                        }
                    }
                };
                variant_sql_parts.push(select_fragment);
            }
            let mut out = if variant_sql_parts.len() == 1 {
                variant_sql_parts.remove(0)
            } else {
                let mut prelude = String::new();
                let mut variant_refs: Vec<String> = Vec::with_capacity(variant_sql_parts.len());
                for (idx, fragment) in variant_sql_parts.iter().enumerate() {
                    let var = format!("pv{}", idx);
                    prelude.push_str(&format!("LET ${} = ({});", var, fragment));
                    variant_refs.push(format!("${}", var));
                }
                prelude.push_str(&format!(
                    "SELECT * FROM array::flatten([{}])",
                    variant_refs.join(", ")
                ));
                prelude
            };
            // Node filters become outer WHERE predicates
            if !matches!(g.return_mode, junction_rs_core::graph::ReturnMode::Edges) {
                let mut first = true;
                for step in &g.steps {
                    // apply filters across full max-length variant (simplification)
                    for f in &step.filters {
                        if let junction_rs_core::graph::StepFilter::Node(expr) = f {
                            let pred = render_graph_expr(expr, &mut pc, &mut params);
                            if first {
                                out.push_str(" WHERE ");
                                first = false;
                            } else {
                                out.push_str(" AND ");
                            }
                            out.push_str(&pred);
                        }
                    }
                }
            }
            // Debug log behind cfg(test) to reduce noise in normal usage
            #[cfg(test)]
            eprintln!("[junction-rs-surrealdb][graph] {}", out);
            out
        }
    };
    Ok(CompiledText { sql, params })
}

fn simple_projection_columns(fields: &[SelectField]) -> Result<Vec<&'static str>, DbError> {
    // Semantics:
    //  * Empty projection (implicit `*`) => treat as grouping by the synthetic `id` column
    //    which all Nodes / Edges expose. This enables the ergonomic pattern:
    //      select::<T>(All).from(...).distinct()
    //    to simply deduplicate returned rows by primary id.
    //  * Explicit `*` (single Star) behaves the same as empty.
    //  * Explicit simple column list => those columns are used.
    //  * Any expression / count / mixed star+columns => currently unsupported.
    if fields.is_empty() {
        return Ok(vec!["id"]);
    }
    if fields.len() == 1 {
        if let SelectField::Star = fields[0] {
            return Ok(vec!["id"]);
        }
    }
    let mut cols = Vec::new();
    for field in fields {
        match field {
            SelectField::Column(col) => cols.push(*col),
            SelectField::Star => {
                return Err(DbError::Adapter(
                    "distinct with star plus other fields currently unsupported".into(),
                ));
            }
            _ => {
                return Err(DbError::Adapter(
                    "distinct currently supports only simple column projections or implicit star"
                        .into(),
                ));
            }
        }
    }
    if cols.is_empty() {
        Ok(vec!["id"])
    } else {
        Ok(cols)
    }
}

fn normalize_insert_values(table: &str, values: &mut Value) {
    match values {
        Value::Array(arr) => {
            for item in arr {
                normalize_insert_object(table, item);
            }
        }
        Value::Object(_) => normalize_insert_object(table, values),
        _ => {}
    }
}

fn normalize_insert_object(table: &str, value: &mut Value) {
    if let Value::Object(map) = value {
        if let Some(id_value) = map.get_mut("id") {
            normalize_id_field(table, id_value);
        }
        if let Some(in_value) = map.get_mut("in") {
            normalize_relation_endpoint(in_value);
        }
        if let Some(out_value) = map.get_mut("out") {
            normalize_relation_endpoint(out_value);
        }
    }
}

fn maybe_inject_relation_keys(obj: &mut Value, src_field: &str, tgt_field: &str) {
    if let Value::Object(map) = obj {
        // Only inject if Surreal keys absent to avoid overwriting explicit user-provided orientation.
        let has_in = map.contains_key("in");
        let has_out = map.contains_key("out");
        if !(has_in && has_out) {
            let src_val = map.get(src_field).cloned();
            let tgt_val = map.get(tgt_field).cloned();
            if !has_in {
                if let Some(v) = src_val.clone() {
                    map.insert("in".to_string(), v);
                }
            }
            if !has_out {
                if let Some(v) = tgt_val.clone() {
                    map.insert("out".to_string(), v);
                }
            }
            // Remove original endpoint fields if they differ to keep payload minimal and prevent user-level names being mistaken for unrelated columns.
            if src_field != "in" {
                map.remove(src_field);
            }
            if tgt_field != "out" {
                map.remove(tgt_field);
            }
        }
    }
}

fn normalize_id_field(table: &str, id_value: &mut Value) {
    match id_value {
        Value::String(s) => {
            if let Some(normalized) = strip_table_prefix(table, s) {
                *id_value = Value::String(normalized);
            }
        }
        Value::Object(obj) => {
            if let Some(tb) = obj.get("tb").and_then(Value::as_str) {
                if tb == table {
                    if let Some(id) = obj.get_mut("id") {
                        normalize_id_field(table, id);
                    }
                }
            }
        }
        _ => {}
    }
}

fn normalize_relation_endpoint(value: &mut Value) {
    match value {
        Value::String(s) => {
            if s.trim().starts_with("type::thing(") {
                return;
            }
            if let Some((table_part, rest)) = s.split_once(':') {
                let cleaned = rest
                    .trim()
                    .trim_start_matches('⟨')
                    .trim_end_matches('⟩')
                    .trim_start_matches('<')
                    .trim_end_matches('>');
                *value = serde_json::json!({
                    "tb": table_part.trim(),
                    "id": cleaned,
                });
            }
        }
        Value::Object(obj) => {
            if let Some(id) = obj.get_mut("id") {
                normalize_relation_endpoint(id);
            }
        }
        _ => {}
    }
}

fn try_compile_relational_insert(
    table: &str,
    values: &Value,
    params: &mut BTreeMap<String, Value>,
) -> Option<String> {
    let arr = values.as_array()?;
    if arr.is_empty() {
        return None;
    }
    if !arr.iter().all(|item| {
        item.as_object()
            .is_some_and(|obj| obj.contains_key("in") && obj.contains_key("out"))
    }) {
        return None;
    }
    use std::collections::BTreeMap as Map;
    let mut endpoints: Map<(String, String), String> = Map::new();
    let mut var_index = 0usize;
    let mut param_index = 0usize;
    let mut sql = String::new();

    for item in arr {
        let obj = item.as_object()?;
        let (from_table, from_id) = extract_endpoint(obj.get("in")?)?;
        let (to_table, to_id) = extract_endpoint(obj.get("out")?)?;
        endpoints
            .entry((from_table.clone(), from_id.clone()))
            .or_insert_with(|| {
                let name = format!("v{}", var_index);
                var_index += 1;
                sql.push_str(&format!(
                    "LET ${} = type::thing(\"{}\", \"{}\");",
                    name, from_table, from_id
                ));
                name
            });
        endpoints
            .entry((to_table.clone(), to_id.clone()))
            .or_insert_with(|| {
                let name = format!("v{}", var_index);
                var_index += 1;
                sql.push_str(&format!(
                    "LET ${} = type::thing(\"{}\", \"{}\");",
                    name, to_table, to_id
                ));
                name
            });
    }

    for item in arr {
        let obj = item.as_object()?;
        let (from_table, from_id) = extract_endpoint(obj.get("in")?)?;
        let (to_table, to_id) = extract_endpoint(obj.get("out")?)?;
        let from_var = endpoints.get(&(from_table, from_id)).cloned()?;
        let to_var = endpoints.get(&(to_table, to_id)).cloned()?;
        let mut stmt = format!("RELATE ${}->{}->${}", from_var, table, to_var);
        let mut set_parts = Vec::new();
        for (key, value) in obj {
            if key == "in" || key == "out" {
                continue;
            }
            param_index += 1;
            let pname = format!("rv{}", param_index);
            params.insert(pname.clone(), value.clone());
            set_parts.push(format!("{} = ${}", key, pname));
        }
        if !set_parts.is_empty() {
            stmt.push_str(" SET ");
            stmt.push_str(&set_parts.join(", "));
        }
        stmt.push(';');
        sql.push_str(&stmt);
    }

    sql.push_str(&format!("SELECT * FROM {};", table));
    Some(sql)
}

fn extract_endpoint(value: &Value) -> Option<(String, String)> {
    match value {
        Value::String(s) => parse_endpoint_string(s),
        Value::Object(obj) => {
            let tb = obj.get("tb")?.as_str()?.trim().to_string();
            let id_val = obj.get("id")?;
            match id_val {
                Value::String(s) => Some((tb, sanitize_id_segment(s))),
                _ => None,
            }
        }
        _ => None,
    }
}

fn parse_endpoint_string(raw: &str) -> Option<(String, String)> {
    let trimmed = raw.trim();
    if trimmed.starts_with("type::thing(") {
        let inner = trimmed
            .trim_start_matches("type::thing(")
            .trim_end_matches(')');
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() >= 2 {
            let table = parts[0].trim().trim_matches('\"');
            let id = parts[1].trim().trim_matches('\"');
            return Some((table.to_string(), sanitize_id_segment(id)));
        }
        return None;
    }
    let (table, id) = trimmed.split_once(':')?;
    Some((table.trim().to_string(), sanitize_id_segment(id)))
}

fn sanitize_id_segment(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('⟨')
        .trim_end_matches('⟩')
        .trim_start_matches('<')
        .trim_end_matches('>')
        .to_string()
}

fn strip_table_prefix(table: &str, raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.starts_with("type::thing(") {
        return None;
    }
    let cleaned = trimmed
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim_start_matches('⟨')
        .trim_end_matches('⟩');
    let parts: Vec<&str> = cleaned.split(':').collect();
    if parts.len() < 2 {
        return None;
    }
    if parts.first()? != &table {
        return None;
    }
    let candidate = parts
        .last()?
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim_start_matches('⟨')
        .trim_end_matches('⟩');
    if Uuid::parse_str(candidate).is_ok() {
        Some(candidate.to_string())
    } else {
        None
    }
}

fn escape_record_id_segment(id: &str) -> String {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return trimmed.to_string();
    }
    if (trimmed.starts_with('<') && trimmed.ends_with('>'))
        || (trimmed.starts_with('⟨') && trimmed.ends_with('⟩'))
    {
        trimmed.to_string()
    } else if Uuid::parse_str(trimmed).is_ok() {
        // Wrap UUIDs in angle brackets to avoid parser interpreting hyphen as minus in complex paths.
        format!("⟨{}⟩", trimmed)
    } else if trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        trimmed.to_string()
    } else {
        format!("⟨{}⟩", trimmed)
    }
}

fn normalize_record_target(table: &str, raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return table.to_string();
    }
    if trimmed.starts_with("type::thing(") {
        return trimmed.to_string();
    }
    if trimmed.contains("->") || trimmed.contains("<-") {
        return trimmed.to_string();
    }
    if let Some((table_part, rest)) = trimmed.split_once(':') {
        let target_table = if table_part.is_empty() {
            table
        } else {
            table_part
        };
        let id_segment = escape_record_id_segment(rest);
        format!("{}:{}", target_table, id_segment)
    } else {
        let id_segment = escape_record_id_segment(trimmed);
        format!("{}:{}", table, id_segment)
    }
}

fn render_pred(p: &PredExpr, pc: &mut usize, params: &mut BTreeMap<String, Value>) -> String {
    match p {
        PredExpr::True => "true".to_string(),
        PredExpr::Binary { col, op, param } => {
            let (key, val) = match param {
                junction_rs_core::query::ast::Param::Inline(v) => {
                    *pc += 1;
                    (format!("p{}", pc), v.clone())
                }
                junction_rs_core::query::ast::Param::Named(name) => (
                    name.clone(),
                    params.get(name).cloned().unwrap_or(Value::Null),
                ),
            };
            params.insert(key.clone(), val);
            format!("{} {} ${}", col, op, key)
        }
        PredExpr::And(a, b) => format!(
            "({} AND {})",
            render_pred(a, pc, params),
            render_pred(b, pc, params)
        ),
        PredExpr::Or(a, b) => format!(
            "({} OR {})",
            render_pred(a, pc, params),
            render_pred(b, pc, params)
        ),
        PredExpr::Not(a) => format!("(NOT {})", render_pred(a, pc, params)),
    }
}

// Lower high-level Expr (used in graph filters) into SurrealQL predicate text with parameter binding similar to render_pred but operating directly on Expr.
fn render_graph_expr(
    e: &junction_rs_core::Expr,
    pc: &mut usize,
    params: &mut BTreeMap<String, Value>,
) -> String {
    use junction_rs_core::expr::ExprKind as K;
    match &e.0 {
        K::True => "true".to_string(),
        K::Binary { col, op, val, .. } => {
            *pc += 1;
            let key = format!("p{}", pc);
            params.insert(key.clone(), val.clone());
            format!("{} {} ${}", col, op, key)
        }
        K::And(a, b) => format!(
            "({} AND {})",
            render_graph_expr(a, pc, params),
            render_graph_expr(b, pc, params)
        ),
        K::Or(a, b) => format!(
            "({} OR {})",
            render_graph_expr(a, pc, params),
            render_graph_expr(b, pc, params)
        ),
        K::Not(a) => format!("(NOT {})", render_graph_expr(a, pc, params)),
    }
}
