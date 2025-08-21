#[path = "support/junction_rs.rs"]
mod junction_rs;

use junction_rs_macros::{Edge as EdgeMacro, Node as NodeMacro};

use junction_rs::id::SimpleId;

#[allow(dead_code)]
#[derive(NodeMacro)]
#[junction(table = "user")]
struct User {
    id: SimpleId<Self>,
    name: String,
}

#[allow(dead_code)]
#[derive(EdgeMacro)]
struct Follows {
    id: String,
    #[junction(source)]
    follower: SimpleId<User>,
    #[junction(target)]
    following: SimpleId<User>,

    since: chrono::DateTime<chrono::Utc>,
}

// #[allow(dead_code)]
// #[derive(EdgeMacro)]
// struct FollowEdge {
//     r#in: SimpleId<User>,
//     r#out: SimpleId<User>,
// }

#[test]
fn edge_macro_sets_table_and_schema_columns() {
    assert_eq!(Follows::TABLE, "follows");

    let schema = Follows::schema();
    assert_eq!(schema.id.table, Follows::TABLE);
    assert_eq!(schema.id.column, "id");
    assert_eq!(schema.follower.table, Follows::TABLE);
    assert_eq!(schema.follower.column, "follower");
    assert_eq!(schema.following.table, Follows::TABLE);
    assert_eq!(schema.following.column, "following");
    assert_eq!(schema.since.column, "since");
}

// #[test]
// fn edge_macro_defaults_table_name_from_struct_identifier() {
//     assert_eq!(FollowEdge::TABLE, "follow_edge");
// }

// #[test]
// fn edge_macro_implements_edge_def() {
//   assert_eq!(<Friendship as EdgeDef>::From::TABLE, User::TABLE);
//   assert_eq!(<Friendship as EdgeDef>::To::TABLE, User::TABLE);
// }
