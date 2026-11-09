//! RPC-457: `fspec workspaces add|list|remove` — the workspace registry the
//! daemon reads (`~/.fspec/workspaces.json`).
//!
//! Feature: spec/features/add-and-remove-workspaces.feature

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use codelet_common::workspace_registry;

use crate::common;

#[derive(clap::Subcommand, Debug)]
pub enum WorkspacesAction {
    /// Add an fspec project (a directory containing spec/).
    Add { path: PathBuf },
    /// List the registered workspaces.
    List,
    /// Remove a workspace from the registry (its files are untouched).
    Remove { path: PathBuf },
}

pub fn run(action: WorkspacesAction) -> Result<()> {
    // The same data directory the daemon uses (common::build_service).
    codelet_common::set_data_directory(common::home_fspec_dir()?)
        .map_err(|error| anyhow!("codelet_common::set_data_directory: {error}"))?;
    match action {
        WorkspacesAction::Add { path } => {
            let workspace = workspace_registry::add(&path)?;
            println!(
                "✓ Added workspace {}  {}",
                workspace.name,
                workspace.path.display()
            );
        }
        WorkspacesAction::List => {
            let workspaces = workspace_registry::list()?;
            if workspaces.is_empty() {
                println!("No workspaces");
            }
            for workspace in workspaces {
                println!("{}  {}", workspace.name, workspace.path.display());
            }
        }
        WorkspacesAction::Remove { path } => {
            let workspace = workspace_registry::remove(&path)?;
            println!(
                "✓ Removed workspace {}  {}",
                workspace.name,
                workspace.path.display()
            );
        }
    }
    Ok(())
}
