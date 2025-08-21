# Testing Patterns

Testing ensures derive macros and query DSL remain stable.

## Integration Tests
Use in-memory SurrealDB adapter:
```rust
let db = setup_adapter().await;
let rows: Vec<Person> = select::<Person>(All)
  .from(Person::table())
  .return_many(db.clone())
  .await?;
```

## Macro UI Tests
Trybuild tests validate compile errors for misuse:
- Missing source/target annotations
- Wrong endpoint types

## Edge Case Tests
- Expression parsing with negation
- Projection onto subset structs
- Traversal path invariants (edges length matches nodes - 1)

## Suggested User Tests
- Round-trip serialization of `SimpleId<T>`
- Custom edge endpoint names insertion & selection
- Complex `Expr` compositions (.and/.or stack)

## Tips
- Use separate modules for traversal tests to isolate experimental APIs.
- Avoid brittle string comparison of raw queries; focus on result semantics.

## Future
Snapshot tests for AST lowering differences across adapters.
