# CLI Usage & Migrations

The `junction-cli` crate provides configuration generation, schema scanning, diffing, and migration support utilities.

## Installation
Run locally from workspace:
```bash
cargo run -p junction-cli -- --help
```

## Initialization
```bash
junction init
```
Prompts for model paths and database settings, producing a `junction.toml` config file.

## Diffing
Generates schema diffs between code-defined models and stored snapshots; assists in planning migrations.

## Migrations
Planned or experimental features may include creating migration scripts based on diffs and applying them to the backend.

## Registry & Scanner
Internal modules analyze crates for Node/Edge definitions to build a registry used by diff logic.

## State Tracking
CLI maintains state about applied migrations to prevent reapplication.

## Workflow Example
1. Define/modify models.
2. Run `junction diff` to preview changes.
3. Review generated plan.
4. Apply via `junction migrate` (future).

## Tips
- Commit `junction.toml` to version control.
- Keep migrations small and review output before applying.

## Roadmap
Enhanced automated migration generation, validation steps, and multi-adapter compatibility.

> Migration Note (vX.Y.Z): Added `get_by_id` helper and `SelectBuilder::by_id` + `return_one` methods for single-record retrieval.
> Old (pre v0.2.0): manual chaining with `.from(...).record_id(uuid).return_many(...).await?.into_iter().next()`.
> Migration Note (v0.2.0): `record_id` now requires a typed `SimpleId<NodeType>` and auto sets the table. Update calls:
> Old: `select::<PersonName>(All).from(Person::table()).record_id(person_id_string)`
> New: `select::<PersonName>(All).record_id(person_id)`
> For full entity retrieval prefer `select::<Person>(All).by_id(person_id)` or the helper `get_by_id::<Person,_>(person_id, db).await?`.
> New: `let user = get_by_id(user_id, db.clone()).await?;` or builder `select::<User>(All).by_id(user_id).return_one(db).await?`.
> Reason: Reduce boilerplate, enforce LIMIT 1 semantics, and provide Option ergonomics for absence.

> Migration Note (v0.X.Y): Public `SimpleId::new()` removed (now crate-private). Use `<NodeOrEdge>::create_simple_id()` or `<NodeOrEdge>::from_uuid(uuid)`. This affects any CLI examples that previously showed direct `SimpleId::<T>::new()` construction.
