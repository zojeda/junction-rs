# Copilot Instructions for JunctionRS

Concise project knowledge so AI agents can be productive immediately.

## 1. Workspace & Crate Roles
- `crates/junction-rs` – Public façade crate (re‑exports core + macros via `prelude`). Prefer importing from here in examples/tests.
- `crates/junction-rs-core` – Core primitives: `Node`, `Edge`, `DbExecutor`, query AST (`query`), expression system (`expr`), schema helpers (`schema`), typed IDs (`SimpleId`). Backend‑agnostic.
- `crates/junction-rs-macros` – Proc macros deriving `Node` / `Edge` and generating per‑model schema structs with `Col<T>` fields used in typed expressions.
- `crates/junction-rs-surrealdb` (adapter) – SurrealDB implementation of `DbExecutor`, AST → SurrealQL compiler, edge utilities.

## 2. Core Concepts & Flow
1. User defines structs and derives `Node` / `Edge` (adds a static `TABLE`, schema accessor, and `HasTableName`).
2. Macros generate a `Schema` struct per entity containing `Col<T>` fields (e.g. `s.age`).
3. Query builders (`select`, `insert`, `delete`, `update`) produce an AST (`qast::Query`).
4. Adapter (`DbExecutor`) compiles & executes AST, returning deserialized rows (Serde). Non‑row statements return empty `Vec`.
5. Expressions are constructed either with the `expr!` macro (infix for binary comparisons) or via column helpers (`s.age.gt(40)`). Logical composition uses `.and() / .or() / .not()` on `Expr`.

## 3. ID & Serialization Pattern
- `SimpleId<T>`: strongly typed UUID wrapper; derives default via `Uuid::new_v4()`.
- Serializes as `"table:uuid"` when `T: HasTableName`; deserializer accepts `table:uuid`, bare uuid, or SurrealDB Thing map with `id` field.
- Helpers: `as_uuid_str()`, `token_part()` (prefix + compact hex) used in adapter path generation.

## 4. Expression / Filtering DSL
- Prefer: `let s = Person::schema(); select::<Person>(All).from(Person::table()).where_(expr!(s.age > 40))`.
- `expr!` supports: `== != > >= < <=` plus unary `!(...)`. No boolean `&&/||`; chain with `.and()` / `.or()` explicitly.
- For ordering: use `s.age.asc()` / `desc()` → `Ordering` enum.

## 5. Projection Pattern
- Any `Deserialize` struct whose fields are a subset of the model can be used in `select::<ProjectionType>(...)` – unmatched fields in source are ignored (Serde).

## 6. Adapter Contract (Implementing `DbExecutor`)
- Trait method: `execute<T: DeserializeOwned + Send>(&self, Query) -> Future<Result<Vec<T>, DbError>>`.
- MUST return `Ok(Vec::new())` for statements without rows (delete/update) – never panic.
- Map backend errors to `DbError::Adapter(String)`; serialization issues become `DbError::Serde`.
- Surreal adapter lives under feature `surreal`; new adapters replicate compile (AST → native) + bind params + deserialize.

## 7. Common Developer Workflows
- Build all crates: `cargo build --all-features` (tests/examples rely on `surreal` feature where applicable).
- Lint: `cargo clippy --all --all-features -D warnings`.
- Format: `cargo fmt --all`.
- Run core examples (from `crates/junction-rs`): `cargo run --example select_person --features surreal` (others: `insert_person`, `select_person_projection`, `delete_person`, `group_by_age_count`, `recommendation_system`).
- Run tests workspace-wide: `cargo test --all-features`.

## 8. Test & Example Conventions
- Tests in `crates/junction-rs/tests` exercise derive macros & query DSL end‑to‑end using in‑memory SurrealDB.
- Integration tests for adapter live in `crates/junction-rs-surrealdb/tests` (naming: `surreal_*_integration.rs`). Use pattern for new adapter tests.
- Keep examples minimal; use `setup_adapter()` helper style (see root README snippet) for ephemeral in-memory DB.

## 9. Derive Macro Expectations
- `#[derive(Node)]` requires `#[junction(table = "<name>")]` attribute providing table identifier.
- `#[derive(Edge)]` requires `#[junction(table = "<edge_table>")]` on the struct and exactly one field annotated with `#[junction(source)]` plus one with `#[junction(target)]` so the macro can infer endpoint node types. Optional `id`/`in`/`out` fields are injected automatically when omitted.
- Node schemas always expose an `id` column (`schema().id`) even if the struct omits the field; edge schemas likewise emit `id`, `in`, and `out` columns, inferring types from the annotated edge endpoints when fields are absent.
- Macros emit: `impl Node/Edge`, `impl HasTableName`, `impl Schema` struct with a `Col<T>` per field.

## 10. Adding Features or Queries
- New query types extend AST in `junction-rs-core/src/query` then add builder function + re‑export in `lib.rs` and `prelude`.
- Ensure adapter compiler updated; otherwise feature is backend-inert.
- Keep binary operations consistent: update `expr.rs` macros & `Col<T>` convenience methods together.

## 11. Style & Patterns
- Public user-facing code goes through the façade crate re-exports; avoid referencing `junction-rs-core` paths directly in docs/examples.
- Composability over inheritance: small structs/enums in AST; avoid builder mutation—prefer chain producing new structs.
- Error surface intentionally small (`DbError` only); do not expose backend-specific error types directly.

## 12. Pitfalls / Gotchas
- Forgetting `--features surreal` leads to missing adapter symbols in examples/tests.
- `expr!` does not handle complex logical precedence; for multi-part logic compose manually: `expr!(s.age > 40).and(expr!(s.marketing == true))`.
- `SimpleId` serialization assumes correct table name; deriving `Node` / `Edge` is required for proper serde output.

## 13. Example Snippet (Canonical Pattern)
```rust
let db = setup_adapter().await; // in-memory Surreal
let s = Person::schema();
let older: Vec<PersonNameAge> = select::<PersonNameAge>(All)
    .from(Person::table())
    .where_(expr!(s.age > 40))
    .order_by(s.age.desc())
    .limit(10)
    .return_many(db.clone())
    .await?;
```

## 14. Extending With Another Backend (High-Level)
- Implement `DbExecutor` for an adapter struct.
- Write AST → backend compiler (mirror Surreal compiler structure in `crates/junction-rs-surrealdb/src/compiler.rs`).
- Add feature flag in root `Cargo.toml` enabling optional dependency + adapter re-export.
- Provide integration tests mirroring the Surreal ones.

## 15. Documentation & Book Updates (MANDATORY ON API CHANGE)
Whenever you change any public API surface you MUST update the mdBook content under `book/src`. Treat docs as part of the build. A PR that alters API without doc updates must either:
1. Include the appropriate chapter edits, or
2. Add an explicit `[no-doc-change]` note in the PR description with a justification (rare: internal refactor leaving all public signatures & behavior intact).

### What Counts as a Public API Change
- Adding/removing/renaming any `pub` item re-exported by the façade crate (`crates/junction-rs`) – functions, structs, enums, traits, macros, constants, modules.
- Changing trait method signatures (`DbExecutor`, `Node`, `Edge`, expression types, query builders).
- Altering macro parameters or generated schema field semantics (derive `Node` / `Edge`).
- Adding/removing expression operators or column convenience methods.
- Modifying `SimpleId` serialization/deserialization rules.
- Extending the query AST with new query kinds, aggregation forms, traversal constructs, ordering or filtering capabilities.
- Adapter-visible behavior changes (compiler output, error mapping contract).

### Chapters to Review / Update (Targeted Mapping)
| Change Type | Chapters |
|-------------|----------|
| New/changed high-level concepts | `01-introduction`, `02-core-concepts`, `03-getting-started` |
| Node / Edge macro or schema alterations | `04-defining-models`, `05-schema-and-columns`, `14-derive-macros` |
| ID or serialization tweaks | `06-ids-and-serialization` |
| Expression DSL updates | `07-expression-dsl` |
| Query builder / projection / ordering | `08-query-builders`, `09-projection` |
| Aggregations / group-by changes | `10-aggregation-group-by` |
| Graph traversal / edge semantics | `11-graph-traversal` |
| Adapter contract / Surreal specifics | `12-surrealdb-adapter`, `13-implementing-new-adapter` |
| CLI migrations or breaking changes | `15-cli-usage-migrations` |
| Examples canonical snippet changes | `16-annotated-examples` & snippet in section 13 here |
| Roadmap / design rationale updates | `18-roadmap-design-notes` |

### Required Actions Checklist
1. Build & test after code change:
```bash
cargo build --all-features
cargo test --all-features
```
2. Identify impacted symbols (replace `SymbolName` as needed):
```bash
grep -R "OldSymbolName" book/src crates/junction-rs/src crates/junction-rs-core/src || true
```
3. Update relevant chapters (see mapping table). Ensure examples reflect new API and still compile conceptually.
4. If breaking change: add a "Migration Note:" block to the closest chapter (e.g., `15-cli-usage-migrations.md`). Format:
```
> Migration Note (vX.Y.Z): <brief description>
> Old: select::<Person>(All)...
> New: select(All::<Person>)... (reason: ...)
```
5. Re-build the book locally (requires `mdbook` installed):
```bash
mdbook build book
```
6. Visually spot-check changed chapters (open `book/build/<chapter>.html`).
7. Update canonical example snippet in section 13 above if its shape changes.
8. Commit code + docs together. Do NOT split across multiple PRs unless coordinated.

### Quality Gate for Docs
- PR Reviewers: reject if any public item diff lacks corresponding doc diff unless `[no-doc-change]` justified.
- Run grep for removed symbol names to ensure no stale references remain.
- Keep examples using façade crate (`junction-rs`) – never switch to core paths in docs.

### Quick Verification Script (Optional)
Add (or run ad-hoc) a script to sanity-check that main examples compile with Surreal feature:
```bash
cargo run --example select_person --features surreal --quiet
cargo run --example insert_person --features surreal --quiet
```

### Style
- Prefer short, actionable code blocks.
- Highlight new API with a dedicated "New in vX.Y.Z" sentence.
- Keep migration notes cumulatively; don't delete old ones.

Failure to update docs increases user friction; treat this as part of Definition of Done.

---
If any area feels under‑specified (e.g., AST layout or compiler internals) request deeper dive into the respective module. Feedback welcome to refine this guide.
