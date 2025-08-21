use crate::config::{DatabaseConfig, ModelsConfig, RootConfig};
use anyhow::{anyhow, bail, Context, Result};
use cargo_metadata::{Metadata, MetadataCommand, Package};
use dialoguer::{Confirm, Input, Select};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = "junction.toml";

pub fn run_init(explicit_config: Option<&Path>) -> Result<()> {
    let workspace_manifest = locate_workspace_manifest()?;
    let metadata = MetadataCommand::new()
        .manifest_path(&workspace_manifest)
        .no_deps()
        .exec()
        .context("failed to inspect Cargo workspace")?;
    let workspace_root = PathBuf::from(&metadata.workspace_root);

    println!("Detected cargo workspace at {}", workspace_root.display());

    let package = select_package(&metadata, &workspace_root)?;
    println!(
        "Configuring Junction for package '{}' ({})",
        package.name, package.manifest_path
    );

    let config_path = determine_config_path(explicit_config, &workspace_root)?;
    if config_path.exists()
        && !Confirm::new()
            .with_prompt(format!(
                "A {} already exists. Overwrite?",
                config_path.display()
            ))
            .default(false)
            .interact()?
    {
        println!("Aborted by user");
        return Ok(());
    }

    let defaults = RootConfig::default();

    let project_name: String = Input::new()
        .with_prompt("Project name")
        .default(
            defaults
                .project
                .name
                .clone()
                .unwrap_or_else(|| package.name.clone()),
        )
        .interact_text()?;

    let default_paths = join_display(&defaults.models.paths);
    let paths_input: String = Input::new()
        .with_prompt("Model glob patterns (comma separated)")
        .default(default_paths)
        .interact_text()?;
    let include_input: String = Input::new()
        .with_prompt("Include regex patterns (comma separated)")
        .default(join_display(&defaults.models.include))
        .interact_text()?;
    let exclude_input: String = Input::new()
        .with_prompt("Exclude regex patterns (comma separated)")
        .default(join_display(&defaults.models.exclude))
        .interact_text()?;

    let migrations_dir: String = Input::new()
        .with_prompt("Migrations directory")
        .default(defaults.migrations.dir.clone())
        .interact_text()?;

    let adapter_options = vec!["surreal"];
    let adapter_index = Select::new()
        .with_prompt("Database adapter")
        .items(&adapter_options)
        .default(
            adapter_options
                .iter()
                .position(|opt| *opt == defaults.database.adapter)
                .unwrap_or(0),
        )
        .interact()?;
    let adapter = adapter_options[adapter_index].to_string();

    let db_url: String = Input::new()
        .with_prompt("Database URL")
        .default(
            defaults
                .database
                .url
                .clone()
                .unwrap_or_else(|| "memory".into()),
        )
        .interact_text()?;
    let db_namespace: String = Input::new()
        .with_prompt("Database namespace")
        .default(
            defaults
                .database
                .namespace
                .clone()
                .unwrap_or_else(|| "test".into()),
        )
        .interact_text()?;
    let db_database: String = Input::new()
        .with_prompt("Database name")
        .default(
            defaults
                .database
                .database
                .clone()
                .unwrap_or_else(|| "test".into()),
        )
        .interact_text()?;

    let models_cfg = ModelsConfig {
        paths: split_csv(&paths_input),
        include: split_csv(&include_input),
        exclude: split_csv(&exclude_input),
    };

    let database_cfg = DatabaseConfig {
        adapter,
        url: Some(db_url),
        namespace: Some(db_namespace),
        database: Some(db_database),
    };

    let mut cfg = RootConfig::default();
    cfg.project.name = Some(project_name);
    cfg.models = models_cfg;
    cfg.migrations.dir = migrations_dir;
    cfg.database = database_cfg;

    let toml = cfg.to_pretty_toml()?;
    fs::write(&config_path, toml)?;
    println!("Wrote {}", config_path.display());

    ensure_dependencies(&package)?;

    println!("Initialization complete. You can now run `junction snapshot generate`.");
    Ok(())
}

fn locate_workspace_manifest() -> Result<PathBuf> {
    let mut current = env::current_dir()?;
    loop {
        let candidate = current.join("Cargo.toml");
        if candidate.exists() {
            return Ok(candidate);
        }
        if !current.pop() {
            bail!("Could not find Cargo.toml in current or parent directories");
        }
    }
}

fn select_package(metadata: &Metadata, workspace_root: &Path) -> Result<Package> {
    let workspace_ids: HashSet<_> = metadata.workspace_members.iter().cloned().collect();
    let mut packages: Vec<Package> = metadata
        .packages
        .iter()
        .filter(|pkg| workspace_ids.contains(&pkg.id))
        .cloned()
        .collect();

    if packages.is_empty() {
        bail!("No packages found in workspace to configure");
    }

    packages.sort_by(|a, b| a.name.cmp(&b.name));

    let current_manifest = locate_package_manifest(&env::current_dir()?).ok();
    let default_index = current_manifest
        .as_ref()
        .and_then(|manifest| {
            packages
                .iter()
                .position(|pkg| pkg.manifest_path.as_std_path() == manifest)
        })
        .unwrap_or(0);

    if packages.len() == 1 {
        return Ok(packages.remove(0));
    }

    let options: Vec<String> = packages
        .iter()
        .map(|pkg| {
            let path = pkg.manifest_path.as_std_path();
            let display = path
                .strip_prefix(workspace_root)
                .map(|rel| rel.display().to_string())
                .unwrap_or_else(|_| path.display().to_string());
            format!("{} ({display})", pkg.name)
        })
        .collect();

    let selection = Select::new()
        .with_prompt("Select the package to configure")
        .items(&options)
        .default(default_index)
        .interact()?;

    Ok(packages.remove(selection))
}

fn determine_config_path(explicit: Option<&Path>, workspace_root: &Path) -> Result<PathBuf> {
    let path = if let Some(explicit) = explicit {
        let path = PathBuf::from(explicit);
        if path.is_absolute() {
            path
        } else {
            env::current_dir()?.join(path)
        }
    } else {
        workspace_root.join(CONFIG_FILE_NAME)
    };

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    Ok(path)
}

fn locate_package_manifest(start: &Path) -> Result<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join("Cargo.toml");
        if candidate.exists() {
            return Ok(candidate);
        }
        if !current.pop() {
            bail!("Could not find package Cargo.toml from {}", start.display());
        }
    }
}

fn join_display(values: &[String]) -> String {
    if values.is_empty() {
        String::new()
    } else {
        values.join(", ")
    }
}

fn split_csv(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn ensure_dependencies(package: &Package) -> Result<()> {
    let manifest_path = package.manifest_path.as_std_path();
    let manifest_content = fs::read_to_string(manifest_path)
        .with_context(|| format!("reading manifest {}", manifest_path.display()))?;

    let mut doc: toml_edit::DocumentMut = manifest_content.parse()?;
    let deps_item = doc
        .as_table_mut()
        .entry("dependencies")
        .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
    let deps_table = deps_item
        .as_table_mut()
        .ok_or_else(|| anyhow!("dependencies section is not a table"))?;

    let mut changed = false;
    changed |= add_dependency_if_missing(deps_table, "junction-rs", toml_edit::value("0.1"));

    if changed {
        fs::write(manifest_path, doc.to_string())?;
        println!("Updated dependencies in {}", manifest_path.display());
    } else {
        println!(
            "Dependencies already satisfied in {}",
            manifest_path.display()
        );
    }

    Ok(())
}

fn add_dependency_if_missing(
    table: &mut toml_edit::Table,
    name: &str,
    item: impl Into<toml_edit::Item>,
) -> bool {
    if table.contains_key(name) {
        false
    } else {
        table.insert(name, item.into());
        true
    }
}
