use anyhow::Result;
use clap::{Parser, Subcommand};
use std::{fs, path::PathBuf};

pub mod adapter;
pub mod adapters;
pub mod config;
pub mod diff;
pub mod init;
pub mod migration;
pub mod registry;
pub mod scanner;
pub mod schema;

pub use config::RootConfig;

/// JunctionRS command line interface root entrypoint.
#[derive(Parser, Debug)]
#[command(
    name = "junction",
    version,
    about = "JunctionRS CLI (migrations & schema tools)"
)]
pub struct Cli {
    /// Optional path to config file (defaults to junction.toml)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a Junction configuration in the current project
    Init,
    /// Migration related commands
    Migrate {
        #[command(subcommand)]
        cmd: MigrateCmd,
    },
    /// Snapshot commands
    Snapshot {
        #[command(subcommand)]
        cmd: SnapshotCmd,
    },
    /// Schema inspection helpers
    Schema {
        #[command(subcommand)]
        cmd: SchemaCmd,
    },
}

#[derive(Subcommand, Debug)]
pub enum MigrateCmd {
    /// List applied / pending migrations
    List,
    /// Apply pending migrations
    Apply {
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Create a new migration based on diff
    New { name: String },
    /// Revert the last (or specific) migration
    Revert {
        #[arg(long)]
        id: Option<String>,
    },
    /// Show concise status summary
    Status,
}

#[derive(Subcommand, Debug)]
pub enum SnapshotCmd {
    /// Generate or refresh schema snapshot
    Generate {
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Diff current code vs snapshot
    Diff,
}

#[derive(Subcommand, Debug)]
pub enum SchemaCmd {
    /// Inspect derived schema from code
    Inspect,
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init => init::run_init(cli.config.as_deref())?,
        Commands::Migrate { cmd } => {
            let (cfg, _cfg_path) = RootConfig::load(cli.config.as_deref())?;
            handle_migrate(cmd, &cfg)?;
        }
        Commands::Snapshot { cmd } => {
            let (cfg, _cfg_path) = RootConfig::load(cli.config.as_deref())?;
            handle_snapshot(cmd, &cfg)?;
        }
        Commands::Schema { cmd } => {
            let (cfg, _cfg_path) = RootConfig::load(cli.config.as_deref())?;
            handle_schema(cmd, &cfg)?;
        }
    }
    Ok(())
}

pub fn run_from_args() -> Result<()> {
    let cli = Cli::parse();
    run(cli)
}

fn handle_migrate(cmd: MigrateCmd, cfg: &RootConfig) -> Result<()> {
    match cmd {
        MigrateCmd::List => {
            println!("[stub] migrate list");
        }
        MigrateCmd::Apply { to, dry_run } => {
            println!("[stub] migrate apply to={to:?} dry_run={dry_run}");
        }
        MigrateCmd::New { name } => {
            migration::generate_new_migration(&name, cfg)?;
        }
        MigrateCmd::Revert { id } => {
            println!("[stub] migrate revert id={id:?}");
        }
        MigrateCmd::Status => {
            let adapters = adapter::all_adapters();
            if adapters.is_empty() {
                println!("no adapters registered. add a crate that calls register_adapter!(...) to enable database operations.");
            } else {
                println!("registered adapters:");
                for entry in adapters {
                    println!(" - {}", entry.name);
                }
            }
            let target = &cfg.database.adapter;
            match adapter::find_adapter(target) {
                Ok(_) => println!("configured adapter '{}' found", target),
                Err(_) => println!("configured adapter '{}' is missing from registry", target),
            }
        }
    }
    Ok(())
}

fn handle_snapshot(cmd: SnapshotCmd, cfg: &RootConfig) -> Result<()> {
    match cmd {
        SnapshotCmd::Generate { output } => {
            let root = std::env::current_dir()?;
            let scan = scanner::scan_schema(&scanner::ScanOptions {
                root: &root,
                paths: &cfg.models.paths,
                include: &cfg.models.include,
                exclude: &cfg.models.exclude,
            })?;
            let path = output.unwrap_or_else(|| PathBuf::from("schema.snapshot.json"));
            let json = serde_json::to_string_pretty(&scan)?;
            fs::write(&path, json)?;
            println!("wrote snapshot to {}", path.display());
        }
        SnapshotCmd::Diff => {
            let path = PathBuf::from("schema.snapshot.json");
            if !path.exists() {
                println!("no snapshot file (schema.snapshot.json) found");
                return Ok(());
            }
            let current: schema::SchemaSnapshot =
                serde_json::from_str(&fs::read_to_string(&path)?)?;
            let root = std::env::current_dir()?;
            let latest = scanner::scan_schema(&scanner::ScanOptions {
                root: &root,
                paths: &cfg.models.paths,
                include: &cfg.models.include,
                exclude: &cfg.models.exclude,
            })?;
            let current_tables: std::collections::HashSet<_> =
                current.models.iter().map(|m| &m.table).collect();
            let latest_tables: std::collections::HashSet<_> =
                latest.models.iter().map(|m| &m.table).collect();
            for t in latest_tables.difference(&current_tables) {
                println!("+ table {t}");
            }
            for t in current_tables.difference(&latest_tables) {
                println!("- table {t}");
            }
            println!("(basic diff complete)");
        }
    }
    Ok(())
}

fn handle_schema(cmd: SchemaCmd, cfg: &RootConfig) -> Result<()> {
    match cmd {
        SchemaCmd::Inspect => {
            let root = std::env::current_dir()?;
            let scan = scanner::scan_schema(&scanner::ScanOptions {
                root: &root,
                paths: &cfg.models.paths,
                include: &cfg.models.include,
                exclude: &cfg.models.exclude,
            })?;
            println!("{}", serde_json::to_string_pretty(&scan)?);
        }
    }
    Ok(())
}
