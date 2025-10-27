# IDs & Serialization

`SimpleId<T>` wraps a UUID and encodes table context for strong typing.

## Creation

Prior to v0.X.Y you could construct IDs directly:
```rust
// Deprecated (v0.X.Y):
// let id = SimpleId::<Person>::new();
```

The `SimpleId::new()` constructor is now crate-private to enforce using type-aware helpers emitted by the derive macros. Use:
```rust
// Random new ID
let id = Person::create_simple_id();

// Wrap an existing UUID you obtained elsewhere
use uuid::Uuid;
let raw = Uuid::new_v4();
let id_from_uuid = Person::from_uuid(raw);
```

> Migration Note (v0.X.Y): Replace any `SimpleId::<T>::new()` calls with `<T>::create_simple_id()` (random) or `<T>::from_uuid(uuid)` (wrap existing). The change centralizes ID creation behind the Node/Edge type for clearer ergonomics and potential future instrumentation.

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
