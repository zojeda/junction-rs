# IDs & Serialization

`SimpleId<T>` wraps a UUID and encodes table context for strong typing.

## Creation
```rust
let id = SimpleId::<Person>::new();
```
Or via helper `Person::create_simple_id()` if exposed by derive.

## Serialized Forms
- Preferred: `"person:550e8400-e29b-41d4-a716-446655440000"`
- Accepts bare UUID (`"550e..."`) during deserialization
- Accepts SurrealDB Thing map with `id` field

## Helper Methods
- `as_uuid_str()` returns the UUID string
- `token_part()` (adapter usage) yields prefix + compact hex for path generation

## Round Trip Guarantees
Deserialization ensures table name matches `T::TABLE` where present. Mismatches yield a `DbError::Serde` variant.

## Edge IDs
Edges behave similarly; synthesized `SimpleId<EdgeType>` when omitted. Edge traversal uses `in` and `out` relation fields in backend storage while your custom field names remain in application-level structs.

## Best Practices
- Avoid storing raw UUID strings separately; keep `SimpleId<T>`.
- When exposing publicly, prefer opaque IDs unless table context aids debugging.

## Single-Row Lookup (New in vX.Y.Z)

You can fetch an entity directly from a `SimpleId<T>` without spelling the table name.

```rust
use junction_rs::prelude::*;
let maybe_user: Option<User> = get_by_id(user_id, db.clone()).await?;
```

Builder form if you need explicit chaining (e.g. projection):

```rust
let maybe_name: Option<UserName> = select::<UserName>(All)
	.by_id(user_id)
	.return_one(db.clone())
	.await?;
```

Absent rows return `Ok(None)`; presence yields `Ok(Some(T))`.
