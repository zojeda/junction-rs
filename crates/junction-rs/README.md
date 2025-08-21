# JunctionRS

Unified ORM crate for Rust, reexporting everything from `junction-rs-core` and `junction-rs-macros`.

## Usage

Add the façade crate to your `Cargo.toml` (adjust the path/version to match your workspace):

```toml
[dependencies]
junction-rs = { path = "../junction-rs", features = ["surreal"] }
```

Import everything via the prelude:

```rust
use junction_rs::prelude::*;
```

## Example: Using Derive Macros

```rust
use serde::{Deserialize, Serialize};
use junction_rs::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
#[junction(table = "user")]
struct User {
    id: SimpleId<User>,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Edge)]
#[junction(table = "follows")]
struct Follows {
    #[junction(source)]
    r#in: SimpleId<User>,
    #[junction(target)]
    r#out: SimpleId<User>,
    since: i64,
}
```

## Features
- Reexports all traits, types, and derive macros from `junction-rs-core` and `junction-rs-macros`.
- Single dependency surface for application crates; enable the `surreal` feature to pull in the SurrealDB adapter re-exported as `junction_rs::SurrealDbAdapter`.
- Convenient `prelude` module for easy imports (`select`, `insert`, `expr!`, `SimpleId`, derive macros, etc.).
- Supports graph modelling with `Node` / `Edge` derives, typed schema accessors, and the fluent query + traversal DSL.
- Schema-aware column projection: use `select::<T>(All).from(T::table()).column(T::schema().field)` thanks to the public `IntoSelectColumn` trait, avoiding duplicate string field names.
- Custom edge endpoint field names: Edge structs can name their endpoint fields arbitrarily (e.g. `from_person` / `to_product`). The SurrealDB adapter maps these to the backend's `in` / `out` internally during INSERT / SELECT. Use `select_edge::<E>(All)` instead of `select::<E>(All)` when selecting an edge type so the adapter can restore your original endpoint field names for deserialization.
  - Example:
    ```rust
    #[derive(Serialize, Deserialize, Edge)]
    #[junction(table = "bought")]
    struct Bought {
        #[junction(source)]
        from_person: SimpleId<Person>,
        #[junction(target)]
        to_product: SimpleId<Product>,
        quantity: i32,
    }

    let edges: Vec<Bought> = select_edge::<Bought>(All)
        .from(Bought::table())
        .return_many(db.clone())
        .await?;
    ```

## License
Dual-licensed under MIT or Apache-2.0.
