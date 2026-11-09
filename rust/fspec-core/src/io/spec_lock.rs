//! BUG-477: the spec write lock as fspec commands use it.
//!
//! Feature: spec/features/concurrent-commands-never-corrupt-spec-json-files.feature
//!
//! Every command that read-modify-writes a spec JSON file takes
//! [`lock_spec_dir`] before its first read of spec state and keeps the guard
//! until it returns, so a concurrent command in another process cannot read
//! the same state and then overwrite this command's update. The protocol
//! itself (stable `spec/.fspec-write.lock`, fs2 exclusive lock, LOCK-001
//! retry budget, per-thread reentrancy) lives in
//! [`codelet_common::spec_lock`] so codelet-core and codelet-agent-loop
//! writers contend on the same lock.

use std::path::{Path, PathBuf};

pub use codelet_common::spec_lock::SpecLock;

use crate::error::FspecCoreError;
use crate::io::project_root::find_or_create_spec_directory;

/// Hold the spec lock for the project rooted at `project_root`.
///
/// The spec directory is resolved exactly as the `ensure_*` loaders resolve
/// it (and created if missing, as they would create it), so the guard
/// covers the files the command is about to read and write.
pub fn lock_spec_dir(
    project_root: &Path,
    command: &'static str,
) -> Result<SpecLock, FspecCoreError> {
    let spec_dir = owned_spec_dir(project_root, command)?;
    lock_dir(&spec_dir, command)
}

/// The spec lock guarding a write of `path` when `path` is a spec state
/// file — a file directly inside a `spec/` directory (work-units.json,
/// tags.json, epics.json, prefixes.json, foundation.json and its draft,
/// schedules.json, fspec-hooks.json, fspec-config.json). Other files get
/// `None`: coverage sidecars under `spec/features/` are guarded by their
/// sidecar transaction, which takes this same lock (BUG-551); nothing else is spec state.
pub fn spec_write_guard(
    path: &Path,
    command: &'static str,
) -> Result<Option<SpecLock>, FspecCoreError> {
    let dir = parent_dir(path, command)?;
    if dir.file_name().is_some_and(|n| n == "spec") {
        lock_dir(dir, command).map(Some)
    } else {
        Ok(None)
    }
}

pub(crate) fn lock_dir(dir: &Path, command: &'static str) -> Result<SpecLock, FspecCoreError> {
    std::fs::create_dir_all(dir).map_err(|source| FspecCoreError::Io { command, source })?;
    SpecLock::acquire(dir).map_err(|source| FspecCoreError::Io { command, source })
}

pub(crate) fn parent_dir<'a>(
    path: &'a Path,
    command: &'static str,
) -> Result<&'a Path, FspecCoreError> {
    match path.parent() {
        Some(p) if p.as_os_str().is_empty() => Ok(Path::new(".")),
        Some(p) => Ok(p),
        None => Err(FspecCoreError::InvalidArgs {
            command,
            reason: format!("{} has no parent directory to lock", path.display()),
        }),
    }
}

/// `<project_root>/spec`, provided that is the spec directory the loaders
/// resolve for `project_root` (created if missing, as they would create it).
///
/// BUG-523: commands write `<project_root>/spec/...` while the loaders walk
/// up to the nearest existing spec directory. When the two differ (a
/// subdirectory of the project handed over as its root) the command would
/// read and lock the project's board and write a second one, losing the
/// update, so it fails here before reading anything. The front doors (the
/// fspec CLI bridges and `dispatch_command`) pass
/// [`crate::io::project_root::resolve_project_root`], which always agrees.
fn owned_spec_dir(project_root: &Path, command: &'static str) -> Result<PathBuf, FspecCoreError> {
    let spec_dir = find_or_create_spec_directory(project_root)?;
    let expected = crate::io::spec_paths::spec_dir_of(project_root);
    if spec_dir == expected {
        return Ok(spec_dir);
    }
    let owner = spec_dir.parent().unwrap_or(&spec_dir);
    Err(FspecCoreError::InvalidArgs {
        command,
        reason: format!(
            "the project root {} does not own the spec directory {}: its writes would go to {} \
             and fork the board. Run the command from {} (or pass that directory as the \
             project root).",
            project_root.display(),
            spec_dir.display(),
            expected.display(),
            owner.display()
        ),
    })
}
