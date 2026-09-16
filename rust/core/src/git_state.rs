//! BUG-182: pure-Rust `GitStateWatcher` — the ONE centralized git polling
//! mechanism for the workspace cwd.
//!
//! Feature: spec/features/git-state-watcher.feature
//!
//! Replaces the BUG-181 `CheckpointsWatcher` (removed) as the
//! checkpoint-count source so there is exactly one watcher. Mirrors the
//! [`crate::work_units::WorkUnitsWatcher`] pattern:
//!
//! - A debounced `notify` watch over `.git/` (recursive when a repo,
//!   non-recursive root watch for `.git` creation otherwise — same
//!   graceful degradation as BUG-181);
//! - PLUS a `tokio::time::interval` periodic poll (default 10 seconds,
//!   configurable via [`Self::with_interval`] for tests) that re-runs the
//!   full [`GitState`] snapshot (R8: the poll interval is injectable so
//!   the periodic-refresh behavior is testable deterministically);
//! - A `tokio::sync::broadcast` channel (capacity 64) on which a fresh
//!   snapshot is published ONLY when it differs from the last published
//!   one (dedup — unchanged poll ticks never re-broadcast).
//!
//! The snapshot combines:
//!
//! - `git_branch` — `codelet_git::status::get_current_branch`;
//! - `checkpoint_counts` — `codelet_git::ghost_commit::count_checkpoints`;
//! - `changed_files` — staged + unstaged + untracked (the same collect as
//!   `codelet_rpc::changed_files::collect_changed_files`);
//! - `checkpoints` — enumerated most-recent-first, capped at 200 (the same
//!   enumerate as `codelet_rpc::checkpoints::collect_checkpoints`).
//!
//! Graceful degradation (matches `count_checkpoints`' ENOENT tolerance):
//!
//! - Missing `.git` / non-repo cwd → empty `GitState` (zero counts, empty
//!   lists, `git_branch: None`) and a root-directory watch for the
//!   appearance of a `.git` entry, so a later `git init` triggers a fresh
//!   snapshot.
//! - Any debouncer/watch setup failure degrades to "no fs-watch, static
//!   snapshot" — the periodic poll still runs; the `snapshot()` accessor
//!   always returns the last known state.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use codelet_git::ghost_commit::{count_checkpoints, list_all_ghost_checkpoints};
use codelet_git::status::{
    get_current_branch, get_staged_files_with_change_type, get_unstaged_files_with_change_type,
    get_untracked_files,
};
use codelet_rpc_types::{ChangedFile, CheckpointInfo, GitState};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind, Debouncer};
use tokio::sync::broadcast;
use tracing::{debug, warn};

/// Broadcast channel capacity — full `GitState` snapshots are bounded
/// (200 checkpoints max), so 64 is generous enough that lagging
/// subscribers simply resync on the next frame (mirrors
/// `WorkUnitsWatcher`).
const GIT_STATE_WATCHER_CAPACITY: usize = 64;

/// BUG-182: default periodic poll interval (the user directive: ~10 s).
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(10);

/// Maximum checkpoints carried in a `GitState` frame — mirrors the cap
/// `codelet_rpc::checkpoints::collect_checkpoints` applies so the wire
/// payload never grows unbounded.
const MAX_CHECKPOINTS: usize = 200;

/// Fallback ISO-8601 timestamp used when the per-work-unit index sidecar
/// is missing or malformed for a checkpoint (mirrors
/// `codelet_rpc::checkpoints::fallback_timestamp`'s contract: "now").
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

/// Build the `checkpoints` leg of a [`GitState`] frame: enumerate every
/// checkpoint, sort most-recent-first, cap at [`MAX_CHECKPOINTS`].
///
/// Non-repo / corrupt-refs degrade to an empty list (ENOENT tolerance
/// mirrors `codelet_rpc::checkpoints`).
fn collect_checkpoints(cwd: &Path) -> Vec<CheckpointInfo> {
    let pairs = match list_all_ghost_checkpoints(cwd) {
        Ok(pairs) => pairs,
        Err(e) => {
            warn!(error = %e, "git-state: checkpoint enumeration failed; empty list");
            return Vec::new();
        }
    };
    let mut out: Vec<CheckpointInfo> = pairs
        .into_iter()
        .take(MAX_CHECKPOINTS)
        .map(|(work_unit_id, name)| CheckpointInfo {
            is_automatic: name.contains(codelet_git::ghost_commit::AUTO_CHECKPOINT_PATTERN),
            work_unit_id,
            name,
            // Without the per-work-unit index sidecar the creation
            // timestamp is unknown — the degraded "now" fallback keeps
            // the frame self-contained (the TUI re-fetches the full list
            // on view open anyway; this leg exists for the dedup diff).
            timestamp: fallback_timestamp(),
        })
        .collect();
    // Newest first: ISO-8601 strings sort lexicographically by chronology
    // (same ordering rule as codelet_rpc::checkpoints).
    out.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    out
}

/// Build the `changed_files` leg of a [`GitState`] frame: staged first,
/// then unstaged modifications/deletions, then untracked (always Added) —
/// byte-identical ordering to
/// `codelet_rpc::changed_files::collect_changed_files`.
fn collect_changed_files(cwd: &Path) -> Vec<ChangedFile> {
    let mut out: Vec<ChangedFile> = Vec::new();
    let staged = match get_staged_files_with_change_type(cwd) {
        Ok(files) => files,
        Err(e) => {
            warn!(error = %e, "git-state: staged-file collect failed; skipping");
            Vec::new()
        }
    };
    for entry in staged {
        out.push(ChangedFile {
            path: entry.path,
            change_type: entry.change_type.as_letter().to_string(),
            staged: true,
        });
    }
    let unstaged = match get_unstaged_files_with_change_type(cwd) {
        Ok(files) => files,
        Err(e) => {
            warn!(error = %e, "git-state: unstaged-file collect failed; skipping");
            Vec::new()
        }
    };
    for entry in unstaged {
        out.push(ChangedFile {
            path: entry.path,
            change_type: entry.change_type.as_letter().to_string(),
            staged: false,
        });
    }
    if let Ok(untracked) = get_untracked_files(cwd) {
        for path in untracked {
            out.push(ChangedFile {
                path,
                change_type: "A".to_string(),
                staged: false,
            });
        }
    } else {
        warn!("git-state: untracked-file collect failed; skipping");
    }
    out
}

/// Capture one full `GitState` snapshot for `cwd` (ENOENT-tolerant: a
/// non-repo directory yields the empty `GitState`, never an error).
fn capture_git_state(cwd: &Path) -> GitState {
    let git_branch = get_current_branch(cwd).unwrap_or_default();
    let checkpoint_counts = match count_checkpoints(cwd) {
        Ok(counts) => counts,
        Err(e) => {
            warn!(error = %e, "git-state: count_checkpoints failed; zero counts");
            codelet_rpc_types::CheckpointCounts::default()
        }
    };
    GitState {
        git_branch,
        checkpoint_counts,
        changed_files: collect_changed_files(cwd),
        checkpoints: collect_checkpoints(cwd),
    }
}

/// Long-lived centralized git-state watcher over the workspace cwd
/// (BUG-182).
///
/// Reads the initial snapshot synchronously in [`Self::new`], starts the
/// debounced fs-watch AND the periodic poll task, and publishes every
/// CHANGED snapshot on a `tokio::sync::broadcast` channel (dedup: an
/// unchanged re-snapshot is never re-broadcast). Drops the underlying
/// debouncer on drop — there is no global state, so multiple workspaces
/// are isolated.
pub struct GitStateWatcher {
    /// Latest known snapshot. Read by [`Self::snapshot`] and updated by
    /// the fs-watch / poll path before the broadcast send.
    state: Arc<RwLock<GitState>>,
    /// Broadcast tx kept alive so subscribers can clone receivers from
    /// it via [`Self::subscribe`].
    tx: broadcast::Sender<GitState>,
    /// Owned debouncer — dropping the watcher tears down fs-watching.
    /// Stored behind a `Mutex<Option<...>>` so [`Drop`] can take it out
    /// without requiring `&mut self`. `None` when the fs-watch setup
    /// degraded away — the periodic poll still runs and the snapshot
    /// accessor still works.
    _debouncer: Arc<Mutex<Option<Debouncer<RecommendedWatcher>>>>,
    /// The workspace cwd the watcher was started with.
    _cwd: PathBuf,
}

impl GitStateWatcher {
    /// Construct a new watcher over the workspace `cwd` with the default
    /// 10-second poll interval.
    ///
    /// Reads the initial snapshot synchronously and broadcasts it
    /// immediately so that fresh subscribers always see at least the
    /// initial state once they `subscribe()` and recv (subject to the
    /// usual broadcast-channel race: subscribers that subscribe AFTER
    /// the initial broadcast must call [`Self::snapshot`] to backfill).
    ///
    /// Never fails: a missing `.git` directory degrades to an empty
    /// `GitState` + a root-directory watch for the `.git` entry; a
    /// debouncer creation failure degrades to a static snapshot (no
    /// fs-watch, periodic poll only). Errors are surfaced to the tracing
    /// layer, never to the caller.
    pub fn new(cwd: &Path) -> Self {
        Self::with_interval(cwd, DEFAULT_POLL_INTERVAL)
    }

    /// Construct a watcher with an explicit poll interval (R8: tests
    /// inject a short interval such as 200ms so the periodic-refresh
    /// behavior is deterministic).
    pub fn with_interval(cwd: &Path, poll_interval: Duration) -> Self {
        let initial = capture_git_state(cwd);
        let (tx, _) = broadcast::channel::<GitState>(GIT_STATE_WATCHER_CAPACITY);
        let state = Arc::new(RwLock::new(initial.clone()));

        let cb_state = Arc::clone(&state);
        let cb_tx = tx.clone();
        let cb_cwd = cwd.to_path_buf();

        let debouncer = build_debouncer(&cb_state, &cb_tx, &cb_cwd);

        // BUG-182: the periodic poll re-captures the full GitState on
        // every tick — catching working-tree changes that emit no `.git`
        // event (agent tool commits, other-terminal work). The tick uses
        // MissedTickBehavior::Skip so a slow capture never bursts.
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let poll_state = Arc::clone(&state);
            let poll_tx = tx.clone();
            let poll_cwd = cwd.to_path_buf();
            handle.spawn(async move {
                let mut interval = tokio::time::interval(poll_interval);
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    interval.tick().await;
                    re_snapshot_and_publish(&poll_state, &poll_tx, &poll_cwd);
                }
            });
        } else {
            // No runtime (unit contexts without a tokio runtime): the
            // watcher still serves the static snapshot + fs-watch.
            warn!("git-state: no tokio runtime; periodic poll disabled");
        }

        // Broadcast the initial snapshot so subscribers that subscribe
        // BEFORE the next change still observe at least one value
        // (tests use this to wait on the initial snapshot
        // deterministically — mirrors WorkUnitsWatcher).
        let _ = tx.send(initial);

        Self {
            state,
            tx,
            _debouncer: Arc::new(Mutex::new(debouncer)),
            _cwd: cwd.to_path_buf(),
        }
    }

    /// Snapshot of the most recent captured git state.
    pub fn snapshot(&self) -> GitState {
        match self.state.read() {
            Ok(guard) => guard.clone(),
            Err(_) => GitState::default(),
        }
    }

    /// Subscribe to broadcasts of fresh `GitState` snapshots. A frame is
    /// emitted on every debounced `.git` change AND every poll tick that
    /// produced a DIFFERENT snapshot (dedup). Subscribing AFTER the
    /// initial broadcast does not receive it; pair this with
    /// [`Self::snapshot`] to backfill.
    pub fn subscribe(&self) -> broadcast::Receiver<GitState> {
        self.tx.subscribe()
    }
}

/// Re-capture the snapshot; publish ONLY when it differs from the last
/// published one (dedup — R1). Shared by the debouncer callback and the
/// periodic poll task.
fn re_snapshot_and_publish(
    state: &Arc<RwLock<GitState>>,
    tx: &broadcast::Sender<GitState>,
    cwd: &Path,
) {
    let snapshot = capture_git_state(cwd);
    if let Ok(mut guard) = state.write() {
        // Dedup: unchanged snapshot → no state write, no broadcast.
        if *guard == snapshot {
            return;
        }
        *guard = snapshot.clone();
    }
    // `send` only errors when there are zero receivers — that is fine;
    // the next subscriber can call `snapshot()` to backfill.
    let _ = tx.send(snapshot);
}

/// Create the debouncer + register the watch. Returns `None` (degraded
/// mode) when the debouncer cannot be created or the watch cannot be
/// registered — the calling constructor then keeps serving the periodic
/// poll + static snapshot without any fs-watch.
fn build_debouncer(
    state: &Arc<RwLock<GitState>>,
    tx: &broadcast::Sender<GitState>,
    cwd: &Path,
) -> Option<Debouncer<RecommendedWatcher>> {
    let git_dir = cwd.join(".git");
    let watch_path = if git_dir.exists() {
        // Recursive watch over `.git` — ref writes, index writes, and
        // packed-refs rewrites all land here.
        (git_dir, RecursiveMode::Recursive)
    } else {
        // Non-repo cwd: watch the root non-recursively and only react to
        // the appearance of a `.git` entry (e.g. a later `git init`).
        // Watching the whole tree recursively would fire on every
        // working-tree write, which is far too broad.
        (cwd.to_path_buf(), RecursiveMode::NonRecursive)
    };

    let cb_state = Arc::clone(state);
    let cb_tx = tx.clone();
    let cb_cwd = cwd.to_path_buf();

    let mut debouncer = match new_debouncer(
        Duration::from_millis(100),
        move |res: std::result::Result<
            Vec<notify_debouncer_mini::DebouncedEvent>,
            notify::Error,
        >| {
            let events = match res {
                Ok(events) => events,
                Err(e) => {
                    warn!("git-state fs-watch error: {:?}", e);
                    return;
                }
            };
            let relevant = events.iter().any(|e| match e.kind {
                DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous => {
                    if e.path == cb_cwd.join(".git") {
                        // The .git entry itself was created (git init) —
                        // always resnapshot.
                        true
                    } else if e.path.starts_with(cb_cwd.join(".git")) {
                        // Anything under .git may have touched refs / the
                        // index / packed-refs — resnapshot.
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            });
            if !relevant {
                return;
            }
            re_snapshot_and_publish(&cb_state, &cb_tx, &cb_cwd);
        },
    ) {
        Ok(d) => d,
        Err(e) => {
            warn!("failed to create git-state watch debouncer: {e}");
            return None;
        }
    };

    debug!(
        path = %watch_path.0.display(),
        "registering git-state watcher"
    );
    match debouncer.watcher().watch(&watch_path.0, watch_path.1) {
        Ok(()) => Some(debouncer),
        Err(e) => {
            warn!(
                error = %e,
                path = %watch_path.0.display(),
                "failed to watch git-state locations; degrading to poll-only"
            );
            None
        }
    }
}

impl Drop for GitStateWatcher {
    fn drop(&mut self) {
        // Take the debouncer out so its (and the spawned poll task's)
        // channels drop after this point. The poll task's handle is not
        // stored (fire-and-forget): when the last broadcast receiver
        // drops, the task's `tx.send` is a no-op and the task lingers
        // harmlessly until runtime shutdown — mirroring the
        // WorkUnitsWatcher debouncer-drop semantics for fs-watch.
        if let Ok(mut guard) = self._debouncer.lock() {
            *guard = None;
        }
    }
}
