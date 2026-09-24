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
//!   snapshot is published ONLY when its STABLE SIGNATURE differs from
//!   the last published one (dedup — BUG-188: the signature excludes
//!   `checkpoints[].timestamp`, which is re-stamped `SystemTime::now()`
//!   per capture and would otherwise defeat full-struct `==` in any repo
//!   that has checkpoint refs). Unchanged poll ticks never re-broadcast.
//!
//! BUG-187: every re-capture (both the periodic poll AND the debounced
//! `.git` fs-event) is dispatched onto the tokio **blocking pool** —
//! never inline on an async worker — so a slow capture can no longer pin
//! a `tokio-rt-worker` and starve the LLM stream dispatch, RPCs, and the
//! TUI's own tasks. An `AtomicBool` in-flight guard DROPS (never queues)
//! a tick / fs-event that lands while a capture is already running.
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

use codelet_git::ghost_commit::{count_checkpoints, list_all_ghost_checkpoints};
use codelet_git::status::{capture_changed_files, get_current_branch};
use codelet_rpc_types::{ChangedFile, CheckpointInfo, GitState};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind, Debouncer};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
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
///
/// BUG-189 R1: the capture runs on the SHARED-HANDLE path
/// (`codelet_git::status::capture_changed_files`) — one `gix::Repository`
/// open per capture, shared across all three collectors — instead of three
/// cold opens. An error degrades to the empty list exactly like the
/// previous per-leg `warn` + skip (the capture never fails the snapshot).
fn collect_changed_files(cwd: &Path) -> Vec<ChangedFile> {
    match capture_changed_files(cwd) {
        Ok(entries) => entries
            .into_iter()
            .map(|e| ChangedFile {
                path: e.path,
                change_type: e.change_type,
                staged: e.staged,
            })
            .collect(),
        Err(e) => {
            warn!(error = %e, "git-state: changed-file capture failed; empty list");
            Vec::new()
        }
    }
}

/// Capture one full `GitState` snapshot for `cwd` (ENOENT-tolerant: a
/// non-repo directory yields the empty `GitState`, never an error).
///
/// BUG-188: exposed so the root-cause pin test can take two raw captures
/// of the same repo and prove they differ in the volatile timestamp leg
/// while their stable signatures agree.
pub fn capture(cwd: &Path) -> GitState {
    capture_git_state(cwd, None)
}

/// BUG-187: `on_capture` is invoked (when `Some`) on the thread that
/// performs the capture — in production that is the blocking pool, so a
/// hook wired there is proof the capture ran off the async pool.
fn capture_git_state(cwd: &Path, on_capture: Option<&(dyn Fn() + Send + Sync)>) -> GitState {
    if let Some(hook) = on_capture {
        hook();
    }
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

/// BUG-188: stable signature of a `GitState` snapshot — the ONLY part of
/// the frame the watcher's dedup gate compares. Mirrors the TUI's
/// BUG-184 signatures (`dispatch_git_state.rs` `changed_files_signature`
/// / `checkpoints_signature`) so both layers agree on "unchanged":
///
/// - `git_branch` + `checkpoint_counts` (the chrome-bar / board legs);
/// - `changed_files` in list order as `path:change_type:staged`;
/// - `checkpoints` in list order as `work_unit_id/name:is_automatic` —
///   **excluding `timestamp`**, which `collect_checkpoints` re-stamps with
///   `SystemTime::now()` on every capture (full-struct `==` is therefore
///   never stable in a repo that has checkpoint refs — BUG-188's root
///   cause).
pub fn git_state_signature(state: &GitState) -> String {
    let changed = state
        .changed_files
        .iter()
        .map(|f| {
            format!(
                "{}:{}:{}",
                f.path,
                f.change_type,
                if f.staged { "s" } else { "w" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let checkpoints = state
        .checkpoints
        .iter()
        .map(|c| {
            format!(
                "{}/{}:{}",
                c.work_unit_id,
                c.name,
                if c.is_automatic { "auto" } else { "manual" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "branch={};counts={}/{};changed=[{}];checkpoints=[{}]",
        state.git_branch.as_deref().unwrap_or("-"),
        state.checkpoint_counts.manual,
        state.checkpoint_counts.auto,
        changed,
        checkpoints
    )
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
///
/// BUG-187: every re-capture (both the periodic poll and the debouncer
/// fs-event path) runs on the tokio BLOCKING pool — never inline on an
/// async worker — and an in-flight guard (`AtomicBool`) drops a tick /
/// fs-event that lands while a capture is already running.
pub struct GitStateWatcher {
    /// Latest known snapshot AND the stable signature of the last
    /// PUBLISHED frame (BUG-188). Read by [`Self::snapshot`] (state leg
    /// only) and compared by the dedup gate before a broadcast.
    state: Arc<RwLock<(GitState, String)>>,
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
    /// BUG-187: capture hook invoked on the thread that performs each
    /// re-capture (the blocking pool in production) — `None` unless the
    /// caller asked for it via [`Self::with_captures`] /
    /// [`Self::with_interval_and_captures`].
    _on_capture: Option<Arc<dyn Fn() + Send + Sync>>,
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
        Self::with_interval_and_captures(cwd, poll_interval, None)
    }

    /// Construct a watcher with an explicit poll interval AND a capture
    /// hook (BUG-187). See [`Self::with_interval_and_captures`] for the
    /// hook's exact firing contract.
    pub fn with_captures<F>(cwd: &Path, poll_interval: Duration, on_capture: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        Self::with_interval_and_captures(cwd, poll_interval, Some(Arc::new(on_capture)))
    }

    /// Construct a watcher with an explicit poll interval AND a capture
    /// hook (BUG-187).
    ///
    /// `on_capture` is invoked on the thread that performs every
    /// re-capture after the initial one — in production that thread is
    /// the tokio blocking pool (the capture is dispatched via
    /// `spawn_blocking` on both the periodic-poll path and the
    /// debouncer-callback path), so a hook wired there is proof the
    /// capture never ran on the async pool. The initial (constructor)
    /// capture does NOT invoke the hook — it runs synchronously in
    /// `with_cwd` by the BUG-182 contract.
    pub fn with_interval_and_captures(
        cwd: &Path,
        poll_interval: Duration,
        on_capture: Option<Arc<dyn Fn() + Send + Sync>>,
    ) -> Self {
        // BUG-182: the initial snapshot is read synchronously (before any
        // runtime wiring) and does NOT run the capture hook — the hook
        // only fires on re-captures dispatched onto the blocking pool.
        let initial = capture_git_state(cwd, None);
        let (tx, _) = broadcast::channel::<GitState>(GIT_STATE_WATCHER_CAPACITY);
        // BUG-188: the dedup gate compares the stable signature of the
        // NEW capture against the last-published signature — the raw
        // GitState values are never `==`-stable once checkpoint refs exist
        // (see `git_state_signature`), so the signature is stored
        // alongside the snapshot.
        let state = Arc::new(RwLock::new((
            initial.clone(),
            git_state_signature(&initial),
        )));

        // BUG-187 R2: at most one capture in flight — a tick / fs-event
        // that lands while a capture is running is dropped, never queued
        // (no unbounded backlog on the blocking pool).
        let in_flight = Arc::new(AtomicBool::new(false));

        let handle = tokio::runtime::Handle::try_current().ok();

        // BUG-187: the periodic poll re-captures the full GitState on
        // every tick — catching working-tree changes that emit no `.git`
        // event (agent tool commits, other-terminal work). The tick uses
        // MissedTickBehavior::Skip so a slow capture never bursts.
        //
        // BUG-187 R1: the capture itself is dispatched onto the BLOCKING
        // pool — a slow capture can no longer pin a `tokio-rt-worker` and
        // starve the LLM stream dispatch, RPCs, and the TUI's own tasks.
        if let Some(handle) = &handle {
            let poll_state = Arc::clone(&state);
            let poll_tx = tx.clone();
            let poll_cwd = cwd.to_path_buf();
            let poll_in_flight = Arc::clone(&in_flight);
            let poll_capture = on_capture.clone();
            // `Handle` is cheaply cloneable — the poll task keeps its own
            // clone so the original stays usable for the debouncer.
            let poll_handle = handle.clone();
            handle.spawn(async move {
                let mut interval = tokio::time::interval(poll_interval);
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    interval.tick().await;
                    // R2: dropped (not queued) while a capture is in flight.
                    if poll_in_flight.swap(true, Ordering::SeqCst) {
                        debug!("git-state: poll tick dropped — capture in flight");
                        continue;
                    }
                    debug!("git-state: poll tick — dispatching capture onto blocking pool");
                    let state = Arc::clone(&poll_state);
                    let tx = poll_tx.clone();
                    let cwd = poll_cwd.clone();
                    let in_flight = poll_in_flight.clone();
                    let capture = poll_capture.clone();
                    // spawn_blocking only fails (cancels) when the runtime
                    // is shutting down — the poll task dies with it, so a
                    // wedged guard cannot outlive the watcher.
                    poll_handle.spawn_blocking(move || {
                        re_snapshot_and_publish(&state, &tx, &cwd, capture.as_deref());
                        in_flight.store(false, Ordering::SeqCst);
                    });
                }
            });
        } else {
            // No runtime (unit contexts without a tokio runtime): the
            // watcher still serves the static snapshot + fs-watch.
            warn!("git-state: no tokio runtime; periodic poll disabled");
        }

        let debouncer = build_debouncer(&state, &tx, cwd, &in_flight, handle, on_capture.as_ref());

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
            _on_capture: on_capture,
        }
    }

    /// Snapshot of the most recent captured git state.
    pub fn snapshot(&self) -> GitState {
        match self.state.read() {
            Ok(guard) => guard.0.clone(),
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

/// Re-capture the snapshot; publish ONLY when the new capture's STABLE
/// SIGNATURE differs from the last published one (dedup — R1, BUG-188).
/// Shared by the debouncer callback and the periodic poll task.
///
/// BUG-188: the gate compares `git_state_signature` — never the full
/// `GitState` `==` — because `checkpoints[].timestamp` is re-stamped
/// `SystemTime::now()` on every capture and two captures of an unchanged
/// repo are therefore never structurally equal. The full snapshot (with
/// its fresh timestamps) is still stored + broadcast on change; only the
/// *decision* uses the stable signature.
///
/// BUG-187: `on_capture` is forwarded to [`capture_git_state`] and runs
/// on whichever thread performs the capture (the blocking pool in
/// production) — see the constructor's spawn_blocking wiring.
fn re_snapshot_and_publish(
    state: &Arc<RwLock<(GitState, String)>>,
    tx: &broadcast::Sender<GitState>,
    cwd: &Path,
    on_capture: Option<&(dyn Fn() + Send + Sync)>,
) {
    let snapshot = capture_git_state(cwd, on_capture);
    let signature = git_state_signature(&snapshot);
    if let Ok(mut guard) = state.write() {
        // Dedup: unchanged signature → no state write, no broadcast.
        if guard.1 == signature {
            return;
        }
        guard.0 = snapshot.clone();
        guard.1 = signature;
    }
    // `send` only errors when there are zero receivers — that is fine;
    // the next subscriber can call `snapshot()` to backfill.
    let _ = tx.send(snapshot);
}

/// Create the debouncer + register the watch. Returns `None` (degraded
/// mode) when the debouncer cannot be created or the watch cannot be
/// registered — the calling constructor then keeps serving the periodic
/// poll + static snapshot without any fs-watch.
///
/// BUG-187 R3: the notify callback is sync-only, so it never calls
/// `re_snapshot_and_publish` inline. When a `tokio::runtime::Handle` is
/// available (it always is in production — the watcher is constructed
/// from an async context) it dispatches the capture onto the BLOCKING
/// pool via the same in-flight guard the poll task uses. When no handle
/// is available (unit contexts without a runtime) the fs-event degrades
/// to a no-op — the periodic poll is disabled too, and `snapshot()`
/// still serves the last known state.
fn build_debouncer(
    state: &Arc<RwLock<(GitState, String)>>,
    tx: &broadcast::Sender<GitState>,
    cwd: &Path,
    in_flight: &Arc<AtomicBool>,
    handle: Option<tokio::runtime::Handle>,
    on_capture: Option<&Arc<dyn Fn() + Send + Sync>>,
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
    // macOS FSEvents reports event paths through the RESOLVED symlink form
    // of the watched path (e.g. `/private/var/folders/...` for a watch
    // registered on `/var/folders/...`, because `/var` -> `/private/var`).
    // Carry a canonicalized prefix too so events reported under either form
    // are recognized as `.git`-relevant — on non-symlinked cwd roots the two
    // prefixes are identical, so behavior is unchanged.
    let cb_cwd_canon = match std::fs::canonicalize(cwd) {
        Ok(canon) if canon != *cwd => Some(canon),
        _ => None,
    };
    let cb_in_flight = Arc::clone(in_flight);
    let cb_handle = handle;
    let cb_capture = on_capture.cloned();

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
            // An event is `.git`-relevant when its path is the `.git`
            // entry itself or lives under it — under EITHER the raw watch
            // prefix or the canonicalized prefix (macOS FSEvents reports
            // paths via the resolved symlink form; see `cb_cwd_canon`).
            let relevant = events.iter().any(|e| {
                matches!(
                    e.kind,
                    DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous
                ) && {
                    let hits =
                        |git_prefix: &Path| e.path == *git_prefix || e.path.starts_with(git_prefix);
                    let raw_git = cb_cwd.join(".git");
                    hits(&raw_git)
                        || cb_cwd_canon
                            .as_ref()
                            .is_some_and(|canon| hits(&canon.join(".git")))
                }
            });
            debug!(
                count = events.len(),
                relevant, "git-state debouncer batch delivered"
            );
            if !relevant {
                return;
            }
            // BUG-187 R3: the notify callback is sync-only — it must
            // never run the capture inline. Dispatch onto the blocking
            // pool from the captured runtime handle (BORROWED — the
            // callback may fire more than once), sharing the poll
            // task's in-flight guard (an event that lands while a
            // capture is running is dropped, not queued).
            let Some(handle) = cb_handle.as_ref() else {
                // No runtime: the periodic poll is disabled too
                // (constructor already warned once) — nothing to do.
                return;
            };
            // R2: dropped (not queued) while a capture is in flight.
            if cb_in_flight.swap(true, Ordering::SeqCst) {
                debug!("git-state: fs-event dropped — capture in flight");
                return;
            }
            let state = Arc::clone(&cb_state);
            let tx = cb_tx.clone();
            let cwd = cb_cwd.clone();
            let in_flight = Arc::clone(&cb_in_flight);
            let capture = cb_capture.clone();
            // spawn_blocking only fails (cancels) when the runtime is
            // shutting down — a wedged guard cannot outlive the process.
            handle.spawn_blocking(move || {
                re_snapshot_and_publish(&state, &tx, &cwd, capture.as_deref());
                in_flight.store(false, Ordering::SeqCst);
            });
        },
    ) {
        Ok(d) => d,
        Err(e) => {
            warn!("failed to create git-state watch debouncer: {e}");
            return None;
        }
    };

    match debouncer.watcher().watch(&watch_path.0, watch_path.1) {
        Ok(()) => {
            debug!(
                path = %watch_path.0.display(),
                "registering git-state watcher"
            );
            Some(debouncer)
        }
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
