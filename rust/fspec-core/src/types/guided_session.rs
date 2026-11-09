//! Guided sessions (AUDIT-004): the session file and step counter shared by
//! `fspec reverse` and `fspec audit --start`.
//!
//! Feature: spec/features/audit-sessions-walk-the-judgment-problems-step-by-step.feature
//!
//! A session is an ephemeral JSON file in the OS temp directory,
//! `fspec-<command>-<hash>.json`, where `<hash>` is the first 12 hex
//! characters of the SHA-256 of the session key: the canonical project root,
//! followed (when the scope is not empty) by a NUL and the scope's parts
//! joined by NULs. With an empty scope the key is the bare project root, so
//! `fspec reverse`'s file (RPC-294, BUG-295) is unchanged. All I/O is
//! blocking `std::fs`.

use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::io::project_root::find_project_root;

/// The canonical project root of `project_root` (BUG-295: symlinked or
/// differently spelled paths to one project share a session).
pub fn canonical_root(project_root: &Path) -> PathBuf {
    let root = find_project_root(project_root);
    std::fs::canonicalize(&root).unwrap_or(root)
}

/// The session file of `command` for `root` (used as given) and `scope`.
pub fn path_for(command: &str, root: &Path, scope: &[String]) -> PathBuf {
    let mut key = root.to_string_lossy().into_owned();
    for part in scope {
        key.push('\0');
        key.push_str(part);
    }
    let digest = Sha256::digest(key.as_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    std::env::temp_dir().join(format!("fspec-{command}-{}.json", &hex[..12]))
}

/// The session file of `command` for the canonical root of `project_root`
/// and `scope`.
pub fn session_path(command: &str, project_root: &Path, scope: &[String]) -> PathBuf {
    path_for(command, &canonical_root(project_root), scope)
}

/// The session at `path`; `None` when it is missing or does not parse.
pub fn load<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Write `session` to `path` as 2-space indented JSON.
pub fn save<T: Serialize>(path: &Path, session: &T) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(session).map_err(std::io::Error::other)?;
    std::fs::write(path, json)
}

/// Delete the session at `path`; a missing file is not an error.
pub fn delete(path: &Path) {
    let _ = std::fs::remove_file(path);
}

/// The step after `current` (`None` counts as step 1).
pub fn next_step(current: Option<u64>) -> u64 {
    current.unwrap_or(1) + 1
}

/// Whether a session on step `current` of `total` has reached its last step
/// (both counters set and positive).
pub fn on_last_step(current: Option<u64>, total: Option<u64>) -> bool {
    match (current, total) {
        (Some(current), Some(total)) if current > 0 && total > 0 => current >= total,
        _ => false,
    }
}

/// Whether a session on step `current` of `total` has continued past its
/// last step, so every step is done.
pub fn every_step_done(current: u64, total: u64) -> bool {
    current > total
}
