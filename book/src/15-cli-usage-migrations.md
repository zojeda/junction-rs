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
> Old: manual chaining with `.from(...).record_id(uuid).return_many(...).await?.into_iter().next()`.
> New: `let user = get_by_id(user_id, db.clone()).await?;` or builder `select::<User>(All).by_id(user_id).return_one(db).await?`.
> Reason: Reduce boilerplate, enforce LIMIT 1 semantics, and provide Option ergonomics for absence.
