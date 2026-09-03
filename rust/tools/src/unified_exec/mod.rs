//! Unified Exec Tool — Provider-agnostic process execution with session management.
//!
//! This module implements the yield-and-resume pattern for interactive processes:
//! - `run` action: spawn a process (pipe or PTY), collect output, return exit_code or session_id
//! - `write` action: send input to a running session's stdin, poll for output
//! - `poll` action: check for new output without sending input
//! - `list` action: enumerate active sessions
//! - `close` action: terminate a session
//!
//! ## Architecture
//!
//! ```text
//! LLM → UnifiedExecTool (action dispatch)
//!          ↓
//!       ProcessStore (HashMap<String, ProcessEntry>)
//!          ↓
//!       tokio::process::Child / PTY handle
//! ```
//!
//! The ProcessStore is a global singleton behind `tokio::sync::Mutex`.
//! Each entry stores the child process handle, stdin sender, output buffer,
//! and metadata for LRU eviction.
//!
//! ## Module Layout
//!
//! - `mod.rs` — constants and clamp functions
//! - `types.rs` — ExecCommand, UnifiedExecResult, UnifiedExecArgs, SessionListEntry
//! - `tool.rs` — UnifiedExecTool, `Tool` impl, action dispatch
//! - `process_store.rs` — ProcessStore with LRU eviction
//! - `spawning.rs` — pipe/PTY process creation with I/O wiring
//! - `output.rs` — output collection and truncation
//! - `reaper.rs` — background cleanup and session ID generation
//! - `exec_stdin.rs` — TOOL-022 P2 deterministic quiet detector + per-agent-session callback

mod exec_stdin;
mod output;
mod process_store;
mod reaper;
mod spawning;
mod tool;
mod types;

pub use crate::bash_process::{
    platform_shell_invocation, pty_liveness_anchor_invocation, windows_shell_fallback_invocation,
};
pub use exec_stdin::{
    emit_exec_stdin_request, set_exec_stdin_request_callback, spawn_exec_stdin_detector,
    ExecStdinRequest, ExecStdinRequestCallback, EXEC_STDIN_COOLDOWN_SECS,
    EXEC_STDIN_QUIET_THRESHOLD_SECS,
};
pub use process_store::{global_store, session_id_to_evict, ChildHandle, ProcessStore};
pub use reaper::{generate_session_id, spawn_reaper};
pub use tool::{poll_session, poll_session_interruptible, UnifiedExecTool};
pub use types::{
    quiet_secs_since, ExecCommand, UnifiedExecArgs, UnifiedExecResult, STILL_RUNNING_STEERING,
};

// ============================================================================
// Constants (from Codex reference)
// ============================================================================

/// Minimum yield time in milliseconds (prevents busy-spinning)
pub const MIN_YIELD_TIME_MS: u64 = 250;

/// Minimum yield time for poll/empty-write (higher to avoid excessive polling)
pub const MIN_EMPTY_YIELD_TIME_MS: u64 = 5_000;

/// TOOL-022 P4: minimum yield for the BashTool delegation poll loop —
/// the cadence of the abort-flag check. Lower than the LLM-facing
/// poll minimum (delegation aborts are USER-driven, not LLM-driven;
/// the pre-P4 Bash abort contract terminates within ~200ms of the
/// ESC).
pub const MIN_BASH_DELEGATION_YIELD_TIME_MS: u64 = 100;

/// Maximum yield time in milliseconds
pub const MAX_YIELD_TIME_MS: u64 = 30_000;

/// Default yield time when not specified
pub const DEFAULT_YIELD_TIME_MS: u64 = 10_000;

/// Maximum number of concurrent processes in the store
pub const MAX_UNIFIED_EXEC_PROCESSES: usize = 64;

/// Number of most-recent sessions protected from LRU eviction
pub const LRU_PROTECT_COUNT: usize = 8;

/// Maximum output buffer size per session (1 MiB)
pub const UNIFIED_EXEC_OUTPUT_MAX_BYTES: usize = 1024 * 1024;

/// Clamp yield time for run/write actions
pub fn clamp_yield_time(requested: u64) -> u64 {
    requested.clamp(MIN_YIELD_TIME_MS, MAX_YIELD_TIME_MS)
}

/// Clamp yield time for poll actions (higher minimum)
pub fn clamp_poll_yield_time(requested: u64) -> u64 {
    requested.clamp(MIN_EMPTY_YIELD_TIME_MS, MAX_YIELD_TIME_MS)
}

/// TOOL-022 P4: clamp yield time for the BashTool delegation poll loop
/// (the abort-flag-check cadence — lower minimum than the LLM poll).
pub fn clamp_bash_delegation_yield_time(requested: u64) -> u64 {
    requested.clamp(MIN_BASH_DELEGATION_YIELD_TIME_MS, MAX_YIELD_TIME_MS)
}
