# JunctionRS Macros

Proc-macros for deriving Node/Edge and generating schema modules for JunctionRS.

## Example

```rust
use serde::{Deserialize, Serialize};
use junction_rs::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
#[junction(table = "user")]
struct User {
    id: SimpleId<User>,
    name: String,
}

let s = User::schema();
assert_eq!(s.name.table, "user");
```

Derive an Edge:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Edge)]
#[junction(table = "follows")]
struct Follows {
    #[junction(source)]
    r#in: SimpleId<User>,
    #[junction(target)]
    r#out: SimpleId<User>,
    since: u32,
}

let e = <Follows as Edge>::schema();
assert_eq!(e.since.table, "follows");
```

> **Note:** Deriving `Node` automatically surfaces `schema().id` even when the struct does not declare an `id` field. Deriving `Edge` likewise always exposes `schema().id`, `schema().r#in`, and `schema().r#out`. Annotate one field with `#[junction(source)]` and one with `#[junction(target)]`; the macro infers the associated node types (and auto-injects the id/in/out columns when you omit them).
