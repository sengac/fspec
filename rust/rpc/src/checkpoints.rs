//! RPC-362: build the checkpoint list + diff/restore/delete helpers for the
//! TUI transport.
//!
//! All git work is delegated to the existing `codelet_git` ghost-commit and
//! diff helpers — no git logic is reimplemented here. The only local logic is
//! enumerating checkpoints across work units, recovering creation timestamps
//! from the `.git/fspec-checkpoints-index/<work_unit_id>.json` sidecar files,
//! sorting most-recent-first, and capping the result at [`MAX_CHECKPOINTS`].

use std::path::Path;

use codelet_git::ghost_commit::{
    delete_ghost_checkpoint, get_checkpoint_diff_files, list_all_ghost_checkpoints,
    restore_ghost_commit, restore_ghost_commit_file, AUTO_CHECKPOINT_PATTERN,
};
use codelet_rpc_types::{ChangedFile, CheckpointInfo};
use serde_json::Value;

/// Maximum number of checkpoints surfaced to the TUI. Mirrors the cap the
/// view applies so the wire payload never grows unbounded.
const MAX_CHECKPOINTS: usize = 200;

/// Full checkpoint ref namespace prefix (mirrors the private constant in
/// `codelet_git::ghost_commit`).
const CHECKPOINT_REF_PREFIX: &str = "refs/fspec-checkpoints";

/// Enumerate every checkpoint across all work units, sort most-recent-first,
/// and cap at [`MAX_CHECKPOINTS`].
///
/// Returns `Err` only when the underlying ref enumeration fails; a non-repo
/// cwd yields an empty Vec via the helper's ENOENT tolerance.
///
/// # Degraded timestamp-ordering contract
/// Ordering is driven by the per-work-unit index sidecar timestamps. When the
/// index is missing or malformed for a checkpoint, [`fallback_timestamp`]
/// substitutes the *current* wall-clock time. In that degraded (no-index) mode
/// the relative ordering of index-less checkpoints is therefore NOT a reliable
/// reflection of true creation order — it only guarantees they sort after
/// (newer than) any checkpoint that still carries a valid recorded timestamp.
pub fn collect_checkpoints(cwd: impl AsRef<Path>) -> codelet_git::Result<Vec<CheckpointInfo>> {
    let cwd = cwd.as_ref();
    let pairs = list_all_ghost_checkpoints(cwd)?;

    let mut out: Vec<CheckpointInfo> = pairs
        .into_iter()
        .map(|(work_unit_id, name)| {
            let is_automatic = name.contains(AUTO_CHECKPOINT_PATTERN);
            let timestamp = read_index(cwd, &work_unit_id)
                .as_ref()
                .and_then(|idx| lookup_timestamp(idx, &name))
                .unwrap_or_else(fallback_timestamp);
            CheckpointInfo {
                work_unit_id,
                name,
                timestamp,
                is_automatic,
            }
        })
        .collect();

    // Newest first: ISO-8601 strings sort lexicographically by chronology.
    out.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    out.truncate(MAX_CHECKPOINTS);
    Ok(out)
}

/// Per-checkpoint changed files — delegates to the ghost-commit diff-files
/// helper and maps each path to a `ChangedFile` (status "M" since the entries
/// describe modifications relative to the checkpoint tree).
pub fn collect_checkpoint_diff_files(
    cwd: impl AsRef<Path>,
    work_unit_id: &str,
    name: &str,
) -> codelet_git::Result<Vec<ChangedFile>> {
    let files = get_checkpoint_diff_files(cwd.as_ref(), work_unit_id, name)?;
    Ok(files
        .into_iter()
        .map(|path| ChangedFile {
            path,
            change_type: "M".to_string(),
            staged: false,
        })
        .collect())
}

/// Unified diff for one file against the checkpoint ref — delegates to
/// `codelet_git::get_checkpoint_file_diff` after resolving the full ref name.
pub fn checkpoint_file_diff(
    cwd: impl AsRef<Path>,
    work_unit_id: &str,
    name: &str,
    path: &str,
) -> codelet_git::Result<Option<String>> {
    let checkpoint_ref = format!("{CHECKPOINT_REF_PREFIX}/{work_unit_id}/{name}");
    codelet_git::get_checkpoint_file_diff(cwd.as_ref(), path, &checkpoint_ref)
}

/// Restore the entire working tree to a checkpoint — delegates to
/// `restore_ghost_commit` (force = true; the TUI confirms beforehand).
pub fn restore_all(
    cwd: impl AsRef<Path>,
    work_unit_id: &str,
    name: &str,
) -> codelet_git::Result<()> {
    restore_ghost_commit(cwd.as_ref(), work_unit_id, name, true)?;
    Ok(())
}

/// Restore a single file from a checkpoint — delegates to
/// `restore_ghost_commit_file`.
pub fn restore_file(
    cwd: impl AsRef<Path>,
    work_unit_id: &str,
    name: &str,
    path: &str,
) -> codelet_git::Result<()> {
    restore_ghost_commit_file(cwd.as_ref(), work_unit_id, name, path)
}

/// Delete a single checkpoint ref and prune its entry from the metadata index.
pub fn delete_one(
    cwd: impl AsRef<Path>,
    work_unit_id: &str,
    name: &str,
) -> codelet_git::Result<()> {
    let cwd = cwd.as_ref();
    delete_ghost_checkpoint(cwd, work_unit_id, name)?;
    remove_index_entry(cwd, work_unit_id, name);
    Ok(())
}

/// Delete every checkpoint across all work units and unlink the index sidecars.
///
/// Source-of-truth contract: deleting each checkpoint *ref* is the propagated
/// operation — its `Err` is returned to the caller and aborts the loop. The
/// subsequent removal of the `.git/fspec-checkpoints-index` directory is a
/// *best-effort* cleanup of now-stale sidecars: its failure is intentionally
/// swallowed (`let _ =`) because the refs (the real checkpoints) are already
/// gone and a leftover index dir is harmless metadata.
pub fn delete_all(cwd: impl AsRef<Path>) -> codelet_git::Result<()> {
    let cwd = cwd.as_ref();
    for (work_unit_id, name) in list_all_ghost_checkpoints(cwd)? {
        delete_ghost_checkpoint(cwd, &work_unit_id, &name)?;
    }
    // Remove the entire index directory — every sidecar is now stale.
    let index_dir = cwd.join(".git").join("fspec-checkpoints-index");
    let _ = std::fs::remove_dir_all(&index_dir);
    Ok(())
}

/// Read `.git/fspec-checkpoints-index/<work_unit_id>.json` (ENOENT / malformed
/// JSON → `None`).
fn read_index(cwd: &Path, work_unit_id: &str) -> Option<Value> {
    let path = cwd
        .join(".git")
        .join("fspec-checkpoints-index")
        .join(format!("{work_unit_id}.json"));
    let raw = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

/// Look up a checkpoint's timestamp in the metadata index by name.
fn lookup_timestamp(index: &Value, name: &str) -> Option<String> {
    index
        .get("checkpoints")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|cp| cp.get("name").and_then(|n| n.as_str()) == Some(name))
        })
        .and_then(|cp| cp.get("timestamp").and_then(|t| t.as_str()))
        .map(String::from)
}

/// Remove a single checkpoint entry from the metadata index sidecar, if present.
fn remove_index_entry(cwd: &Path, work_unit_id: &str, name: &str) {
    let path = cwd
        .join(".git")
        .join("fspec-checkpoints-index")
        .join(format!("{work_unit_id}.json"));
    let Some(mut index) = read_index(cwd, work_unit_id) else {
        return;
    };
    if let Some(arr) = index.get_mut("checkpoints").and_then(|v| v.as_array_mut()) {
        arr.retain(|cp| cp.get("name").and_then(|n| n.as_str()) != Some(name));
    }
    if let Ok(serialized) = serde_json::to_string_pretty(&index) {
        let _ = std::fs::write(&path, serialized);
    }
}

/// Fallback ISO-8601 timestamp used when the index is missing or malformed.
/// Civil-time decomposition over `SystemTime::UNIX_EPOCH` avoids a chrono dep,
/// matching `codelet_core::commands::list_checkpoints::fallback_timestamp`.
fn fallback_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs_total = dur.as_secs() as i64;
    let millis = dur.subsec_millis();

    let days = secs_total.div_euclid(86_400);
    let secs_of_day = secs_total.rem_euclid(86_400);
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;

    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}
