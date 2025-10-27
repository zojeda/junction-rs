# Annotated Examples

This chapter explains the official examples found in `crates/junction-rs/examples`.

## Insert Person
Demonstrates batch insertion and returning created rows.

Key points:
- Use `insert(vec![...]).into()` to infer table from item type.
- Returns fully populated structs with generated `id` if omitted initially.

## Select Person
Shows filtering by numeric comparison and returning ordered results.

Concepts:
- `select::<Person>(All).from(Person::table())`
- `where_` with `expr!`

## Projection
Illustrates selecting only subset of columns and projecting into subset struct.

## Delete Person
Demonstrates predicate-based deletion returning empty vector.

## Group By Age Count
Shows primitive aggregation pattern using projection struct for aggregated fields.

## Recommendation System
Combines traversal and filtering to build simple recommendation outputs.

## Custom Edge Endpoints
Example where edge uses semantic endpoint names; adapter maps internally while preserving names in Rust types.

## Patterns
- Always capture schema `let s = Type::schema();` early.
- Prefer typed columns over raw strings.
- Keep builder chains readable by aligning dots.

Refer to source code for full runnable context.

## Fetch Single User By ID (New in vX.Y.Z)

```rust
use junction_rs::prelude::*;

#[derive(Serialize, Deserialize, Node)]
#[junction(table = "user")]
struct User { id: SimpleId<User>, name: String }

async fn fetch(db: impl DbExecutor, user_id: SimpleId<User>) -> Result<Option<User>, DbError> {
	get_by_id(user_id, db).await
}
```

Projection variant:

```rust
#[derive(Deserialize)]
struct UserName { name: String }
let maybe_name: Option<UserName> = select::<UserName>(All)
	.by_id(user_id)
	.return_one(db.clone())
	.await?;
```
