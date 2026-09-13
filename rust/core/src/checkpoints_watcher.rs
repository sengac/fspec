//! BUG-181: pure-Rust `CheckpointsWatcher` for the checkpoint ref/index
//! locations in the workspace cwd.
//!
//! Mirrors the [`crate::work_units::WorkUnitsWatcher`] pattern: a
//! debounced `notify` watch that re-runs
//! `codelet_git::ghost_commit::count_checkpoints` on each debounced event
//! and publishes every snapshot (initial + every change) on a
//! `tokio::sync::broadcast` channel.
//!
//! Public surface (per spec/features/live-checkpoint-counts-in-the-board-header.feature):
//!
//! - [`CheckpointsWatcher`] — long-lived debounced fs-watcher that
//!   publishes `CheckpointCounts` snapshots to subscribers via a
//!   `tokio::sync::broadcast` channel.
//!
//! Watched locations (relative to the workspace cwd):
//!
//! - `.git/` (recursive) — ref file writes
//!   (`refs/fspec-checkpoints/<WU>/<name>`), the metadata index
//!   (`.git/fspec-checkpoints-index/<WU>.json`), and `packed-refs` all
//!   live under `.git`, so a single recursive watch covers every
//!   mutation source: in-process agent tool calls, CLI in another
//!   terminal, and raw `git update-ref`.
//!
//! Graceful degradation (matches `count_checkpoints`' ENOENT tolerance):
//!
//! - Missing `.git` / non-repo cwd → the initial snapshot is
//!   `CheckpointCounts::default()` (zero counts, no error) and the
//!   watcher falls back to watching the workspace root non-recursively
//!   for the creation of a `.git` entry, so a later `git init` still
//!   triggers a recount.
//! - Any debouncer/watch setup failure degrades to "no fs-watch, static
//!   snapshot" — the `snapshot()` accessor still returns the last known
//!   counts and `subscribe()` still yields the initial broadcast.

use codelet_git::ghost_commit::count_checkpoints;
use codelet_rpc_types::CheckpointCounts;
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind, Debouncer};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, warn};

/// Broadcast channel capacity — full `CheckpointCounts` snapshots are
/// tiny (two u32s), so 64 is generous enough that lagging subscribers
/// simply resync on the next frame (mirrors `WorkUnitsWatcher`).
const CHECKPOINTS_WATCHER_CAPACITY: usize = 64;

/// Long-lived debounced file-system watcher over the checkpoint
/// locations in the workspace cwd (BUG-181).
///
/// Reads the initial snapshot synchronously in [`Self::new`], starts a
/// notify-debouncer task that re-counts on checkpoint-location changes,
/// and publishes every snapshot (initial + every change) on a
/// `tokio::sync::broadcast` channel. Drops the underlying debouncer on
/// drop — there is no global state, so multiple workspaces are isolated.
pub struct CheckpointsWatcher {
    /// Latest known snapshot. Read by [`Self::snapshot`] and updated by
    /// the debouncer callback before the broadcast send.
    state: Arc<RwLock<CheckpointCounts>>,
    /// Broadcast tx kept alive so subscribers can clone receivers from
    /// it via [`Self::subscribe`].
    tx: broadcast::Sender<CheckpointCounts>,
    /// Owned debouncer — dropping the watcher tears down fs-watching.
    /// Stored behind a `Mutex<Option<...>>` so [`Drop`] can take it out
    /// without requiring `&mut self`. `None` when the fs-watch setup
    /// degraded away (non-repo cwd without even a root watch, or a
    /// debouncer creation failure) — the snapshot accessor still works.
    _debouncer: Arc<Mutex<Option<Debouncer<RecommendedWatcher>>>>,
    /// The workspace cwd the watcher was started with — used inside the
    /// debouncer callback to re-run `count_checkpoints`.
    _cwd: PathBuf,
}

impl CheckpointsWatcher {
    /// Construct a new watcher over the checkpoint locations in `cwd`.
    ///
    /// Reads the initial snapshot synchronously and broadcasts it
    /// immediately so that fresh subscribers always see at least the
    /// initial state once they `subscribe()` and recv (subject to the
    /// usual broadcast-channel race: subscribers that subscribe AFTER
    /// the initial broadcast must call [`Self::snapshot`] to backfill).
    ///
    /// Never fails: a missing `.git` directory degrades to zero counts +
    /// a root-directory watch for the `.git` entry; a debouncer
    /// creation failure degrades to a static snapshot (no fs-watch).
    /// Errors are surfaced to the tracing layer, never to the caller.
    pub fn new(cwd: &Path) -> Self {
        // ENOENT tolerance: `count_checkpoints` already returns zero
        // counts for non-repo directories; any residual gix error (e.g. a
        // corrupt .git) degrades to zero counts rather than failing the
        // watcher — the header would then simply show
        // `Checkpoints: None` until the next successful recount.
        let initial = match count_checkpoints(cwd) {
            Ok(counts) => counts,
            Err(e) => {
                warn!(error = %e, "initial checkpoint count failed; degrading to zero counts");
                CheckpointCounts::default()
            }
        };
        let (tx, _) = broadcast::channel::<CheckpointCounts>(CHECKPOINTS_WATCHER_CAPACITY);
        let state = Arc::new(RwLock::new(initial));

        let cb_state = Arc::clone(&state);
        let cb_tx = tx.clone();
        let cb_cwd = cwd.to_path_buf();

        let debouncer = build_debouncer(&cb_state, &cb_tx, &cb_cwd);

        // Broadcast the initial snapshot so subscribers that subscribe
        // BEFORE the next file change still observe at least one value
        // on the channel. (Tests use this to wait on the initial
        // snapshot deterministically — mirrors WorkUnitsWatcher.)
        let _ = tx.send(initial);

        Self {
            state,
            tx,
            _debouncer: Arc::new(Mutex::new(debouncer)),
            _cwd: cwd.to_path_buf(),
        }
    }

    /// Snapshot of the most recent successful count.
    pub fn snapshot(&self) -> CheckpointCounts {
        match self.state.read() {
            Ok(guard) => *guard,
            Err(_) => CheckpointCounts::default(),
        }
    }

    /// Subscribe to broadcasts of fresh `CheckpointCounts` snapshots
    /// whenever a checkpoint-location change is observed. Subscribing
    /// AFTER the initial broadcast does not receive it; pair this with
    /// [`Self::snapshot`] to backfill.
    pub fn subscribe(&self) -> broadcast::Receiver<CheckpointCounts> {
        self.tx.subscribe()
    }
}

/// Create the debouncer + register the watch. Returns `None` (degraded
/// mode) when the debouncer cannot be created or the watch cannot be
/// registered — the calling constructor then keeps serving the static
/// snapshot without any fs-watch.
fn build_debouncer(
    state: &Arc<RwLock<CheckpointCounts>>,
    tx: &broadcast::Sender<CheckpointCounts>,
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
                    warn!("checkpoint fs-watch error: {:?}", e);
                    return;
                }
            };
            let relevant = events.iter().any(|e| match e.kind {
                DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous => {
                    if e.path == cb_cwd.join(".git") {
                        // The .git entry itself was created (git init) —
                        // always recount.
                        true
                    } else if e.path.starts_with(cb_cwd.join(".git")) {
                        // Anything under .git may have touched refs / the
                        // index / packed-refs — recount.
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
            // ENOENT tolerance mirrors count_checkpoints: recounting a
            // non-repo directory yields zero counts, never an error.
            match count_checkpoints(&cb_cwd) {
                Ok(counts) => {
                    if let Ok(mut guard) = cb_state.write() {
                        *guard = counts;
                    }
                    // `send` only errors when there are zero receivers —
                    // that is fine; the next subscriber can call
                    // `snapshot()` to backfill.
                    let _ = cb_tx.send(counts);
                }
                Err(e) => {
                    // count_checkpoints only errors on unexpected gix
                    // failures (corrupt refs). Log and keep the last
                    // known counts — the header stays correct until the
                    // next successful recount.
                    warn!(error = %e, "failed to recount checkpoints");
                }
            }
        },
    ) {
        Ok(d) => d,
        Err(e) => {
            warn!("failed to create checkpoint watch debouncer: {e}");
            return None;
        }
    };

    debug!(
        path = %watch_path.0.display(),
        "registering checkpoint watcher"
    );
    match debouncer.watcher().watch(&watch_path.0, watch_path.1) {
        Ok(()) => Some(debouncer),
        Err(e) => {
            warn!(
                error = %e,
                path = %watch_path.0.display(),
                "failed to watch checkpoint locations; degrading to static snapshot"
            );
            None
        }
    }
}
