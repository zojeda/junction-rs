use junction_rs_core::expr::{Expr, ExprKind};
use junction_rs_core::schema::Col;
use serde_json::json;

#[test]
fn expr_macro_builds_binary_ops() {
    let age: Col<i32> = Col::new("people", "age");
    // >
    assert_eq!(
        junction_rs_core::expr!(age > 10),
        Expr(ExprKind::Binary {
            table: "people",
            col: "age",
            op: ">",
            val: json!(10)
        })
    );
    // >=
    assert_eq!(
        junction_rs_core::expr!(age >= 18),
        Expr(ExprKind::Binary {
            table: "people",
            col: "age",
            op: ">=",
            val: json!(18)
        })
    );
    // <
    assert_eq!(
        junction_rs_core::expr!(age < 5),
        Expr(ExprKind::Binary {
            table: "people",
            col: "age",
            op: "<",
            val: json!(5)
        })
    );
    // <=
    assert_eq!(
        junction_rs_core::expr!(age <= 21),
        Expr(ExprKind::Binary {
            table: "people",
            col: "age",
            op: "<=",
            val: json!(21)
        })
    );
}

#[test]
fn expr_macro_equality_and_inequality() {
    let name: Col<String> = Col::new("people", "name");
    assert_eq!(
        junction_rs_core::expr!(name.clone() == "Alice"),
        Expr(ExprKind::Binary {
            table: "people",
            col: "name",
            op: "=",
            val: json!("Alice")
        })
    );
    assert_eq!(
        junction_rs_core::expr!(name != "Bob"),
        Expr(ExprKind::Binary {
            table: "people",
            col: "name",
            op: "!=",
            val: json!("Bob")
        })
    );
}

#[test]
fn expr_macro_not_and_parentheses() {
    let name: Col<String> = Col::new("people", "name");
    let base = junction_rs_core::expr!(name.clone() == "Alice");
    let wrapped = junction_rs_core::expr!((name.clone() == "Alice"));
    assert_eq!(base, wrapped);

    let neg = junction_rs_core::expr!(!(name.clone() == "Alice"));
    assert_eq!(
        neg,
        Expr(ExprKind::Not(Box::new(Expr(ExprKind::Binary {
            table: "people",
            col: "name",
            op: "=",
            val: json!("Alice")
        }))))
    );
}

#[test]
fn expr_macro_fallback_expr_passthrough() {
    let age: Col<i32> = Col::new("people", "age");
    let by_method = age.clone().gt(10);
    let by_macro = junction_rs_core::expr!(age > 10);
    // The shape should be identical between builder method and macro form
    assert_eq!(by_method, by_macro);
}
