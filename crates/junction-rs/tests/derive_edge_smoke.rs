use expr::{Expr, ExprKind, Ordering};
use junction_rs::prelude::*;

#[derive(Node, Serialize, Deserialize, Debug, Clone)]
#[junction(table = "space_ship")]
#[serde(rename_all = "camelCase")]
pub struct SpaceShip {
    pub id: SimpleId<SpaceShip>,
    pub name: String,
    pub age: u8,
}

#[derive(Node, Serialize, Deserialize, Debug, Clone)]
#[junction(table = "planet")]
#[serde(rename_all = "camelCase")]
pub struct Planet {
    pub id: SimpleId<Planet>,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Edge)]
#[serde(rename_all = "camelCase")]
#[junction(table = "docked_at")]
pub struct DockedAt {
    #[junction(source)]
    pub r#in: SimpleId<SpaceShip>,
    #[junction(target)]
    pub r#out: SimpleId<Planet>,
    pub since: u16,
}

#[tokio::test]
async fn test_edge_table_and_schema_columns() {
    let et = <DockedAt as Edge>::table();
    assert_eq!(et.name, "docked_at");

    let schema = <DockedAt as Edge>::schema();
    // Ensure newly required in/out columns exist in schema
    let _ = schema.r#in.clone();
    let _ = schema.r#out.clone();
    assert_eq!(schema.since.table, "docked_at");
    assert_eq!(schema.since.name, "since");
}

#[tokio::test]
async fn test_edge_expr_and_ordering() {
    let schema = <DockedAt as Edge>::schema();

    let e = schema.since.clone().gt(2000u16);
    match e {
        Expr(ExprKind::Binary {
            table,
            col,
            op,
            val,
        }) => {
            assert_eq!(table, "docked_at");
            assert_eq!(col, "since");
            assert_eq!(op, ">");
            assert_eq!(val, serde_json::json!(2000));
        }
        other => panic!("unexpected expr: {:?}", other),
    }

    let ord = schema.since.clone().desc();
    match ord {
        Ordering::Desc(t, c) => {
            assert_eq!(t, "docked_at");
            assert_eq!(c, "since");
        }
        _ => panic!("unexpected ordering: {:?}", ord),
    }
}
