use junction_rs_core::expr::ExprKind;

#[derive(Clone)]
struct DummySchema {
    pub age: junction_rs_core::schema::Col<i32>,
    pub name: junction_rs_core::schema::Col<String>,
}

fn schema() -> DummySchema {
    DummySchema {
        age: junction_rs_core::schema::Col::new("person", "age"),
        name: junction_rs_core::schema::Col::new("person", "name"),
    }
}

#[test]
fn col_between_helper() {
    let s = schema();
    let e = s.age.between(5, 15);
    match e.0 {
        ExprKind::And(_, _) => {}
        _ => panic!("expected And composite"),
    }
}

#[test]
fn col_isin_helper() {
    let s = schema();
    let e = s.age.isin(vec![1, 2, 3]);
    match e.0 {
        ExprKind::Binary { op, .. } => assert_eq!(op, "IN"),
        _ => panic!("expected binary"),
    }
}

#[test]
fn col_like_helper() {
    let s = schema();
    let e = s.name.like("%a%");
    match e.0 {
        ExprKind::Binary { op, .. } => assert_eq!(op, "LIKE"),
        _ => panic!("expected binary"),
    }
}
