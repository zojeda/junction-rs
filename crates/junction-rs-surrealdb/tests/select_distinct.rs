use junction_rs_core::query::ast::{Distinct, Query, Select, SelectField, Stmt};
use junction_rs_surrealdb::compiler::compile_to_surql;

#[test]
fn compile_distinct_all_projection() {
    let select = Select {
        table: "person",
        record_id: None,
        fields: vec![SelectField::Column("name")],
        filter: None,
        order_by: vec![],
        limit: None,
        group_by: vec![],
        traversal: vec![],
        distinct: Some(Distinct::All),
    };
    let query = Query::new(Stmt::Select(select));
    let compiled = compile_to_surql(&query).expect("compile distinct");
    assert_eq!(compiled.sql, "SELECT name FROM person GROUP BY name");
}

#[test]
fn compile_distinct_implicit_star() {
    // No fields provided => implicit star; distinct should group by id.
    let select = Select {
        table: "person",
        record_id: None,
        fields: vec![], // implicit *
        filter: None,
        order_by: vec![],
        limit: None,
        group_by: vec![],
        traversal: vec![],
        distinct: Some(Distinct::All),
    };
    let query = Query::new(Stmt::Select(select));
    let compiled = compile_to_surql(&query).expect("compile distinct implicit star");
    assert_eq!(compiled.sql, "SELECT * FROM person -- jxn_distinct_star");
}

#[test]
fn compile_distinct_on_matching_columns() {
    let select = Select {
        table: "product",
        record_id: None,
        fields: vec![SelectField::Column("name"), SelectField::Column("category")],
        filter: None,
        order_by: vec![],
        limit: None,
        group_by: vec![],
        traversal: vec![],
        distinct: Some(Distinct::On(vec!["name", "category"])),
    };
    let query = Query::new(Stmt::Select(select));
    let compiled = compile_to_surql(&query).expect("compile distinct on");
    assert_eq!(
        compiled.sql,
        "SELECT name, category FROM product GROUP BY name, category"
    );
}

#[test]
fn compile_distinct_on_rejects_expr_projection() {
    let select = Select {
        table: "orders",
        record_id: None,
        fields: vec![SelectField::Expr {
            expr: "price * qty".into(),
            alias: Some("total"),
        }],
        filter: None,
        order_by: vec![],
        limit: None,
        group_by: vec![],
        traversal: vec![],
        distinct: Some(Distinct::On(vec!["total"])),
    };
    let query = Query::new(Stmt::Select(select));
    let err = compile_to_surql(&query).expect_err("distinct_on should reject expressions");
    match err {
        junction_rs_core::traits::DbError::Adapter(msg) => {
            assert!(
                msg.contains("supports only simple column projections"),
                "unexpected message: {msg}",
                msg = msg
            );
        }
        other => panic!("unexpected error: {:?}", other),
    }
}
