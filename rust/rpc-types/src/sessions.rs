//! Session wire types (RPC-007).

use super::*;

// ============================================================================
// RPC-007: Session types
// ============================================================================

/// Stable identifier for a session. Newtype around `String` so the wire
/// shape stays a plain string but the type system distinguishes session
/// IDs from arbitrary strings.
#[derive(
    Debug,
    Clone,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
pub struct SessionId {
    pub value: String,
}

impl SessionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }
}

impl From<String> for SessionId {
    fn from(value: String) -> Self {
        Self { value }
    }
}

impl From<&str> for SessionId {
    fn from(value: &str) -> Self {
        Self {
            value: value.to_string(),
        }
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.value)
    }
}

/// Coarse session lifecycle state.
///
/// Variant ORDER is preserved exactly so that `as u8` casts in
/// `rust/napi/src/session_manager.rs`
/// (`AtomicU8::new(SessionStatus::Idle as u8)` /
/// `status.swap(status as u8, ...)`) keep the historical discriminant
/// values 0..=4 stable after the type was lifted out of NAPI. `Cleared`
/// is appended (5) as RPC-007's only new variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SessionStatus {
    #[default]
    Idle,
    Running,
    Interrupted,
    /// PAUSE-001: Session is paused waiting for user input (Enter/Y/N/Esc)
    Paused,
    /// PERF-002: Session is compacting context - supports progress tracking
    Compacting,
    /// RPC-007: Session has been cleared (post-cleanup terminal state).
    Cleared,
}

impl SessionStatus {
    /// Convert status to string representation for TypeScript / log output.
    /// Lifted from `rust/napi/src/session_manager.rs` as part of the
    /// RPC-007 type-uniqueness rule so callers in `rust/napi` can use
    /// the inherent method via the re-export.
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Idle => "idle",
            SessionStatus::Running => "running",
            SessionStatus::Interrupted => "interrupted",
            SessionStatus::Paused => "paused",
            SessionStatus::Compacting => "compacting",
            SessionStatus::Cleared => "cleared",
        }
    }
}

impl From<u8> for SessionStatus {
    /// Inverse of `SessionStatus as u8`, lifted from
    /// `rust/napi/src/session_manager.rs` so the historical
    /// `AtomicU8`-based round-trip continues to compile after the
    /// lift. Unknown values fall back to `Idle` (matches the pre-lift
    /// behaviour).
    fn from(v: u8) -> Self {
        match v {
            0 => SessionStatus::Idle,
            1 => SessionStatus::Running,
            2 => SessionStatus::Interrupted,
            3 => SessionStatus::Paused,
            4 => SessionStatus::Compacting,
            5 => SessionStatus::Cleared,
            _ => SessionStatus::Idle,
        }
    }
}

/// Public metadata about a session returned by `list_sessions`.
///
/// Field names match the NAPI surface verbatim (see
/// rust/napi/src/session_manager.rs:380) so that rust/napi can
/// re-export this type without changing the existing TypeScript shape.
/// `id` is a plain `String` rather than a [`SessionId`] newtype so the
/// TS shape stays a flat string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub project: String,
    pub message_count: u32,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    /// GIT-029: Whether this is an isolated session with a git worktree
    pub is_isolated: bool,
    /// GIT-029: Path to the worktree (if isolated)
    pub worktree_path: Option<String>,
    /// RPC-007: optional role string the session was created with.
    pub role: Option<String>,
    /// TUI-096: optional timestamp (epoch milliseconds) for time-ago display.
    pub updated_at_ms: Option<i64>,
    #[serde(default)]
    pub work_unit_focus: WorkUnitFocus,
}

/// RPC-433: a session lifecycle event pushed to ALL connected clients.
///
/// Server-to-client only (the WS server rejects client-pushed frames of
/// this payload — see `codelet_rpc_server::Envelope::SessionEvent`).
///
/// `Removed` carries only the id: a connected client already holds all
/// tab state for that session and needs nothing else to close the tab.
/// The connect-time "add replay" is deliberately NOT a `replay` flag on
/// this event — it is a separate `Envelope::SessionSnapshot` frame, which
/// keeps the live event stream pure (research:
/// `spec/attachments/RPC-433/session-events-daemon-fanout-research.md` §2.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionEvent {
    /// A session was created (any client, spawned subordinate, or /resume).
    Created(SessionInfo),
    /// A session was destroyed / removed from the daemon's live map.
    Removed(SessionId),
    /// A session's history was cleared (/clear). APPENDED after Removed so
    /// existing bincode variant indices (Created=0, Removed=1) stay stable.
    /// Carries only the id (RPC-436): a connected client already holds all
    /// tab state for that session and needs nothing else to react to the
    /// clear (mirrors `Removed`).
    Cleared(SessionId),
    FocusChanged {
        session_id: SessionId,
        focus: WorkUnitFocus,
    },
}

/// Structured log event payload pushed to subscribers via the
/// `logs_rx` broadcast channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogRecord {
    pub level: String,
    pub target: String,
    pub message: String,
    /// Unix epoch milliseconds when the event was captured.
    #[serde(rename = "timestampMs")]
    pub timestamp_ms: i64,
}

/// RPC-011: live health summary returned by `FspecService::health` and
/// reused by the `fspec status` subcommand for human-readable output.
///
/// All counters are point-in-time reads from `ServerStats`. `version`
/// is the daemon process's `env!("CARGO_PKG_VERSION")` so the caller
/// can sanity-check protocol compatibility.
///
/// Lifted into `codelet-rpc-types` (rather than living on the server
/// crate) so both transports — `EmbeddedFspecBackend` (which reads
/// `ServerStats` directly) and `WebSocketFspecBackend` (which receives
/// the struct over tarpc) — share the SAME wire shape. The napi
/// feature gate follows the existing `WorkUnitInfo`/`SessionInfo`
/// pattern so the type can be re-exported through `codelet-napi`
/// verbatim if a future JS surface needs it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthInfo {
    /// Seconds since the daemon's `ServerStats::started_at` instant.
    ///
    /// Typed `i64` (rather than `u64`) so the `napi(object)` cfg-gate
    /// compiles under napi-derive v3 + `napi4` feature, which does not
    /// support `u64` in `napi(object)` field positions. The wire format
    /// (tarpc bincode) carries the same 8 bytes either way, and uptime
    /// can never exceed 2^63 seconds in practice.
    pub uptime_secs: i64,
    /// Live count of attached WebSocket clients (decremented via the
    /// `ConnectedClientGuard` Drop impl when each connection task
    /// exits).
    pub connected_clients: i64,
    /// Elapsed seconds since the workspace watcher last fired an Ok
    /// snapshot into the work-units fanout task. `None` if no
    /// snapshot has ever been observed by this daemon.
    pub last_watcher_event_secs_ago: Option<i64>,
    /// Cumulative `RecvError::Lagged` count surfaced by the chunks
    /// broadcast fanout.
    pub lag_chunks: i64,
    /// Cumulative `RecvError::Lagged` count surfaced by the logs
    /// broadcast fanout.
    pub lag_logs: i64,
    /// Cumulative `RecvError::Lagged` count surfaced by the work-units
    /// broadcast fanout.
    pub lag_work_units: i64,
    /// Daemon process's `env!("CARGO_PKG_VERSION")`.
    pub version: String,
}
