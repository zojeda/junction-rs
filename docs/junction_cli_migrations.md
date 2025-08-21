# Junction CLI Migrations Feature & Implementation Plan

Status: Draft (Initial Specification)
Author: AI Assistant
Date: 2025-09-29
Related Crates: (planned) `junction-cli`, existing: `junction-rs`, `junction-rs-core`, `junction-rs-macros`, `junction-rs-surrealdb`

## 1. Problem Statement
Applications using Junction need a consistent, engine-agnostic way to evolve database schema and data over time. Currently there is no tool to:
- Capture an expected schema state derived from Rust model definitions
- Track and apply ordered migrations across multiple supported backends
- Generate new migrations by diffing current DB state vs code expectations
- Allow reversible (up/down) operations with optional raw backend-specific statements when necessary

## 2. High-Level Goals
1. Provide a `junction-cli` binary with migration subcommands.
2. Maintain a canonical, version-controlled snapshot of expected schema derived from Rust source via lightweight static analysis + configuration.
3. Support migration lifecycle: create, list (applied/pending), apply (up), revert (down), diff, snapshot regenerate.
4. Keep migration definitions backend-agnostic while permitting opt-in backend-specific raw segments.
5. Ensure idempotent and deterministic execution ordering.
6. Avoid coupling to a single runtime database; allow pointing at different adapters via configuration or CLI flags.

## 3. Non-Goals (for initial version)
- Automatic detection of all complex schema aspects (indexes, constraints) beyond an initial minimal set (tables & fields) unless explicitly configured.
- Introspection for every supported backend (will start with SurrealDB; design pluggable).
- Live hot-reload watching of model changes.
- Data seeding beyond what a migration author writes manually.

## 4. Key Concepts & Artifacts
### 4.1 Configuration File
Proposed default path: `junction.toml` (override via `--config`). Example:
```toml
[project]
name = "my-app"

[models]
# Glob or regex patterns to scan for Rust source containing model derives
paths = ["crates/app/src/**/*.rs"]
# Optional explicit include/exclude regex for struct names
include = [".*"]
exclude = ["Temp.*"]

[migrations]
# Directory to store migration files
dir = "migrations"
# Generation strategy: incremental | timestamp
naming = "timestamp"

[database]
# Adapter: surreal | <future adapters>
adapter = "surreal"
# Connection options per adapter (key-value; adapter chooses usage)
url = "memory"  # surreal in-memory example
# Optional: authentication, namespace, database
namespace = "test"
database = "test"

[schema]
# Control what is captured in snapshot
capture_fields = true
capture_edge_relationships = true
```

### 4.2 Schema Snapshot
Stored at `schema.snapshot.json` (or versioned file). JSON structure representing discovered models:
```json
{
  "version": 1,
  "generated_at": "2025-09-29T12:34:56Z",
  "models": [
    {
      "table": "person",
      "kind": "node",
      "fields": [
        {"name": "id", "type": "uuid", "primary": true},
        {"name": "age", "type": "i64"},
        {"name": "name", "type": "string"}
      ]
    }
  ],
  "edges": [
    {
      "table": "friendship",
      "from": "person",
      "to": "person",
      "fields": [ {"name": "since", "type": "datetime"} ]
    }
  ]
}
```

### 4.3 Migration File Format
Directory: `migrations/`. File naming: `20250929_123456_create_person.rs` (timestamp prefix) or `0001_create_person.rs` if incremental.

Rust source migration file compiled as part of a generated `junction-cli` build script OR dynamically loaded via WASM/dylib (initial: compile-time).

Each migration implements trait:
```rust
#[async_trait]
pub trait Migration {
    fn id(&self) -> &'static str; // matches filename prefix, unique ordering key
    fn description(&self) -> &'static str;
    async fn up(&self, ctx: &mut MigrationContext) -> Result<(), MigrationError>;
    async fn down(&self, ctx: &mut MigrationContext) -> Result<(), MigrationError>;
}
```

`MigrationContext` exposes:
- `executor(): &dyn DbExecutor` – engine-agnostic operations
- Helper DSL to declare table/edge creation (abstract)
- Escape hatch: `raw("SURREALQL", Backend::Surreal)` or `raw_all("...generic...")`

A registry macro collects migrations: `inventory` crate or manual `mod` + vector construction.

### 4.4 Migration State Tracking
Dedicated system table per adapter, e.g. `__junction_migrations` with fields:
- `id` (string)
- `applied_at` (datetime)
- `checksum` (sha256 of file contents to detect drift)

Operations:
- List: query ordered list of local migration definitions + DB applied state → classify (applied, pending, divergent, unknown-in-db).
- Apply: run pending migrations in order, insert state rows transactionally (if backend supports; else best-effort with ordering guarantees).
- Revert: run `down` for most recent applied migration or specific `--id`.

### 4.5 Diff / Generation Workflow
`junction migrate new <name>`:
1. Load latest schema snapshot (expected prior state). If absent, treat as empty state.
2. Re-scan code to derive new expected schema.
3. Compute diff (table added/removed, field added/removed/changed, edge added/removed, type changes).
4. Generate a migration template file with `up` performing transformations and `down` reversing them.
5. Update snapshot (or optional separate step `junction snapshot generate`).

### 4.6 Engine Abstraction for Schema Ops
Define schema mutation DSL in core or CLI crate (possibly new module in `junction-rs-core`):
```rust
enum SchemaOp {
    CreateTable { name: String, fields: Vec<FieldDef> },
    DropTable { name: String },
    AddField { table: String, field: FieldDef },
    DropField { table: String, field: String },
    CreateEdge { table: String, from: String, to: String, fields: Vec<FieldDef> },
    DropEdge { table: String },
}
```
Adapter adds compiler for `Vec<SchemaOp>` to backend-specific statements. Raw fallbacks stored in migration if user edits.

### 4.7 Error Handling
Define `MigrationError` variants: Config, Io, Parse, DiffConflict, ChecksumMismatch, Adapter, Execution.

### 4.8 Concurrency & Safety
- Migrations executed sequentially.
- Optional advisory lock mechanism (future) to avoid parallel runs.
- Checksum verification before applying down / re-run.

## 5. CLI Command Set
```
junction init [--config <path>]
junction migrate list          # Show applied/pending/divergent
junction migrate apply [--to <id>] [--dry-run]
junction migrate new <name>    # Generate next migration from schema diff
junction migrate revert [--id <id>]  # Revert last or specific
junction migrate status        # Summarize current state
junction snapshot generate     # Produce (or refresh) schema snapshot
junction snapshot diff         # Show diff between snapshot and current code
junction schema inspect        # Print derived schema from code (debug)
```

## 6. Internal Architecture
Component Layers:
- Config Loader (TOML → structs) with validation.
- Model Scanner: Walk configured paths, parse Rust AST (syn) to find `#[derive(Node)]` / `#[derive(Edge)]` and collect struct field names & types.
- Schema Normalizer: Map Rust types -> abstract types (string, int, float, bool, datetime, uuid, json, custom?).
- Snapshot Manager: Read/Write JSON snapshot.
- Diff Engine: Compare two `SchemaSnapshot` values → `Vec<SchemaOp>`.
- Migration Generator: Convert ops to Rust code template.
- Registry Loader: Discover local migration modules.
- State Store: Query and mutate `__junction_migrations` via `DbExecutor`.
- Executor: Applies migrations with logging, timing, checksum validation.
- Renderer: CLI output (table format / JSON `--output json`).

## 7. Data Structures (Draft)
```rust
struct SchemaSnapshot { version: u32, generated_at: DateTime<Utc>, models: Vec<Model>, edges: Vec<Edge> }
struct Model { table: String, fields: Vec<Field> }
struct Edge { table: String, from: String, to: String, fields: Vec<Field> }
struct Field { name: String, ty: FieldType, primary: bool }

enum FieldType { String, Int64, Float64, Bool, DateTime, Uuid, Json, Other(String) }

struct MigrationRecord { id: String, applied_at: DateTime<Utc>, checksum: String }
```

## 8. Migration File Template (Generated)
```rust
use junction_cli::prelude::*; // to be defined

pub struct Migration20250929_123456;

#[async_trait]
impl Migration for Migration20250929_123456 {
    fn id(&self) -> &'static str { "20250929_123456" }
    fn description(&self) -> &'static str { "create person table" }

    async fn up(&self, ctx: &mut MigrationContext) -> Result<(), MigrationError> {
        ctx.apply(vec![
            SchemaOp::CreateTable { name: "person".into(), fields: vec![
                field_uuid_pk("id"),
                field_string("name"),
                field_int("age"),
            ]},
        ]).await?;
        Ok(())
    }

    async fn down(&self, ctx: &mut MigrationContext) -> Result<(), MigrationError> {
        ctx.apply(vec![SchemaOp::DropTable { name: "person".into() }]).await?;
        Ok(())
    }
}

register_migration!(Migration20250929_123456);
```

`register_migration!` macro adds a boxed instance to a static registry consumed at runtime.

## 9. Implementation Phases
### Phase 1: Foundations
- Create `junction-cli` crate.
- Implement config loader + command parsing (clap).
- Add snapshot generation (model scanning minimal).
- Provide `snapshot generate` and `schema inspect` commands.

### Phase 2: Diff & Migration Generation
- Implement diff engine and `SchemaOp` set.
- Add `migrate new` generating migration file + update snapshot.
- Implement registry macro & dynamic discovery (compile-time list).

### Phase 3: Apply & Track
- Implement migration trait, context, and state table management (Surreal adapter first).
- Commands: `migrate list`, `migrate apply`, `migrate status`.

### Phase 4: Revert & Integrity
- Add `migrate revert` with checksum checks.
- Divergence detection (file modified after apply) warnings.
- `--dry-run` generate plan.

### Phase 5: Backend Abstraction Hardening
- Move/extend `SchemaOp` and compilation logic into `junction-rs-core` with traits so other adapters can implement.
- Add raw statement support (per backend tagging) within migration DSL.

### Phase 6: Polish
- Output formatting options.
- Extensive docs & examples (`examples/migrations_demo`).
- Tests: unit (diff, parsing), integration (apply/revert cycles, divergence), snapshot determinism.

## 10. Risks & Mitigations
| Risk | Impact | Mitigation |
|------|--------|------------|
| Parsing Rust source complexity | Missed models | Use `syn` only for structs with target derives; fallback regex for quick scan |
| Divergence across adapters | Non-portable migrations | Encourage schema ops vs raw; clearly tag raw backend blocks |
| Large projects performance | Slow scans | Cache file mtimes + incremental snapshot regeneration |
| Concurrent migration runs | Corruption | Future advisory lock table; doc best practices |
| Type mapping ambiguity | Incorrect diffs | Provide explicit override attributes in code or config |

## 11. Open Questions (to refine later)
- Should migrations compile into a separate dynamic plugin to avoid rebuilding CLI after each new migration? (initial: rebuild required)
- Support for data backfill operations with progress reporting?
- Namespacing of multiple apps sharing a database?

## 12. Example User Flow
1. Run `junction init` in the workspace root to scaffold `junction.toml`.
2. Run `junction snapshot generate` → creates `schema.snapshot.json`.
3. Modify a model (add field).
4. Run `junction migrate new add_age_field` → generates migration file with add/drop field operations and updates snapshot.
5. Run `junction migrate apply` → applies pending migrations to DB.
6. Run `junction migrate list` → shows applied status.
7. If needed, `junction migrate revert` to undo last change.

## 13. Testing Strategy
- Unit tests: config parsing, diff logic, snapshot serialization determinism, migration file template generation.
- Integration (Surreal in-memory): create/apply/revert sequences, divergence detection, dry-run output.
- Property tests (optional later): diff(symmetry) property (applying ops then reversing returns to original snapshot).

## 14. Future Extensions
- Seed scripts & environment tagging (dev/stage/prod).
- Advisory locking across distributed runners.
- Automatic index generation & diff.
- Data migration DSL (transform columns, recompute derived data).
- Watch mode: auto-create draft migration on file change.
- Graph-specific schema evolution helpers (edge constraints, cardinalities).

## 15. Acceptance Criteria (Initial Release)
- CLI builds and provides all listed commands (Phase 1–3 complete).
- Snapshot accurately reflects models & edges from annotated code in a real project example.
- `migrate new` produces compilable migration file with correct id ordering and reversible operations for adds/drops.
- `migrate apply` applies sequential pending migrations and records state.
- `migrate list` shows correct classification of migration states.
- Documentation (this file + README section) explains workflow end-to-end.

---
End of specification.
