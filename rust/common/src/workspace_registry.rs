//! RPC-441: persisted registry of workspaces opened through a daemon.
//!
//! Feature: spec/features/workspace-registry.feature
//!
//! A workspace is identified by its canonicalized directory path and named
//! after the final path component. Opened workspaces are recorded once in
//! `{data_dir}/workspaces.json` so every client sees the same list across
//! daemon restarts.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

const REGISTRY_FILE: &str = "workspaces.json";
const LOCK_DIR: &str = "workspaces.json.lock";
const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub path: PathBuf,
    pub name: String,
}

#[derive(Debug, Error)]
pub enum WorkspaceRegistryError {
    #[error("workspace path is not a directory: {0}")]
    NotADirectory(PathBuf),
    /// RPC-457: an explicitly added workspace must be an fspec project.
    #[error("not an fspec project (no spec/ directory): {0}")]
    NotAnFspecProject(PathBuf),
    /// RPC-457: removing a workspace the registry does not hold.
    #[error("workspace is not registered: {0}")]
    NotRegistered(PathBuf),
    #[error("workspace registry unavailable: {0}")]
    Unavailable(String),
    #[error("workspace registry I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("workspace registry is malformed: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Default, Serialize, Deserialize)]
struct RegistryFile {
    version: u32,
    workspaces: Vec<Workspace>,
}

/// Resolve `path` to its canonical workspace identity without recording it.
pub fn identify(path: &Path) -> Result<Workspace, WorkspaceRegistryError> {
    let canonical = std::fs::canonicalize(path)
        .ok()
        .filter(|canonical| canonical.is_dir())
        .ok_or_else(|| WorkspaceRegistryError::NotADirectory(path.to_path_buf()))?;
    let name = canonical
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| canonical.to_string_lossy().into_owned());
    Ok(Workspace {
        path: canonical,
        name,
    })
}

/// Open `path` as a workspace, remembering it the first time it is seen.
pub fn open(path: &Path) -> Result<Workspace, WorkspaceRegistryError> {
    let workspace = identify(path)?;
    let directory = registry_dir()?;
    std::fs::create_dir_all(&directory)?;
    let recorded = workspace.clone();
    crate::file_lock::with_file_lock(&directory.join(LOCK_DIR), move || {
        let mut registry = read(&directory).map_err(|error| error.to_string())?;
        if !registry
            .workspaces
            .iter()
            .any(|known| known.path == recorded.path)
        {
            registry.workspaces.push(recorded);
            registry.version = VERSION;
            write(&directory, &registry).map_err(|error| error.to_string())?;
        }
        Ok(())
    })
    .map_err(WorkspaceRegistryError::Unavailable)?;
    Ok(workspace)
}

/// RPC-457: explicitly add `path` — an existing directory containing
/// `spec/` — to the registry (once).
pub fn add(path: &Path) -> Result<Workspace, WorkspaceRegistryError> {
    let workspace = identify(path)?;
    if !crate::spec_paths::has_spec_dir(&workspace.path) {
        return Err(WorkspaceRegistryError::NotAnFspecProject(workspace.path));
    }
    open(&workspace.path)
}

/// RPC-457: forget `path`. A directory that still exists is matched by its
/// canonical path; one that was deleted is matched as given. Forgetting a
/// workspace the registry does not hold is an error.
pub fn remove(path: &Path) -> Result<Workspace, WorkspaceRegistryError> {
    let target = match std::fs::canonicalize(path) {
        Ok(canonical) => canonical,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => path.to_path_buf(),
        Err(error) => return Err(error.into()),
    };
    let directory = registry_dir()?;
    let removed = crate::file_lock::with_file_lock(&directory.join(LOCK_DIR), {
        let directory = directory.clone();
        let target = target.clone();
        move || {
            let mut registry = read(&directory).map_err(|error| error.to_string())?;
            let index = registry
                .workspaces
                .iter()
                .position(|known| known.path == target);
            let Some(index) = index else {
                return Ok(None);
            };
            let removed = registry.workspaces.remove(index);
            registry.version = VERSION;
            write(&directory, &registry).map_err(|error| error.to_string())?;
            Ok(Some(removed))
        }
    })
    .map_err(WorkspaceRegistryError::Unavailable)?;
    removed.ok_or(WorkspaceRegistryError::NotRegistered(target))
}

/// Every remembered workspace whose directory still exists, in the order it
/// was first opened (RPC-442: removed projects drop out of the dropdown).
pub fn list() -> Result<Vec<Workspace>, WorkspaceRegistryError> {
    let mut workspaces = read(&registry_dir()?)?.workspaces;
    workspaces.retain(|workspace| workspace.path.is_dir());
    Ok(workspaces)
}

fn registry_dir() -> Result<PathBuf, WorkspaceRegistryError> {
    crate::get_data_dir().map_err(WorkspaceRegistryError::Unavailable)
}

fn read(directory: &Path) -> Result<RegistryFile, WorkspaceRegistryError> {
    match std::fs::read_to_string(directory.join(REGISTRY_FILE)) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(RegistryFile::default()),
        Err(error) => Err(error.into()),
    }
}

fn write(directory: &Path, registry: &RegistryFile) -> Result<(), WorkspaceRegistryError> {
    let target = directory.join(REGISTRY_FILE);
    let staged = directory.join(format!("{REGISTRY_FILE}.tmp"));
    std::fs::write(&staged, serde_json::to_vec_pretty(registry)?)?;
    std::fs::rename(staged, target)?;
    Ok(())
}
