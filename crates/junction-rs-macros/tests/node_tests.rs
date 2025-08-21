#[path = "support/junction_rs.rs"]
mod junction_rs;

use junction_rs::traits::Node;
use junction_rs_macros::Node as NodeMacro;

#[allow(dead_code)]
#[derive(NodeMacro)]
struct Person {
    name: String,
    age: i32,
}

#[test]
fn node_macro_sets_table_and_schema_columns() {
    assert_eq!(Person::TABLE, "person");

    let schema = Person::schema();
    assert_eq!(schema.id.table, Person::TABLE);
    assert_eq!(schema.id.column, "id");
    assert_eq!(schema.name.table, Person::TABLE);
    assert_eq!(schema.name.column, "name");
    assert_eq!(schema.age.table, Person::TABLE);
    assert_eq!(schema.age.column, "age");
}

#[test]
fn node_macro_provides_simple_id_constructor() {
    let _id = Person::create_simple_id();
}
