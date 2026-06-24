//! Pure-Rust `WorkUnitsWatcher` for `spec/work-units.json` (RPC-006).
//!
//! Lifted out of `codelet/napi/src/work_units_watcher.rs` so that it can
//! be reached from the shared `FspecService` implementation in
//! `codelet/rpc` without dragging the NAPI runtime into the dependency
//! arrow. The legacy NAPI shim remains in `codelet/napi` and is now a
//! thin wrapper that subscribes to this watcher and forwards each
//! broadcast event into the existing `ThreadsafeFunction` callback.
//!
//! Public surface (per RPC-006 acceptance criteria):
//!
//! - [`read_snapshot`] — one-shot read of `spec/work-units.json`.
//! - [`WorkUnitsWatcher`] — long-lived debounced fs-watcher that
//!   publishes the current snapshot to subscribers via a
//!   `tokio::sync::broadcast` channel.
//!
//! Architecture invariants (from spec/attachments/RPC-006/plan.md):
//!
//! - The legacy NAPI watcher used a global `lazy_static!`. This pure-Rust
//!   re-implementation is instance-bound so multiple test workspaces can
//!   coexist in the same process, and so the embedded transport's
//!   `EmbeddedTransport::work_units_rx` returns a subscription tied to
//!   THIS watcher rather than process-global state.
//! - Broadcast capacity is bounded at 64 (architecture note 12). Lagging
//!   subscribers receive `RecvError::Lagged` and resync on the next
//!   snapshot — acceptable because every payload is a full snapshot,
//!   not an incremental delta.
//! - The fs-watch event filter only fires for events touching
//!   `work-units.json` itself; spec/ also contains lock files
//!   (proper-lockfile `.lock` directories) that would otherwise create
//!   an infinite read→lock→event→read feedback loop.

use anyhow::{Context as _, Result};
use codelet_rpc_types::WorkUnitInfo;
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind, Debouncer};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, warn};

/// Wire format for a single work unit in `spec/work-units.json`.
///
/// Matches the camelCase JSON the `fspec` CLI writes; `type` maps to
/// `work_type` per the existing NAPI shim's serde rename.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkUnitRecord {
    id: String,
    title: String,
    #[serde(rename = "type", default)]
    work_type: Option<String>,
    status: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    estimate: Option<i32>,
    #[serde(default)]
    epic: Option<String>,
    /// RPC-014: attachment file paths read from `spec/work-units.json`.
    /// `#[serde(default)]` so legacy files without the field still parse
    /// as an empty Vec.
    #[serde(default)]
    attachments: Vec<String>,
    /// RPC-016: the `stateHistory` array from `spec/work-units.json` —
    /// only the timestamp of the LAST entry is exposed downstream as
    /// `WorkUnitInfo.last_state_change_at`. Legacy records without
    /// `stateHistory` default to an empty Vec and yield `None`.
    #[serde(default)]
    state_history: Vec<StateHistoryEntry>,
}

/// RPC-016: a single entry in `spec/work-units.json::stateHistory`.
///
/// Only `timestamp` is read by codelet_core — the full record may
/// contain `from`/`to`/`actor` fields the CLI writes but the RPC
/// surface does not currently expose. `#[serde(default)]` is applied
/// to the unused fields so historical entries with absent fields still
/// deserialize; `timestamp` itself is left strict because every
/// stateHistory entry the CLI writes carries one.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StateHistoryEntry {
    timestamp: String,
    #[serde(default, rename = "from")]
    _from: Option<String>,
    #[serde(default, rename = "to")]
    _to: Option<String>,
    #[serde(default, rename = "actor")]
    _actor: Option<String>,
}

impl From<WorkUnitRecord> for WorkUnitInfo {
    fn from(record: WorkUnitRecord) -> Self {
        let work_type = record.work_type.unwrap_or_else(|| {
            warn!(
                "Work unit {} is missing 'type' field, defaulting to 'story'",
                record.id
            );
            "story".to_string()
        });
        WorkUnitInfo {
            id: record.id,
            title: record.title,
            work_type,
            status: record.status,
            description: record.description,
            estimate: record.estimate,
            epic: record.epic,
            attachments: record.attachments,
            // RPC-016: pick the timestamp of the LAST stateHistory entry.
            // Legacy records (empty Vec) yield None.
            last_state_change_at: record
                .state_history
                .last()
                .map(|entry| entry.timestamp.clone()),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkUnitsFile {
    work_units: HashMap<String, WorkUnitRecord>,
    /// Display-order arrays per column status. Mirrors the TS
    /// `states` field in `spec/work-units.json`. The TS TUI builds its
    /// `workUnits[]` by iterating `states[<column>]` in column order
    /// and looking up records by id — preserving the manual ordering
    /// the user has imposed via `[`/`]` priority moves and the
    /// "most-recent-first" ordering of the `done` column. Optional
    /// because legacy/test JSON may not include it.
    #[serde(default)]
    states: HashMap<String, Vec<String>>,
}

/// Canonical column order — must match `COLUMN_ORDER` in
/// `codelet/fspec-tui/src/store/board.rs` and `STATES` in
/// `src/tui/components/UnifiedBoardLayout.tsx`.
const COLUMN_ORDER: [&str; 7] = [
    "backlog",
    "specifying",
    "testing",
    "implementing",
    "validating",
    "done",
    "blocked",
];

/// One-shot read of `<workspace>/spec/work-units.json`.
///
/// Returns an empty vec (NOT an error) if the file does not exist —
/// matches the legacy NAPI behaviour so the rpc-server binary can start
/// in workspaces that have not yet been bootstrapped.
///
/// Ordering: when the file contains a top-level `states` object (the
/// standard `fspec` schema), records are emitted in the order
/// `states[<column>]` for each column in `COLUMN_ORDER`. This mirrors
/// the TS `fspecStore.loadData()` behaviour and preserves the user's
/// manual `[`/`]` priority ordering on the kanban board. Records not
/// referenced from `states` (and records referenced from unknown
/// columns) are appended afterwards, sorted by id for determinism.
pub fn read_snapshot(workspace: &Path) -> Result<Vec<WorkUnitInfo>> {
    let path = workspace.join("spec").join("work-units.json");
    if !path.exists() {
        debug!(
            "work-units.json does not exist at {}; returning empty snapshot",
            path.display()
        );
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let file: WorkUnitsFile =
        serde_json::from_str(&content).with_context(|| format!("parse {}", path.display()))?;

    let mut records = file.work_units;
    let mut ordered: Vec<WorkUnitInfo> = Vec::with_capacity(records.len());

    // First pass: emit records in `states[<column>]` order.
    for column in COLUMN_ORDER {
        if let Some(ids) = file.states.get(column) {
            for id in ids {
                if let Some(record) = records.remove(id) {
                    ordered.push(WorkUnitInfo::from(record));
                }
            }
        }
    }

    // Second pass: anything still in `records` (records whose status
    // didn't appear in any `states` array, or workspaces with no
    // `states` field at all) is appended sorted by id so the
    // cross-transport parity tests still get deterministic output.
    let mut leftover: Vec<WorkUnitInfo> = records.into_values().map(WorkUnitInfo::from).collect();
    leftover.sort_by(|a, b| a.id.cmp(&b.id));
    ordered.extend(leftover);
    Ok(ordered)
}

/// Long-lived debounced file-system watcher for `spec/work-units.json`.
///
/// Reads the initial snapshot synchronously in [`Self::new`], starts a
/// notify-debouncer task that re-reads the file when it changes, and
/// publishes every snapshot (initial + every change) on a
/// `tokio::sync::broadcast` channel. Drops the underlying debouncer on
/// drop — there is no global state, so multiple workspaces are isolated.
pub struct WorkUnitsWatcher {
    /// Latest known snapshot. Read by [`Self::snapshot`] and updated by
    /// the debouncer callback before the broadcast send.
    state: Arc<RwLock<Vec<WorkUnitInfo>>>,
    /// Broadcast tx kept alive so subscribers can clone receivers from
    /// it via [`Self::subscribe`].
    tx: broadcast::Sender<Vec<WorkUnitInfo>>,
    /// Owned debouncer — dropping the watcher tears down fs-watching.
    /// Stored behind a `Mutex<Option<...>>` so [`Drop`] can take it out
    /// without requiring `&mut self`.
    _debouncer: Arc<Mutex<Option<Debouncer<RecommendedWatcher>>>>,
    /// Path the watcher was started with — used to resolve the
    /// `work-units.json` filter inside the debouncer callback.
    _workspace: PathBuf,
}

impl WorkUnitsWatcher {
    /// Construct a new watcher observing `<workspace>/spec/work-units.json`.
    ///
    /// Reads the initial snapshot synchronously and broadcasts it
    /// immediately so that fresh subscribers always see at least the
    /// initial state once they `subscribe()` and recv (subject to the
    /// usual broadcast-channel race: subscribers that subscribe AFTER
    /// the initial broadcast must call `snapshot()` to fill in).
    pub fn new(workspace: &Path) -> Result<Self> {
        let initial = read_snapshot(workspace)?;
        let (tx, _) = broadcast::channel::<Vec<WorkUnitInfo>>(64);
        let state = Arc::new(RwLock::new(initial.clone()));

        let workspace_buf = workspace.to_path_buf();
        let work_units_path = workspace_buf.join("spec").join("work-units.json");
        let spec_dir = work_units_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        let cb_state = Arc::clone(&state);
        let cb_tx = tx.clone();
        let cb_path = work_units_path;
        let mut debouncer = new_debouncer(
            Duration::from_millis(100),
            move |res: std::result::Result<
                Vec<notify_debouncer_mini::DebouncedEvent>,
                notify::Error,
            >| {
                let events = match res {
                    Ok(events) => events,
                    Err(e) => {
                        warn!("fs-watch error: {:?}", e);
                        return;
                    }
                };
                // Only react to events on `work-units.json` itself —
                // proper-lockfile creates `.lock` dirs in spec/ that
                // would otherwise feedback-loop.
                let relevant = events.iter().any(|e| {
                    matches!(
                        e.kind,
                        DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous
                    ) && e.path.file_name().is_some_and(|n| n == "work-units.json")
                });
                if !relevant {
                    return;
                }
                match read_snapshot(workspace_root(&cb_path)) {
                    Ok(snapshot) => {
                        if let Ok(mut guard) = cb_state.write() {
                            *guard = snapshot.clone();
                        }
                        // `send` only errors when there are zero
                        // receivers — that is fine; the next subscriber
                        // can call `snapshot()` to backfill.
                        let _ = cb_tx.send(snapshot);
                    }
                    Err(e) => warn!("failed to reload work-units snapshot: {e}"),
                }
            },
        )
        .context("create notify debouncer")?;

        // The watcher requires the parent directory to exist. If it
        // doesn't, create it so the watch call succeeds in workspaces
        // that have not yet been bootstrapped.
        if !spec_dir.exists() {
            fs::create_dir_all(&spec_dir)
                .with_context(|| format!("create {}", spec_dir.display()))?;
        }
        debouncer
            .watcher()
            .watch(&spec_dir, RecursiveMode::NonRecursive)
            .with_context(|| format!("watch {}", spec_dir.display()))?;

        // Broadcast the initial snapshot so subscribers that subscribe
        // BEFORE the next file change still observe at least one value
        // on the channel. (Tests use this to wait on the initial
        // snapshot deterministically.)
        let _ = tx.send(initial);

        Ok(Self {
            state,
            tx,
            _debouncer: Arc::new(Mutex::new(Some(debouncer))),
            _workspace: workspace_buf,
        })
    }

    /// Snapshot of the most recent successful read of `work-units.json`.
    pub fn snapshot(&self) -> Vec<WorkUnitInfo> {
        match self.state.read() {
            Ok(guard) => guard.clone(),
            Err(_) => Vec::new(),
        }
    }

    /// Subscribe to broadcasts of full snapshots whenever the watched
    /// file changes. Subscribing AFTER the initial broadcast does not
    /// receive it; pair this with [`Self::snapshot`] to backfill.
    pub fn subscribe(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
        self.tx.subscribe()
    }
}

/// Recover the workspace root from the watched path
/// `<workspace>/spec/work-units.json`. Used inside the debouncer
/// callback so we never call back into `&Path` borrows that the move
/// closure can't capture.
fn workspace_root(work_units_path: &Path) -> &Path {
    work_units_path
        .parent()
        .and_then(Path::parent)
        .unwrap_or(work_units_path)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_workspace(tmp: &TempDir, json: &str) {
        let spec = tmp.path().join("spec");
        std::fs::create_dir_all(&spec).unwrap();
        let mut f = std::fs::File::create(spec.join("work-units.json")).unwrap();
        f.write_all(json.as_bytes()).unwrap();
    }

    #[test]
    fn read_snapshot_preserves_states_array_order_per_column() {
        // TS reference: src/tui/__tests__/done-column-sorting.test.ts.
        // The Rust port must mirror `fspecStore.loadData()` — build the
        // workUnits vec by walking `states[<column>]` in column order.
        let tmp = TempDir::new().unwrap();
        write_workspace(
            &tmp,
            r#"{
              "workUnits": {
                "BOARD-001": {"id": "BOARD-001", "title": "Backlog Unit 1", "type": "story", "status": "backlog"},
                "BOARD-002": {"id": "BOARD-002", "title": "Backlog Unit 2", "type": "story", "status": "backlog"},
                "BOARD-003": {"id": "BOARD-003", "title": "Backlog Unit 3", "type": "story", "status": "backlog"},
                "DONE-001":  {"id": "DONE-001",  "title": "Done Unit 1",    "type": "story", "status": "done"},
                "DONE-002":  {"id": "DONE-002",  "title": "Done Unit 2",    "type": "story", "status": "done"}
              },
              "states": {
                "backlog":      ["BOARD-002", "BOARD-003", "BOARD-001"],
                "specifying":   [],
                "testing":      [],
                "implementing": [],
                "validating":   [],
                "done":         ["DONE-002", "DONE-001"],
                "blocked":      []
              }
            }"#,
        );
        let snap = read_snapshot(tmp.path()).unwrap();
        let ids: Vec<&str> = snap.iter().map(|u| u.id.as_str()).collect();
        // Column order: backlog first (in its file order), then done.
        assert_eq!(
            ids,
            vec![
                "BOARD-002",
                "BOARD-003",
                "BOARD-001",
                "DONE-002",
                "DONE-001"
            ]
        );
    }

    #[test]
    fn read_snapshot_appends_orphan_records_sorted_by_id() {
        let tmp = TempDir::new().unwrap();
        write_workspace(
            &tmp,
            r#"{
              "workUnits": {
                "Z-001": {"id": "Z-001", "title": "z", "type": "story", "status": "backlog"},
                "A-001": {"id": "A-001", "title": "a", "type": "story", "status": "backlog"}
              },
              "states": {
                "backlog":      [],
                "specifying":   [],
                "testing":      [],
                "implementing": [],
                "validating":   [],
                "done":         [],
                "blocked":      []
              }
            }"#,
        );
        let snap = read_snapshot(tmp.path()).unwrap();
        let ids: Vec<&str> = snap.iter().map(|u| u.id.as_str()).collect();
        assert_eq!(ids, vec!["A-001", "Z-001"]);
    }
}
