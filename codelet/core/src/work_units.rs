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
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkUnitsFile {
    work_units: HashMap<String, WorkUnitRecord>,
}

/// One-shot read of `<workspace>/spec/work-units.json`.
///
/// Returns an empty vec (NOT an error) if the file does not exist —
/// matches the legacy NAPI behaviour so the rpc-server binary can start
/// in workspaces that have not yet been bootstrapped.
pub fn read_snapshot(workspace: &Path) -> Result<Vec<WorkUnitInfo>> {
    let path = workspace.join("spec").join("work-units.json");
    if !path.exists() {
        debug!("work-units.json does not exist at {}; returning empty snapshot", path.display());
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    let file: WorkUnitsFile = serde_json::from_str(&content)
        .with_context(|| format!("parse {}", path.display()))?;
    let mut out: Vec<WorkUnitInfo> = file
        .work_units
        .into_values()
        .map(WorkUnitInfo::from)
        .collect();
    // Sort for deterministic output across HashMap iteration order so
    // cross-transport parity tests can compare bytewise.
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
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
        let cb_path = work_units_path.clone();
        let mut debouncer = new_debouncer(
            Duration::from_millis(100),
            move |res: std::result::Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
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
                    matches!(e.kind, DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous)
                        && e.path.file_name().is_some_and(|n| n == "work-units.json")
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
