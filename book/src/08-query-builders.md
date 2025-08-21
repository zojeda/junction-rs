# Query Builders

JunctionRS provides chainable builders returning an AST before execution via an adapter implementing `DbExecutor`.

## Select
```rust
select::<Person>(All)
    .from(Person::table())
    .where_(expr!(Person::schema().age > 40))
    .order_by(Person::schema().age.desc())
    .limit(10)
    .return_many(db.clone())
    .await?;
```
Use `select_edge::<EdgeType>(All)` for edges to ensure endpoint field name restoration.

## Insert
```rust
insert(vec![person1, person2])
    .into()
    .return_many::<_>(db.clone())
    .await?;
```

## Delete
```rust
delete(Person::table())
    .where_(expr!(Person::schema().age < 30))
    .run(db.clone())
    .await?;
```

## Update (Planned)
Update semantics will mirror other builders: `update(Person::table()).set(s.age, 41).where_(...)`.

## Execution Contract
`return_many(db)` returns `Vec<T>` for row-producing queries. Non-row statements return empty `Vec`.

## Parameters & Safety
Values converted to `serde_json::Value` and bound (adapter-specific). Avoid manual string concatenation.

## Projection Integration
Chain `.column(...)` or project into subset structs.

## Error Handling
All builder executions yield `Result<Vec<T>, DbError>`; inspect adapter vs serde variants for diagnostics.
