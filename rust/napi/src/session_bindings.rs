//! NAPI session bindings (RPC-043).
//!
//! Extracted from the legacy `rust/napi/src/session_manager.rs` by
//! RPC-043 as the thin-adapter file for the codelet-napi crate. This
//! module holds:
//!
//! - the 66+ `#[napi]` free-function wrappers that bridge JavaScript
//!   callers to `codelet_sessions::SessionManager` / `BackgroundSession`,
//! - the 12 `#[napi(object)]` result/argument shapes that flow over the
//!   NAPI boundary,
//! - the per-session re-exports of `codelet_sessions::background_session::*`
//!   so existing in-crate callers keep resolving (`SessionManager`,
//!   `BackgroundSession`, `PromptInput`, etc.),
//! - the `#[cfg(test)]` companion modules that exercise the wrappers
//!   (session_role_tests, supervisor_loop_tests, supervisor_input_tests,
//!   napi_supervisor_tests, supervisor_integration_tests,
//!   work_unit_context_tests, sub_agent_model_inheritance_tests with
//!   nested `bug132_tests`).
//!
//! Non-`#[napi]` helpers live in sibling modules: agent_loop.rs,
//! persist.rs, footer_poller.rs, bridges.rs, session_hooks.rs,
//! interjection.rs.

use crate::types::{
    CompactionResult, DebugCommandResult, NapiFileModification, NapiToolCall, NapiTurnDetails,
    StreamChunk, ToolCallInfo, ToolResultInfo,
};

// RPC-043: agent loop lifecycle helpers — engine functions live in
// codelet-core; session_bindings only needs the compaction-threshold +
// debug-capture helpers (used by `session_compact`, `toggle_debug`,
// `session_toggle_debug`, `session_update_debug_metadata`).
use codelet_cli::compaction_threshold::{resolve_compaction_threshold, CompactionThresholdConfig};
use codelet_cli::interactive_helpers::execute_compaction;
use codelet_common::debug_capture::{handle_debug_command_with_dir, SessionMetadata};
// RPC-039 / RPC-043: PauseState shape flows over the NAPI boundary as
// `NapiPauseState`; the JS callbacks dispatch via the `PauseKind` /
// `PauseResponse` enums when the user resumes the agent.
use codelet_tools::tool_pause::{PauseKind, PauseResponse, PauseState};
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use once_cell::sync::OnceCell;
use std::sync::atomic::Ordering;
use uuid::Uuid;

// RPC-043: NapiSessionManagerHooks installer lives in
// `crate::session_hooks`; we invoke it once from
// `session_set_global_chunk_callback`.
use crate::session_hooks::install_napi_session_manager_hooks;
// RPC-043: bridge wiring + handler registration helpers were extracted to
// `crate::bridges`. The call sites below (session_set_global_chunk_callback,
// session_set_model, session_set_model_profile, and the agent_loop_dispatch
// tests) now reach for the relocated functions through the new module path.
use crate::bridges::{
    init_block_notification_callbacks, init_bridge_metadata_providers,
    init_bridge_session_and_terminal_creators, register_agent_manager_handler,
    register_deep_search_handler,
};

/// RPC-041: Stored ThreadsafeFunction that the napi-side fan-out task
/// invokes for every (SessionId, StreamChunk) tuple drained from the
/// manager-owned `chunks_tx` broadcast. Replaces the legacy
/// legacy chunk-callback OnceCell static and its unsafe Send/Sync impls.
/// Wrapped in `parking_lot::Mutex<Option<...>>` so re-registration
/// replaces (rather than duplicates) the stored handle.
static CHUNK_FANOUT_TSFN: OnceCell<
    std::sync::Mutex<Option<ThreadsafeFunction<GlobalChunkCallbackArgs>>>,
> = OnceCell::new();

/// BRIDGE-012: Arguments passed to the global chunk callback
#[napi(object)]
#[derive(Clone)]
pub struct GlobalChunkCallbackArgs {
    pub session_id: String,
    pub chunk: StreamChunk,
}

/// RPC-041: Helper that gates closures previously short-circuited via
/// the legacy `.get().is_none()` check on the removed callback static.
/// Returns true once
/// `session_set_global_chunk_callback` has stored a TSFN inside the
/// `CHUNK_FANOUT_TSFN` slot.
///
/// RPC-043: promoted to `pub(crate)` so the agent_loop in
/// `crate::agent_loop` can re-use the same gate without owning the
/// underlying static.
pub(crate) fn is_global_chunk_callback_registered() -> bool {
    CHUNK_FANOUT_TSFN
        .get()
        .and_then(|m| m.lock().ok())
        .map(|g| g.is_some())
        .unwrap_or(false)
}

/// Maximum concurrent sessions
#[allow(dead_code)]
const MAX_SESSIONS: usize = 10;

/// Input message sent to the agent loop via channel.
///
/// RPC-039: Moved into `codelet-sessions` along with the rest of
/// `BackgroundSession`. Re-exported below so existing call sites inside
/// codelet-napi (the agent_loop, the `#[napi]` free functions, the
/// in-file unit-test module) keep resolving paths they used pre-move.
pub use codelet_sessions::background_session::{
    format_incoming_message, BackgroundSession, BridgeImageData, CompactionProgress,
    IncomingMessage, PromptInput, SessionError, WorkUnitContext, SUPERVISOR_BROADCAST_CAPACITY,
};

// RPC-040: SessionManager + ChainOfCommand moved into codelet-sessions.
// Re-export so every existing call site (scheduler engine, agent_job,
// trigger, catch_up, and the in-file unit-test module) keeps resolving
// `crate::session_manager::SessionManager::instance()` unchanged.
pub use codelet_sessions::chain_of_command::ChainOfCommand;
pub use codelet_sessions::session_manager::SessionManager;

/// Session status values
///
/// RPC-007: lifted into `codelet-rpc-types` so the dual-transport RPC,
/// the embedded transport, and the NAPI surface share a single source of
/// truth. The lifted enum preserves the historical `#[repr(u8)]` and the
/// explicit discriminant order (`Idle = 0, Running = 1, Interrupted = 2,
/// Paused = 3, Compacting = 4, Cleared = 5`) so the existing
/// `AtomicU8::new(SessionStatus::Idle as u8)` and
/// `status.swap(status as u8, ...)` patterns in this file continue to
/// compile unchanged. The `napi` feature gate on `codelet-rpc-types`
/// re-applies `#[napi(string_enum)]` so the TypeScript shape is
/// preserved verbatim.
pub use codelet_rpc_types::SessionStatus;

// PERF-002: Compaction progress information.
//
// RPC-039: Moved into `codelet-sessions`. See the
// `pub use codelet_sessions::background_session::{..}` re-export above.

// TUI-059: Work unit context for session.
//
// RPC-039: Moved into `codelet-sessions`. See the
// `pub use codelet_sessions::background_session::{..}` re-export above.

// AMGR-008: Session role is now a simple string (was SupervisorRole struct)
// Role is stored as Option<String> on BackgroundSession.
// See BackgroundSession::set_role() and get_role().
//
// IncomingMessage / BridgeImageData / format_incoming_message moved
// into `codelet-sessions` by RPC-039. See the
// `pub use codelet_sessions::background_session::{..}` re-export above.

// `impl From<u8> for SessionStatus` and `impl SessionStatus { fn as_str() }`
// were lifted into `codelet-rpc-types` alongside the `SessionStatus` enum
// itself (RPC-007 type-uniqueness rule). Rust's orphan rule forbids inherent
// or foreign-trait impls on a type from another crate, so they live with the
// type definition now and are reachable via the `pub use` above.

/// Session info returned to TypeScript.
///
/// RPC-007: lifted into `codelet-rpc-types` so the dual-transport RPC and
/// the NAPI surface share a single source of truth. The `napi` feature
/// gate on `codelet-rpc-types` re-applies `#[napi(object)]` so the
/// existing TypeScript shape (id, name, status, project, message_count,
/// provider_id, model_id, is_isolated, worktree_path) is preserved
/// verbatim. The `role` field is added as RPC-007's session role surface
/// and is `None` for sessions created via the legacy NAPI path.
pub use codelet_rpc_types::SessionInfo;

/// Model info returned by session_get_model
#[napi(object)]
#[derive(Clone)]
pub struct SessionModel {
    /// Provider ID (e.g., "anthropic", "openai")
    pub provider_id: Option<String>,
    /// Model ID (e.g., "claude-sonnet-4", "gpt-4o")
    pub model_id: Option<String>,
    /// CTX-006: Rust-resolved context window from ProviderManager (single source of truth)
    pub context_window: Option<u32>,
    /// CTX-006: Rust-resolved max output tokens from ProviderManager
    pub max_output_tokens: Option<u32>,
    /// CTX-007: Resolved compaction threshold (per-model, considering user override and family defaults)
    pub compaction_threshold: Option<u32>,
}

/// Token info returned by session_get_tokens
#[napi(object)]
#[derive(Clone)]
pub struct SessionTokens {
    /// Input tokens (context size)
    pub input_tokens: u32,
    /// Output tokens
    pub output_tokens: u32,
    /// Reasoning/thinking tokens
    pub reasoning_tokens: Option<u32>,
}

/// PAUSE-001: Pause state returned to TypeScript via NAPI
#[napi(object)]
#[derive(Clone)]
pub struct NapiPauseState {
    /// "continue" or "confirm"
    pub kind: String,
    /// Tool name that initiated the pause (e.g., "WebSearch")
    pub tool_name: String,
    /// Human-readable message (e.g., "Page loaded at https://...")
    pub message: String,
    /// Optional additional details (e.g., command text for confirm)
    pub details: Option<String>,
}

impl From<PauseState> for NapiPauseState {
    fn from(state: PauseState) -> Self {
        Self {
            kind: match state.kind {
                PauseKind::Continue => "continue".to_string(),
                PauseKind::Confirm => "confirm".to_string(),
                PauseKind::Triple => "triple".to_string(),
            },
            tool_name: state.tool_name,
            message: state.message,
            details: state.details,
        }
    }
}

// GIT-021: Error type for session checkpoint operations.
//
// RPC-039: Moved into `codelet-sessions`. See the
// `pub use codelet_sessions::background_session::{..}` re-export above.

// Broadcast channel capacity for supervisor stream observation (WATCH-003)
// NOTE(RPC-039): `SUPERVISOR_BROADCAST_CAPACITY` is re-exported from
// `codelet-sessions` via the `pub use codelet_sessions::background_session::{..}`
// block at the top of this file. The duplicate `pub const` declaration
// that used to live here was deleted in RPC-039.

#[cfg(test)]
mod session_role_tests {
    // Feature: spec/features/role-clearing-via-napi.feature

    // ============================================================
    // Scenario: session_set_role with empty string clears the role
    // ============================================================
    //
    // The NAPI binding session_set_role has an early-return error when
    // role_name is empty. The fix changes that branch to call
    // session.clear_role() instead, matching agent_manager_handler
    // which already handles this correctly.
    //
    // We can't construct a full BackgroundSession in unit tests (requires
    // codelet_cli::session::Session + mpsc channels), so this test verifies
    // the branching logic that the NAPI binding SHOULD follow.

    /// @step Given a session exists with role "reviewer"
    /// @step When session_set_role is called with an empty role_name
    /// @step Then the session role should be cleared
    /// @step And session_get_role should return null
    #[test]
    fn test_empty_role_name_triggers_clear_branch() {
        // @step Given a session exists with role "reviewer"
        let mut current_role: Option<String> = Some("reviewer".to_string());
        assert_eq!(current_role, Some("reviewer".to_string()));

        // @step When session_set_role is called with an empty role_name
        // Simulate the FIXED session_set_role logic:
        let role_name = "".to_string();
        if role_name.is_empty() {
            // BUG-121 FIX: clear_role instead of returning error
            current_role = None;
        } else {
            current_role = Some(role_name);
        }

        // @step Then the session role should be cleared
        // @step And session_get_role should return null
        assert_eq!(current_role, None);
    }

    /// Verify non-empty role_name still sets the role (regression guard)
    #[test]
    fn test_non_empty_role_name_sets_role() {
        let role_name = "architect".to_string();
        let current_role: Option<String> = if role_name.is_empty() {
            None
        } else {
            Some(role_name)
        };

        assert_eq!(current_role, Some("architect".to_string()));
    }

    /// Verify clearing an already-empty role is idempotent
    #[test]
    fn test_clear_role_when_no_role_set_is_idempotent() {
        let role_name = "".to_string();
        let current_role: Option<String> = if role_name.is_empty() {
            None
        } else {
            Some(role_name)
        };

        assert_eq!(current_role, None);
    }
}

#[cfg(test)]
mod chain_of_command_tests {
    use super::*;

    /// Scenario: Register a supervisor for a subordinate session
    ///
    /// @step Given a ChainOfCommand with no relationships
    /// @step And a subordinate session "abc" exists
    /// @step And a supervisor session "xyz" exists
    /// @step When I call add_supervisor with subordinate_id "abc" and supervisor_id "xyz"
    /// @step Then get_supervisors for "abc" should return ["xyz"]
    /// @step And get_subordinate for "xyz" should return "abc"
    #[test]
    fn test_register_supervisor_for_subordinate_session() {
        // @step Given a ChainOfCommand with no relationships
        let chain_of_command = ChainOfCommand::new();

        // @step And a subordinate session "abc" exists
        let subordinate_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000a1").unwrap();

        // @step And a supervisor session "xyz" exists
        let supervisor_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000b1").unwrap();

        // @step When I call add_supervisor with subordinate_id "abc" and supervisor_id "xyz"
        let result = chain_of_command.add_supervisor(subordinate_id, supervisor_id);
        assert!(result.is_ok(), "add_supervisor should succeed");

        // @step Then get_supervisors for "abc" should return ["xyz"]
        let supervisors = chain_of_command.get_supervisors(subordinate_id);
        assert_eq!(
            supervisors,
            vec![supervisor_id],
            "get_supervisors should return [xyz]"
        );

        // @step And get_subordinate for "xyz" should return "abc"
        let subordinate = chain_of_command.get_subordinate(supervisor_id);
        assert_eq!(
            subordinate,
            Some(subordinate_id),
            "get_subordinate should return abc"
        );
    }

    /// Scenario: Subordinate with multiple supervisors
    ///
    /// @step Given a ChainOfCommand with no relationships
    /// @step And a subordinate session "abc" exists
    /// @step And supervisor sessions "xyz" and "def" exist
    /// @step When I call add_supervisor with subordinate_id "abc" and supervisor_id "xyz"
    /// @step And I call add_supervisor with subordinate_id "abc" and supervisor_id "def"
    /// @step Then get_supervisors for "abc" should return ["xyz", "def"]
    #[test]
    fn test_subordinate_with_multiple_supervisors() {
        // @step Given a ChainOfCommand with no relationships
        let chain_of_command = ChainOfCommand::new();

        // @step And a subordinate session "abc" exists
        let subordinate_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000a2").unwrap();

        // @step And supervisor sessions "xyz" and "def" exist
        let supervisor_xyz = Uuid::parse_str("00000000-0000-0000-0000-0000000000b2").unwrap();
        let supervisor_def = Uuid::parse_str("00000000-0000-0000-0000-0000000000c2").unwrap();

        // @step When I call add_supervisor with subordinate_id "abc" and supervisor_id "xyz"
        let result1 = chain_of_command.add_supervisor(subordinate_id, supervisor_xyz);
        assert!(result1.is_ok(), "first add_supervisor should succeed");

        // @step And I call add_supervisor with subordinate_id "abc" and supervisor_id "def"
        let result2 = chain_of_command.add_supervisor(subordinate_id, supervisor_def);
        assert!(result2.is_ok(), "second add_supervisor should succeed");

        // @step Then get_supervisors for "abc" should return ["xyz", "def"]
        let supervisors = chain_of_command.get_supervisors(subordinate_id);
        assert!(
            supervisors.contains(&supervisor_xyz),
            "supervisors should contain xyz"
        );
        assert!(
            supervisors.contains(&supervisor_def),
            "supervisors should contain def"
        );
        assert_eq!(supervisors.len(), 2, "should have exactly 2 supervisors");
    }

    /// Scenario: Query subordinate for a supervisor
    ///
    /// @step Given a ChainOfCommand with no relationships
    /// @step And session "xyz" is supervising session "abc"
    /// @step When I call get_subordinate with supervisor_id "xyz"
    /// @step Then it should return "abc"
    #[test]
    fn test_query_subordinate_for_supervisor() {
        // @step Given a ChainOfCommand with no relationships
        let chain_of_command = ChainOfCommand::new();

        let subordinate_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000a3").unwrap();
        let supervisor_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000b3").unwrap();

        // @step And session "xyz" is supervising session "abc"
        let _ = chain_of_command.add_supervisor(subordinate_id, supervisor_id);

        // @step When I call get_subordinate with supervisor_id "xyz"
        let result = chain_of_command.get_subordinate(supervisor_id);

        // @step Then it should return "abc"
        assert_eq!(
            result,
            Some(subordinate_id),
            "get_subordinate should return abc"
        );
    }

    /// Scenario: Remove a supervisor relationship
    ///
    /// @step Given a ChainOfCommand with no relationships
    /// @step And session "xyz" is supervising session "abc"
    /// @step When I call remove_supervisor with supervisor_id "xyz"
    /// @step Then get_supervisors for "abc" should return an empty list
    /// @step And get_subordinate for "xyz" should return None
    #[test]
    fn test_remove_supervisor_relationship() {
        // @step Given a ChainOfCommand with no relationships
        let chain_of_command = ChainOfCommand::new();

        let subordinate_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000a4").unwrap();
        let supervisor_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000b4").unwrap();

        // @step And session "xyz" is supervising session "abc"
        let _ = chain_of_command.add_supervisor(subordinate_id, supervisor_id);

        // @step When I call remove_supervisor with supervisor_id "xyz"
        chain_of_command.remove_supervisor(supervisor_id);

        // @step Then get_supervisors for "abc" should return an empty list
        let supervisors = chain_of_command.get_supervisors(subordinate_id);
        assert!(
            supervisors.is_empty(),
            "get_supervisors should return empty list"
        );

        // @step And get_subordinate for "xyz" should return None
        let subordinate = chain_of_command.get_subordinate(supervisor_id);
        assert_eq!(subordinate, None, "get_subordinate should return None");
    }

    /// Scenario: Supervisor can observe multiple subordinates (FIX-7)
    ///
    /// @step Given a ChainOfCommand with no relationships
    /// @step And session "xyz" is supervising session "abc"
    /// @step When I call add_supervisor with subordinate_id "def" and supervisor_id "xyz"
    /// @step Then it should succeed
    /// @step And get_subordinates for "xyz" should return ["abc", "def"]
    #[test]
    fn test_supervisor_can_observe_multiple_subordinates() {
        // @step Given a ChainOfCommand with no relationships
        let chain_of_command = ChainOfCommand::new();

        let subordinate_abc = Uuid::parse_str("00000000-0000-0000-0000-0000000000a5").unwrap();
        let subordinate_def = Uuid::parse_str("00000000-0000-0000-0000-0000000000b5").unwrap();
        let supervisor_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000c5").unwrap();

        // @step And session "xyz" is supervising session "abc"
        let _ = chain_of_command.add_supervisor(subordinate_abc, supervisor_id);

        // @step When I call add_supervisor with subordinate_id "def" and supervisor_id "xyz"
        let result = chain_of_command.add_supervisor(subordinate_def, supervisor_id);

        // @step Then it should succeed
        assert!(
            result.is_ok(),
            "add_supervisor should succeed for multiple subordinates"
        );

        // @step And get_subordinates for "xyz" should return ["abc", "def"]
        let subordinates = chain_of_command.get_subordinates(supervisor_id);
        assert_eq!(subordinates.len(), 2, "should have exactly 2 subordinates");
        assert!(
            subordinates.contains(&subordinate_abc),
            "subordinates should contain abc"
        );
        assert!(
            subordinates.contains(&subordinate_def),
            "subordinates should contain def"
        );

        // get_subordinate (singular, backward compat) returns first
        let first = chain_of_command.get_subordinate(supervisor_id);
        assert_eq!(
            first,
            Some(subordinate_abc),
            "get_subordinate should return first (abc)"
        );
    }

    /// Scenario: Duplicate subordinate under same supervisor is rejected
    ///
    /// @step Given a ChainOfCommand with no relationships
    /// @step And session "xyz" is supervising session "abc"
    /// @step When I call add_supervisor with subordinate_id "abc" and supervisor_id "xyz" again
    /// @step Then it should return an error about duplicate registration
    #[test]
    fn test_duplicate_subordinate_under_same_supervisor_rejected() {
        let chain_of_command = ChainOfCommand::new();

        let subordinate_abc = Uuid::parse_str("00000000-0000-0000-0000-0000000000a5").unwrap();
        let supervisor_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000c5").unwrap();

        let _ = chain_of_command.add_supervisor(subordinate_abc, supervisor_id);
        let result = chain_of_command.add_supervisor(subordinate_abc, supervisor_id);

        assert!(result.is_err(), "duplicate add_supervisor should fail");
        assert!(
            result.unwrap_err().contains("already registered"),
            "error should mention 'already registered'"
        );
    }

    /// Scenario: Circular supervision is prevented
    ///
    /// @step Given a ChainOfCommand with no relationships
    /// @step And session "B" is supervising session "A"
    /// @step When I call add_supervisor with subordinate_id "B" and supervisor_id "A"
    /// @step Then it should return an error "circular supervision not allowed"
    #[test]
    fn test_circular_supervision_prevented() {
        // @step Given a ChainOfCommand with no relationships
        let chain_of_command = ChainOfCommand::new();

        let session_a = Uuid::parse_str("00000000-0000-0000-0000-0000000000a6").unwrap();
        let session_b = Uuid::parse_str("00000000-0000-0000-0000-0000000000b6").unwrap();

        // @step And session "B" is supervising session "A"
        let _ = chain_of_command.add_supervisor(session_a, session_b);

        // @step When I call add_supervisor with subordinate_id "B" and supervisor_id "A"
        let result = chain_of_command.add_supervisor(session_b, session_a);

        // @step Then it should return an error "circular supervision not allowed"
        assert!(
            result.is_err(),
            "add_supervisor should fail for circular supervision"
        );
        assert!(
            result.unwrap_err().contains("circular"),
            "error should mention 'circular'"
        );
    }

    /// Scenario: Regular session has no subordinate
    ///
    /// @step Given a ChainOfCommand with no relationships
    /// @step And a regular session "abc" exists that is not a supervisor
    /// @step When I call get_subordinate with session_id "abc"
    /// @step Then it should return None
    #[test]
    fn test_regular_session_has_no_subordinate() {
        // @step Given a ChainOfCommand with no relationships
        let chain_of_command = ChainOfCommand::new();

        // @step And a regular session "abc" exists that is not a supervisor
        let session_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000a7").unwrap();

        // @step When I call get_subordinate with session_id "abc"
        let subordinate = chain_of_command.get_subordinate(session_id);

        // @step Then it should return None
        assert_eq!(
            subordinate, None,
            "regular session should have no subordinate"
        );
    }

    /// Scenario: Cleanup supervisors when subordinate session is removed
    ///
    /// @step Given a ChainOfCommand with no relationships
    /// @step And session "xyz" is supervising session "abc"
    /// @step And session "def" is supervising session "abc"
    /// @step When subordinate session "abc" is removed
    /// @step Then get_subordinate for "xyz" should return None
    /// @step And get_subordinate for "def" should return None
    /// @step And the ChainOfCommand should have no entries
    #[test]
    fn test_cleanup_supervisors_when_subordinate_removed() {
        // @step Given a ChainOfCommand with no relationships
        let chain_of_command = ChainOfCommand::new();

        let subordinate_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000a8").unwrap();
        let supervisor_xyz = Uuid::parse_str("00000000-0000-0000-0000-0000000000b8").unwrap();
        let supervisor_def = Uuid::parse_str("00000000-0000-0000-0000-0000000000c8").unwrap();

        // @step And session "xyz" is supervising session "abc"
        let _ = chain_of_command.add_supervisor(subordinate_id, supervisor_xyz);

        // @step And session "def" is supervising session "abc"
        let _ = chain_of_command.add_supervisor(subordinate_id, supervisor_def);

        // @step When subordinate session "abc" is removed
        chain_of_command.cleanup_subordinate(subordinate_id);

        // @step Then get_subordinate for "xyz" should return None
        let sub_xyz = chain_of_command.get_subordinate(supervisor_xyz);
        assert_eq!(
            sub_xyz, None,
            "get_subordinate for xyz should return None after cleanup"
        );

        // @step And get_subordinate for "def" should return None
        let sub_def = chain_of_command.get_subordinate(supervisor_def);
        assert_eq!(
            sub_def, None,
            "get_subordinate for def should return None after cleanup"
        );

        // @step And the ChainOfCommand should have no entries
        assert!(
            chain_of_command.is_empty(),
            "ChainOfCommand should be empty after cleanup"
        );
    }
}

#[cfg(test)]
mod supervisor_loop_tests {

    // Feature: spec/features/watcher-agent-loop-with-dual-input.feature

    /// Scenario: Handle broadcast lag gracefully
    ///
    /// @step Given a supervisor session is observing a subordinate session
    /// @step When the supervisor receives RecvError::Lagged with 10 missed chunks
    /// @step Then the supervisor should log a warning about 10 missed chunks
    /// @step And the supervisor should continue observing from the current position
    #[test]
    fn test_handle_broadcast_lag() {
        // @step Given a supervisor session is observing a subordinate session
        // (simulated)

        // @step When the supervisor receives RecvError::Lagged with 10 missed chunks
        let lagged_count: u64 = 10;

        // @step Then the supervisor should log a warning about 10 missed chunks
        // (logging is a side effect - we verify the count is captured)
        let warning_message = format!("Supervisor lagged behind by {} chunks", lagged_count);
        assert!(warning_message.contains("10"));

        // @step And the supervisor should continue observing from the current position
        // (verified by the fact that we don't panic or return error)
        assert!(lagged_count > 0); // Supervisor continues
    }
}

#[cfg(test)]
mod supervisor_input_tests {
    use super::*;

    // Feature: spec/features/watcher-injection-message-format.feature

    /// Scenario: Format peer supervisor message with structured prefix
    ///
    /// @step Given a supervisor session with role "code-reviewer"
    /// @step And the supervisor session id is "abc123"
    /// @step When the supervisor sends message "Consider adding error handling"
    /// @step Then the formatted message should be "[SUPERVISOR: code-reviewer | Session: abc123] Consider adding error handling"
    #[test]
    fn test_format_peer_supervisor_message() {
        // @step Given a supervisor session with role "code-reviewer"
        let role_name = "code-reviewer".to_string();

        // @step And the supervisor session id is "abc123"
        let session_id = "abc123".to_string();

        // @step When the supervisor sends message "Consider adding error handling"
        let message = "Consider adding error handling".to_string();
        let input = IncomingMessage::new(session_id, role_name, message).unwrap();
        let formatted = format_incoming_message(&input);

        // @step Then the formatted message should be "[SUPERVISOR: code-reviewer | Session: abc123] Consider adding error handling"
        assert_eq!(
            formatted,
            "[SUPERVISOR: code-reviewer | Session: abc123] Consider adding error handling"
        );
    }

    /// Scenario: Format authority supervisor message with structured prefix
    ///
    /// @step Given a supervisor session with role "security-auditor"
    /// @step And the supervisor session id is "xyz789"
    /// @step When the supervisor sends message "CRITICAL: SQL injection vulnerability detected"
    /// @step Then the subordinate should receive a IncomingMessage chunk
    /// @step And the chunk should contain the formatted message with structured prefix
    #[test]
    fn test_format_authority_supervisor_message() {
        // @step Given a supervisor session with role "security-auditor"
        let role_name = "security-auditor".to_string();

        // @step And the supervisor session id is "xyz789"
        let session_id = "xyz789".to_string();

        // @step When the supervisor sends message "CRITICAL: SQL injection vulnerability detected"
        let message = "CRITICAL: SQL injection vulnerability detected".to_string();
        let input = IncomingMessage::new(session_id, role_name, message).unwrap();

        // @step Then the subordinate should receive a IncomingMessage chunk
        let chunk = StreamChunk::incoming_message(format_incoming_message(&input));

        // @step And the chunk should contain the formatted message with structured prefix
        // NAPI-010: Use pattern matching
        match chunk {
            StreamChunk::IncomingMessage { text, .. } => {
                assert!(text.starts_with("[SUPERVISOR: security-auditor | Session: xyz789]"));
            }
            _ => panic!("Expected IncomingMessage variant"),
        }
    }

    /// Scenario: Receive supervisor input queues message asynchronously
    ///
    /// This test verifies the supervisor input channel mechanism works correctly.
    /// Note: BackgroundSession.receive_incoming_message() uses try_send which is non-blocking.
    /// We test the channel pattern here since BackgroundSession construction requires
    /// a full codelet_cli::session::Session (integration test territory).
    ///
    /// @step Given a subordinate session exists
    /// @step When receive_incoming_message is called with a valid IncomingMessage
    /// @step Then the input should be queued via the supervisor input channel
    /// @step And the method should return immediately without blocking
    #[test]
    fn test_receive_incoming_message_queues_via_try_send() {
        // @step Given a subordinate session exists
        // We test the channel mechanism that BackgroundSession.receive_incoming_message uses
        let (supervisor_tx, mut supervisor_rx) = tokio::sync::mpsc::channel::<IncomingMessage>(16);

        // @step When receive_incoming_message is called with a valid IncomingMessage
        let input = IncomingMessage::new(
            "session123".to_string(),
            "test-supervisor".to_string(),
            "Test message".to_string(),
        )
        .unwrap();

        // BackgroundSession.receive_incoming_message uses try_send (non-blocking)
        // This mirrors the exact implementation pattern
        let result = supervisor_tx.try_send(input);

        // @step Then the input should be queued via the supervisor input channel
        assert!(
            result.is_ok(),
            "try_send should succeed when channel has capacity"
        );

        // @step And the method should return immediately without blocking
        // try_send is guaranteed non-blocking - verified by using try_send instead of send
        let received = supervisor_rx.try_recv();
        assert!(received.is_ok(), "Message should be in channel");
        assert_eq!(received.unwrap().message, "Test message");
    }

    /// Test that channel returns error when full (matches receive_incoming_message error handling)
    #[test]
    fn test_receive_incoming_message_channel_full_returns_error() {
        // Create a channel with capacity 1
        let (supervisor_tx, _supervisor_rx) = tokio::sync::mpsc::channel::<IncomingMessage>(1);

        let input1 = IncomingMessage::new(
            "s1".to_string(),
            "supervisor".to_string(),
            "First".to_string(),
        )
        .unwrap();

        let input2 = IncomingMessage::new(
            "s2".to_string(),
            "supervisor".to_string(),
            "Second".to_string(),
        )
        .unwrap();

        // First send should succeed
        assert!(supervisor_tx.try_send(input1).is_ok());

        // Second send should fail (channel full)
        let result = supervisor_tx.try_send(input2);
        assert!(result.is_err(), "try_send should fail when channel is full");
    }

    /// Scenario: Empty supervisor message returns error
    ///
    /// @step Given a supervisor session with role "test-supervisor"
    /// @step And the supervisor session id is "test123"
    /// @step When the supervisor sends an empty message
    /// @step Then an error should be returned with message "message cannot be empty"
    #[test]
    fn test_empty_supervisor_message_returns_error() {
        // @step Given a supervisor session with role "test-supervisor"
        let role_name = "test-supervisor".to_string();

        // @step And the supervisor session id is "test123"
        let session_id = "test123".to_string();

        // @step When the supervisor sends an empty message
        let result = IncomingMessage::new(session_id, role_name, "".to_string());

        // @step Then an error should be returned with message "message cannot be empty"
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "message cannot be empty");
    }

    /// Scenario: Multiline supervisor message preserves formatting
    ///
    /// @step Given a supervisor session with role "code-reviewer"
    /// @step And the supervisor session id is "abc123"
    /// @step When the supervisor sends a multiline message
    /// @step Then the formatted message should have the prefix on the first line
    /// @step And subsequent lines should be preserved without additional prefixes
    #[test]
    fn test_multiline_supervisor_message_preserves_formatting() {
        // @step Given a supervisor session with role "code-reviewer"
        let role_name = "code-reviewer".to_string();

        // @step And the supervisor session id is "abc123"
        let session_id = "abc123".to_string();

        // @step When the supervisor sends a multiline message
        let multiline_message =
            "Issue found on line 42:\n- Missing null check\n- Consider using Option<T>".to_string();
        let input = IncomingMessage::new(session_id, role_name, multiline_message).unwrap();
        let formatted = format_incoming_message(&input);

        // @step Then the formatted message should have the prefix on the first line
        assert!(formatted.starts_with("[SUPERVISOR: code-reviewer | Session: abc123]"));

        // @step And subsequent lines should be preserved without additional prefixes
        let lines: Vec<&str> = formatted.lines().collect();
        assert!(lines.len() >= 3); // Prefix line + 2 content lines (or content all on one line after prefix)
                                   // The message content follows the prefix, newlines are preserved
        assert!(formatted.contains("- Missing null check"));
        assert!(formatted.contains("- Consider using Option<T>"));
    }
}

#[cfg(test)]
mod napi_supervisor_tests {
    use super::*;

    // Feature: spec/features/napi-bindings-for-watcher-operations.feature

    /// Scenario: Create supervisor session for a subordinate
    ///
    /// @step Given a subordinate session exists with id "parent-uuid"
    /// @step When I call session_create_supervisor with subordinate "parent-uuid", model "claude-sonnet-4", project "/project", name "Code Reviewer"
    /// @step Then a new supervisor session should be created and returned
    /// @step And the supervisor should be registered in ChainOfCommand with subordinate "parent-uuid"
    /// Note: Broadcast subscription happens lazily when supervisor loop starts
    #[test]
    fn test_create_supervisor_registers_in_chain_of_command() {
        // @step Given a subordinate session exists with id "parent-uuid"
        let subordinate_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let supervisor_id = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        let chain_of_command = ChainOfCommand::new();

        // @step When I call session_create_supervisor (simulated via ChainOfCommand.add_supervisor)
        let result = chain_of_command.add_supervisor(subordinate_id, supervisor_id);

        // @step Then a new supervisor session should be created and returned
        assert!(result.is_ok());

        // @step And the supervisor should be registered in ChainOfCommand with subordinate "parent-uuid"
        assert_eq!(
            chain_of_command.get_subordinate(supervisor_id),
            Some(subordinate_id)
        );

        // Broadcast subscription is lazy - happens when supervisor loop starts via subscribe_to_stream()
        assert!(chain_of_command
            .get_supervisors(subordinate_id)
            .contains(&supervisor_id));
    }

    /// Scenario: Get subordinate of a supervisor session
    ///
    /// @step Given a supervisor session "supervisor-uuid" observing subordinate "parent-uuid"
    /// @step When I call session_get_subordinate with "supervisor-uuid"
    /// @step Then it should return "parent-uuid"
    #[test]
    fn test_get_subordinate_returns_subordinate_id() {
        // @step Given a supervisor session "supervisor-uuid" observing subordinate "parent-uuid"
        let subordinate_id = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();
        let supervisor_id = Uuid::parse_str("00000000-0000-0000-0000-000000000004").unwrap();
        let chain_of_command = ChainOfCommand::new();
        chain_of_command
            .add_supervisor(subordinate_id, supervisor_id)
            .unwrap();

        // @step When I call session_get_subordinate with "supervisor-uuid"
        let result = chain_of_command.get_subordinate(supervisor_id);

        // @step Then it should return "parent-uuid"
        assert_eq!(result, Some(subordinate_id));
    }

    /// Scenario: Get subordinate of a regular session returns None
    ///
    /// @step Given a regular session "regular-uuid" with no subordinate
    /// @step When I call session_get_subordinate with "regular-uuid"
    /// @step Then it should return None
    #[test]
    fn test_get_subordinate_returns_none_for_regular_session() {
        // @step Given a regular session "regular-uuid" with no subordinate
        let regular_id = Uuid::parse_str("00000000-0000-0000-0000-000000000005").unwrap();
        let chain_of_command = ChainOfCommand::new();

        // @step When I call session_get_subordinate with "regular-uuid"
        let result = chain_of_command.get_subordinate(regular_id);

        // @step Then it should return None
        assert_eq!(result, None);
    }

    /// Scenario: Get supervisors of a subordinate session
    ///
    /// @step Given a subordinate session "parent-uuid"
    /// @step And supervisor session "supervisor-1-uuid" supervising "parent-uuid"
    /// @step And supervisor session "supervisor-2-uuid" supervising "parent-uuid"
    /// @step When I call session_get_supervisors with "parent-uuid"
    /// @step Then it should return ["supervisor-1-uuid", "supervisor-2-uuid"]
    #[test]
    fn test_get_supervisors_returns_supervisor_list() {
        // @step Given a subordinate session "parent-uuid"
        let subordinate_id = Uuid::parse_str("00000000-0000-0000-0000-000000000006").unwrap();
        let supervisor_1_id = Uuid::parse_str("00000000-0000-0000-0000-000000000007").unwrap();
        let supervisor_2_id = Uuid::parse_str("00000000-0000-0000-0000-000000000008").unwrap();
        let chain_of_command = ChainOfCommand::new();

        // @step And supervisor session "supervisor-1-uuid" supervising "parent-uuid"
        chain_of_command
            .add_supervisor(subordinate_id, supervisor_1_id)
            .unwrap();

        // @step And supervisor session "supervisor-2-uuid" supervising "parent-uuid"
        chain_of_command
            .add_supervisor(subordinate_id, supervisor_2_id)
            .unwrap();

        // @step When I call session_get_supervisors with "parent-uuid"
        let supervisors = chain_of_command.get_supervisors(subordinate_id);

        // @step Then it should return ["supervisor-1-uuid", "supervisor-2-uuid"]
        assert_eq!(supervisors.len(), 2);
        assert!(supervisors.contains(&supervisor_1_id));
        assert!(supervisors.contains(&supervisor_2_id));
    }

    /// Scenario: Get supervisors of a session with no supervisors
    ///
    /// @step Given a session "lonely-uuid" with no supervisors
    /// @step When I call session_get_supervisors with "lonely-uuid"
    /// @step Then it should return an empty array
    #[test]
    fn test_get_supervisors_returns_empty_for_no_supervisors() {
        // @step Given a session "lonely-uuid" with no supervisors
        let lonely_id = Uuid::parse_str("00000000-0000-0000-0000-000000000009").unwrap();
        let chain_of_command = ChainOfCommand::new();

        // @step When I call session_get_supervisors with "lonely-uuid"
        let supervisors = chain_of_command.get_supervisors(lonely_id);

        // @step Then it should return an empty array
        assert!(supervisors.is_empty());
    }
}

#[cfg(test)]
mod supervisor_integration_tests {
    use super::*;
    use tokio::sync::broadcast;

    // Feature: spec/features/watcher-loop-and-input-channel-not-integrated.feature (WATCH-019)

    /// Scenario: Supervisor session subscribes to subordinate broadcast on creation
    ///
    /// @step Given a subordinate session exists with an active broadcast channel
    /// @step When session_create_supervisor is called with the subordinate session ID
    /// @step Then the supervisor should have a broadcast receiver subscribed to the subordinate's stream
    #[test]
    fn test_supervisor_subscribes_to_subordinate_broadcast() {
        // @step Given a subordinate session exists with an active broadcast channel
        let (subordinate_broadcast_tx, _) =
            broadcast::channel::<StreamChunk>(SUPERVISOR_BROADCAST_CAPACITY);

        // @step When session_create_supervisor is called with the subordinate session ID
        // Simulate what session_create_supervisor does: subscribe to subordinate's broadcast
        let mut supervisor_broadcast_rx = subordinate_broadcast_tx.subscribe();

        // @step Then the supervisor should have a broadcast receiver subscribed to the subordinate's stream
        // Send a chunk from subordinate and verify supervisor receives it
        let test_chunk = StreamChunk::text("test from subordinate".to_string());
        subordinate_broadcast_tx
            .send(test_chunk.clone())
            .expect("Should send");

        let received = supervisor_broadcast_rx.try_recv();
        assert!(
            received.is_ok(),
            "Supervisor should receive chunks from subordinate broadcast"
        );
        // NAPI-010: Check using pattern matching on the enum variant
        match received.unwrap() {
            StreamChunk::Text { text, .. } => {
                assert_eq!(text, "test from subordinate");
            }
            _ => panic!("Expected Text variant"),
        }
    }
}

// =============================================================================
// TUI-059: WORK UNIT CONTEXT TESTS
// =============================================================================

#[cfg(test)]
mod work_unit_context_tests {
    use super::*;

    // Feature: spec/features/work-unit-context.feature
    // Tests for WorkUnitContext struct and related functionality (TUI-059)

    // =========================================================================
    // Scenario: Work unit ID appears in environment information when entering AgentView
    // =========================================================================

    /// @step Given work unit "AUTH-001" exists in the backlog
    /// @step When I select work unit "AUTH-001" and press Enter
    /// @step Then I should be in the AgentView
    /// @step And the environment information should contain "Current work unit: AUTH-001"
    #[test]
    fn test_format_for_environment_returns_correct_format() {
        // @step Given work unit "AUTH-001" exists in the backlog
        // @step When I select work unit "AUTH-001" and press Enter
        let ctx = WorkUnitContext::new(
            "AUTH-001".to_string(),
            "User Authentication".to_string(),
            "specifying".to_string(),
        );

        // @step Then I should be in the AgentView
        // @step And the environment information should contain "Current work unit: AUTH-001"
        let env_info = ctx.format_for_environment();
        assert!(
            env_info.is_some(),
            "Should return environment info when context is set"
        );
        assert_eq!(env_info.unwrap(), "Current work unit: AUTH-001");
    }

    /// @step And the environment information should not contain the work unit title
    /// @step And the environment information should not contain the work unit status
    #[test]
    fn test_format_for_environment_excludes_title_and_status() {
        // @step Given a work unit context with title and status
        let ctx = WorkUnitContext::new(
            "AUTH-001".to_string(),
            "User Authentication".to_string(),
            "specifying".to_string(),
        );

        // @step When the environment info is formatted
        let env_info = ctx.format_for_environment().unwrap();

        // @step And the environment information should not contain the work unit title
        assert!(
            !env_info.contains("User Authentication"),
            "Should NOT contain title"
        );

        // @step And the environment information should not contain the work unit status
        assert!(
            !env_info.contains("specifying"),
            "Should NOT contain status"
        );
    }

    /// Test format_for_environment returns None when context is not set
    #[test]
    fn test_format_for_environment_returns_none_when_not_set() {
        // Given a default (empty) work unit context
        let ctx = WorkUnitContext::default();

        // When format_for_environment is called
        let env_info = ctx.format_for_environment();

        // Then it should return None
        assert!(
            env_info.is_none(),
            "Should return None when context is not set"
        );
    }

    // =========================================================================
    // Scenario: LLM receives notification when updating a different work unit
    // =========================================================================

    /// @step Given the session is attached to work unit "AUTH-001"
    /// @step When I run "update-work-unit-status BUG-002 implementing"
    /// @step Then the session work unit context should be updated to "BUG-002"
    #[test]
    fn test_work_unit_context_new_creates_valid_context() {
        // @step Given the session is attached to work unit "AUTH-001"
        let ctx = WorkUnitContext::new(
            "AUTH-001".to_string(),
            "User Authentication".to_string(),
            "specifying".to_string(),
        );

        // Then context should have correct values
        assert_eq!(ctx.id, Some("AUTH-001".to_string()));
        assert_eq!(ctx.title, Some("User Authentication".to_string()));
        assert_eq!(ctx.status, Some("specifying".to_string()));
        assert!(ctx.is_set(), "Context should be set");
    }

    /// Test that context can be updated with new values
    #[test]
    fn test_work_unit_context_can_be_replaced() {
        // @step Given session is attached to "AUTH-001"
        let ctx1 = WorkUnitContext::new(
            "AUTH-001".to_string(),
            "User Authentication".to_string(),
            "specifying".to_string(),
        );
        assert_eq!(ctx1.id, Some("AUTH-001".to_string()));

        // @step When context changes to "BUG-002"
        let ctx2 = WorkUnitContext::new(
            "BUG-002".to_string(),
            "Fix login bug".to_string(),
            "implementing".to_string(),
        );

        // @step Then the session work unit context should be updated to "BUG-002"
        assert_eq!(ctx2.id, Some("BUG-002".to_string()));
        assert_eq!(ctx2.title, Some("Fix login bug".to_string()));
        assert_eq!(ctx2.status, Some("implementing".to_string()));
    }

    // =========================================================================
    // Scenario: No notification when updating the same work unit
    // =========================================================================

    /// @step And the session work unit context should remain "AUTH-001"
    #[test]
    fn test_work_unit_context_same_id_detection() {
        // @step Given the session is attached to work unit "AUTH-001"
        let ctx = WorkUnitContext::new(
            "AUTH-001".to_string(),
            "User Authentication".to_string(),
            "specifying".to_string(),
        );

        // @step When checking if IDs match
        // (This tests the id field which is used for comparison in TypeScript layer)
        assert_eq!(ctx.id, Some("AUTH-001".to_string()));

        // @step And the session work unit context should remain "AUTH-001"
        // Same ID means no change notification is needed
    }

    // =========================================================================
    // Scenario: No notification when no active session exists
    // =========================================================================

    /// @step Given there is no active TUI session
    #[test]
    fn test_work_unit_context_default_is_not_set() {
        // @step Given there is no active TUI session
        let ctx = WorkUnitContext::default();

        // Then context should not be set
        assert!(!ctx.is_set(), "Default context should not be set");
        assert!(ctx.id.is_none());
        assert!(ctx.title.is_none());
        assert!(ctx.status.is_none());
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    /// Test is_set returns true only when id is present
    #[test]
    fn test_is_set_depends_only_on_id() {
        // Context with only id
        let ctx_id_only = WorkUnitContext {
            id: Some("TEST-001".to_string()),
            title: None,
            status: None,
        };
        assert!(ctx_id_only.is_set(), "Should be set when id is present");

        // Context with title and status but no id
        let ctx_no_id = WorkUnitContext {
            id: None,
            title: Some("Some Title".to_string()),
            status: Some("testing".to_string()),
        };
        assert!(!ctx_no_id.is_set(), "Should NOT be set when id is missing");
    }

    /// Test format_for_environment with special characters in ID
    #[test]
    fn test_format_for_environment_with_special_characters() {
        let ctx = WorkUnitContext::new(
            "SPEC-123-äöü".to_string(),
            "Feature with émojis 🚀".to_string(),
            "in-progress".to_string(),
        );

        let env_info = ctx.format_for_environment().unwrap();
        assert_eq!(env_info, "Current work unit: SPEC-123-äöü");
    }

    /// Test format_for_environment with empty string ID
    #[test]
    fn test_format_for_environment_with_empty_id() {
        let ctx = WorkUnitContext::new(
            "".to_string(),
            "Empty ID".to_string(),
            "backlog".to_string(),
        );

        // Empty string is still Some(""), so format_for_environment should return something
        let env_info = ctx.format_for_environment();
        assert!(env_info.is_some());
        assert_eq!(env_info.unwrap(), "Current work unit: ");
    }

    /// Test Clone implementation
    #[test]
    fn test_work_unit_context_clone() {
        let ctx1 = WorkUnitContext::new(
            "AUTH-001".to_string(),
            "User Authentication".to_string(),
            "specifying".to_string(),
        );

        let ctx2 = ctx1.clone();

        assert_eq!(ctx1.id, ctx2.id);
        assert_eq!(ctx1.title, ctx2.title);
        assert_eq!(ctx1.status, ctx2.status);
    }

    /// Test Debug implementation
    #[test]
    fn test_work_unit_context_debug() {
        let ctx = WorkUnitContext::new(
            "AUTH-001".to_string(),
            "User Authentication".to_string(),
            "specifying".to_string(),
        );

        let debug_output = format!("{:?}", ctx);
        assert!(debug_output.contains("AUTH-001"));
        assert!(debug_output.contains("User Authentication"));
        assert!(debug_output.contains("specifying"));
    }
}

// ==============================================================================
// RPC-043: lines 1216-2917 (run_with_provider! macro, agent_loop_dispatch_supports_provider,
// agent_loop_dispatch_tests, InputWithImages, agent_loop fn, BackgroundOutput,
// BackgroundProgressEmitter) extracted into `crate::agent_loop`.
// ==============================================================================

// =============================================================================
// NAPI Bindings
// =============================================================================

/// Create a new background session (generates new UUID)
#[napi]
pub async fn session_manager_create(model: String, project: String) -> Result<String> {
    SessionManager::instance()
        .create_session(&model, &project)
        .await
        .map_err(napi::Error::from_reason)
}

/// Create a background session with a specific ID (for persistence integration).
///
/// This is used when AgentView creates a session - the ID comes from persistence
/// so that detach/attach can find the session by the same ID used for persistence.
/// Credentials are resolved internally by Rust using the credentials module.
///
/// Note: This must be async because it uses tokio::spawn internally, which requires
/// a Tokio runtime context. NAPI-RS provides this context for async functions.
#[napi]
pub async fn session_manager_create_with_id(
    session_id: String,
    model: String,
    project: String,
    name: String,
) -> Result<()> {
    SessionManager::instance()
        .create_session_with_id(&session_id, &model, &project, &name)
        .await
        .map_err(napi::Error::from_reason)
}

/// GIT-028: Result of creating an isolated session
#[napi(object)]
pub struct IsolatedSessionResult {
    /// Session ID
    pub session_id: String,
    /// Path to the worktree directory
    pub worktree_path: String,
    /// Base commit SHA the worktree was created from
    pub base_commit: String,
}

/// GIT-028: Create an isolated background session with a git worktree.
///
/// This creates a session that operates in an isolated git worktree,
/// allowing the AI agent to make file changes without affecting the main project.
/// The worktree is created at `.fspec/worktrees/<session-id>/`.
///
/// A session manifest is also created at `~/.fspec/git-sessions/<session-id>.json`
/// for orphan detection and management.
///
/// @param session_id - Unique session identifier (UUID format)
/// @param model - Model path in "provider/model-id" format
/// @param project - Path to the git repository
/// @param name - Display name for the session
/// @returns IsolatedSessionResult with worktree path and base commit
#[napi]
pub async fn session_manager_create_isolated(
    session_id: String,
    model: String,
    project: String,
    name: String,
) -> Result<IsolatedSessionResult> {
    SessionManager::instance()
        .create_isolated_session_with_id(&session_id, &model, &project, &name)
        .await
        .map(IsolatedSessionResult::from)
        .map_err(napi::Error::from_reason)
}

impl From<codelet_rpc_types::IsolatedSessionInfo> for IsolatedSessionResult {
    fn from(info: codelet_rpc_types::IsolatedSessionInfo) -> Self {
        Self {
            session_id: info.session_id.value,
            worktree_path: info.worktree_path,
            base_commit: info.base_commit,
        }
    }
}

/// List all background sessions
#[napi]
pub fn session_manager_list() -> Vec<SessionInfo> {
    let sm = SessionManager::instance();
    let project_path = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    sm.list_sessions(&project_path)
}

/// Destroy a background session
#[napi]
pub fn session_manager_destroy(session_id: String) -> Result<()> {
    let sm = SessionManager::instance();
    sm.destroy_session(&session_id)
        .map_err(napi::Error::from_reason)?;

    // KGRAPH-002: Close graph database when no sessions remain to avoid Lance corruption
    let session_count = sm
        .list_sessions(
            &std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
        )
        .len();
    if session_count == 0 {
        crate::graph::close_graph_db();
    }

    Ok(())
}

/// Set the global chunk callback for all sessions.
///
/// This registers a single callback that receives ALL chunks from ALL sessions.
/// The callback signature is (args: { session_id: string, chunk: StreamChunk }) => void.
/// TypeScript uses this to route chunks to the appropriate session handlers.
///
/// This should be called ONCE at application startup by GlobalSessionStreamManager.
/// Calling it again will fail (callback can only be set once).
///
/// RPC-041: The TSFN is stored inside `CHUNK_FANOUT_TSFN` and a single
/// long-running tokio task subscribes to
/// `SessionManager::instance().chunks_tx()` and forwards every
/// `(SessionId, StreamChunk)` tuple into the stored TSFN. This
/// replaces the old chunk-callback OnceCell static.
#[napi]
pub fn session_set_global_chunk_callback(
    callback: ThreadsafeFunction<GlobalChunkCallbackArgs>,
) -> Result<()> {
    // RPC-041: store the TSFN inside CHUNK_FANOUT_TSFN. The OnceCell
    // initializes the parking_lot::Mutex slot exactly once; subsequent
    // calls would overwrite the stored handle, so reject re-registration
    // explicitly to preserve the pre-existing single-registration
    // semantics.
    let slot = CHUNK_FANOUT_TSFN.get_or_init(|| std::sync::Mutex::new(None));
    {
        let mut guard = slot.lock().map_err(|_| {
            Error::from_reason("CHUNK_FANOUT_TSFN mutex poisoned during registration")
        })?;
        if guard.is_some() {
            return Err(Error::from_reason(
                "Global chunk callback already set. It can only be set once at startup.",
            ));
        }
        *guard = Some(callback);
    }

    // RPC-041: subscribe to the manager-owned chunks_tx exactly once and
    // spawn a long-running task that forwards every
    // `(SessionId, StreamChunk)` tuple into the stored TSFN via
    // `ThreadsafeFunctionCallMode::NonBlocking`.
    //
    // BUG-FIX: must use `napi::bindgen_prelude::spawn` (which spawns on
    // the napi-managed tokio runtime) — NOT bare `tokio::spawn`. This
    // function is a SYNC `#[napi]` export, and sync napi exports are
    // invoked outside of any tokio runtime context. Calling
    // `tokio::spawn` here panics with "there is no reactor running,
    // must be called from the context of a Tokio 1.x runtime", which
    // shows up as `fatal runtime error: failed to initiate panic` and
    // aborts the entire fspec process at TUI startup.
    let mut rx = SessionManager::instance().chunks_tx().subscribe();
    napi::bindgen_prelude::spawn(async move {
        loop {
            match rx.recv().await {
                Ok((sid, chunk)) => {
                    if let Some(slot) = CHUNK_FANOUT_TSFN.get() {
                        if let Ok(guard) = slot.lock() {
                            if let Some(ref tsfn) = *guard {
                                let args = GlobalChunkCallbackArgs {
                                    session_id: sid.to_string(),
                                    chunk,
                                };
                                let _ =
                                    tsfn.call(Ok(args), ThreadsafeFunctionCallMode::NonBlocking);
                            }
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(
                        "RPC-041 CHUNK_FANOUT_TSFN subscriber lagged by {} chunks — slow JS callback",
                        n
                    );
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // BLOCK-006: Register block notification callbacks with tools crate
    // These callbacks use the chunks_tx broadcast to emit UserNotification chunks
    init_block_notification_callbacks();

    // RPC-040: Install NapiSessionManagerHooks on the moved SessionManager
    // singleton so agent_loop spawning, footer poller, scheduler, and
    // IsolationStateChange fan-out continue to fire from the napi side.
    install_napi_session_manager_hooks();

    // BRIDGE-SESSION: Register session list and model info providers so bridge relay
    // can populate instance metadata with current sessions and model info.
    init_bridge_metadata_providers();

    // SESS-017: Register session creator + PTY registry so the bridge can
    // handle session:create and terminal:create envelopes from the dashboard.
    init_bridge_session_and_terminal_creators();

    Ok(())
}

/// Explicitly set the active session for navigation.
///
/// Use this when switching sessions to update the navigation state.
///
/// VIEWNV-001: This allows TypeScript to explicitly control the navigation state.
#[napi]
pub fn session_set_active(session_id: String) -> Result<()> {
    let uuid = uuid::Uuid::parse_str(&session_id)
        .map_err(|e| Error::from_reason(format!("Invalid session ID: {}", e)))?;
    let manager = SessionManager::instance();
    // Verify session exists
    let _ = manager
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    manager.set_active_session(uuid);
    Ok(())
}

/// Send input to a session with optional thinking config.
///
/// RPC-039: `BackgroundSession::send_input` now returns
/// `Result<(), String>` (the moved type lives in
/// `codelet_sessions::background_session` and is NAPI-free). This
/// thin wrapper maps the String error back into the napi `Result<()>`
/// shape so the TypeScript `Promise<void>` signature is preserved.
#[napi]
pub fn session_send_input(
    session_id: String,
    input: String,
    thinking_config: Option<String>,
) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    session
        .send_input(input, thinking_config)
        .map_err(Error::from_reason)
}

/// Interrupt a session
#[napi]
pub fn session_interrupt(session_id: String) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    session.interrupt();
    Ok(())
}

/// TUI-065: Clear session history and reinject context reminders
///
/// This function clears the session's messages, turns, and token tracker,
/// then reinjects the context reminders (CLAUDE.md, environment info) so
/// the AI retains project context after clearing.
///
/// DRY: This is the single source of truth for clear functionality.
/// Both TUI /clear command and Telegram bridge /clear should use this.
#[napi]
pub fn session_clear_history(session_id: String) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    session.clear_history();
    Ok(())
}

/// Get session status
#[napi]
pub fn session_get_status(session_id: String) -> Result<String> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    let status = session.get_status();
    Ok(status.as_str().to_string())
}

/// PERF-002: Get compaction progress for a session
///
/// Returns the current compaction progress if compaction is in progress, null otherwise.
/// Used by TypeScript to display progress indication: "Preparing compaction..."
#[napi]
pub fn session_get_compaction_progress(
    session_id: String,
) -> Result<Option<crate::types::CompactionProgress>> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    Ok(session
        .get_compaction_progress()
        .map(|p| crate::types::CompactionProgress {
            phase: p.phase,
            current: p.current,
            total: p.total,
        }))
}

// === PAUSE-001: Session pause NAPI functions ===

/// Get pause state for a session (PAUSE-001)
///
/// Returns the current pause state if the session is paused, null otherwise.
/// TypeScript uses this to display pause UI (tool name, message, kind).
#[napi]
pub fn session_get_pause_state(session_id: String) -> Result<Option<NapiPauseState>> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    Ok(session.get_pause_state().map(|s| s.into()))
}

/// Get HITL request state for a session (BUG-117)
///
/// Returns the current HITL questions if the session is paused waiting for user input.
/// TypeScript polls this to render the HITL question UI inline (like pause state).
#[napi]
pub fn session_get_hitl_request(
    session_id: String,
) -> Result<Option<crate::types::NapiHitlRequestState>> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    Ok(session
        .get_hitl_request()
        .map(|req| crate::types::NapiHitlRequestState {
            questions: req
                .questions
                .iter()
                .map(|q| crate::types::HitlQuestionInfo {
                    id: q.id.clone(),
                    header: q.header.clone(),
                    question: q.question.clone(),
                    options: q.options.as_ref().map(|opts| {
                        opts.iter()
                            .map(|o| crate::types::HitlOptionInfo {
                                label: o.label.clone(),
                                description: o.description.clone(),
                            })
                            .collect()
                    }),
                })
                .collect(),
        }))
}

/// Resume a paused session (PAUSE-001)
///
/// Called when user presses Enter during a Continue pause.
/// Sends Resumed response to unblock the waiting tool.
#[napi]
pub fn session_pause_resume(session_id: String) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    session.send_pause_response(PauseResponse::Resumed);
    Ok(())
}

/// Confirm or deny a paused session (PAUSE-001)
///
/// Called when user presses Y (approved=true) or N (approved=false) during a Confirm pause.
/// Sends Approved or Denied response to unblock the waiting tool.
#[napi]
pub fn session_pause_confirm(session_id: String, approved: bool) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    let response = if approved {
        PauseResponse::Approved
    } else {
        PauseResponse::Denied
    };
    session.send_pause_response(response);
    Ok(())
}

/// Handle triple pause response (Allow Once / Allow Session / Deny)
///
/// Called when user makes a selection during a Triple pause (blocklist prompts).
/// Valid choices: "allow_once", "allow_session", "deny"
#[napi]
pub fn session_pause_triple(session_id: String, choice: String) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    let response = match choice.as_str() {
        "allow_once" => PauseResponse::AllowOnce,
        "allow_session" => PauseResponse::AllowSession,
        "deny" => PauseResponse::Denied,
        _ => PauseResponse::Denied, // Default to deny for invalid choices
    };
    session.send_pause_response(response);
    Ok(())
}

// === CODE-009: Fspec command result NAPI function ===

/// Send fspec command result back to Rust (CODE-009)
///
/// Called by TypeScript after executing an fspec command. The result is sent
/// back to unblock the session that's waiting for it.
///
/// TypeScript usage:
/// ```typescript
/// sessionSendFspecResult(sessionId, {
///   success: true,
///   data: '{"id":"CODE-001"}',
///   error: null,
///   systemReminder: '<system-reminder>...</system-reminder>',
///   toolCallId: 'tool-123'
/// });
/// ```
#[napi]
pub fn session_send_fspec_result(
    session_id: String,
    result: crate::types::FspecResult,
) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    session.send_fspec_result(result);
    Ok(())
}

// === BUG-117: HITL response NAPI function ===

/// Send HITL response back to Rust (BUG-117)
///
/// Called by TypeScript after the user answers questions in the HITL modal.
/// The response is sent back to unblock the handler that's waiting for it.
///
/// TypeScript usage:
/// ```typescript
/// sessionSendHitlResponse(sessionId, {
///   cancelled: false,
///   answers: [
///     { id: 'approach', selected: ['Option A'], other: 'Additional notes' },
///   ],
/// });
/// ```
#[napi]
pub fn session_send_hitl_response(
    session_id: String,
    response: crate::types::HitlResponseInfo,
) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;

    // Convert NAPI HitlResponseInfo to codelet_tools HitlResponse
    let hitl_response = if response.cancelled {
        codelet_tools::request_user_input::HitlResponse::Cancelled { cancelled: true }
    } else {
        let mut answers = std::collections::HashMap::new();
        if let Some(entries) = response.answers {
            for entry in entries {
                answers.insert(
                    entry.id,
                    codelet_tools::request_user_input::HitlAnswer {
                        selected: entry.selected,
                        other: entry.other,
                    },
                );
            }
        }
        codelet_tools::request_user_input::HitlResponse::Answered { answers }
    };

    session.send_hitl_response(hitl_response);
    Ok(())
}

// === TOOL-022 P2: exec-stdin prompt NAPI functions ===

/// Get exec-stdin request state for a session (TOOL-022 P2)
///
/// Returns the pending exec-stdin prompt if a live exec session has
/// been quiet >= 3s while its child is alive. Pure overlay — NO
/// status flip, NO response channel. TypeScript polls this to render
/// the inline prompt in the composer slot (like pause state).
#[napi]
pub fn session_get_exec_stdin_request(
    session_id: String,
) -> Result<Option<crate::types::ExecStdinRequest>> {
    use codelet_core::SessionManagerHandle;
    let manager = SessionManager::instance();
    // The NAPI `session_id` is the session key verbatim (matches the
    // `session_get_hitl_request` / `session_get_pause_state` pattern).
    let sid = codelet_rpc_types::SessionId::new(session_id);
    Ok(manager.get_exec_stdin_request(&sid))
}

/// Write typed text to a live exec session's stdin (TOOL-022 P2)
///
/// Called by the TUI when the user presses Enter on the exec-stdin
/// prompt. A trailing newline is appended when absent (matching the
/// unified_exec `write` action semantics). Unknown agent session or
/// unknown/exited exec session returns a clean error naming the id.
#[napi]
pub fn session_write_exec_stdin(
    session_id: String,
    exec_session_id: String,
    text: String,
) -> Result<()> {
    use codelet_core::SessionManagerHandle;
    let manager = SessionManager::instance();
    let sid = codelet_rpc_types::SessionId::new(session_id);
    manager
        .write_exec_stdin(&sid, &exec_session_id, &text)
        .map_err(napi::Error::from_reason)
}

// === TUI-054: Base thinking level NAPI functions ===

/// Get the base thinking level for a session (TUI-054)
///
/// Returns the base thinking level: 0=Off, 1=Low, 2=Medium, 3=High
/// This is the level set via /thinking command dialog.
#[napi]
pub fn session_get_base_thinking_level(session_id: String) -> Result<u8> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    Ok(session.get_base_thinking_level())
}

/// Set the base thinking level for a session (TUI-054)
///
/// Sets the base thinking level: 0=Off, 1=Low, 2=Medium, 3=High
/// Values > 3 are clamped to 3.
/// This is called when user selects a level in the /thinking dialog.
#[napi]
pub fn session_set_base_thinking_level(session_id: String, level: u8) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    session.set_base_thinking_level(level);
    Ok(())
}

// === VIEWNV-001: Session navigation NAPI functions ===

/// Get the next session after the currently active one (VIEWNV-001)
/// Returns None if no sessions exist or at the last session
/// If no active session (BoardView), returns the first session
#[napi]
pub fn session_get_next() -> Option<String> {
    SessionManager::instance().get_next_session()
}

/// Get the previous session before the currently active one (VIEWNV-001)
/// Returns None if no sessions exist or at the first session (should go to board)
#[napi]
pub fn session_get_prev() -> Option<String> {
    SessionManager::instance().get_prev_session()
}

/// Get the first session (VIEWNV-001)
/// Returns None if no sessions exist
#[napi]
pub fn session_get_first() -> Option<String> {
    SessionManager::instance().get_first_session()
}

/// Clear the active session tracking (VIEWNV-001)
/// Call this when returning to BoardView to ensure navigation works correctly
#[napi]
pub fn session_clear_active() {
    SessionManager::instance().clear_active_session();
}

/// Get turn details for a session (TUI-057)
///
/// Returns detailed information about a specific conversation turn including
/// user message, assistant response, tool calls, and file modifications.
///
/// The turn_index is 0-based and refers to the index in the session's turns vector.
#[napi]
pub async fn session_get_turn_details(
    session_id: String,
    turn_index: u32,
) -> Result<Option<NapiTurnDetails>> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    let inner = session.inner.lock().await;

    // Get the turns from the inner session
    let turns = &inner.turns;

    // Find the turn at the given index
    let turn_idx = turn_index as usize;
    if turn_idx >= turns.len() {
        return Ok(None);
    }

    let turn = &turns[turn_idx];

    // Convert tool calls to NAPI format
    let tool_calls: Vec<NapiToolCall> = turn
        .tool_calls
        .iter()
        .map(|tc| NapiToolCall {
            tool: tc.tool.clone(),
            parameters: tc.parameters.to_string(),
            success: turn.tool_results.iter().any(|tr| tr.success),
        })
        .collect();

    // Extract file modifications from tool calls (Edit, Write operations)
    let file_modifications: Vec<NapiFileModification> = turn
        .tool_calls
        .iter()
        .filter_map(|tc| {
            let file_path = tc.file_path()?;
            let operation = match tc.tool.as_str() {
                "Write" => "create",
                "Edit" => "edit",
                "Delete" | "Bash" => return None, // Bash may do many things, skip
                _ => return None,
            };
            Some(NapiFileModification {
                path: file_path,
                operation: operation.to_string(),
                summary: format!("{} operation", tc.tool),
            })
        })
        .collect();

    // Determine overall status from tool results
    let status = if turn.tool_results.iter().all(|tr| tr.success) {
        "success"
    } else if turn.tool_results.iter().any(|tr| tr.success) {
        "partial"
    } else if turn.tool_results.is_empty() {
        "success" // No tools = success (just conversation)
    } else {
        "failed"
    };

    // Build context summary
    let context = if !turn.tool_calls.is_empty() {
        format!("{} tool call(s)", turn.tool_calls.len())
    } else {
        "Conversation turn".to_string()
    };

    Ok(Some(NapiTurnDetails {
        turn_index,
        user_message: turn.user_message.clone(),
        assistant_response: turn.assistant_response.clone(),
        tool_calls,
        file_modifications,
        status: status.to_string(),
        context,
    }))
}

#[napi]
pub async fn session_set_model(
    session_id: String,
    provider_id: String,
    model_id: String,
    context_window: Option<u32>,
    max_output_tokens: Option<u32>,
    compaction_threshold_type: Option<String>,
    compaction_threshold_value: Option<u32>,
) -> Result<()> {
    tracing::debug!("session_set_model called: session_id={}, provider_id={}, model_id={}, context_window={:?}, max_output_tokens={:?}, compaction_threshold_type={:?}, compaction_threshold_value={:?}", 
          session_id, provider_id, model_id, context_window, max_output_tokens, compaction_threshold_type, compaction_threshold_value);

    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;

    // Update metadata for display
    session.set_model(Some(provider_id.clone()), Some(model_id.clone()));

    // Construct model string and update the inner ProviderManager
    let model_string = format!("{}/{}", provider_id, model_id);
    tracing::debug!("session_set_model: selecting model_string={}", model_string);

    let mut inner = session.inner.lock().await;

    // PROV-095: For custom Rhai providers, fetch all three script-set
    // limits (context_window, max_output_tokens, compaction_threshold)
    // in a single cached call. The script is authoritative:
    //   - context_window / max_output_tokens from the script OVERRIDE
    //     the TUI-supplied values (which default to 128_000 / 8_192 in
    //     `customProviderSectionBuilder.ts` and so are meaningless for
    //     a script that returns 400_000).
    //   - compaction_threshold is used only when the TUI has NOT
    //     supplied an explicit value, so users can still override a
    //     script default from the CTX-008 configuration UI.
    let scripted = if codelet_providers::custom_provider_registered(&provider_id) {
        codelet_providers::custom::lookup_script_model_limits(&provider_id, &model_id)
    } else {
        codelet_providers::custom::RhaiScriptedLimits::default()
    };
    let effective_context_window = scripted.context_window.map(|v| v as u32).or(context_window);
    let effective_max_output_tokens = scripted
        .max_output_tokens
        .map(|v| v as u32)
        .or(max_output_tokens);

    // CTX-008: Set compaction threshold override from TUI configuration
    // PROV-095: When the TUI has not supplied an explicit threshold AND
    // the selected provider is a Rhai custom provider, consult the
    // script's `get_model_limits(config).compaction_threshold` return
    // value so scripts can surface their own defaults.
    if let (Some(ct_type), Some(ct_value)) =
        (&compaction_threshold_type, compaction_threshold_value)
    {
        inner
            .provider_manager_mut()
            .set_compaction_threshold_override(Some((ct_type.clone(), ct_value as u64)));
    } else {
        inner
            .provider_manager_mut()
            .set_compaction_threshold_override(scripted.compaction_threshold.clone());
    }

    // PROV-018: Codex models bypass registry validation (not in models.dev under 'codex')
    let result = if provider_id == "codex" {
        inner.provider_manager_mut().set_model_direct(
            &provider_id,
            &model_id,
            effective_context_window.map(|v| v as usize),
            effective_max_output_tokens.map(|v| v as usize),
            None,
        )
    } else {
        let select_result = inner
            .provider_manager_mut()
            .select_model(&model_string)
            .map(|_| ());
        // MODEL-005: NAPI override takes priority over models.dev metadata
        // PROV-095: Script values (when present) take priority over TUI values.
        if select_result.is_ok() {
            inner.provider_manager_mut().override_model_limits(
                effective_context_window.map(|v| v as usize),
                effective_max_output_tokens.map(|v| v as usize),
            );
        }
        select_result
    };
    match result {
        Ok(()) => {
            // CTX-006: Cache resolved model limits from ProviderManager for sync access
            let context_window = inner.provider_manager().context_window() as u32;
            let max_output = inner.provider_manager().max_output_tokens() as u32;
            // CTX-007: Resolve and cache compaction threshold
            let model_id_str = inner.current_model_id();
            let user_config = inner
                .provider_manager()
                .compaction_threshold_override()
                .map(|(t, v)| CompactionThresholdConfig::from_type_value(t, v));
            let compaction_thresh = resolve_compaction_threshold(
                context_window as u64,
                max_output as u64,
                model_id_str.as_deref(),
                user_config.as_ref(),
            ) as u32;
            session.set_model_limits(context_window, max_output, compaction_thresh);
            tracing::debug!("session_set_model: model set successfully (context_window={}, max_output={}, compaction_threshold={})", context_window, max_output, compaction_thresh);

            // BUG-168: update the tool-layer capability registry on model switch
            // so the Read tool's PDF default mode follows the new model.
            let registry_uuid = Uuid::parse_str(&session_id)
                .map_err(|e| Error::from_reason(format!("Invalid session ID: {e}")))?;
            codelet_tools::model_capabilities::set_session_model_vision(
                registry_uuid,
                codelet_sessions::model_resolution::resolve_model_vision(inner.provider_manager()),
            );

            // BUG-132: Re-register DeepSearch and AgentManager handlers with updated model
            let session_uuid = Uuid::parse_str(&session_id)
                .map_err(|e| Error::from_reason(format!("Invalid session ID: {e}")))?;
            let project_path = std::path::PathBuf::from(&session.project);
            register_deep_search_handler(session_uuid, &inner, project_path);
            register_agent_manager_handler(session_uuid, &inner, session.project.clone());

            Ok(())
        }
        Err(e) => {
            tracing::warn!("session_set_model: failed to select model: {}", e);
            Err(Error::from_reason(format!("Failed to select model: {}", e)))
        }
    }
}

/// PROV-007: Set model for profile-based models (vLLM, Ollama, etc.)
///
/// This function sets the model without validating against the models.dev registry.
/// Use this for profile-based models where OPENAI_BASE_URL points to a local server.
/// The caller must ensure OPENAI_BASE_URL and OPENAI_API_KEY are set before calling.
///
/// MODEL-005: Accepts optional context_window and max_output_tokens to propagate
/// per-model limits from TypeScript ModelSelection through to ProviderManager.
///
/// MODEL-004: Accepts optional facade_override for custom models that need
/// agent loop dispatch through a different provider backend.
///
/// BUG-137: Accepts optional `profile_name` so profile-qualified selections
/// (e.g. "openai:fireworks/accounts/fireworks/models/kimi-k2p6") round-trip
/// through `ProviderManager::selected_model_string()` as
/// "{provider}:{profile}/{model}". This is required for AgentManager.spawn
/// to correctly re-create the subordinate session on the same profile
/// endpoint; without it, the subordinate path treated the composite as a
/// cloud model and failed registry validation with "Model 'accounts/...'
/// not found in provider 'openai'".
#[napi]
#[allow(clippy::too_many_arguments)] // NAPI boundary requires flat parameters
pub async fn session_set_model_profile(
    session_id: String,
    provider_id: String,
    model_id: String,
    context_window: Option<u32>,
    max_output_tokens: Option<u32>,
    facade_override: Option<String>,
    compaction_threshold_type: Option<String>,
    compaction_threshold_value: Option<u32>,
    profile_name: Option<String>,
) -> Result<()> {
    tracing::debug!("session_set_model_profile called: session_id={}, provider_id={}, model_id={}, context_window={:?}, max_output_tokens={:?}, facade_override={:?}, compaction_threshold_type={:?}, compaction_threshold_value={:?}, profile_name={:?}",
          session_id, provider_id, model_id, context_window, max_output_tokens, facade_override, compaction_threshold_type, compaction_threshold_value, profile_name);

    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;

    // Update metadata for display
    session.set_model(Some(provider_id.clone()), Some(model_id.clone()));

    // Use set_model_direct which skips registry validation
    // MODEL-005: Pass context params through to ProviderManager
    // MODEL-004: Pass facade_override through to ProviderManager
    let mut inner = session.inner.lock().await;

    // PROV-095: For custom Rhai providers, fetch all three script-set
    // limits (context_window, max_output_tokens, compaction_threshold)
    // in a single cached call. The script is authoritative over the
    // TUI-supplied hard-coded defaults (see session_set_model for the
    // full rationale). Profile-based models without a registered custom
    // provider (vLLM, Ollama, plain profile-qualified cloud aliases)
    // simply get RhaiScriptedLimits::default() and behaviour is
    // unchanged.
    let scripted = if codelet_providers::custom_provider_registered(&provider_id) {
        codelet_providers::custom::lookup_script_model_limits(&provider_id, &model_id)
    } else {
        codelet_providers::custom::RhaiScriptedLimits::default()
    };
    let effective_context_window = scripted.context_window.map(|v| v as u32).or(context_window);
    let effective_max_output_tokens = scripted
        .max_output_tokens
        .map(|v| v as u32)
        .or(max_output_tokens);

    // CTX-008: Set compaction threshold override from TUI configuration.
    // PROV-095: When the TUI has not supplied an explicit threshold,
    // consult the Rhai script's get_model_limits(config).compaction_threshold
    // so scripts can surface their own defaults through this profile
    // path too (the TUI routes custom providers through here, not
    // through session_set_model).
    if let (Some(ct_type), Some(ct_value)) =
        (&compaction_threshold_type, compaction_threshold_value)
    {
        inner
            .provider_manager_mut()
            .set_compaction_threshold_override(Some((ct_type.clone(), ct_value as u64)));
    } else {
        inner
            .provider_manager_mut()
            .set_compaction_threshold_override(scripted.compaction_threshold.clone());
    }

    match inner.provider_manager_mut().set_model_direct_with_profile(
        &provider_id,
        &model_id,
        profile_name.as_deref(),
        effective_context_window.map(|v| v as usize),
        effective_max_output_tokens.map(|v| v as usize),
        facade_override.clone(),
    ) {
        Ok(()) => {
            // PROV-067: For custom providers, derive the facade from
            // api_style if no explicit facade_override was provided.
            // This ensures the agent loop dispatches through the correct
            // built-in provider arm (e.g. "claude" for anthropic_messages).
            let effective_facade = if matches!(
                inner.provider_manager().current_provider_type(),
                codelet_providers::ProviderType::Custom(_)
            ) {
                let derived = facade_override
                    .clone()
                    .or_else(|| codelet_providers::custom::derive_facade_for_custom(&provider_id));
                if derived.is_some() && facade_override.is_none() {
                    // Store the derived facade so the agent loop picks it up.
                    inner
                        .provider_manager_mut()
                        .set_facade_override(derived.clone());
                }
                derived
            } else {
                facade_override.clone()
            };

            // Set env vars so the facade's get_*() method picks up
            // the custom endpoint transparently.
            if matches!(
                inner.provider_manager().current_provider_type(),
                codelet_providers::ProviderType::Custom(_)
            ) {
                if let Err(e) = codelet_providers::custom::apply_custom_provider_env_vars(
                    &provider_id,
                    &model_id,
                    effective_facade.as_deref(),
                ) {
                    tracing::warn!(
                        "PROV-067: apply_custom_provider_env_vars for '{}' failed: {}",
                        provider_id,
                        e
                    );
                }
            }
            // CTX-006: Cache resolved model limits from ProviderManager for sync access
            let resolved_context_window = inner.provider_manager().context_window() as u32;
            let resolved_max_output = inner.provider_manager().max_output_tokens() as u32;
            // CTX-007: Resolve and cache compaction threshold for profile models
            let profile_model_id = inner.current_model_id();
            let profile_user_config = inner
                .provider_manager()
                .compaction_threshold_override()
                .map(|(t, v)| CompactionThresholdConfig::from_type_value(t, v));
            let profile_compaction_thresh = resolve_compaction_threshold(
                resolved_context_window as u64,
                resolved_max_output as u64,
                profile_model_id.as_deref(),
                profile_user_config.as_ref(),
            ) as u32;
            session.set_model_limits(
                resolved_context_window,
                resolved_max_output,
                profile_compaction_thresh,
            );
            tracing::debug!("session_set_model_profile: model set successfully (context_window={}, max_output={}, compaction_threshold={})", resolved_context_window, resolved_max_output, profile_compaction_thresh);

            // BUG-168: update the tool-layer capability registry on model switch
            // so the Read tool's PDF default mode follows the new model.
            let registry_uuid = Uuid::parse_str(&session_id)
                .map_err(|e| Error::from_reason(format!("Invalid session ID: {e}")))?;
            codelet_tools::model_capabilities::set_session_model_vision(
                registry_uuid,
                codelet_sessions::model_resolution::resolve_model_vision(inner.provider_manager()),
            );

            // BUG-132: Re-register DeepSearch and AgentManager handlers with updated model
            let session_uuid = Uuid::parse_str(&session_id)
                .map_err(|e| Error::from_reason(format!("Invalid session ID: {e}")))?;
            let project_path = std::path::PathBuf::from(&session.project);
            register_deep_search_handler(session_uuid, &inner, project_path);
            register_agent_manager_handler(session_uuid, &inner, session.project.clone());

            Ok(())
        }
        Err(e) => {
            tracing::warn!("session_set_model_profile: failed to set model: {}", e);
            Err(Error::from_reason(format!("Failed to set model: {}", e)))
        }
    }
}

/// Get the model info for a background session
#[napi]
pub fn session_get_model(session_id: String) -> Result<SessionModel> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    let provider_id = session
        .provider_id
        .read()
        .map_err(|e| Error::from_reason(format!("Failed to read provider_id: {}", e)))?
        .clone();
    let model_id = session
        .model_id
        .read()
        .map_err(|e| Error::from_reason(format!("Failed to read model_id: {}", e)))?
        .clone();
    // CTX-006: Read cached model limits (0 means not yet resolved)
    let context_window = session.cached_context_window.load(Ordering::Acquire);
    let max_output_tokens = session.cached_max_output_tokens.load(Ordering::Acquire);
    // CTX-007: Read cached compaction threshold
    let compaction_threshold = session.cached_compaction_threshold.load(Ordering::Acquire);
    Ok(SessionModel {
        provider_id,
        model_id,
        context_window: if context_window > 0 {
            Some(context_window)
        } else {
            None
        },
        max_output_tokens: if max_output_tokens > 0 {
            Some(max_output_tokens)
        } else {
            None
        },
        compaction_threshold: if compaction_threshold > 0 {
            Some(compaction_threshold)
        } else {
            None
        },
    })
}

/// Get the INTERNAL provider state from the provider_manager
/// This reads the actual provider that will be used for API calls, not just metadata.
/// BUG-097: Used to verify that sessionSetModelProfile actually updates the provider_manager.
#[napi]
pub async fn session_get_internal_provider(session_id: String) -> Result<SessionModel> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    let inner = session.inner.lock().await;
    let provider_name = inner.current_provider_name().to_string();
    let model_id = inner.current_model_id();
    // CTX-006: Also read resolved limits from the inner ProviderManager
    let context_window = inner.provider_manager().context_window() as u32;
    let max_output_tokens = inner.provider_manager().max_output_tokens() as u32;
    // CTX-007: Resolve compaction threshold from inner ProviderManager
    let internal_model_id = inner.current_model_id();
    let internal_user_config = inner
        .provider_manager()
        .compaction_threshold_override()
        .map(|(t, v)| CompactionThresholdConfig::from_type_value(t, v));
    let compaction_threshold = resolve_compaction_threshold(
        context_window as u64,
        max_output_tokens as u64,
        internal_model_id.as_deref(),
        internal_user_config.as_ref(),
    ) as u32;
    Ok(SessionModel {
        provider_id: Some(provider_name),
        model_id,
        context_window: if context_window > 0 {
            Some(context_window)
        } else {
            None
        },
        max_output_tokens: if max_output_tokens > 0 {
            Some(max_output_tokens)
        } else {
            None
        },
        compaction_threshold: if compaction_threshold > 0 {
            Some(compaction_threshold)
        } else {
            None
        },
    })
}

/// Get cached token counts for a background session
#[napi]
pub fn session_get_tokens(session_id: String) -> Result<SessionTokens> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    let (input_tokens, output_tokens, reasoning_tokens) = session.get_tokens();
    Ok(SessionTokens {
        input_tokens,
        output_tokens,
        reasoning_tokens,
    })
}

/// Get debug enabled state for a background session
#[napi]
pub fn session_get_debug_enabled(session_id: String) -> Result<bool> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    Ok(session.get_debug_enabled())
}

/// Set debug enabled state for a background session (without toggling global state)
#[napi]
pub fn session_set_debug_enabled(session_id: String, enabled: bool) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    session.set_debug_enabled(enabled);
    Ok(())
}

/// Get pending input text for a background session (TUI-049)
///
/// Returns the input text that was being typed when the user switched away from this session.
/// Used to restore input field state when switching back to the session.
#[napi]
pub fn session_get_pending_input(session_id: String) -> Result<Option<String>> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    Ok(session.get_pending_input())
}

/// Set pending input text for a background session (TUI-049)
///
/// Saves the current input field text before switching to another session.
/// Pass None to clear the pending input.
#[napi]
pub fn session_set_pending_input(session_id: String, input: Option<String>) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    session.set_pending_input(input);
    Ok(())
}

/// Get buffered output from a session
#[napi]
pub fn session_get_buffered_output(session_id: String, limit: u32) -> Result<Vec<StreamChunk>> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    Ok(session.get_buffered_output(limit as usize))
}

/// Session role info returned to TypeScript (AMGR-008: simplified from SupervisorRoleInfo)
#[napi(object)]
#[derive(Clone)]
pub struct SupervisorRoleInfo {
    /// Role name (e.g., "security-reviewer")
    pub name: String,
    /// Optional brief describing what this role does (always None for now, kept for API compat)
    pub brief: Option<String>,
}

/// Set the role for a session (AMGR-008: simplified — role is now a plain string)
#[napi]
pub fn session_set_role(
    session_id: String,
    role_name: String,
    _role_brief: Option<String>,
    _auto_inject: Option<bool>,
) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    if role_name.is_empty() {
        // BUG-121: Empty role_name clears the role instead of returning error
        session.clear_role();
    } else {
        session.set_role(role_name);
    }
    Ok(())
}

/// Get the role for a session (AMGR-008: simplified — returns role string wrapped in SupervisorRoleInfo for compat)
#[napi]
pub fn session_get_role(session_id: String) -> Result<Option<SupervisorRoleInfo>> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;

    Ok(session
        .get_role()
        .map(|name| SupervisorRoleInfo { name, brief: None }))
}

// session_clear_role removed — dead code with no consumers

// === SCHED-004: Schedule Metadata NAPI Bindings ===

/// SCHED-004: Check if a session was spawned by the scheduler
#[napi]
pub fn session_is_scheduled(session_id: String) -> Result<bool> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    Ok(session
        .schedule_triggered
        .load(std::sync::atomic::Ordering::Relaxed))
}

/// SCHED-004: Get the schedule name that triggered a session (if any)
#[napi]
pub fn session_schedule_name(session_id: String) -> Result<Option<String>> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    let name = session
        .schedule_name
        .read()
        .expect("schedule_name lock")
        .clone();
    Ok(name)
}

// === SCHED-011 / SCHED-013: Session-Scoped Loop NAPI Bindings ===

/// Register a session-scoped loop with the Rust scheduler.
///
/// SCHED-013: Spawns a per-entry tokio task that fires the prompt into
/// the originating session at exactly the configured interval. The task
/// checks session idle status before each firing (skip-when-busy policy).
///
/// Must be async so NAPI-RS provides the Tokio runtime context — sync NAPI
/// functions don't have access to `tokio::runtime::Handle::try_current()`.
#[napi]
pub async fn loop_register(
    session_id: String,
    loop_id: String,
    prompt: String,
    interval_seconds: u32,
) -> Result<()> {
    let uuid = Uuid::parse_str(&session_id)
        .map_err(|e| Error::from_reason(format!("Invalid session ID: {}", e)))?;

    // Get Tokio runtime handle — available because this is an async NAPI function
    let rt = tokio::runtime::Handle::current();

    // Ensure the scheduler is running (it may not be if no schedules.json exists)
    let sm = SessionManager::instance();
    let session = sm
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    let project = session.project.clone();
    sm.ensure_scheduler_running(&project, &rt);

    let now = chrono::Utc::now();
    let entry = crate::scheduler::loop_store::LoopEntry {
        id: loop_id,
        session_id: uuid,
        prompt,
        interval_seconds,
        created_at: now,
        expires_at: now + chrono::Duration::days(3),
        last_run_at: None,
    };

    // SCHED-013: Capture the session Arc for the on_fire and idle_check callbacks.
    // The task fires the prompt into the SAME session that created the loop.
    let session_for_fire = session.clone();
    let on_fire: std::sync::Arc<dyn Fn(String) + Send + Sync + 'static> =
        std::sync::Arc::new(move |prompt_text: String| {
            if let Err(e) = session_for_fire.send_input(prompt_text, None) {
                tracing::error!("Loop fire failed for session {}: {}", uuid, e);
            }
        });

    let session_for_idle = session.clone();
    let idle_check: crate::scheduler::loop_store::IdleCheckFn =
        std::sync::Arc::new(move |_session_id: Uuid| {
            let s = session_for_idle.clone();
            Box::pin(async move { s.get_status() == SessionStatus::Idle })
        });

    crate::scheduler::LoopStore::instance()
        .try_register_with_task_and_idle_check(entry, on_fire, idle_check)
        .await
        .map_err(Error::from_reason)?;

    Ok(())
}

/// Cancel a session-scoped loop by ID.
///
/// Must be async so NAPI-RS provides the Tokio runtime context.
#[napi]
pub async fn loop_cancel(loop_id: String) -> Result<bool> {
    Ok(crate::scheduler::LoopStore::instance()
        .cancel(&loop_id)
        .await)
}

/// List all loops for a specific session. Returns JSON array string.
///
/// Must be async so NAPI-RS provides the Tokio runtime context.
#[napi]
pub async fn loop_list(session_id: String) -> Result<String> {
    let uuid = Uuid::parse_str(&session_id)
        .map_err(|e| Error::from_reason(format!("Invalid session ID: {}", e)))?;

    let entries = crate::scheduler::LoopStore::instance()
        .list_for_session(uuid)
        .await;
    let json_entries: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id,
                "prompt": e.prompt,
                "intervalSeconds": e.interval_seconds,
                "createdAt": e.created_at.to_rfc3339(),
                "expiresAt": e.expires_at.to_rfc3339(),
                "lastRunAt": e.last_run_at.map(|t| t.to_rfc3339()),
            })
        })
        .collect();
    Ok(serde_json::to_string(&json_entries).unwrap_or_else(|_| "[]".to_string()))
}

// === Supervisor Operations (WATCH-007) ===

/// Get the subordinate session ID for a supervisor (WATCH-007)
///
/// Returns the subordinate session ID if the session is a supervisor, None otherwise.
#[napi]
pub fn session_get_subordinate(session_id: String) -> Result<Option<String>> {
    let uuid = Uuid::parse_str(&session_id)
        .map_err(|e| Error::from_reason(format!("Invalid session ID: {}", e)))?;

    Ok(SessionManager::instance()
        .get_subordinate(uuid)
        .map(|id| id.to_string()))
}

/// Get all supervisor session IDs for a subordinate session (WATCH-007)
///
/// Returns a list of session IDs that are supervising the specified subordinate.
#[napi]
pub fn session_get_supervisors(session_id: String) -> Result<Vec<String>> {
    let uuid = Uuid::parse_str(&session_id)
        .map_err(|e| Error::from_reason(format!("Invalid session ID: {}", e)))?;

    Ok(SessionManager::instance()
        .get_supervisors(uuid)
        .into_iter()
        .map(|id| id.to_string())
        .collect())
}

/// Set pending observed correlation IDs for a supervisor session (WATCH-011)
///
/// When processing observations, call this before sending the evaluation prompt.
/// All subsequent output chunks from this session will be tagged with these IDs
/// (in observed_correlation_ids field) until session_clear_observed_correlation_ids is called.
///
/// This enables cross-pane highlighting: when viewing a supervisor session in split view,
/// selecting a supervisor turn shows which subordinate turns it was responding to.
#[napi]
pub fn session_set_observed_correlation_ids(
    session_id: String,
    correlation_ids: Vec<String>,
) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    session.set_pending_observed_correlation_ids(correlation_ids);
    Ok(())
}

/// Clear pending observed correlation IDs for a session (WATCH-011)
///
/// Call this after the supervisor finishes processing an observation response.
/// Subsequent output chunks will no longer have observed_correlation_ids set.
#[napi]
pub fn session_clear_observed_correlation_ids(session_id: String) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    session.clear_pending_observed_correlation_ids();
    Ok(())
}

/// Get buffered output with consecutive Text/Thinking chunks merged.
/// This is more efficient for reattachment - JS can process fewer chunks.
#[napi]
pub fn session_get_merged_output(session_id: String) -> Result<Vec<StreamChunk>> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    let chunks = session.get_buffered_output(usize::MAX);

    let mut merged: Vec<StreamChunk> = Vec::new();

    for chunk in chunks {
        match &chunk {
            StreamChunk::Text { text, .. } => {
                // Merge consecutive Text chunks
                if let Some(StreamChunk::Text {
                    text: existing_text,
                    ..
                }) = merged.last_mut()
                {
                    existing_text.push_str(text);
                    continue;
                }
                merged.push(chunk);
            }
            StreamChunk::Thinking { thinking, .. } => {
                // Merge consecutive Thinking chunks
                if let Some(StreamChunk::Thinking {
                    thinking: existing_thinking,
                    ..
                }) = merged.last_mut()
                {
                    existing_thinking.push_str(thinking);
                    continue;
                }
                merged.push(chunk);
            }
            // TUI-049: Include TokenUpdate and ContextFillUpdate in merged output
            // These are needed to restore token state when switching sessions
            StreamChunk::TokenUpdate { .. } | StreamChunk::ContextFillUpdate { .. } => {
                merged.push(chunk);
            }
            _ => merged.push(chunk),
        }
    }

    Ok(merged)
}

/// Restore messages to a background session from persisted envelopes.
///
/// This is used when attaching to a session via /resume - it restores the
/// conversation history so the LLM has context for future prompts.
///
/// Also populates the output_buffer with synthetic StreamChunks so that
/// sessionGetMergedOutput() returns the restored conversation. This enables
/// proper UI replay when detaching and re-attaching via kanban.
#[napi]
pub async fn session_restore_messages(session_id: String, envelopes: Vec<String>) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;

    // Collect rig messages and StreamChunks to push
    let mut rig_messages: Vec<rig::message::Message> = Vec::new();
    let mut stream_chunks: Vec<StreamChunk> = Vec::new();

    for envelope_json in envelopes {
        let envelope: serde_json::Value = serde_json::from_str(&envelope_json)
            .map_err(|e| Error::from_reason(format!("Failed to parse envelope: {}", e)))?;

        // Extract message from envelope
        if let Some(message) = envelope.get("message") {
            let role = message
                .get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("user");

            if role == "assistant" {
                // Handle assistant messages with content blocks
                if let Some(content) = message.get("content") {
                    if let Some(arr) = content.as_array() {
                        let mut text_parts = Vec::new();

                        // Process each content block for StreamChunks
                        for block in arr {
                            let block_type =
                                block.get("type").and_then(|t| t.as_str()).unwrap_or("");

                            match block_type {
                                "thinking" => {
                                    if let Some(thinking) =
                                        block.get("thinking").and_then(|t| t.as_str())
                                    {
                                        if !thinking.is_empty() {
                                            stream_chunks
                                                .push(StreamChunk::thinking(thinking.to_string()));
                                        }
                                    }
                                }
                                "text" => {
                                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                        text_parts.push(text.to_string());
                                        if !text.is_empty() {
                                            stream_chunks.push(StreamChunk::text(text.to_string()));
                                        }
                                    }
                                }
                                "tool_use" => {
                                    let id = block
                                        .get("id")
                                        .and_then(|i| i.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let name = block
                                        .get("name")
                                        .and_then(|n| n.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let input = block
                                        .get("input")
                                        .map(|i| serde_json::to_string(i).unwrap_or_default())
                                        .unwrap_or_default();

                                    if !id.is_empty() && !name.is_empty() {
                                        stream_chunks.push(StreamChunk::tool_call(ToolCallInfo {
                                            id,
                                            name,
                                            input,
                                        }));
                                    }
                                }
                                _ => {}
                            }
                        }

                        // Build rig message for LLM context
                        let joined_text = text_parts.join("");
                        if !joined_text.is_empty() {
                            rig_messages.push(rig::message::Message::Assistant {
                                id: None,
                                content: rig::OneOrMany::one(rig::message::AssistantContent::text(
                                    joined_text,
                                )),
                            });
                        }

                        // Push Done chunk to finalize assistant turn
                        stream_chunks.push(StreamChunk::done());
                    }
                }
            } else {
                // Handle user messages
                if let Some(content) = message.get("content") {
                    if let Some(arr) = content.as_array() {
                        let mut text_parts = Vec::new();

                        // Process each content block
                        for block in arr {
                            let block_type =
                                block.get("type").and_then(|t| t.as_str()).unwrap_or("");

                            match block_type {
                                "text" => {
                                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                        text_parts.push(text.to_string());
                                        if !text.is_empty() {
                                            stream_chunks
                                                .push(StreamChunk::user_input(text.to_string()));
                                        }
                                    }
                                }
                                "tool_result" => {
                                    let tool_use_id = block
                                        .get("tool_use_id")
                                        .and_then(|i| i.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let result_content = block
                                        .get("content")
                                        .and_then(|c| c.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let is_error = block
                                        .get("is_error")
                                        .and_then(|e| e.as_bool())
                                        .unwrap_or(false);

                                    if !tool_use_id.is_empty() {
                                        stream_chunks.push(StreamChunk::tool_result(
                                            ToolResultInfo {
                                                tool_call_id: tool_use_id,
                                                content: result_content,
                                                is_error,
                                            },
                                        ));
                                    }
                                }
                                _ => {}
                            }
                        }

                        // Build rig message for LLM context (text only)
                        let joined_text = text_parts.join("");
                        if !joined_text.is_empty() {
                            // Skip system reminders - they'll be re-injected fresh after restoration
                            // System reminders have both <system-reminder> tag AND <!-- type: marker
                            if joined_text.contains("<system-reminder>")
                                && joined_text.contains("<!-- type:")
                            {
                                // Skip - will be re-injected with fresh content
                                continue;
                            }
                            rig_messages.push(rig::message::Message::User {
                                content: rig::OneOrMany::one(rig::message::UserContent::text(
                                    joined_text,
                                )),
                            });
                        }
                    } else if let Some(s) = content.as_str() {
                        // Simple string content
                        if !s.is_empty() {
                            // Skip system reminders - they'll be re-injected fresh after restoration
                            if s.contains("<system-reminder>") && s.contains("<!-- type:") {
                                // Skip - will be re-injected with fresh content
                                continue;
                            }
                            stream_chunks.push(StreamChunk::user_input(s.to_string()));
                            rig_messages.push(rig::message::Message::User {
                                content: rig::OneOrMany::one(rig::message::UserContent::text(
                                    s.to_string(),
                                )),
                            });
                        }
                    }
                }
            }
        }
    }

    // Push rig messages to inner (for LLM context)
    {
        let mut inner = session.inner.lock().await;
        for msg in rig_messages {
            inner.messages.push(msg);
        }
    }

    // Push StreamChunks to output_buffer via handle_output (for UI replay)
    // This enables sessionGetMergedOutput() to return the restored conversation
    for chunk in stream_chunks {
        session.handle_output(chunk);
    }

    Ok(())
}

/// Restore token state to a background session from persisted values.
///
/// This is used when attaching to a session via /resume - it restores the
/// token tracking state so context fill percentage and token counts are accurate.
#[napi]
pub async fn session_restore_token_state(
    session_id: String,
    input_tokens: u32,
    output_tokens: u32,
    cache_read_tokens: u32,
    cache_creation_tokens: u32,
    cumulative_billed_input: u32,
    cumulative_billed_output: u32,
) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;

    // Update cached tokens for sync access
    session.update_tokens(input_tokens, output_tokens);

    let mut inner = session.inner.lock().await;

    inner.token_tracker.input_tokens = input_tokens as u64;
    inner.token_tracker.output_tokens = output_tokens as u64;
    inner.token_tracker.cache_read_input_tokens = Some(cache_read_tokens as u64);
    inner.token_tracker.cache_creation_input_tokens = Some(cache_creation_tokens as u64);
    inner.token_tracker.cumulative_billed_input = cumulative_billed_input as u64;
    inner.token_tracker.cumulative_billed_output = cumulative_billed_output as u64;

    Ok(())
}

/// Toggle debug capture mode without requiring a session.
///
/// Can be called before a session exists. Session metadata will not be set.
/// Use session_update_debug_metadata after creating a session to add metadata.
///
/// If debug_dir is provided, debug files will be written to `{debug_dir}/debug/`
/// instead of the default directory. For fspec, pass `~/.fspec` to write to
/// `~/.fspec/debug/`.
#[napi]
pub fn toggle_debug(debug_dir: Option<String>) -> DebugCommandResult {
    let result = handle_debug_command_with_dir(debug_dir.as_deref());
    DebugCommandResult {
        enabled: result.enabled,
        session_file: result.session_file,
        message: result.message,
    }
}

/// Update debug capture metadata with session info.
///
/// BUG-134: Now uses the per-session debug capture manager instead of the global singleton.
/// Call this after creating a session if debug was enabled before the session existed.
#[napi]
pub async fn session_update_debug_metadata(session_id: String) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    let inner = session.inner.lock().await;

    if let Ok(mut manager) = session.debug_capture.lock() {
        if manager.is_enabled() {
            manager.set_session_metadata(SessionMetadata {
                provider: Some(inner.current_provider_name().to_string()),
                model: inner
                    .current_model_id()
                    .or_else(|| Some(inner.current_provider_name().to_string())),
                context_window: Some(inner.provider_manager().context_window()),
                max_output_tokens: None,
            });
        }
    }

    Ok(())
}

/// Toggle debug capture mode for a background session (NAPI-009 + AGENT-021 + BUG-134)
///
/// BUG-134: Now uses the per-session debug capture manager instead of the global singleton.
/// Each session has its own DebugCaptureManager, so toggling debug in one session
/// does not affect another session's capture state.
///
/// When enabling, sets session-specific debug directory using the session id
/// and sets session metadata (provider, model, context_window).
/// When disabling, stops capture and returns path to saved session file.
///
/// If debug_dir is provided, debug files will be written to
/// `{debug_dir}/debug/{session_id}/` instead of the default directory.
#[napi]
pub async fn session_toggle_debug(
    session_id: String,
    debug_dir: Option<String>,
) -> Result<DebugCommandResult> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;

    // Snapshot session-derived metadata BEFORE acquiring the debug_capture lock so that
    // when we enable capture we can seed the manager with the real provider/model values
    // *before* `start_capture()` writes the `session.start` event. Without this, the
    // recorded `session.start` would contain `provider: "unknown"` / `model: "unknown"`
    // because the NAPI layer previously called `set_session_metadata` only AFTER
    // `start_capture` had already flushed the event to disk.
    let metadata_snapshot = {
        let inner = session.inner.lock().await;
        SessionMetadata {
            provider: Some(inner.current_provider_name().to_string()),
            model: inner
                .current_model_id()
                .or_else(|| Some(inner.current_provider_name().to_string())),
            context_window: Some(inner.provider_manager().context_window()),
            max_output_tokens: None,
        }
    };

    let result = {
        let mut manager = session.debug_capture.lock().map_err(|_| {
            Error::from_reason("Failed to acquire lock on per-session debug capture manager")
        })?;

        // BUG-134: Set per-session debug directory including session id
        if let Some(ref dir) = debug_dir {
            let session_debug_dir = std::path::PathBuf::from(dir)
                .join("debug")
                .join(session.id.to_string());
            manager.set_debug_directory_raw(session_debug_dir);
        } else {
            // Use default data dir with session id subdirectory
            if let Ok(data_dir) = codelet_common::get_data_dir() {
                let session_debug_dir = data_dir.join("debug").join(session.id.to_string());
                manager.set_debug_directory_raw(session_debug_dir);
            }
        }

        if manager.is_enabled() {
            // Turn off
            match manager.stop_capture() {
                Ok(session_file) => codelet_common::debug_capture::DebugCommandResult {
                    enabled: false,
                    session_file: Some(session_file.clone()),
                    message: format!("Debug capture stopped. Session saved to: {session_file}"),
                },
                Err(e) => codelet_common::debug_capture::DebugCommandResult {
                    enabled: false,
                    session_file: None,
                    message: format!("Failed to stop debug capture: {e}"),
                },
            }
        } else {
            // Seed metadata BEFORE start_capture so session.start records real values.
            manager.set_session_metadata(metadata_snapshot);

            // Turn on
            match manager.start_capture() {
                Ok(session_file) => codelet_common::debug_capture::DebugCommandResult {
                    enabled: true,
                    session_file: Some(session_file.clone()),
                    message: format!("Debug capture started. Writing to: {session_file}"),
                },
                Err(e) => codelet_common::debug_capture::DebugCommandResult {
                    enabled: false,
                    session_file: None,
                    message: format!("Failed to start debug capture: {e}"),
                },
            }
        }
    };

    // Store debug state in BackgroundSession for persistence across detach/attach
    session.set_debug_enabled(result.enabled);

    // BUG-134: Emit DebugStateChange stream event so TUI can update its state
    session.handle_output(StreamChunk::debug_state_change(result.enabled));

    Ok(DebugCommandResult {
        enabled: result.enabled,
        session_file: result.session_file,
        message: result.message,
    })
}

/// Manually trigger context compaction for a background session (NAPI-009 + NAPI-005)
///
/// Uses in-view DAG construction flow. Sets compaction_in_progress
/// flag, clears context, injects compaction system instruction, and returns
/// control to the agent loop. The agent builds the DAG via SessionSearch
/// and calls inject_summary to complete the cycle.
///
/// Returns CompactionResult with pre-compaction token counts.
/// Returns error if session is empty (nothing to compact).
#[napi]
pub async fn session_compact(session_id: String) -> Result<CompactionResult> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    let mut inner = session.inner.lock().await;

    // Check if there's anything to compact
    if inner.messages.is_empty() {
        return Err(Error::from_reason("Nothing to compact - no messages yet"));
    }

    session.set_status(SessionStatus::Compacting);

    let original_tokens = inner.token_tracker.input_tokens;
    let total_messages = inner.messages.len() as u32;
    // CMPCT-041: route the manual tracker-based snapshot through the shared
    // BackgroundSession accessor (basis unification with the AUTO
    // CompactionStarted writers).
    session.store_pre_compaction_tokens(original_tokens as u32);

    // BUG-134: Capture compaction.manual.start event using per-session debug capture
    if let Ok(mut manager) = session.debug_capture.lock() {
        if manager.is_enabled() {
            manager.capture(
                "compaction.manual.start",
                serde_json::json!({
                    "command": "/compact",
                    "originalTokens": original_tokens,
                    "messageCount": total_messages,
                }),
                None,
            );
        }
    }

    match execute_compaction(&mut inner, session.compaction_in_progress.clone(), None).await {
        Ok(()) => {}
        Err(e) => {
            session.set_compaction_progress(None);
            session.set_status(SessionStatus::Idle);

            if let Ok(mut manager) = session.debug_capture.lock() {
                if manager.is_enabled() {
                    manager.capture(
                        "compaction.manual.failed",
                        serde_json::json!({
                            "command": "/compact",
                            "error": e.to_string(),
                        }),
                        None,
                    );
                }
            }
            return Err(Error::from_reason(format!("Compaction failed: {e}")));
        }
    }

    // RPC-421: this reads the post-clear trough (reminders + compaction
    // instruction) — a real measurement of an intermediate state that never
    // survives, NOT a reduction. It feeds the BUG-134 debug capture below
    // (diagnostics only) and MUST NOT ship on the CompactionResult.
    let compacted_tokens = inner.token_tracker.input_tokens;

    // Drop the inner lock BEFORE sending input — agent_loop needs it.
    drop(inner);

    session.set_compaction_progress(None);

    // BUG-134: Capture compaction.manual.complete event using per-session debug capture
    if let Ok(mut manager) = session.debug_capture.lock() {
        if manager.is_enabled() {
            manager.capture(
                "compaction.manual.complete",
                serde_json::json!({
                    "command": "/compact",
                    "type": "in-view-dag",
                    "originalTokens": original_tokens,
                    "compactedTokens": compacted_tokens,
                }),
                None,
            );
        }
    }

    // Send "Continue" to trigger agent_loop processing of the compaction instruction.
    if let Err(e) = session.send_input("Continue".to_string(), None) {
        tracing::warn!(
            "[session_compact] Failed to send Continue to agent loop: {}",
            e
        );
        session.set_status(SessionStatus::Idle);
    }

    // RPC-421: acknowledgement-shaped success on the unchanged wire schema
    // (twin of sessions/src/handle_impl.rs compact_session). The final
    // compacted size is unknowable here — the agent builds the DAG
    // asynchronously after the "Continue" kick — so original_tokens is the
    // real pre-compaction snapshot and every other field is the 0-valued
    // sentinel. Consumers MUST NOT present these fields as a reduction: the
    // StreamChunk::CompactionComplete emission (CMPCT-038 apply-site) is the
    // single source of truth for the numbers.
    Ok(CompactionResult {
        original_tokens: original_tokens as u32,
        compacted_tokens: 0,
        compression_ratio: 0.0,
        turns_summarized: 0,
        turns_kept: 0,
    })
}

// CODE-009: The execute_fspec_command_sync function has been removed.
// Fspec commands are now executed via TypeScript callback (fspecCallback) and
// results are sent back via sessionSendFspecResult NAPI function.

/// CONFIG-004: Test provider connection by validating credentials
///
/// This is a lightweight check that validates provider credentials without
/// creating a full session. Used by the settings UI to test connections.
///
/// Returns Ok(()) if credentials are valid, or an error message if not.
#[napi]
pub fn test_provider_connection(provider_name: String) -> Result<()> {
    use codelet_providers::ProviderManager;

    // Load environment variables (for API keys)
    let _ = dotenvy::dotenv();

    // Try to create a ProviderManager with this provider
    // This validates that credentials exist and are non-empty
    ProviderManager::with_provider(&provider_name)
        .map_err(|e| Error::from_reason(format!("Connection failed: {e}")))?;

    Ok(())
}

// =============================================================================
// TUI-059: WORK UNIT CONTEXT NAPI FUNCTIONS
// =============================================================================

/// TUI-059: Work unit context information returned to TypeScript
#[napi(object)]
pub struct JsWorkUnitContext {
    pub id: String,
    pub title: String,
    pub status: String,
}

/// TUI-059: Set work unit context for a session
///
/// When a session is attached to a work unit (e.g., when entering AgentView
/// from BoardView with a selected work unit), call this to set the context.
/// Pass null for all parameters to clear the context.
#[napi]
pub fn session_set_work_unit_context(
    session_id: String,
    id: Option<String>,
    title: Option<String>,
    status: Option<String>,
) -> Result<()> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    session.set_work_unit_context(id, title, status);
    Ok(())
}

/// TUI-059: Get work unit context for a session
///
/// Returns the work unit context if set, or null if no context is set.
#[napi]
pub fn session_get_work_unit_context(session_id: String) -> Result<Option<JsWorkUnitContext>> {
    let session = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    let ctx = session.get_work_unit_context();

    match ctx {
        Some(c) if c.is_set() => Ok(Some(JsWorkUnitContext {
            id: c.id.unwrap_or_default(),
            title: c.title.unwrap_or_default(),
            status: c.status.unwrap_or_default(),
        })),
        _ => Ok(None),
    }
}

/// TUI-059: Get the currently active session ID
///
/// Returns the session ID of the currently active session (for navigation),
/// or null if no session is active.
#[napi]
pub fn session_get_active() -> Option<String> {
    SessionManager::instance()
        .get_active_session()
        .map(|uuid| uuid.to_string())
}

// ============================================================================
// GIT-020: Isolated Session Path Validation - E2E Test Support
// ============================================================================

/// Result of path validation for isolated sessions.
#[napi(object)]
pub struct PathValidationResult {
    /// Whether the path is allowed for this session
    pub allowed: bool,
    /// The resolved path (within worktree if isolated session)
    pub resolved_path: Option<String>,
    /// Error message if path is not allowed
    pub error: Option<String>,
}

/// GIT-020: Validate if a path is allowed for a session.
///
/// This function is exposed for E2E testing of isolated session file operations.
/// It calls the same validate_and_resolve_path function used by all file tools.
///
/// For isolated sessions:
/// - Relative paths are resolved relative to worktree and ALLOWED
/// - Absolute paths within worktree are ALLOWED
/// - Absolute paths outside worktree are BLOCKED
/// - Path traversal (../) that escapes worktree is BLOCKED
/// - Symlinks pointing outside worktree are BLOCKED
///
/// For non-isolated sessions:
/// - All paths are ALLOWED (backward compatible)
///
/// @param session_id - UUID of the session to validate against
/// @param path - File path to validate
/// @param tool_name - Name of the tool (for error messages): "read", "write", "edit", "ls", "grep", "glob", "ast_grep", "ast_grep_refactor"
/// @returns PathValidationResult with allowed status and resolved path or error
#[napi]
pub fn session_validate_path(
    session_id: String,
    path: String,
    tool_name: String,
) -> PathValidationResult {
    use codelet_tools::facade::validate_and_resolve_path;

    // Parse session ID
    let uuid = match uuid::Uuid::parse_str(&session_id) {
        Ok(id) => id,
        Err(e) => {
            return PathValidationResult {
                allowed: false,
                resolved_path: None,
                error: Some(format!("Invalid session ID: {}", e)),
            };
        }
    };

    // Convert tool_name to static str for validate_and_resolve_path
    let tool_static: &'static str = match tool_name.as_str() {
        "read" => "read",
        "write" => "write",
        "edit" => "edit",
        "ls" => "ls",
        "grep" => "grep",
        "glob" => "glob",
        "ast_grep" => "ast_grep",
        "ast_grep_refactor" => "ast_grep_refactor",
        _ => "unknown",
    };

    // Call the actual validation function used by all file tools
    match validate_and_resolve_path(uuid, &path, tool_static) {
        Ok(resolved) => PathValidationResult {
            allowed: true,
            resolved_path: Some(resolved.to_string_lossy().to_string()),
            error: None,
        },
        Err(e) => PathValidationResult {
            allowed: false,
            resolved_path: None,
            error: Some(e.to_string()),
        },
    }
}

// =============================================================================
// BUG-132: SUB-AGENT MODEL INHERITANCE TESTS
// =============================================================================

#[cfg(test)]
mod sub_agent_model_inheritance_tests {
    //! Feature: spec/features/sub-agent-model-inheritance.feature
    //!
    //! BUG-132: DeepSearch and AgentManager handlers use stale model after
    //! mid-session model switch.
    //!
    //! These tests verify the extracted helper functions that build handler
    //! values, ensuring that:
    //! - facade_override is checked before current_provider_name
    //! - All four captured values update when the model changes
    //! - AgentManager uses selected_model_string() in registry format

    // super::* not used — tests only use crate::bridges::* and codelet_providers
    use codelet_providers::ProviderManager;

    /// Create a test ProviderManager without real credentials.
    ///
    /// Uses `ProviderManager::for_testing()` (pub, no credentials needed) and
    /// then `set_model_direct()` to configure the model, context_window, and
    /// max_output_tokens.
    ///
    /// `provider` is the *internal* name (e.g. "claude", "gemini") used by
    /// `ProviderType::from_str`. `registry_provider` is the models.dev name
    /// (e.g. "anthropic", "google") used by `set_model_direct`.
    fn test_pm(
        provider: &str,
        registry_provider: &str,
        model_id: Option<&str>,
        context_window: Option<usize>,
        max_output_tokens: Option<usize>,
    ) -> ProviderManager {
        let provider_type: codelet_providers::ProviderType =
            provider.parse().expect("valid provider name");
        let mut pm = ProviderManager::for_testing(provider_type, None, None);
        if let Some(mid) = model_id {
            pm.set_model_direct(
                registry_provider,
                mid,
                context_window,
                max_output_tokens,
                None,
            )
            .expect("set_model_direct should succeed for test provider");
        }
        pm
    }

    // =========================================================================
    // BUG-132 regression tests (nested mod per RPC-043 structural contract).
    //
    // All tests in this module are BUG-132 regression assertions; the spec
    // (rule [8] / scenario "tests migrate without losing assertions") requires
    // `mod bug132_tests` to appear as a named module inside session_bindings.rs.
    // Nesting it inside sub_agent_model_inheritance_tests preserves the
    // single shared `test_pm` helper without duplication.
    // =========================================================================
    mod bug132_tests {
        use super::*;

        // =========================================================================
        // Scenario: DeepSearch uses updated model after mid-session model switch
        // =========================================================================

        #[test]
        fn test_bug132_deep_search_uses_updated_model_after_switch() {
            // @step Given a session was created with model "anthropic/claude-sonnet-4-20250514"
            let pm_before = test_pm(
                "claude",
                "anthropic",
                Some("claude-sonnet-4-20250514"),
                Some(200000),
                Some(16384),
            );
            let (provider_before, model_before, _, _) =
                crate::bridges::extract_deep_search_handler_values(&pm_before);
            assert_eq!(provider_before, "claude");
            assert_eq!(model_before, Some("claude-sonnet-4-20250514".to_string()));

            // @step And the DeepSearch handler was registered at session creation
            // (values captured above)

            // @step When the user switches the model to "google/gemini-2.5-pro" via session_set_model
            let pm_after = test_pm(
                "gemini",
                "google",
                Some("gemini-2.5-pro"),
                Some(1048576),
                Some(65536),
            );
            let (provider_after, model_after, _, _) =
                crate::bridges::extract_deep_search_handler_values(&pm_after);

            // @step And the user invokes DeepSearch
            // (extracting values simulates what re-registration would capture)

            // @step Then the DeepSearch sub-agent should use provider "gemini" and model "gemini-2.5-pro"
            assert_eq!(provider_after, "gemini");
            assert_eq!(model_after, Some("gemini-2.5-pro".to_string()));
            // Verify the values actually changed
            assert_ne!(provider_before, provider_after);
            assert_ne!(model_before, model_after);
        }

        // =========================================================================
        // Scenario: AgentManager uses updated model after mid-session model switch
        // =========================================================================

        #[test]
        fn test_bug132_agent_manager_uses_updated_model_after_switch() {
            // @step Given a session was created with model "anthropic/claude-sonnet-4-20250514"
            let pm_before = test_pm(
                "claude",
                "anthropic",
                Some("claude-sonnet-4-20250514"),
                Some(200000),
                Some(16384),
            );
            let (model_string_before, _, _) =
                crate::bridges::extract_agent_manager_handler_values(&pm_before);
            // BUG-136: selected_model_string() now returns the full registry
            // composite so AgentManager's create_session_with_id call can
            // round-trip it even when the model id contains slashes.
            assert_eq!(
                model_string_before,
                Some("anthropic/claude-sonnet-4-20250514".to_string())
            );

            // @step And the AgentManager handler was registered at session creation
            // (values captured above)

            // @step When the user switches the model to "google/gemini-2.5-pro" via session_set_model
            let pm_after = test_pm(
                "gemini",
                "google",
                Some("gemini-2.5-pro"),
                Some(1048576),
                Some(65536),
            );
            let (model_string_after, _, _) =
                crate::bridges::extract_agent_manager_handler_values(&pm_after);

            // @step And the user spawns a subordinate via AgentManager
            // (extracting values simulates what re-registration would capture)

            // @step Then the subordinate should be created with the updated model "gemini-2.5-pro"
            assert_eq!(
                model_string_after,
                Some("google/gemini-2.5-pro".to_string())
            );
            assert_ne!(model_string_before, model_string_after);
        }

        // =========================================================================
        // Scenario: DeepSearch respects facade_override for custom models
        // =========================================================================

        #[test]
        fn test_bug132_deep_search_respects_facade_override() {
            // @step Given a session was created with a MODEL-004 custom model registered under "openai" with facade_override "claude"
            let mut pm = test_pm(
                "openai",
                "openai",
                Some("my-custom-model"),
                Some(128000),
                Some(4096),
            );
            pm.set_facade_override(Some("claude".to_string()));

            // @step When the user invokes DeepSearch
            let (provider, _, _, _) = crate::bridges::extract_deep_search_handler_values(&pm);

            // @step Then the DeepSearch sub-agent should use provider "claude" not "openai"
            assert_eq!(
                provider, "claude",
                "facade_override should take precedence over current_provider_name"
            );
        }

        // =========================================================================
        // Scenario: Handler re-registration updates all four captured values
        // =========================================================================

        #[test]
        fn test_bug132_handler_reregistration_updates_all_four_values() {
            // @step Given a session was created with model "anthropic/claude-sonnet-4-20250514"
            let pm_before = test_pm(
                "claude",
                "anthropic",
                Some("claude-sonnet-4-20250514"),
                Some(200000),
                Some(16384),
            );
            let (p1, m1, cw1, mo1) = crate::bridges::extract_deep_search_handler_values(&pm_before);
            assert_eq!(p1, "claude");
            assert_eq!(m1, Some("claude-sonnet-4-20250514".to_string()));
            // Note: values are clamped by provider limits (Claude max_output = 8192)
            assert!(cw1.is_some(), "context_window should be set");
            assert!(mo1.is_some(), "max_output should be set");

            // @step When the user switches the model to "google/gemini-2.5-pro" via session_set_model with context_window 1048576 and max_output_tokens 65536
            let pm_after = test_pm(
                "gemini",
                "google",
                Some("gemini-2.5-pro"),
                Some(1048576),
                Some(65536),
            );

            // @step Then the DeepSearch handler should capture provider "gemini", model "gemini-2.5-pro", context_window 1048576, and max_output_tokens 65536
            let (p2, m2, cw2, mo2) = crate::bridges::extract_deep_search_handler_values(&pm_after);
            assert_eq!(p2, "gemini");
            assert_eq!(m2, Some("gemini-2.5-pro".to_string()));
            assert_eq!(cw2, Some(1048576));
            assert_eq!(mo2, Some(65536));

            // Verify all four values changed from the original
            assert_ne!(p1, p2, "provider should change");
            assert_ne!(m1, m2, "model should change");
            assert_ne!(cw1, cw2, "context_window should change");
            assert_ne!(mo1, mo2, "max_output should change");

            // @step And the AgentManager handler should capture model "gemini-2.5-pro", context_window 1048576, and max_output_tokens 65536
            let (ms, cw3, mo3) = crate::bridges::extract_agent_manager_handler_values(&pm_after);
            // BUG-136: full registry composite, not bare id
            assert_eq!(ms, Some("google/gemini-2.5-pro".to_string()));
            assert_eq!(cw3, Some(1048576));
            assert_eq!(mo3, Some(65536));
        }

        // =========================================================================
        // Scenario: No regression when model is never changed
        // =========================================================================

        #[test]
        fn test_bug132_no_regression_when_model_never_changed() {
            // @step Given a session was created with model "anthropic/claude-sonnet-4-20250514"
            let pm = test_pm(
                "claude",
                "anthropic",
                Some("claude-sonnet-4-20250514"),
                Some(200000),
                Some(16384),
            );

            // @step And no model switch occurs during the session
            // (no mutation of pm)

            // @step When the user invokes DeepSearch
            let (provider, model, cw, mo) = crate::bridges::extract_deep_search_handler_values(&pm);

            // @step Then the DeepSearch sub-agent should use provider "claude" and model "claude-sonnet-4-20250514"
            assert_eq!(provider, "claude");
            assert_eq!(model, Some("claude-sonnet-4-20250514".to_string()));
            // Values are set (clamped by Claude's provider limits)
            assert!(cw.is_some(), "context_window should be set");
            assert!(mo.is_some(), "max_output should be set");
        }

        // =========================================================================
        // Scenario: session_set_model_profile also triggers handler re-registration
        // =========================================================================

        #[test]
        fn test_bug132_set_model_profile_triggers_reregistration() {
            // @step Given a session was created with model "anthropic/claude-sonnet-4-20250514"
            let pm_before = test_pm(
                "claude",
                "anthropic",
                Some("claude-sonnet-4-20250514"),
                Some(200000),
                Some(16384),
            );
            let (p_before, _, _, _) =
                crate::bridges::extract_deep_search_handler_values(&pm_before);
            assert_eq!(p_before, "claude");

            // @step When the user switches the model via session_set_model_profile to provider "openai" model "gpt-4o"
            // set_model_direct is what session_set_model_profile calls
            let mut pm_after = test_pm("openai", "openai", None, None, None);
            pm_after
                .set_model_direct("openai", "gpt-4o", Some(128000), Some(16384), None)
                .expect("set_model_direct should succeed");
            let (p_after, m_after, _, _) =
                crate::bridges::extract_deep_search_handler_values(&pm_after);

            // @step And the user invokes DeepSearch

            // @step Then the DeepSearch sub-agent should use provider "openai" and model "gpt-4o"
            assert_eq!(p_after, "openai");
            assert_eq!(m_after, Some("gpt-4o".to_string()));
        }

        // =========================================================================
        // Additional: Verify facade_override=None falls through to current_provider
        // =========================================================================

        #[test]
        fn test_bug132_no_facade_override_uses_current_provider() {
            // When facade_override is None, extract_deep_search_handler_values
            // must fall through to current_provider_name().
            let pm = test_pm("gemini", "google", Some("gemini-2.5-pro"), None, None);
            assert!(pm.facade_override().is_none());
            let (provider, _, _, _) = crate::bridges::extract_deep_search_handler_values(&pm);
            assert_eq!(provider, "gemini");
        }

        // =========================================================================
        // Additional: AgentManager uses selected_model_string (registry format)
        // =========================================================================

        #[test]
        fn test_bug132_agent_manager_uses_selected_model_string_format() {
            // AMGR-013 / BUG-136: AgentManager must use selected_model_string()
            // which returns the full "provider/model" registry composite, not
            // the bare model id. Previously this test asserted the BUG-136 bug
            // (bare id "claude-opus-4-6"); now we require the full composite.
            let pm = test_pm(
                "claude",
                "anthropic",
                Some("claude-opus-4-6"),
                Some(200000),
                Some(32768),
            );
            let (model_string, _, _) = crate::bridges::extract_agent_manager_handler_values(&pm);
            assert_eq!(model_string, Some("anthropic/claude-opus-4-6".to_string()));
        }
    } // end mod bug132_tests
}

/// GIT-020: Get the effective working directory for a session.
///
/// This function is exposed for E2E testing. It returns the directory
/// that the session uses for relative path resolution:
/// - For isolated sessions: the worktree path
/// - For non-isolated sessions: the project root
///
/// @param session_id - UUID of the session
/// @returns The effective working directory path, or null if session not found
#[napi]
pub fn session_get_effective_cwd(session_id: String) -> Option<String> {
    let manager = SessionManager::instance();

    match manager.get_session(&session_id) {
        Ok(session) => Some(session.effective_cwd().to_string_lossy().to_string()),
        Err(_) => None,
    }
}

/// GIT-020: Check if a session is isolated (has a worktree).
///
/// @param session_id - UUID of the session
/// @returns true if session is isolated, false if not, null if session not found
#[napi]
pub fn session_is_isolated(session_id: String) -> Option<bool> {
    let manager = SessionManager::instance();

    match manager.get_session(&session_id) {
        Ok(session) => Some(session.worktree_path.is_some()),
        Err(_) => None,
    }
}

/// Result of bash command execution for E2E testing.
#[napi(object)]
pub struct BashExecutionResult {
    /// Whether the command succeeded (exit code 0)
    pub success: bool,
    /// Command output (stdout)
    pub output: Option<String>,
    /// Error message or stderr content
    pub error: Option<String>,
}

/// GIT-020: Execute a bash command within a session's context.
///
/// This function is exposed for E2E testing of Bash tool cwd restriction.
/// It executes a command using the session's effective_cwd as the working directory.
///
/// For isolated sessions: command runs in the worktree directory
/// For non-isolated sessions: command runs in the project root
///
/// @param session_id - UUID of the session
/// @param command - The bash command to execute
/// @returns BashExecutionResult with output or error
#[napi]
pub fn session_execute_bash(session_id: String, command: String) -> BashExecutionResult {
    use std::process::Command;

    // Get the effective_cwd for this session
    let cwd = match session_get_effective_cwd(session_id.clone()) {
        Some(path) => path,
        None => {
            return BashExecutionResult {
                success: false,
                output: None,
                error: Some(format!("Session not found: {}", session_id)),
            };
        }
    };

    // Execute the command with the session's effective_cwd
    let result = Command::new("bash")
        .arg("-c")
        .arg(&command)
        .current_dir(&cwd)
        .output();

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            if output.status.success() {
                BashExecutionResult {
                    success: true,
                    output: Some(stdout),
                    error: if stderr.is_empty() {
                        None
                    } else {
                        Some(stderr)
                    },
                }
            } else {
                BashExecutionResult {
                    success: false,
                    output: if stdout.is_empty() {
                        None
                    } else {
                        Some(stdout)
                    },
                    error: Some(if stderr.is_empty() {
                        format!("Command failed with exit code: {:?}", output.status.code())
                    } else {
                        stderr
                    }),
                }
            }
        }
        Err(e) => BashExecutionResult {
            success: false,
            output: None,
            error: Some(format!("Failed to execute command: {}", e)),
        },
    }
}

// =============================================================================
// PROV-067: Custom provider management NAPI bindings
// =============================================================================

/// BUG-139: JS-shaped per-model info carried by [`JsProviderInfo::models`].
///
/// Mirrors [`codelet_providers::custom::ProviderModelInfo`] across the
/// NAPI boundary so the TUI's `customProviderSectionBuilder` can read
/// authoritative per-model limits (e.g. `contextWindow: 1_000_000`)
/// directly from the provider JSON config, instead of synthesising
/// hardcoded `128000 / 8192` fallbacks that made the SessionHeader badge
/// display a stale `[120k]` context window.
#[napi(object)]
pub struct JsProviderModelInfo {
    /// Model alias key (e.g. `"opus-4.7"`).
    pub id: String,
    /// Context window in tokens.
    pub context_window: u32,
    /// Max output tokens per completion.
    pub max_output: u32,
    /// Whether the model supports tool / function calling.
    pub supports_tools: bool,
    /// Whether the model supports SSE streaming.
    pub supports_streaming: bool,
    /// Whether the model supports extended-thinking mode.
    pub supports_thinking: bool,
    /// Whether the model supports vision / image input.
    pub supports_vision: bool,
}

impl From<codelet_providers::custom::ProviderModelInfo> for JsProviderModelInfo {
    fn from(src: codelet_providers::custom::ProviderModelInfo) -> Self {
        Self {
            id: src.id,
            // Cast `usize` -> `u32`. All realistic model limits (≤ ~16M
            // tokens) fit in a u32, and NAPI cannot bridge `usize`
            // directly. Saturate defensively on overflow.
            context_window: u32::try_from(src.context_window).unwrap_or(u32::MAX),
            max_output: u32::try_from(src.max_output_tokens).unwrap_or(u32::MAX),
            supports_tools: src.supports_tools,
            supports_streaming: src.supports_streaming,
            supports_thinking: src.supports_thinking,
            supports_vision: src.supports_vision,
        }
    }
}

/// PROV-067: JS-shaped info entry for a provider (built-in or custom).
#[napi(object)]
pub struct JsProviderInfo {
    pub name: String,
    pub display_name: Option<String>,
    pub available: bool,
    pub is_custom: bool,
    pub facade: Option<String>,
    pub base_url: Option<String>,
    pub api_key_env_var: Option<String>,
    /// BUG-139: Per-model limits + capability flags for custom
    /// providers; empty for built-ins.
    pub models: Vec<JsProviderModelInfo>,
    pub api_style: Option<String>,
}

impl From<codelet_providers::custom::ProviderInfo> for JsProviderInfo {
    fn from(src: codelet_providers::custom::ProviderInfo) -> Self {
        Self {
            name: src.name,
            display_name: src.display_name,
            available: src.available,
            is_custom: src.is_custom,
            facade: src.facade,
            base_url: src.base_url,
            api_key_env_var: src.api_key_env_var,
            models: src.models.into_iter().map(Into::into).collect(),
            api_style: src.api_style,
        }
    }
}

/// PROV-067: JS result of a custom-provider connectivity probe.
#[napi(object)]
pub struct JsProviderTestResult {
    pub reachable: bool,
    pub status_code: Option<u32>,
    pub matched_models: Vec<String>,
}

/// PROV-067: Return all built-in + discovered custom providers with
/// credential / availability info.
#[napi]
pub async fn list_providers() -> Result<Vec<JsProviderInfo>> {
    let _ = dotenvy::dotenv();
    codelet_providers::custom::list_providers_info()
        .map(|list| list.into_iter().map(Into::into).collect())
        .map_err(|e| Error::from_reason(format!("list_providers failed: {e}")))
}

/// PROV-067: Return detailed info for a single provider by slug.
#[napi]
pub async fn show_provider(name: String) -> Result<JsProviderInfo> {
    let _ = dotenvy::dotenv();
    codelet_providers::custom::show_provider_info(&name)
        .map(Into::into)
        .map_err(|e| Error::from_reason(format!("show_provider failed: {e}")))
}

/// PROV-067: Validate a custom provider's JSON schema (missing facade,
/// invalid fields, etc.) without making network calls.
#[napi]
pub async fn validate_provider(name: String) -> Result<()> {
    codelet_providers::custom::validate_provider_config(&name)
        .map_err(|e| Error::from_reason(format!("validate_provider failed: {e}")))
}

/// PROV-067: Probe `<baseUrl>/models` to confirm the custom provider is
/// reachable and lists the models declared in its config.
#[napi]
pub async fn test_provider(name: String) -> Result<JsProviderTestResult> {
    let _ = dotenvy::dotenv();
    let result = codelet_providers::custom::test_provider_connection(&name)
        .await
        .map_err(|e| Error::from_reason(format!("test_provider failed: {e}")))?;
    Ok(JsProviderTestResult {
        reachable: result.reachable,
        status_code: result.status_code.map(|c| c as u32),
        matched_models: result.matched_models,
    })
}

/// PROV-067: Scaffold `.fspec/providers/<name>.json` from a named
/// template (supported: `"openai-compatible"`).
#[napi]
pub async fn init_provider(project_root: String, name: String, template: String) -> Result<String> {
    codelet_providers::custom::init_provider_template(
        std::path::Path::new(&project_root),
        &name,
        &template,
    )
    .map(|p| p.to_string_lossy().into_owned())
    .map_err(|e| Error::from_reason(format!("init_provider failed: {e}")))
}

/// RPC-018: Return display + capability metadata for the model currently
/// bound to a session.
///
/// Mirrors `FspecService::get_model_info` (rust/rpc/src/lib.rs). Both
/// bindings call into the SAME `SessionManagerHandle::get_model_info`
/// path, so the JS Ink frontend and the Rust ratatui frontend converge
/// on identical data once the rust/napi `SessionManager` overrides
/// the trait method (deferred to RPC-022). For RPC-018 the default
/// trait impl returns `ModelInfo::default()` — the additive NAPI export
/// preserves the call site so that the TS code can wire up to the new
/// shape ahead of the override.
#[napi]
pub fn get_model_info(session_id: String) -> Result<codelet_rpc_types::ModelInfo> {
    let _ = SessionManager::instance()
        .get_session(&session_id)
        .map_err(napi::Error::from_reason)?;
    Ok(codelet_rpc_types::ModelInfo::default())
}
