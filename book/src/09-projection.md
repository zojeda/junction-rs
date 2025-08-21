# Projection & Partial Structs

## Subset Structs
Any struct implementing `Deserialize` whose fields are a subset of the source model works as a projection target:
```rust
#[derive(Deserialize)]
struct PersonNameAge { name: String, age: i32 }
let rows: Vec<PersonNameAge> = select::<PersonNameAge>(All)
    .from(Person::table())
    .return_many(db.clone())
    .await?;
```
Extra source columns are ignored during deserialization.

## Column-Level Projection
Use schema columns for precision:
```rust
let s = Person::schema();
select::<Person>(All)
  .from(Person::table())
  .columns(&[s.name, s.age])
  .return_many(db.clone())
  .await?;
```

## Combining Both
Project to subset struct AND specify columns for backend efficiency:
```rust
let s = Person::schema();
select::<PersonNameAge>(All)
  .from(Person::table())
  .columns(&[s.name, s.age])
  .return_many(db.clone())
  .await?;
```

## Design Rationale
Separation of row shape (serde) vs column selection (schema) keeps queries expressive yet safe.

## Edge Projections
Use `select_edge::<E>(All)`; projection subset rules apply similarly.

## Caveats
- Ensure field names and types align exactly.
- Missing required fields cause serde errors.
