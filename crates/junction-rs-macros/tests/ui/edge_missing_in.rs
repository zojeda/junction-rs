#[path = "../support/junction_rs.rs"]
mod junction_rs;

use junction_rs::id::SimpleId;
use junction_rs_macros::Edge;

#[derive(Edge)]
struct MissingIn {
    r#out: SimpleId<MissingIn>,
}
