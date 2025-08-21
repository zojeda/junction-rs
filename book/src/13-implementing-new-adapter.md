# Implementing a New Adapter

Adapters allow JunctionRS to target different backends while keeping the core DB-agnostic.

## Trait Contract
Implement `DbExecutor`:
```rust
async fn execute<T: DeserializeOwned + Send>(&self, query: qast::Query) -> Result<Vec<T>, DbError>;
```
Return `Ok(Vec::new())` for statements without rows.

## Steps
1. Create new crate with feature flag in workspace.
2. Implement AST compiler translating JunctionRS query structs into backend-native language.
3. Bind parameters safely (avoid string concatenation). Use serde value conversions.
4. Execute statement(s) asynchronously.
5. Deserialize payload into `T` via `serde_json` or backend driver facilities.
6. Map errors: backend -> `DbError::Adapter`, serde -> `DbError::Serde`.

## Testing
- Mirror Surreal adapter integration tests.
- Provide in-memory or ephemeral fixture setup similar to `setup_adapter()`.
- Ensure edge endpoint remapping semantics if backend expects canonical relation keys.

## Traversal Support
Implement lowering for traversal AST nodes first minimally (single-hop) then expand to multi-hop path semantics.

## Performance Considerations
Start with correctness; add batching / reuse once stable. Document known limitations early.

## Pitfalls
- Forgetting empty `Vec` for non-row statements.
- Incorrect table name usage from `HasTableName`.
- Losing edge orientation consistency (`in` always source, `out` always destination) in returned edge documents.

## Checklist
- [ ] Feature flag gating in root `Cargo.toml`
- [ ] Re-export adapter type in facade crate under feature
- [ ] Basic select/insert/delete implemented
- [ ] Expression operators mapped
- [ ] Edge remapping logic
- [ ] Traversal (start, forward/backward) minimal support
- [ ] Tests pass under `--all-features`
