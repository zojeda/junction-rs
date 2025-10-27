### Selecting a Specific Record by Typed Id

To anchor a traversal or fetch a single row starting from a known record you can pass a typed `SimpleId<N>` directly via `record_id`:

```rust
let pid: SimpleId<Person> = /* obtained or stored earlier */;
let name_only: Option<PersonName> = select::<PersonName>(All)
    .record_id(&pid) // borrow; sets table automatically based on `Person`
    .column(Person::schema().name)
    .return_one(db.clone())
    .await?;
```

This replaces the earlier string-based variant `record_id("uuid")`. The builder now infers and sets the table from the type parameter of the `SimpleId`, eliminating accidental cross-table lookups and avoiding manual `from(Person::table())` calls. For projections whose target struct is not the same Node type, the generic after `select::<ProjectionType>` does not need to match the id's type – the `record_id` method is generic over any `Node`.

Use `by_id(id)` for the common pattern of retrieving a single full entity (it sets a defensive `LIMIT 1`). Prefer `record_id` when you need traversal segments (e.g. `forward` / `backward`) or a projection rather than the full Node.

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

### Single Record Retrieval (New in vX.Y.Z)

Use `get_by_id` for concise single-row lookups by typed id:

```rust
let user: Option<User> = get_by_id(user_id, db.clone()).await?;
```

Alternatively, the builder form provides more control:

```rust
let user = select::<User>(All)
    .by_id(user_id) // sets table, record id, LIMIT 1
    .return_one(db.clone()) // Result<Option<User>, DbError>
    .await?;
```

Projection example:

```rust
#[derive(Deserialize)]
struct UserName { name: String }
let user_name: Option<UserName> = select::<UserName>(All)
    .by_id(user_id)
    .return_one(db.clone())
    .await?;
```
