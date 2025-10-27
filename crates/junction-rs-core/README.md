
# junction-rs-core

Low-level building blocks behind JunctionRS: strongly typed IDs, schema helpers, the expression + query AST, graph traversal structures, and the traits adapters implement.

Application crates should normally depend on the façade crate (`junction-rs`). Reach for `junction-rs-core` directly when you're writing a custom adapter, tooling, or tests that need lower-level access.

## Adding the crate

```toml
[dependencies]
junction-rs = { git = "https://github.com/zojeda/junction-rs.git",  features = ["surreal"] }
```


## Core pieces

* `traits`: `Node`, `Edge`, `HasTableName`, `DbExecutor`, and `DbError` describing the abstraction between the DSL and database backends.
* `id::SimpleId<T>`: UUID-backed strongly typed identifiers with serde support and helper methods (`as_uuid_str`, `token_part`).
* `schema::{Col, Table}`: typed column helpers used by the expression DSL and builder chains.
* `expr` module + `expr!` macro: declarative boolean expressions that compile to backend predicates.
* `query::ast`: backend-agnostic AST for select/insert/update/delete plus graph traversals.
* `schema_ops`: migration-oriented operations (`SchemaOp`, `FieldDef`, `FieldType`) shared by CLI tooling and adapters.

## Deriving models

Models derive `Node` / `Edge` using the macros crate (re-exported by `junction-rs`). Edge endpoints are annotated explicitly with `#[junction(source)]` / `#[junction(target)]` so the macro knows how to infer `From`/`To` types.

```rust
use serde::{Deserialize, Serialize};
use junction_rs::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
#[junction(table = "user")]
struct User {
    id: SimpleId<User>,
    name: String,
    marketing: bool,
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

let user_schema = User::schema();
assert_eq!(user_schema.name.table, "user");
```

## Query DSL

```rust
use junction_rs::prelude::*;

let s = User::schema();
let older_users = select::<User>(All)
    .from(User::table())
    .where_(expr!(s.age > 40))
    .order_by(s.name.asc())
    .limit(10);

// Execute against any DbExecutor implementation:
// let db = acquire_executor_somehow();
// let rows: Vec<User> = older_users.return_many(db.clone()).await?;

// let created: Vec<User> = insert(vec![User {
//         id: SimpleId::new(),
//         name: "Alice".into(),
//         marketing: true,
//     }])
//     .into()
//     .return_many(db.clone())
//     .await?;

// delete(User::table())
//     .where_(expr!(s.marketing == false))
//     .run(db)
//     .await?;
```

For graph traversals, see `graph::start`, `.forward::<E>()`, `return_path()`, and friends.

## SurrealDB adapter helpers

The SurrealDB adapter lives in the `junction-rs-surrealdb` crate and is re-exported through the façade when you enable the `surreal` feature:

```toml
[dependencies]
junction-rs = { version = "0.1", features = ["surreal"] }
```

```rust
use junction_rs::prelude::*;
use junction_rs::SurrealDbAdapter;

# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
let client = surrealdb::engine::any::connect("mem://").await?;
client.use_ns("demo").use_db("demo").await?;
let db = SurrealDbAdapter::new(client);

let rows: Vec<User> = select::<User>(All)
    .from(User::table())
    .return_many(db)
    .await?;
# Ok(())
# }
```

## For adapter authors

Implement `DbExecutor` for your backend by consuming `query::ast::Query`. You can also leverage `schema_ops::SchemaOp` to translate CLI-generated migration plans into backend-specific DDL.

---

Browse the rustdocs for full API coverage and see `crates/junction-rs/examples` for runnable end-to-end samples.

## Quickstart

Add the crates to your workspace or `Cargo.toml`:

```toml
[dependencies]
JunctionRS = { path = "../junction-rs" }
JunctionRS-macros = { path = "../junction-rs-macros" }

# Optional: SurrealDB adapter
surrealdb = { version = "2", optional = true }

[features]
surreal = ["JunctionRS/surreal"]
```

## Define a model

```rust
use serde::{Deserialize, Serialize};
use junction_rs::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, junction_rs_macros::Node)]
#[junction(table = "user")]
struct User {
    id: SimpleId<User>,
    name: String,
}

// Access a typed schema for column-safe expressions
let s = User::schema();
assert_eq!(s.name.table, "user");
```

## Query examples

Select with filter and ordering:

```rust
use junction_rs::prelude::*;

let s = User::schema();
let q = select::<User>(All)
    .from(User::table())
    .where_(expr!(s.name == "Alice").or(expr!(s.name == "Bob")))
    .order_by(s.name.asc())
    .limit(10);

// Execute with any DbExecutor implementation:
// let rows: Vec<User> = q.return_many(db).await?;
```

Insert rows:

```rust
let users = vec![
    User { id: SimpleId::new(), name: "Alice".into() },
    User { id: SimpleId::new(), name: "Bob".into() },
];
// let inserted: Vec<User> = insert(users).into().return_many(db).await?;
```

    User { id: User::create_simple_id(), name: "Alice".into() },
    User { id: User::create_simple_id(), name: "Bob".into() },
```rust
let s = User::schema();
// let updated: Vec<User> = update(User::table())
//     .content(serde_json::json!({"active": true}))
//     .where_(s.name.eq("Alice"))
//     .return_many(db)
//     .await?;
```

Delete:

```rust
let s = User::schema();
// delete::<User>(User::table()).where_(s.name.eq("Bob")).run(db).await?;
```

## Using the SurrealDB adapter (optional)

Enable the `surreal` feature and pass a `surrealdb` client:

```rust
use junction_rs::prelude::*;
use junction_rs::adapter::surrealdb::SurrealDbAdapter;

# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
let db = surrealdb::Surreal::new::<surrealdb::engine::any::Any>("mem://").await?;
let db = SurrealDbAdapter::new(db);

let s = User::schema();
let _rows: Vec<User> = select::<User>(All)
    .from(User::table())
    .where_(expr!(s.name == "Alice"))
    .return_many(db)
    .await?;
# Ok(())
# }
```

## Design highlights

- `Node` and `Edge` marker traits define your graph/domain entities.
- `SimpleId<T>` provides type-safe IDs bound to your model type.
- Small, composable AST in `query::ast` for backend compilation.

---

For more, see rustdocs in the source. Examples are runnable and cover select/insert/update/delete and adapter usage.
