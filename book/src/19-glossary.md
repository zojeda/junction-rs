# Glossary

**Node**: Entity record type derived with `#[derive(Node)]`.
**Edge**: Relationship record connecting two Nodes derived with `#[derive(Edge)]`.
**Schema**: Generated struct containing typed column handles (`Col<T>`).
**Column (Col<T>)**: Lightweight descriptor of a table field used in expressions and projection.
**SimpleId<T>**: Strongly typed UUID wrapper serializing as `table:uuid`.
**Expression (Expr)**: Logical predicate tree used to filter queries.
**Projection**: Deserialization into a subset struct with fewer fields than the source model.
**Traversal**: Graph navigation via edge steps (forward/backward) and filters.
**PathSegment**: Structure representing nodes (and optionally edge ids) from a traversal path.
**Adapter**: Backend implementation of `DbExecutor` compiling & executing queries.
**SurrealDB Adapter**: Current adapter translating AST to SurrealQL.
**Builder**: Immutable query construction function returning a new chained state.
**Feature Flag**: Cargo feature enabling optional adapter code (`surreal`).
**Migration**: Process of evolving backend schema to match model changes.
