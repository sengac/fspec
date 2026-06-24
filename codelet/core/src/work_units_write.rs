//! Pure-Rust write helper for `spec/work-units.json` (RPC-017).
//!
//! Sibling of [`crate::work_units`] (read-side: `read_snapshot` +
//! `WorkUnitsWatcher`). Lives in its own file so the existing
//! `work_units.rs` stays read-focused and under its current LoC
//! ceiling (architecture note 2 in RPC-017).
//!
//! Public surface:
//!
//! - [`Direction`] — `Up | Down` for [`move_work_unit`].
//! - [`move_work_unit`] — atomically reorder a work unit one slot
//!   within its current `states[<column>]` array.
//!
//! Semantics mirror `src/commands/prioritize-work-unit.ts`:
//!   - Reorder within `states[<column>]` only; never across columns.
//!   - Done column refuses reorders (returns `Err`).
//!   - Boundary moves (Up at index 0; Down at index len-1) are no-ops
//!     that return `Ok(())` without rewriting the file.
//!   - Unknown ids return `Err`.
//!
//! Persistence uses [`codelet_common::file_lock::with_file_lock`] for
//! the inter-process lock (proper-lockfile-compatible mkdir lock at
//! `spec/work-units.json.lock`) plus an atomic temp-file + rename for
//! crash safety. Concurrent TS `fspec prioritize-work-unit` commands
//! cooperate transparently through the same lock protocol.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context as _, Result};
use chrono::Utc;
use codelet_common::file_lock::with_file_lock;
use serde_json::Value;
use uuid::Uuid;

/// Direction of a single-step reorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

/// Move the work unit with `id` one position in `direction` within its
/// current `states[<column>]` array in `<cwd>/spec/work-units.json`.
///
/// See module docs for the full semantics.
pub fn move_work_unit(cwd: &Path, id: &str, direction: Direction) -> Result<()> {
    let work_units_path = cwd.join("spec").join("work-units.json");
    let lock_dir = cwd.join("spec").join("work-units.json.lock");

    // Ensure the parent directory exists before attempting to acquire
    // the lock (mkdir requires the parent to exist on POSIX).
    if let Some(parent) = lock_dir.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }

    let id = id.to_string();
    with_file_lock(&lock_dir, || -> Result<(), String> {
        move_work_unit_locked(&work_units_path, &id, direction).map_err(|e| format!("{e:#}"))
    })
    .map_err(|e| anyhow!("{e}"))
}

/// Inner reorder that runs while the inter-process lock is held.
fn move_work_unit_locked(work_units_path: &Path, id: &str, direction: Direction) -> Result<()> {
    let content = fs::read_to_string(work_units_path)
        .with_context(|| format!("read {}", work_units_path.display()))?;
    let mut root: Value = serde_json::from_str(&content)
        .with_context(|| format!("parse {}", work_units_path.display()))?;

    // Resolve the work unit's current status.
    let status = root
        .get("workUnits")
        .and_then(|wus| wus.get(id))
        .and_then(|wu| wu.get("status"))
        .and_then(|s| s.as_str())
        .ok_or_else(|| anyhow!("work unit '{id}' not found in spec/work-units.json"))?
        .to_string();

    // Done column is not reorderable (matches TS prioritize-work-unit.ts:38-42).
    if status == "done" {
        return Err(anyhow!(
            "Cannot prioritize work units in done column. Done items are ordered by completion time and cannot be manually reordered. ('{id}')"
        ));
    }

    // Borrow the states[<column>] array, swap in place when in-bounds.
    let states_obj = root
        .get_mut("states")
        .and_then(|s| s.as_object_mut())
        .ok_or_else(|| anyhow!("spec/work-units.json is missing the 'states' object"))?;
    let column = states_obj
        .get_mut(&status)
        .and_then(|c| c.as_array_mut())
        .ok_or_else(|| {
            anyhow!("spec/work-units.json::states.{status} is missing or not an array")
        })?;

    let pos = column
        .iter()
        .position(|v| v.as_str() == Some(id))
        .ok_or_else(|| anyhow!(
            "Data integrity error: work unit '{id}' has status '{status}' but is not in states.{status}. Run 'fspec repair-work-units' to fix data corruption."
        ))?;

    match direction {
        Direction::Up => {
            if pos == 0 {
                // Top boundary — no-op (no file rewrite).
                return Ok(());
            }
            column.swap(pos, pos - 1);
        }
        Direction::Down => {
            if pos + 1 >= column.len() {
                // Bottom boundary — no-op.
                return Ok(());
            }
            column.swap(pos, pos + 1);
        }
    }

    // Bump meta.lastUpdated so consumers can observe the change.
    if let Some(meta) = root.get_mut("meta").and_then(|m| m.as_object_mut()) {
        meta.insert(
            "lastUpdated".to_string(),
            Value::String(Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
        );
    }

    // Atomic write: serialise to a sibling temp file, then rename.
    let pretty = serde_json::to_string_pretty(&root).context("re-serialise work-units.json")?;
    let temp_path: PathBuf =
        work_units_path.with_file_name(format!("work-units.json.tmp.{}", Uuid::new_v4()));
    fs::write(&temp_path, &pretty)
        .with_context(|| format!("write temp file {}", temp_path.display()))?;
    fs::rename(&temp_path, work_units_path).with_context(|| {
        let _ = fs::remove_file(&temp_path);
        format!(
            "atomic rename {} -> {}",
            temp_path.display(),
            work_units_path.display()
        )
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use tempfile::TempDir;

    fn write_file(workspace: &Path, body: &str) {
        let spec = workspace.join("spec");
        fs::create_dir_all(&spec).unwrap();
        fs::write(spec.join("work-units.json"), body).unwrap();
    }

    #[test]
    fn unit_test_module_compiles() {
        // Smoke-level: just verify the module compiles and Direction
        // values participate in pattern matching. Behavioural assertions
        // live in `codelet/core/tests/work_units_write_test.rs`.
        let dir = TempDir::new().unwrap();
        write_file(
            dir.path(),
            r#"{
              "meta": {"version":"1.0.0","lastUpdated":"2026-01-01T00:00:00.000Z"},
              "workUnits": {"A": {"id":"A","title":"a","type":"story","status":"backlog"}},
              "states": {"backlog":["A"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
            }"#,
        );
        // Top-boundary no-op — must succeed without rewriting.
        move_work_unit(dir.path(), "A", Direction::Up).unwrap();
    }
}
