<p align="center">
  <img src="junction_rs.png" alt="JunctionRS Logo" width="200"/>
</p>

# JunctionRS

An experimental, async-first, type-safe Object Graph Mapper (OGM) for Rust — a graph‑oriented ORM-ish toolkit focused on Nodes, Edges & fluent traversal-friendly queries.

Instead of rows & joins, JunctionRS thinks in terms of Nodes & Edges and gives you a fluent, typed DSL to model, query, and manipulate graph data. It is deliberately minimal today and under active iteration.

> ⚠️ Work In Progress: Expect sharp edges, missing features, and breaking changes between minor versions while the core stabilizes.

---

## ✨ Goals & Highlights

* **Async-first** – designed around `async`/`await`.
* **DB-agnostic core** – pluggable adapters (only SurrealDB today).
* **Node / Edge model types** – derive macros remove boilerplate.
* **Typed query DSL** – build `SELECT`, `INSERT`, `DELETE` (and more coming) with compile-time guidance.
* **Serde-powered projections** – select only part of a record into custom structs.
* **Graph semantics** – future focus on traversals & relationship queries.

---

## 📦 Current Status

The only implemented adapter is **[SurrealDB](https://surrealdb.com/)** behind the `surreal` feature flag. The internal abstractions (`Adapter`, query builder, schema traits) are backend-agnostic so additional graph / document stores can be added later.

### Experimental Graph Traversal & Path Support

An initial graph traversal API exists (`graph::start`, `.forward::<Edge>()`, `.backward::<Edge>()`, `.filter_edge()`, `.filter_node()`, `.project_fields()`, `.return_edges()`, `.return_path()`).

Currently implemented capabilities:
* Multi-step node & edge traversal (fan-out via edge unions, naive repetition duplication for now).
* Edge filtering (relationship property predicates) and node filtering (terminal frontier predicates) with parameter binding.
* `return_edges()` – fetches final hop edge documents with consistent orientation (source always accessible via `in`, destination via `out`).
* `return_path()` –
        * Single-hop: `PathSegment { nodes: [start, end], edges: [edge_id] }`.
        * Multi-hop (Phases B & C): full node sequence plus (Phase C) parallel edge id sequence `PathSegment { nodes: [start, n1, ..., terminal], edges: Some([e0, e1, ...]) }` with invariant `edges.len() == nodes.len() - 1`.
            * Current constraints: all steps outward, exactly one edge type per hop (no unions yet), no edge-level filters (terminal node filters allowed). Violations produce descriptive errors.

Limitations / Not Yet Implemented:
* Tail repetition variant enumeration for `return_path()`.
* Edge unions / fan-out enumeration for path mode (single edge type per hop today).
* Edge-level filters during multi-hop path capture (currently rejected; planned once unions/fan-out semantics added).
* Embedding edge property subsets inside `PathSegment` (only ids today).
* Intermediate node alias predicates (only final node predicates supported externally; internal design reserves aliasing for future).

Design & Roadmap: see `docs/path_design.md` for the evolution plan (multi-hop LET chain strategy, repetition handling, edge id invariants).

### Custom Edge Endpoint Field Names

Edges can use semantic endpoint field names (e.g. `src` / `dst`, `from_user` / `to_user`). Annotate one with `#[junction(source)]` and one with `#[junction(target)]`. The derive macro records those names; when you insert via `insert_edge(...)` the Surreal adapter automatically synthesizes the internal `in` / `out` relation keys while preserving your original fields in returned rows. Traversals and graph queries work transparently.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Node)]
#[junction(table = "person")] 
struct Person { id: SimpleId<Person>, name: String }

#[derive(Debug, Clone, Serialize, Deserialize, Edge)]
#[junction(table = "knows")] 
struct Knows {
    #[junction(source)]
    src: SimpleId<Person>,
    #[junction(target)]
    dst: SimpleId<Person>,
    strength: i32,
}

let db = setup_adapter().await; // in-memory Surreal
let _ = insert(vec![p1.clone(), p2.clone()]).return_many::<_>(db.clone()).await?;
let _ = insert_edge(vec![Knows { src: p1.id.clone(), dst: p2.id.clone(), strength: 42 }])
    .return_many::<_>(db.clone()).await?;

let results: Vec<Person> = select::<Person>(All)
    .from(Person::table())
    .record_id(format!("person:{}", p1.id.as_uuid_str()))
    .forward::<Knows, Person>()
    .return_many(db.clone())
    .await?;
```

Stability Note: Path semantics are experimental; JSON shape may evolve (we will aim to preserve `nodes` ordering + `edges.len() == nodes.len()-1`).

---

## 🧱 Workspace Layout

Crates in this repository:

| Crate | Purpose |
| ----- | ------- |
| `junction-rs` | End‑user façade that re‑exports core + macros. Add this to your `Cargo.toml`. |
| `junction-rs-core` | Core traits (`Node`, `Edge`, `Adapter`), query builder, schema & expression utilities. |
| `junction-rs-macros` | Procedural macros: `#[derive(Node)]`, `#[derive(Edge)]`, helper attributes. |

---

## 🔧 Installation

Enable the SurrealDB adapter feature (version placeholder – adjust to the published crate version you use):

```toml
[dependencies]
junction-rs = { version = "0.1.0", features = ["surreal"] }
```

To generate a starter `junction.toml` configuration, install the CLI (`cargo run -p junction-cli -- --help` for local builds) and run `junction init` from your workspace root. The wizard detects your Cargo package, prompts for model paths and database settings, and writes the config file ready for snapshot and migration commands.

---

## 🚀 Quick Start

Define a node & edge, then interact through the DSL. (Simplified conceptual example; see concrete runnable examples further below.)

```rust
use serde::{Serialize, Deserialize};
use junction_rs::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, Node)]
#[junction(table = "user")] // maps to the underlying table / record type
pub struct User {
    pub name: String,
    pub joined: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Edge)]
#[junction(table = "follows")]
pub struct Follows {
    #[junction(source)]
    pub r#in: SimpleId<User>,
    #[junction(target)]
    pub r#out: SimpleId<User>,
    pub since: i64,
}

// `schema().id`, `schema().r#in`, and `schema().r#out` are always available on
// derived edges even if you omit those fields entirely; nodes likewise expose
// `schema().id` automatically when you skip an explicit id in the struct.

// Adapter setup (SurrealDB in-memory for examples)
async fn setup() -> junction_rs::adapter::SurrealDbAdapter {
    use surrealdb::engine::any::connect;
    let db = connect("mem://").await.unwrap();
    db.use_ns("demo").use_db("demo").await.unwrap();
    junction_rs::adapter::SurrealDbAdapter::new(db)
}
```

---

## 🧪 Real Examples (From `crates/junction-rs/examples`)

Below are distilled excerpts. Each is runnable—see the commands in [Running the Examples](#-running-the-examples).

### 1. Defining a Model

```rust
use serde::{Deserialize, Serialize};
use junction_rs::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, Node, PartialEq)]
#[junction(table = "person")]
pub struct Person { pub name: String, pub age: i32, pub marketing: bool }
```

### 2. Adapter Setup (In‑Memory SurrealDB)

```rust
pub async fn setup_adapter() -> junction_rs::adapter::SurrealDbAdapter {
    use surrealdb::engine::any::connect;
    let db = connect("mem://").await.unwrap();
    db.use_ns("test").use_db("test").await.unwrap();
    junction_rs::adapter::SurrealDbAdapter::new(db)
}
```

### 3. Insert

```rust
let people = vec![
    Person { name: "Tom".into(), age: 42, marketing: true },
    Person { name: "Jaime".into(), age: 40, marketing: false },
];

let created = insert(people.clone())
    .into()
    .return_many::<_>(db.clone())
    .await
    .unwrap();
```

### 4. Select with Filter

```rust
let s = Person::schema();
let selected: Vec<Person> = select::<Person>(All)
    .from(Person::table())
    .where_(expr!(s.age > 40))
    .return_many(db.clone())
    .await
    .unwrap();
```

### 4b. Selecting Specific Columns (Schema-Aware)

You can project just a subset of columns using the same typed schema handles you use for expressions — thanks to the public `IntoSelectColumn` trait:

```rust
let s = Person::schema();
// Fetch only name & age columns into the full Person struct (other fields default/ignored by backend)
let people: Vec<Person> = select::<Person>(All)
    .from(Person::table())
    .column(s.name) // schema column
    .column(s.age)
    .return_many(db.clone())
    .await?;

// Or chain multiple quickly
let minimal: Vec<PersonNameAge> = select::<PersonNameAge>(All)
    .from(Person::table())
    .columns(&[s.name, s.age]) // slice of schema columns
    .return_many(db.clone())
    .await?;
```

Raw string field names still work: `.column("name")`, but preferring schema columns keeps refactors safe.

### 5. Projection (Subset Struct)

Serde makes partial projections ergonomic—extra fields are ignored.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersonNameAge { name: String, age: i32 }

let rows: Vec<PersonNameAge> = select::<PersonNameAge>(All)
    .from(Person::table())
    .return_many(db.clone())
    .await
    .unwrap();
```

### 6. Delete

```rust
let s = Person::schema();
delete(Person::table())
    .where_(expr!(s.age < 41))
    .run(db.clone())
    .await
    .unwrap();
```

---

## 🔍 Projections & Type Safety

Any struct whose fields are a subset (and compatible types) of the underlying model can be a projection target as long as it implements / derives `Deserialize`. This keeps fetch payloads full while letting application code narrow to what it needs.

Schema-based column selection pairs naturally with this: pick columns via `schema()` and target a subset struct with only those fields.

---

## 🧪 Experimental Graph Traversal

An initial graph traversal DSL is available (experimental). It lets you:

* Start from all nodes of a type (`graph::start::<Person>()`) or a specific id (`graph::start_at(id)`)
* Traverse typed edges forward (`.forward::<Knows>()`) or backward (`.backward::<Knows>()`)
* Apply edge property predicates (`.filter_edge(expr!(e.strength > 3))`)
* Apply node predicates (`.filter_node(expr!(s.age > 18))`)
* Project a subset of terminal node fields (`.project_fields::<NameOnly>(&["name"])`)

### Example

```rust
use junction_rs::prelude::*;

#[derive(Node)]
#[junction(table = "person")]
struct Person { id: SimpleId<Person>, name: String, age: i32 }

#[derive(Edge)]
#[junction(table = "knows")]
struct Knows {
    #[junction(source)]
    r#in: SimpleId<Person>,
    #[junction(target)]
    r#out: SimpleId<Person>,
    strength: i32,
}

#[derive(serde::Deserialize)]
struct NameOnly { name: String }

async fn graph_sample(db: impl DbExecutor) -> Result<Vec<NameOnly>, DbError> {
    let s = Person::schema();
    let e = Knows::schema();
    let alice_id: SimpleId<Person> = /* obtain existing id */ todo!();
    let res: Vec<NameOnly> = junction_rs::graph::start_at::<Person>(alice_id)
    .forward::<Knows>()                  // traverse knows edges
        .filter_edge(expr!(e.strength > 3)) // edge predicate (on relationship properties)
        .filter_node(expr!(s.age > 18))     // terminal node predicate
        .project_fields::<NameOnly>(&["name"]) // only fetch name field
        .return_many(db)
        .await?;
    Ok(res)
}
```

### How Edge Filters Work
Edge filters are lowered into a subselect: `(SELECT * FROM <path_to_edges> WHERE <edge_pred>)` before appending the destination node hop. This ensures predicates evaluate against edge documents, not the terminal nodes.

### Current Limitations
* Path mode: single-hop traversals return `PathSegment { nodes, edges }` with one edge id. Multi-hop capture requires `start_at(id)` plus outward-only edges, no unions, and no edge-level filters; results include both node and edge id sequences.
* Repetition: `.repeat(min, max)` expands tail variants up to a safety cap (10). Mixed-direction or union steps in the repeated segment are rejected until smarter lowering lands.
* Edge unions: supported up to 5 edges in a tuple (compile-time convenience blanket impls).
* Projection inside traversal limited to terminal nodes (`project_fields`) or final edges (`return_edges`).
* Path mode edge property projection/embedding is still on the roadmap.

> Path roadmap: Phase C (multi-hop node + edge capture) is live with the above restrictions. Next steps: unions + edge filters, richer repetition lowering, and payload projection helpers. See `docs/path_design.md` for the design notes.

For contributors: see `docs/path_design.md` for the evolving design draft.

Projection Unwrap Behavior: The Surreal adapter normalizes certain single-element primitive arrays returned by nested SELECT projections (e.g. `{"name": ["Bob"]}`) into scalars (`{"name": "Bob"}`) for non-path graph queries, while preserving semantic arrays like `edges: [edge_id]` in path results.

Feedback and real-world traversal use cases are welcome—these directly drive stabilization priorities.

---

## 🛣️ Roadmap (Indicative)

* Update & Patch semantics
* Edge creation & traversal helpers
* More expressive filter / expression operators
* Relationship querying conveniences
* Additional adapters (open to community experimentation)
* Migration / schema evolution utilities

---

## 🏃 Running the Examples

From the `crates/junction-rs` directory:

```bash
cd crates/junction-rs

# Insert example
cargo run --example insert_person --features surreal

# Select example
cargo run --example select_person --features surreal

# Projection example
cargo run --example select_person_projection --features surreal

# Delete example
cargo run --example delete_person --features surreal
```

---

## 🤝 Contributing

Early days! Feedback, issues, small PRs, and adapter experiments are welcome. Please:

1. Open an issue to discuss larger proposals first.
2. Keep changes focused; add tests where possible.
3. Follow Rust fmt & clippy guidance.

---

## 🧠 What is an OGM?

An Object Graph Mapper (OGM) plays a role similar to an ORM, but optimized for graph-shaped data. Instead of focusing on tables/rows and joins, an OGM centers on:

* Nodes (entity records) and Edges (relationships) as first-class concepts.
* Fluent traversal or relationship-aware querying.
* Schema-derived type safety to reduce stringly-typed query errors.
* Projection into partial structs (serde-driven) to minimize over-fetching.
* Column projection using typed schema accessors (public `IntoSelectColumn`).

JunctionRS aims to stay lean: it provides derivations, a typed expression / query DSL, and adapter abstraction—while leaving business logic, higher-level patterns, and heavy migration tooling to user space (for now).

## 📜 License

Dual-licensed under either:

* MIT (`LICENSE-MIT`)
* Apache-2.0 (`LICENSE-APACHE`)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion is provided under the same dual license terms. See the `LICENSE` file for more details.

---

Happy hacking — and expect things to evolve quickly.
