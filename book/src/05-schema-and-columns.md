# Schema & Columns

Each derived Node or Edge type provides a `schema()` function returning a `Schema` struct with typed column handles (`Col<T>`).

## Example
```rust
let s = Person::schema();
let filter = expr!(s.age > 40).and(expr!(s.name != "Tom"));
```

## Column Projection
```rust
let s = Person::schema();
let subset: Vec<Person> = select::<Person>(All)
    .from(Person::table())
    .column(s.name)
    .column(s.age)
    .return_many(db.clone())
    .await?;
```

Multiple at once:
```rust
select::<Person>(All)
    .from(Person::table())
    .columns(&[s.name, s.age])
    .return_many(db.clone())
    .await?;
```

## Why Use Typed Columns?
- Compile-time guidance; avoids typos.
- Refactors rename struct fields and keep queries aligned.
- Enables future linting / static analysis.

## Raw String Columns
Fallback: `.column("name")`. Use sparingly.

## Edge Schemas
Edges expose relationship fields plus synthesized `id` / `in` / `out` columns even if omitted.

## Ordering
Columns provide `.asc()` / `.desc()` (via `Ordering` enum) used in `.order_by(...)` within query builders.

## Design Constraints
Schemas are lightweight; cloning is cheap. They embed static `&'static str` references for table/column names.
