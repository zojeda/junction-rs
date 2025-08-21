# Aggregation & Group By

Aggregation patterns use projection structs with aggregate fields.

## Example: Group By Age Count
Suppose an example `group_by_age_count` returns counts per age:
```rust
#[derive(Deserialize)]
struct AgeCount { age: i32, count: i64 }

// Pseudocode; actual implementation uses a specialized builder or raw expression mapping.
let rows: Vec<AgeCount> = select::<AgeCount>(All)
  .from(Person::table())
  // .group_by(s.age) (future DSL)
  // .aggregate(count()) (future DSL)
  .return_many(db.clone())
  .await?;
```

Current state: Aggregation helpers may be limited or manual; consult examples for supported patterns using adapter-specific lowering.

## Design Goals
- Keep aggregation ergonomic while backend-agnostic.
- Provide typed wrappers for common aggregates (planned: `count`, `sum`, `avg`).

## Roadmap
Aggregation builder methods will formalize group by + aggregate selection returning structured results.
