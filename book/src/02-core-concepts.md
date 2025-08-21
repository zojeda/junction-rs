# Core Concepts Overview

This chapter introduces the foundational building blocks used throughout JunctionRS.

## Node
A Node represents an entity record (e.g. `Person`, `User`). You derive `Node` to inject:
- Static table name (`TABLE`)
- A generated `Schema` struct with `Col<T>` fields for each property (plus an automatic `id` column)

## Edge
An Edge models a relationship between two Nodes. You derive `Edge` and annotate exactly one `#[junction(source)]` and one `#[junction(target)]` field of type `SimpleId<SourceNode>` / `SimpleId<TargetNode>`.

The macro also handles optional presence of `id`, `in`, and `out` fields; if you omit them they are synthesized.

## Schema & Columns
Each Node/Edge gets a `Schema` struct exposing fields as `Col<T>` values. These power the typed expression DSL and column projection (e.g. `let s = Person::schema(); expr!(s.age > 40)`).

## SimpleId<T>
A strongly-typed UUID wrapper that serializes as `table:uuid`. Supports creation via `SimpleId::new()` and acceptance of multiple input formats during deserialization.

## Query Builders
Pure functional-style builders produce an AST:
- `select::<T>(All)`
- `insert(items)` / `insert_into::<T>(...)`
- `delete(table)` / (update planned)

## Expression DSL
Use the `expr!` macro for binary comparisons and compose with `.and()` / `.or()` / `!`. Example: `expr!(s.age > 40).and(expr!(s.marketing == true))`.

## Projection
Any `Deserialize` struct with a subset of fields can be a projection target in `select`. Column-level projection also available via `.column(s.name)` or `.columns(&[s.name, s.age])`.

## Graph Traversal API (Experimental)
Start from nodes and traverse typed edges forward or backward, apply predicates on edges/nodes, project terminal node fields, and return either edges or path segments.

## Adapter Abstraction
`DbExecutor` executes compiled queries. SurrealDB adapter currently implements AST → SurrealQL compilation. Future adapters follow the same trait.

## Error Surface
Single `DbError` enum classifies adapter vs serialization vs user errors. Builders avoid panics; execution returns `Result<Vec<T>, DbError>`.

## Putting It Together
Define models, use schemas for expressions, chain builder methods, execute via an adapter, deserialize into strongly-typed structs.

Subsequent chapters dive into each concept.
