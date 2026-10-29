/// RPC-011 broadcast capacity for the StreamChunk channel — sized to
/// absorb sustained token-delta storms across multiple connected
/// clients. Bumped from 256 → 1024 alongside the multi-client
/// hardening work in RPC-011.
pub const DEFAULT_CHUNKS_CAPACITY: usize = 1024;
/// RPC-011 broadcast capacity for the LogRecord channel — sized for
/// tracing storms plus per-client lag warnings riding the same
/// channel. Bumped from 1024 → 4096 alongside the multi-client
/// hardening work in RPC-011.
pub const DEFAULT_LOGS_CAPACITY: usize = 4096;
/// RPC-011 broadcast capacity for the work-units update channel —
/// snapshot replacement semantics mean a lagging subscriber simply
/// resyncs from the latest snapshot, so a small capacity is fine.
pub const DEFAULT_WORK_UNITS_CAPACITY: usize = 256;
/// TUI-109 broadcast capacity for the checkpoint-enumeration progress
/// channel — one frame per item (plus the done frame); a lagging
/// subscriber simply drops intermediate frames (the final done frame
/// still lands, and the list RPC itself carries the full result).
pub const DEFAULT_CHECKPOINTS_PROGRESS_CAPACITY: usize = 256;

/// BUG-182: broadcast capacity for the git-state push. Frames are full
/// `GitState` snapshots (bounded: 200 checkpoints max), so a lagging
/// subscriber simply resyncs on the next frame — 64 mirrors the
/// `GitStateWatcher`'s own channel capacity.
pub const DEFAULT_GIT_STATE_CAPACITY: usize = 64;

/// RPC-011: read-only handle to per-server runtime stats so the shared
/// service can answer `health()` from BOTH transports. The concrete
/// `ServerStats` lives in `codelet-rpc-server`; this trait abstracts
/// only the read-side accessors `health()` needs so the rpc crate
/// stays free of a server-side dependency.
pub trait ServerStatsHandle: Send + Sync + std::fmt::Debug {
    fn connected_clients(&self) -> u64;
    fn last_watcher_event_secs_ago(&self) -> Option<u64>;
    fn lag_chunks(&self) -> u64;
    fn lag_logs(&self) -> u64;
    fn lag_work_units(&self) -> u64;
}

/// The shared `FspecService` state.
///
/// Holds the workspace watcher, the session manager handle (RPC-007),
/// the per-process broadcast senders for streaming chunks and log
/// records, and a per-process invocation counter. RPC-011 additionally
/// records `started_at` (used by `health()` for uptime_secs) and an
/// optional `stats` accessor wired in by the host transport.
pub struct SharedFspecService {
    /// RPC-011 rule [26]: the watcher slot is wrapped in
    /// [`arc_swap::ArcSwap`] so the daemon's SIGHUP handler can
    /// atomically replace it with a freshly-built `WorkUnitsWatcher`
    /// without blocking concurrent `list_work_units` readers. Lock-free
    /// reads via `.load()` are cheaper than `RwLock` and the swap is a
    /// rare event (only on SIGHUP).
    watcher: ArcSwap<WorkUnitsWatcher>,
    focus_subscription:
        std::sync::Mutex<Option<codelet_core::work_unit_subscription::WorkUnitSubscription>>,
    session_manager: Option<Arc<dyn SessionManagerHandle>>,
    chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>,
    logs_tx: broadcast::Sender<LogRecord>,
    list_work_units_calls: Arc<AtomicU64>,
    /// RPC-011: process startup instant — `health()` reports
    /// `(now - started_at).as_secs()` as `uptime_secs`.
    started_at: Instant,
    /// RPC-011: read-only stats accessor wired in by the host
    /// transport (e.g. `bind_and_serve` for the WebSocket server).
    /// `None` for in-process embedded callers that don't run a server
    /// — in that case `health()` returns zeroed counters.
    stats: AsyncMutex<Option<Arc<dyn ServerStatsHandle>>>,
    /// RPC-015: workspace cwd used by `checkpoint_counts()` to walk
    /// `refs/fspec-checkpoints/...`. `None` for in-process callers
    /// that don't care about checkpoint counts (e.g. RPC-005..014
    /// tests) — in that case `checkpoint_counts()` returns
    /// `CheckpointCounts::default()`.
    cwd: Option<PathBuf>,
    /// TUI-109: per-process broadcast sender for checkpoint-enumeration
    /// progress frames. `FspecServiceImpl::list_checkpoints` publishes a
    /// frame per collected item (plus the done frame); the embedded
    /// transport's `checkpoints_progress_rx()` surfaces the receiver to
    /// the TUI. Transports that don't forward the frames degrade to
    /// spinner-only automatically (no producer → no frames).
    checkpoints_progress_tx: broadcast::Sender<CheckpointsProgress>,
    /// BUG-182: the ONE centralized git-state watcher over the attached
    /// cwd (replaces BUG-181's `CheckpointsWatcher`). `None` when no cwd
    /// has been attached (in-process callers that don't care about git
    /// state) — in that case `git_state_snapshot()` returns
    /// `GitState::default()` and `git_state_changed_rx()` returns a
    /// closed receiver.
    git_state_watcher: Option<GitStateWatcher>,
    /// BUG-187: per-process counter of GitState captures that ran (on the
    /// blocking pool). `with_cwd` wires `GitStateWatcher::with_captures`
    /// with a closure that bumps this counter from inside the capture
    /// (a blocking-pool task in production), so tests can assert
    /// captures actually ran off the async pool.
    git_state_captures: Arc<AtomicU64>,
}
