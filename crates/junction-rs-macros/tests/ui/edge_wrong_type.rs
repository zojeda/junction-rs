#[path = "../support/junction_rs.rs"]
mod junction_rs;

use junction_rs_macros::Edge;

#[derive(Edge)]
struct WrongType {
    r#in: bool,
    r#out: bool,
}
