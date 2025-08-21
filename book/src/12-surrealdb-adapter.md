# SurrealDB Adapter Internals

The SurrealDB adapter compiles JunctionRS ASTs into SurrealQL and executes them.

## Feature Activation
Enable with `features = ["surreal"]` on the `junction-rs` dependency.

## Responsibilities
- Translate query builders to SurrealQL statements
- Bind parameters (serde values)
- Execute asynchronously
- Deserialize rows into target structs (using Serde)

## Error Mapping
SurrealDB driver errors -> `DbError::Adapter(String)`; deserialization issues -> `DbError::Serde(String)`.

## Edge Endpoint Remapping
Custom edge endpoint field names are transformed to Surreal's `in` / `out` internally during insertion and selection, then restored on deserialization.

## Traversal Lowering
Traversal chains produce nested Surreal SELECT statements, applying edge predicates before node predicates to ensure correct filtering semantics.

## Path Results
Normalization distinguishes scalar unwraps vs preserving arrays for path edge ids.

## Performance Notes (Early)
Focus remains correctness & ergonomics before deep optimization. Avoid premature micro-optimizations until semantics stabilize.

## Future Improvements
- Prepared statement caching
- Batch insertion optimizations
- Streaming large result sets
