# junction-cli

Command line interface utilities for the JunctionRS ecosystem.

Status: early milestones shipped (init wizard, schema snapshotting, migration diff generation). Several subcommands are still scaffolds while the migration executor lands.

## Quick Start

From the root of a Rust workspace that hosts your application crate, run:

```
junction init
```

The init wizard confirms you're inside a Cargo project, lets you pick the
package to configure, then interactively asks for:

- project display name (defaults to the Cargo package name)
- glob patterns for scanning models (include / exclude)
- migrations directory name
- database adapter selection (today: SurrealDB)
- adapter connection settings (URL, namespace, database)

When finished it writes a `junction.toml` alongside your workspace `Cargo.toml`
and ensures the selected package depends on `junction-rs`. Re-run with
`--config <path>` to place the file somewhere else.

To try the built-in SurrealDB adapter wiring, enable the optional feature when you install the CLI:

```toml
[dependencies]
junction-cli = { path = "../junction-cli", features = ["adapter-surreal"] }
```

## Adapter Integration

`junction-cli` discovers database adapters at compile time. Any crate in the
workspace can register an adapter by calling the exported
`register_adapter!("name", async_factory_fn)` macro. The factory receives the
resolved [`DatabaseConfig`](../junction-cli/src/config.rs) and must return a
type that implements `junction_rs_core::traits::DbExecutor`.

Example (SurrealDB) – the `adapter-surreal` feature pulls in the reference implementation located in `crates/junction-cli/src/adapters/surreal.rs`:

```rust
use anyhow::Result;
use junction_cli::register_adapter;
use junction_cli::config::DatabaseConfig;
use junction_rs_surrealdb::SurrealDbAdapter;
use surrealdb::engine::any::connect;

async fn connect_surreal(cfg: &DatabaseConfig) -> Result<SurrealDbAdapter> {
	let url = cfg.url.as_deref().unwrap_or("memory");
	let client = connect(url).await?;
	if let (Some(ns), Some(db)) = (cfg.namespace.as_deref(), cfg.database.as_deref()) {
		client.use_ns(ns).use_db(db).await?;
	}
	Ok(SurrealDbAdapter::new(client))
}

register_adapter!("surreal", connect_surreal);
```

Custom projects can ship their own adapter crate that depends on `junction-cli`
and calls the macro during build, ensuring CLI commands automatically wire to the
project’s primary database backend.

junction init [--config <path>]
junction migrate list
junction migrate apply [--to <id>] [--dry-run]
junction migrate new <name>
junction migrate revert [--id <id>]
junction migrate status
junction snapshot generate
junction snapshot diff
junction schema inspect
```

### Command status (2025-10)

| Command | Status |
| --- | --- |
| `junction init` | ✅ Interactive wizard writes `junction.toml` and adds the façade dependency |
| `junction migrate new` | ✅ Diffs the previous snapshot → generates Rust migration file + updates snapshot |
| `junction migrate status` | ✅ Shows registered adapters + config wiring |
| `junction snapshot generate` | ✅ Emits `schema.snapshot.json` derived from annotated models |
| `junction snapshot diff` | ✅ Compares latest code scan vs snapshot (table-level)
| `junction schema inspect` | ✅ Prints derived schema JSON to stdout |
| `junction migrate list` / `apply` / `revert` | 🚧 Currently stubbed (print placeholder output) while execution engine is implemented |

See `docs/junction_cli_migrations.md` for the end-state design the CLI is converging toward.

## Development
```
cargo run -p junction-cli -- --help
```

## License
Dual-licensed under MIT or Apache-2.0.
