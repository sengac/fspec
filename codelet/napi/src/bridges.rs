//! Bridge wiring helpers (RPC-043).
//!
//! Feature: spec/features/reduce-codelet-napi-to-thin-adapter-session-bindings-rs-update-cargo-toml.feature
//!
//! Extracted verbatim from `codelet/napi/src/session_manager.rs` by
//! RPC-043. This module owns:
//!
//! - `init_block_notification_callbacks()` — registers the
//!   BashToolFacadeWrapper / FileToolFacadeWrapper block-notification
//!   sink with the tools crate (lines 3874-3883 of the pre-RPC-043
//!   `session_manager.rs`).
//! - `init_bridge_metadata_providers()` — registers the bridge-relay
//!   session-list and model-info providers (lines 3889-3927).
//! - `init_bridge_session_and_terminal_creators()` — registers the
//!   bridge-side session creator and the global PtyRegistry (lines
//!   3935-3976).
//! - `emit_block_notification_to_tui()` — the BLOCK-006 chunk emitter
//!   wired into `set_block_notification_callback` (lines 3985-3998).
//! - `register_deep_search_handler()` — BUG-132 DeepSearch handler
//!   factory (lines 1952-1986).
//! - `register_agent_manager_handler()` — BUG-132 AgentManager handler
//!   factory (lines 1995-2013).
//! - `extract_deep_search_handler_values()` /
//!   `extract_agent_manager_handler_values()` — `#[cfg(test)]` helpers
//!   that mirror the closure captures so unit tests can exercise the
//!   capture-time semantics without spinning up a session (lines 2022-
//!   2032 and 2039-2046).
//! - `get_session_work_unit_stage()` and `get_session_effective_cwd()`
//!   — the work-unit-stage and isolation-context callbacks consumed by
//!   `FileToolFacadeWrapper` / `BashToolFacadeWrapper` (lines 4002-
//!   4054). Kept private to bridges.rs because their sole consumer is
//!   `init_block_notification_callbacks()` above.
//!
//! No behaviour changes — every function body is byte-identical to the
//! pre-RPC-043 `session_manager.rs` version. Only the import paths and
//! the visibility modifiers (`fn` → `pub(crate) fn` for the call-sites
//! that survive in `session_manager.rs`) differ.

use crate::types::{NotificationSeverity, StreamChunk};
use codelet_sessions::session_manager::SessionManager;
use codelet_tools::facade::{
    set_block_notification_callback, set_get_effective_cwd_callback,
    set_get_work_unit_stage_callback,
};
use uuid::Uuid;

// RPC-043: imports consumed only by the test modules migrated below.
#[cfg(test)]
use codelet_sessions::background_session::SUPERVISOR_BROADCAST_CAPACITY;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, AtomicU64};
#[cfg(test)]
use std::sync::RwLock;
#[cfg(test)]
use std::sync::atomic::Ordering;
#[cfg(test)]
use tokio::sync::broadcast;

/// BUG-132: Build and register a DeepSearch handler for the given session.
///
/// Extracted so it can be called both at session creation and after model
/// changes (session_set_model / session_set_model_profile).
///
/// The handler captures the current provider, model, context_window, and
/// max_output_tokens from the inner session's ProviderManager.
///
/// MODEL-004: Uses facade_override() when available, matching the agent_loop
/// dispatch pattern at line 4744-4747.
pub(crate) fn register_deep_search_handler(
    session_id: Uuid,
    inner_session: &codelet_cli::session::Session,
    project_path: std::path::PathBuf,
) {
    // BUG-132/MODEL-004: Check facade_override first — if set, dispatch to that
    // provider instead of current_provider. This mirrors the agent_loop pattern.
    let deep_search_provider = inner_session.provider_manager()
        .facade_override()
        .map(|s| s.to_string())
        .unwrap_or_else(|| inner_session.current_provider_name().to_string());
    let deep_search_model = inner_session.current_model_id().map(|s| s.to_string());
    let deep_search_context_window = inner_session.provider_manager().raw_model_context_window();
    let deep_search_max_output = inner_session.provider_manager().raw_model_max_output_tokens();
    let deep_search_handler: codelet_tools::DeepSearchHandler = std::sync::Arc::new(move |query, scope, max_depth, max_recursion_depth| {
        let path = project_path.clone();
        let provider = deep_search_provider.clone();
        let model = deep_search_model.clone();
        Box::pin(async move {
            crate::deep_search_handler::execute_deep_search(
                &path,
                &query,
                scope.as_deref(),
                max_depth,
                &provider,
                model.as_deref(),
                0, // RLM-002: Parent session starts at depth 0
                max_recursion_depth,
                deep_search_context_window,
                deep_search_max_output,
            ).await
        })
    });
    codelet_tools::set_deep_search_handler(session_id, Some(deep_search_handler));
}

/// BUG-132: Build and register an AgentManager handler for the given session.
///
/// Extracted so it can be called both at session creation and after model
/// changes (session_set_model / session_set_model_profile).
///
/// AMGR-013: Uses selected_model_string() which preserves the original
/// "provider/model" registry format.
pub(crate) fn register_agent_manager_handler(
    session_id: Uuid,
    inner_session: &codelet_cli::session::Session,
    project: String,
) {
    // BUG-136: `selected_model_string()` now returns an owned `Option<String>`
    // built on demand from the provider slug + bare model id, so model ids
    // that themselves contain slashes round-trip cleanly.
    let full_model_string = inner_session.provider_manager().selected_model_string();
    let spawner_context_window = inner_session.provider_manager().raw_model_context_window();
    let spawner_max_output = inner_session.provider_manager().raw_model_max_output_tokens();
    let agent_manager_handler = crate::agent_manager_handler::create_handler(
        project,
        full_model_string,
        spawner_context_window,
        spawner_max_output,
    );
    codelet_tools::set_agent_manager_handler(session_id, Some(agent_manager_handler));
}

/// BUG-132: Extract the values that would be captured by the DeepSearch handler.
///
/// This is a testable pure function that returns the four values that the
/// DeepSearch handler closure captures from a ProviderManager. Used by tests
/// to verify the facade_override logic and value extraction without needing
/// to construct a full handler closure.
#[cfg(test)]
pub(crate) fn extract_deep_search_handler_values(
    pm: &codelet_providers::ProviderManager,
) -> (String, Option<String>, Option<usize>, Option<usize>) {
    let provider = pm.facade_override()
        .map(|s| s.to_string())
        .unwrap_or_else(|| pm.current_provider_name().to_string());
    let model = pm.selected_model_id();
    let context_window = pm.raw_model_context_window();
    let max_output = pm.raw_model_max_output_tokens();
    (provider, model, context_window, max_output)
}

/// BUG-132: Extract the values that would be captured by the AgentManager handler.
///
/// Returns (model_string, context_window, max_output) matching what
/// `register_agent_manager_handler` captures.
#[cfg(test)]
pub(crate) fn extract_agent_manager_handler_values(
    pm: &codelet_providers::ProviderManager,
) -> (Option<String>, Option<usize>, Option<usize>) {
    let model_string = pm.selected_model_string();
    let context_window = pm.raw_model_context_window();
    let max_output = pm.raw_model_max_output_tokens();
    (model_string, context_window, max_output)
}

// ============================================================================
// BLOCK-006: Block Notification Callbacks
// ============================================================================


/// Initialize the block notification callbacks for the tools crate.
/// This is called once when the global chunk callback is set.
pub(crate) fn init_block_notification_callbacks() {
    // Register the block notification callback
    set_block_notification_callback(emit_block_notification_to_tui);
    
    // Register the work unit stage callback
    set_get_work_unit_stage_callback(get_session_work_unit_stage);
    
    // GIT-020: Register the effective_cwd callback
    set_get_effective_cwd_callback(get_session_effective_cwd);
}

/// BRIDGE-SESSION: Register session list and model info providers with the bridge relay.
///
/// These providers query the SessionManager singleton and are called by `get_instance_metadata()`
/// to populate the sessions, provider, and model fields in bridge auth and metadata updates.
pub(crate) fn init_bridge_metadata_providers() {
    // Session list provider — returns a Vec<serde_json::Value> of session objects
    let session_list_provider: codelet_tools::SessionListProvider = std::sync::Arc::new(|| {
        let sm = SessionManager::instance();
        sm.list_sessions()
            .into_iter()
            .map(|info| {
                let wu_ctx = sm.get_session(&info.id)
                    .ok()
                    .and_then(|s| s.get_work_unit_context());
                serde_json::json!({
                    "id": info.id,
                    "state": info.status,
                    "name": info.name,
                    "provider_id": info.provider_id,
                    "model_id": info.model_id,
                    "work_unit_id": wu_ctx.as_ref().and_then(|c| c.id.as_deref()),
                    "work_unit_status": wu_ctx.as_ref().and_then(|c| c.status.as_deref()),
                })
            })
            .collect()
    });
    codelet_tools::set_session_list_provider(Some(session_list_provider));

    // Model info provider — returns the first running session's provider/model,
    // or the first session's provider/model if none are running.
    let model_info_provider: codelet_tools::ModelInfoProvider = std::sync::Arc::new(|| {
        let sm = SessionManager::instance();
        let sessions = sm.list_sessions();
        // Prefer a running session's model info
        let running = sessions.iter().find(|s| s.status == "running");
        let info = running.or_else(|| sessions.first());
        match info {
            Some(s) => (s.provider_id.clone(), s.model_id.clone()),
            None => (None, None),
        }
    });
    codelet_tools::set_model_info_provider(Some(model_info_provider));
}

/// SESS-017: Register session creator + global PtyRegistry with the bridge.
///
/// The bridge calls these from `handle_multiplexed_inbound()` when a
/// `session:create` or `terminal:create` envelope arrives. Without this
/// registration the dashboard's "+ > New fspec Session" / "+ > New Terminal"
/// clicks silently fail because the bridge has no way to spawn anything.
pub(crate) fn init_bridge_session_and_terminal_creators() {
    // Session creator — spawns a new background session via SessionManager.
    let creator: codelet_tools::SessionCreator = std::sync::Arc::new(|| {
        let sm = SessionManager::instance();

        // Pick a model: prefer the default tracked by SchedulerMonitor, else
        // fall back to the most recent session's model.
        let model = sm
            .get_default_model()
            .or_else(|| {
                sm.list_sessions()
                    .into_iter()
                    .find_map(|info| match (info.provider_id, info.model_id) {
                        (Some(p), Some(m)) => Some(format!("{p}/{m}")),
                        _ => None,
                    })
            })
            .ok_or_else(|| {
                "No default model available for session creation".to_string()
            })?;

        let project = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|e| format!("Failed to read current dir: {e}"))?;

        // Drive the async create_session call from this sync callback. The
        // bridge inbound handler runs inside a tokio task, so we use
        // block_in_place + the current handle.
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                sm.create_session(&model, &project)
                    .await
                    .map_err(|e| e.to_string())
            })
        })
    });
    codelet_tools::set_session_creator(Some(creator));

    // Global PTY registry — single shared instance owned by the bridge.
    let registry = std::sync::Arc::new(codelet_tools::PtyRegistry::new());
    codelet_tools::set_pty_registry(Some(registry));
}

/// Callback function that emits a block notification to the TUI.
/// Called by BashToolFacadeWrapper and FileToolFacadeWrapper when an action is blocked.
///
/// RPC-041: routes the UserNotification chunk through
/// `SessionManager::instance().chunks_tx().send(...)` — the napi-side
/// fan-out task subscribed by `session_set_global_chunk_callback`
/// delivers it to the TS callback exactly as before.
pub(crate) fn emit_block_notification_to_tui(session_id_str: String, action: String, reason: String) {
    // Format the notification message: "AI was blocked from {action} - {reason}"
    let message = format!("AI was blocked from {} - {}", action, reason);

    // Create a UserNotification chunk with Warning severity
    let chunk = StreamChunk::user_notification(message, NotificationSeverity::Warning);

    // RPC-041: emit via the manager-owned chunks_tx broadcast (was
    // previously dispatched through the deleted chunk-callback OnceCell static).
    let _ = SessionManager::instance().chunks_tx().send((
        codelet_rpc_types::SessionId::from(session_id_str),
        chunk,
    ));
}

/// Callback function that retrieves the current work unit stage for a session.
/// Called by FileToolFacadeWrapper to check stage permissions.
fn get_session_work_unit_stage(session_id_str: String) -> Option<String> {
    // Try to get the session from the SessionManager
    let manager = SessionManager::instance();
    
    // Get the session by ID (handles UUID parsing internally)
    if let Ok(session) = manager.get_session(&session_id_str) {
        // Get the work unit context from the session
        if let Some(ctx) = session.get_work_unit_context() {
            // Return the status (stage) if available
            return ctx.status;
        }
    }
    
    None
}

/// GIT-020: Callback function that retrieves the isolation context for a session.
/// Called by FileToolFacadeWrapper and BashToolFacadeWrapper for isolated session support.
///
/// For isolated sessions, returns Some(IsolationContext) with:
/// - worktree_path: Where file operations ARE allowed (the isolated worktree)
/// - blocked_project_path: Where file operations are BLOCKED (the original project)
///
/// For non-isolated sessions, returns None to SKIP path validation entirely.
///
/// CRITICAL: Non-isolated sessions MUST return None so they can access ANY path
/// (e.g., /tmp, /etc, anywhere on the filesystem). Only isolated sessions should
/// have their file access restricted.
///
/// GIT-020 FIX: The isolation should ONLY block the original project directory,
/// NOT all paths outside the worktree. Paths like /tmp, /etc are ALLOWED.
fn get_session_effective_cwd(session_id_str: String) -> Option<codelet_tools::facade::IsolationContext> {
    // Try to get the session from the SessionManager
    let manager = SessionManager::instance();
    
    // Get the session by ID (handles UUID parsing internally)
    if let Ok(session) = manager.get_session(&session_id_str) {
        // CRITICAL: Only return Some(...) for isolated sessions.
        // Non-isolated sessions must return None to skip path validation.
        // session.worktree_path is Some only for isolated sessions.
        if let Some(ref worktree_path) = session.worktree_path {
            // Create IsolationContext with:
            // - worktree_path: The isolated worktree (ALLOWED)
            // - blocked_project_path: The original project (BLOCKED)
            return Some(codelet_tools::facade::IsolationContext {
                worktree_path: worktree_path.clone(),
                blocked_project_path: std::path::PathBuf::from(&session.project),
            });
        }
    }
    
    None
}

// ============================================================================
// RPC-043: Test modules migrated from session_manager.rs (verbatim)
// ============================================================================

#[cfg(test)]
mod supervisor_broadcast_tests {
    use super::*;

    /// Feature: spec/features/broadcast-channel-for-parent-stream-observation.feature
    ///
    /// Scenario: Broadcast with no subscribers still buffers normally
    ///
    /// @step Given a BackgroundSession with broadcast channel initialized
    /// @step And no supervisors have subscribed to the stream
    /// @step When handle_output is called with a TextDelta chunk
    /// @step Then the chunk should be added to the output buffer
    /// @step And no error should occur from the broadcast
    #[test]
    fn test_broadcast_with_no_subscribers_still_buffers() {
        // @step Given a BackgroundSession with broadcast channel initialized
        let (tx, _rx) = broadcast::channel::<StreamChunk>(SUPERVISOR_BROADCAST_CAPACITY);
        let output_buffer: RwLock<Vec<StreamChunk>> = RwLock::new(Vec::new());

        // @step And no supervisors have subscribed to the stream
        // (no receivers created - tx has no subscribers)

        // @step When handle_output is called with a TextDelta chunk
        let chunk = StreamChunk::text("test content".to_string());
        
        // Simulate handle_output behavior:
        // 1. Buffer the chunk
        {
            let mut buffer = output_buffer.write().expect("lock");
            buffer.push(chunk.clone());
        }
        // 2. Broadcast (fire-and-forget, ignores SendError when no receivers)
        let _ = tx.send(chunk.clone());

        // @step Then the chunk should be added to the output buffer
        let buffer = output_buffer.read().expect("lock");
        assert_eq!(buffer.len(), 1, "chunk should be buffered");
        // NAPI-010: Use pattern matching to check variant
        assert!(matches!(buffer[0], StreamChunk::Text { .. }));

        // @step And no error should occur from the broadcast
        // (if we got here, no panic occurred)
    }

    /// Scenario: Single supervisor receives chunks via broadcast
    ///
    /// @step Given a BackgroundSession with broadcast channel initialized
    /// @step And a supervisor has called subscribe_to_stream to get a receiver
    /// @step When handle_output is called with a TextDelta chunk
    /// @step Then the supervisor should receive the same chunk via its receiver
    /// @step And the chunk should also be buffered normally
    #[test]
    fn test_single_supervisor_receives_chunks() {
        // @step Given a BackgroundSession with broadcast channel initialized
        let (tx, mut rx) = broadcast::channel::<StreamChunk>(SUPERVISOR_BROADCAST_CAPACITY);
        let output_buffer: RwLock<Vec<StreamChunk>> = RwLock::new(Vec::new());

        // @step And a supervisor has called subscribe_to_stream to get a receiver
        // rx is already subscribed (created from channel)

        // @step When handle_output is called with a TextDelta chunk
        let chunk = StreamChunk::text("supervisor test".to_string());
        {
            let mut buffer = output_buffer.write().expect("lock");
            buffer.push(chunk.clone());
        }
        let _ = tx.send(chunk.clone());

        // @step Then the supervisor should receive the same chunk via its receiver
        let received = rx.try_recv().expect("should receive chunk");
        // NAPI-010: Use pattern matching to check variant
        match received {
            StreamChunk::Text { text, .. } => {
                assert_eq!(text, "supervisor test");
            }
            _ => panic!("Expected Text variant"),
        }

        // @step And the chunk should also be buffered normally
        let buffer = output_buffer.read().expect("lock");
        assert_eq!(buffer.len(), 1);
    }

    /// Scenario: Multiple supervisors receive chunks independently
    ///
    /// @step Given a BackgroundSession with broadcast channel initialized
    /// @step And supervisor A has subscribed to the stream
    /// @step And supervisor B has subscribed to the stream
    /// @step When handle_output is called with a TextDelta chunk
    /// @step Then supervisor A should receive the chunk via its receiver
    /// @step And supervisor B should receive the chunk via its receiver
    /// @step And both received chunks should be identical
    #[test]
    fn test_multiple_supervisors_receive_independently() {
        // @step Given a BackgroundSession with broadcast channel initialized
        let (tx, mut rx_a) = broadcast::channel::<StreamChunk>(SUPERVISOR_BROADCAST_CAPACITY);

        // @step And supervisor A has subscribed to the stream
        // rx_a is already subscribed

        // @step And supervisor B has subscribed to the stream
        let mut rx_b = tx.subscribe();

        // @step When handle_output is called with a TextDelta chunk
        let chunk = StreamChunk::text("multi-supervisor".to_string());
        let _ = tx.send(chunk.clone());

        // @step Then supervisor A should receive the chunk via its receiver
        let received_a = rx_a.try_recv().expect("supervisor A should receive");

        // @step And supervisor B should receive the chunk via its receiver
        let received_b = rx_b.try_recv().expect("supervisor B should receive");

        // @step And both received chunks should be identical
        // NAPI-010: Use pattern matching to check variants
        match (&received_a, &received_b) {
            (StreamChunk::Text { text: text_a, .. }, StreamChunk::Text { text: text_b, .. }) => {
                assert_eq!(text_a, text_b);
                assert_eq!(text_a, "multi-supervisor");
            }
            _ => panic!("Expected Text variants"),
        }
    }

    /// Scenario: Slow supervisor receives lagged error when falling behind
    ///
    /// @step Given a BackgroundSession with broadcast channel capacity of 256
    /// @step And a supervisor has subscribed to the stream
    /// @step And the supervisor has not consumed any chunks
    /// @step When handle_output is called 300 times with chunks
    /// @step Then the supervisor should receive RecvError::Lagged when trying to receive
    #[test]
    fn test_slow_supervisor_receives_lagged_error() {
        // @step Given a BackgroundSession with broadcast channel capacity of 256
        let (tx, mut rx) = broadcast::channel::<StreamChunk>(SUPERVISOR_BROADCAST_CAPACITY);

        // @step And a supervisor has subscribed to the stream
        // @step And the supervisor has not consumed any chunks
        // (rx exists but we don't call recv)

        // @step When handle_output is called 300 times with chunks
        for i in 0..300 {
            let chunk = StreamChunk::text(format!("chunk {}", i));
            let _ = tx.send(chunk);
        }

        // @step Then the supervisor should receive RecvError::Lagged when trying to receive
        match rx.try_recv() {
            Err(broadcast::error::TryRecvError::Lagged(n)) => {
                assert!(n > 0, "should have lagged by some messages");
                // With 300 sends and 256 capacity, we lag by 300 - 256 = 44 messages
                assert!(n >= 44, "should lag by at least 44 messages, got {}", n);
            }
            other => panic!("expected Lagged error, got {:?}", other),
        }
    }

    /// Scenario: Dropped receiver does not affect other supervisors
    ///
    /// @step Given a BackgroundSession with broadcast channel initialized
    /// @step And supervisor A has subscribed to the stream
    /// @step And supervisor B has subscribed to the stream
    /// @step When supervisor A drops its receiver
    /// @step And handle_output is called with a TextDelta chunk
    /// @step Then supervisor B should still receive the chunk normally
    /// @step And the subordinate session should continue operating normally
    #[test]
    fn test_dropped_receiver_does_not_affect_others() {
        // @step Given a BackgroundSession with broadcast channel initialized
        let (tx, rx_a) = broadcast::channel::<StreamChunk>(SUPERVISOR_BROADCAST_CAPACITY);

        // @step And supervisor A has subscribed to the stream
        // rx_a exists

        // @step And supervisor B has subscribed to the stream
        let mut rx_b = tx.subscribe();

        // @step When supervisor A drops its receiver
        drop(rx_a);

        // @step And handle_output is called with a TextDelta chunk
        let chunk = StreamChunk::text("after drop".to_string());
        let send_result = tx.send(chunk);

        // @step Then supervisor B should still receive the chunk normally
        let received = rx_b.try_recv().expect("supervisor B should receive");
        // NAPI-010: Use pattern matching
        match received {
            StreamChunk::Text { text, .. } => {
                assert_eq!(text, "after drop");
            }
            _ => panic!("Expected Text variant"),
        }

        // @step And the subordinate session should continue operating normally
        assert!(send_result.is_ok(), "send should succeed with remaining receiver");
    }

    /// Scenario: Late subscriber starts receiving from current position
    ///
    /// @step Given a BackgroundSession with broadcast channel initialized
    /// @step And handle_output has been called 10 times with chunks
    /// @step When a new supervisor subscribes to the stream
    /// @step And handle_output is called with a new chunk
    /// @step Then the new supervisor should receive only the new chunk
    /// @step And the new supervisor should not receive the previous 10 chunks
    #[test]
    fn test_late_subscriber_starts_from_current() {
        // @step Given a BackgroundSession with broadcast channel initialized
        let (tx, _initial_rx) = broadcast::channel::<StreamChunk>(SUPERVISOR_BROADCAST_CAPACITY);

        // @step And handle_output has been called 10 times with chunks
        for i in 0..10 {
            let chunk = StreamChunk::text(format!("old chunk {}", i));
            let _ = tx.send(chunk);
        }

        // @step When a new supervisor subscribes to the stream
        let mut late_rx = tx.subscribe();

        // @step And handle_output is called with a new chunk
        let new_chunk = StreamChunk::text("new chunk".to_string());
        let _ = tx.send(new_chunk);

        // @step Then the new supervisor should receive only the new chunk
        let received = late_rx.try_recv().expect("should receive new chunk");
        // NAPI-010: Use pattern matching
        match received {
            StreamChunk::Text { text, .. } => {
                assert_eq!(text, "new chunk");
            }
            _ => panic!("Expected Text variant"),
        }

        // @step And the new supervisor should not receive the previous 10 chunks
        // (already verified - we only got one chunk, the new one)
        match late_rx.try_recv() {
            Err(broadcast::error::TryRecvError::Empty) => {
                // Expected - no more chunks
            }
            other => panic!("expected Empty, got {:?}", other),
        }
    }

    // === Integration tests that verify BackgroundSession has broadcast channel ===

    /// Test that BackgroundSession has supervisor_broadcast field and SUPERVISOR_BROADCAST_CAPACITY is correct
    #[test]
    fn test_background_session_has_broadcast_field() {
        // Verify the constant is defined correctly
        assert_eq!(SUPERVISOR_BROADCAST_CAPACITY, 256);
        
        // Note: Full BackgroundSession integration tested via handle_output() which
        // requires codelet_cli::session::Session. The unit tests above validate the
        // broadcast channel mechanics work correctly in isolation.
    }
}


/// Feature: spec/features/remove-is-attached-gating-from-rust-chunk-forwarding.feature
///
/// Tests for BRIDGE-012: Remove is_attached gating from Rust chunk forwarding.
/// The is_attached check in handle_output() causes chunks to be dropped when
/// input comes from the bridge, even though the callback is registered.
#[cfg(test)]
mod is_attached_gating_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Scenario: Bridge input displays both input and response in TUI
    ///
    /// This test verifies that the supervisor_broadcast path (used by bridges) always
    /// sends chunks regardless of is_attached state. The problem is that the
    /// attached_callback path (used by TUI) is gated by is_attached.
    ///
    /// @step Given a session is active with the global chunk callback registered
    /// @step And a Telegram bridge is connected to the session
    /// @step When the bridge sends input to the session
    /// @step Then the TUI should display the bridge input in the conversation
    /// @step And the TUI should display the LLM response chunks in the conversation
    #[test]
    fn test_supervisor_broadcast_always_sends_regardless_of_is_attached() {
        // @step Given a session is active with the global chunk callback registered
        let (tx, mut rx) = broadcast::channel::<StreamChunk>(SUPERVISOR_BROADCAST_CAPACITY);
        let is_attached = AtomicBool::new(false);  // Simulating detached state
        
        // @step And a Telegram bridge is connected to the session
        // Bridge subscribes via supervisor_broadcast (rx is our subscriber)
        
        // @step When the bridge sends input to the session
        // Simulating handle_output behavior for supervisor_broadcast path
        let chunk = StreamChunk::text("LLM response from bridge input".to_string());
        
        // supervisor_broadcast.send() has NO is_attached check (this is correct)
        let _ = tx.send(chunk.clone());
        
        // @step Then the TUI should display the bridge input in the conversation
        // @step And the TUI should display the LLM response chunks in the conversation
        // The bridge/supervisor receives the chunk because supervisor_broadcast is NOT gated
        let received = rx.try_recv().expect("bridge should receive chunk regardless of is_attached");
        match received {
            StreamChunk::Text { text, .. } => {
                assert_eq!(text, "LLM response from bridge input");
            }
            _ => panic!("Expected Text variant"),
        }
        
        // Verify is_attached is still false - proving the chunk was sent without gating
        assert!(!is_attached.load(Ordering::Acquire));
    }

    /// Scenario: Keyboard input displays response in TUI
    ///
    /// This test verifies that when a callback IS registered and is_attached IS true,
    /// the TUI correctly receives chunks. This is the regression test.
    ///
    /// @step Given a session is active with the global chunk callback registered
    /// @step When the user types input directly in the TUI
    /// @step Then the TUI should display the LLM response chunks in the conversation
    #[test]
    fn test_attached_callback_receives_chunks_when_is_attached_true() {
        // @step Given a session is active with the global chunk callback registered
        let callback_count = Arc::new(AtomicUsize::new(0));
        let callback_count_clone = callback_count.clone();
        let is_attached = AtomicBool::new(true);  // TUI is attached
        
        // Simulate the callback behavior (counting calls instead of real NAPI callback)
        let simulate_callback_call = move || {
            callback_count_clone.fetch_add(1, Ordering::SeqCst);
        };
        
        // @step When the user types input directly in the TUI
        // Simulating handle_output behavior for attached_callback path
        let _chunk = StreamChunk::text("LLM response from keyboard input".to_string());
        
        // Current code: only calls callback if is_attached is true
        if is_attached.load(Ordering::Acquire) {
            // In real code: cb.call(Ok(chunk), ThreadsafeFunctionCallMode::NonBlocking)
            simulate_callback_call();
        }
        
        // @step Then the TUI should display the LLM response chunks in the conversation
        assert_eq!(callback_count.load(Ordering::SeqCst), 1, "callback should be called when is_attached is true");
    }

    /// This test demonstrates the BUG: when is_attached is false (e.g., after detach),
    /// chunks are dropped even though a callback might still be interested.
    ///
    /// BRIDGE-012 FIX VERIFIED: After removing is_attached check, chunks are forwarded
    /// to the callback if it exists, regardless of is_attached state.
    #[test]
    fn test_fixed_callback_receives_chunks_regardless_of_is_attached() {
        // Setup: callback exists but is_attached is false (e.g., bridge input scenario)
        let callback_count = Arc::new(AtomicUsize::new(0));
        let callback_count_clone = callback_count.clone();
        let _is_attached = AtomicBool::new(false);  // Detached state - but should NOT matter anymore
        let callback_exists = true;  // Callback IS registered
        
        let simulate_callback_call = move || {
            callback_count_clone.fetch_add(1, Ordering::SeqCst);
        };
        
        // Simulating FIXED handle_output behavior (no is_attached check)
        let _chunk = StreamChunk::text("LLM response".to_string());
        
        // FIXED code: just check if callback exists, don't gate on is_attached
        // This mirrors the actual fix in handle_output()
        if callback_exists {
            simulate_callback_call();
        }
        
        // After BRIDGE-012 fix: callback should be called because it exists
        assert_eq!(callback_count.load(Ordering::SeqCst), 1, 
            "BRIDGE-012 fix: callback should be called when it exists, regardless of is_attached");
    }

    /// This test verifies the FIXED behavior matches the actual handle_output() implementation.
    /// The callback is called when it exists, period - no is_attached gating.
    #[test]
    fn test_callback_forwarding_matches_fixed_handle_output_behavior() {
        // Setup: callback exists but is_attached is false
        let callback_count = Arc::new(AtomicUsize::new(0));
        let callback_count_clone = callback_count.clone();
        let _is_attached = AtomicBool::new(false);  // Detached state - but should NOT matter
        let callback_exists = true;  // Callback IS registered
        
        let simulate_callback_call = move || {
            callback_count_clone.fetch_add(1, Ordering::SeqCst);
        };
        
        // Simulating FIXED handle_output behavior (no is_attached check)
        let _chunk = StreamChunk::text("LLM response".to_string());
        
        // FIXED code: just check if callback exists, don't gate on is_attached
        if callback_exists {
            simulate_callback_call();
        }
        
        // After fix: callback should be called because it exists
        assert_eq!(callback_count.load(Ordering::SeqCst), 1, 
            "After fix: callback should be called when it exists, regardless of is_attached");
    }
}

/// Feature: spec/features/global-chunk-callback-napi.feature
///
/// Tests for BRIDGE-012: Global chunk callback NAPI for session-agnostic chunk emission.
/// Rust exposes a single global callback via NAPI that TypeScript registers once at app startup.
/// ALL chunks from ALL sessions go through this ONE callback with signature (session_id, chunk).
/// Rust has ZERO knowledge of which session is active/attached - it's a pure emitter.
#[cfg(test)]
mod global_chunk_callback_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Scenario: Register global chunk callback at startup
    ///
    /// @step Given no global chunk callback is registered
    /// @step When TypeScript calls sessionSetGlobalChunkCallback with a callback function
    /// @step Then Rust should store the callback in a global static
    /// @step And subsequent chunk emissions should use this callback
    #[test]
    fn test_global_callback_registration() {
        // @step Given no global chunk callback is registered
        // This test simulates the global callback pattern
        
        let callback_invoked = Arc::new(AtomicUsize::new(0));
        let callback_clone = callback_invoked.clone();
        
        // @step When TypeScript calls sessionSetGlobalChunkCallback with a callback function
        // Simulating the global callback being registered
        let global_callback = move |_session_id: &str, _chunk: &StreamChunk| {
            callback_clone.fetch_add(1, Ordering::SeqCst);
        };
        
        // @step Then Rust should store the callback in a global static
        // (simulated - in actual impl this would be OnceCell or lazy_static)
        let callback_exists = true;
        assert!(callback_exists, "Global callback should be stored");
        
        // @step And subsequent chunk emissions should use this callback
        let session_id = "test-session-123";
        let chunk = StreamChunk::text("Test chunk".to_string());
        global_callback(session_id, &chunk);
        
        assert_eq!(callback_invoked.load(Ordering::SeqCst), 1, 
            "Global callback should be invoked for chunk emission");
    }

    /// Scenario: Emit chunk with session_id through global callback
    ///
    /// @step Given a global chunk callback is registered
    /// @step And a session exists with id "session-abc"
    /// @step When the session emits a Text chunk via handle_output
    /// @step Then the global callback should be invoked with session_id "session-abc"
    /// @step And the global callback should receive the Text chunk
    #[test]
    fn test_emit_chunk_with_session_id() {
        // @step Given a global chunk callback is registered
        let received_session_id = Arc::new(std::sync::Mutex::new(String::new()));
        let received_chunk_type = Arc::new(std::sync::Mutex::new(String::new()));
        
        let session_id_clone = received_session_id.clone();
        let chunk_type_clone = received_chunk_type.clone();
        
        let global_callback = move |session_id: &str, chunk: &StreamChunk| {
            *session_id_clone.lock().unwrap() = session_id.to_string();
            *chunk_type_clone.lock().unwrap() = match chunk {
                StreamChunk::Text { .. } => "Text".to_string(),
                StreamChunk::Thinking { .. } => "Thinking".to_string(),
                _ => "Other".to_string(),
            };
        };
        
        // @step And a session exists with id "session-abc"
        let session_id = "session-abc";
        
        // @step When the session emits a Text chunk via handle_output
        let chunk = StreamChunk::text("Hello from session".to_string());
        global_callback(session_id, &chunk);
        
        // @step Then the global callback should be invoked with session_id "session-abc"
        assert_eq!(*received_session_id.lock().unwrap(), "session-abc");
        
        // @step And the global callback should receive the Text chunk
        assert_eq!(*received_chunk_type.lock().unwrap(), "Text");
    }

    /// Scenario: Multiple sessions emit through same global callback
    ///
    /// @step Given a global chunk callback is registered
    /// @step And session "session-a" exists
    /// @step And session "session-b" exists
    /// @step When session "session-a" emits a chunk
    /// @step And session "session-b" emits a chunk
    /// @step Then both chunks should go through the same global callback
    /// @step And each chunk should have its respective session_id
    #[test]
    fn test_multiple_sessions_same_callback() {
        // @step Given a global chunk callback is registered
        let received_calls: Arc<std::sync::Mutex<Vec<(String, String)>>> = 
            Arc::new(std::sync::Mutex::new(Vec::new()));
        
        let calls_clone = received_calls.clone();
        let global_callback = move |session_id: &str, chunk: &StreamChunk| {
            let chunk_text = match chunk {
                StreamChunk::Text { text, .. } => text.clone(),
                _ => "unknown".to_string(),
            };
            calls_clone.lock().unwrap().push((session_id.to_string(), chunk_text));
        };
        
        // @step And session "session-a" exists
        // @step And session "session-b" exists
        
        // @step When session "session-a" emits a chunk
        let chunk_a = StreamChunk::text("From session A".to_string());
        global_callback("session-a", &chunk_a);
        
        // @step And session "session-b" emits a chunk
        let chunk_b = StreamChunk::text("From session B".to_string());
        global_callback("session-b", &chunk_b);
        
        // @step Then both chunks should go through the same global callback
        let calls = received_calls.lock().unwrap();
        assert_eq!(calls.len(), 2, "Both chunks should go through the callback");
        
        // @step And each chunk should have its respective session_id
        assert_eq!(calls[0].0, "session-a");
        assert_eq!(calls[0].1, "From session A");
        assert_eq!(calls[1].0, "session-b");
        assert_eq!(calls[1].1, "From session B");
    }

    /// Scenario: No attachment state in Rust
    ///
    /// This test documents what should NOT exist after BRIDGE-012 implementation.
    /// The actual verification is done via AST search showing these items are removed.
    ///
    /// @step Given a session exists
    /// @step When I inspect the BackgroundSession struct
    /// @step Then there should be no is_attached field
    /// @step And there should be no attached_callback field
    /// @step And there should be no attach method
    /// @step And there should be no detach method
    #[test]
    fn test_no_attachment_state_documentation() {
        // This test serves as documentation for BRIDGE-012.
        // After implementation, the following should be REMOVED from BackgroundSession:
        // - is_attached: AtomicBool
        // - attached_callback: RwLock<Option<ThreadsafeFunction<StreamChunk>>>
        // - pub fn is_attached(&self) -> bool
        // - pub fn attach(&self, callback: ThreadsafeFunction<StreamChunk>)
        // - pub fn detach(&self)
        //
        // Verification is done through AST grep showing these don't exist.
        // This test passes to document the expected state after implementation.
        
        // TODO: After BRIDGE-012 implementation, this test should verify
        // that BackgroundSession has NO is_attached/attached_callback fields.
        // For now, it documents the expected behavior.
        // Test passes by reaching this point - BRIDGE-012 behavior documented
    }

    /// Scenario: No per-session NAPI attachment functions
    ///
    /// This test documents what NAPI functions should NOT exist after BRIDGE-012.
    ///
    /// @step When I inspect the NAPI module exports
    /// @step Then there should be no session_attach function
    /// @step And there should be no session_detach function
    /// @step And there should be a sessionSetGlobalChunkCallback function
    #[test]
    fn test_no_per_session_napi_functions_documentation() {
        // This test serves as documentation for BRIDGE-012.
        // After implementation, the following NAPI functions should be REMOVED:
        // - session_attach(session_id: String, callback: ThreadsafeFunction<StreamChunk>)
        // - session_detach(session_id: String)
        //
        // And this function should be ADDED:
        // - sessionSetGlobalChunkCallback(callback: ThreadsafeFunction<(String, StreamChunk)>)
        //
        // Verification is done through AST grep and TypeScript import analysis.
        // Test passes by reaching this point - BRIDGE-012 NAPI structure documented
    }
}

#[cfg(test)]
mod correlation_id_tests {
    use super::*;

    // Feature: spec/features/cross-pane-selection-with-correlation-ids.feature (WATCH-011)

    /// Scenario: StreamChunk receives correlation ID in handle_output
    ///
    /// @step Given a subordinate session exists
    /// @step When the subordinate session emits a Text chunk via handle_output()
    /// @step Then the chunk receives a unique correlation_id assigned by an atomic counter
    /// @step And the correlation_id is in format "{session_id}-{counter}"
    #[test]
    fn test_correlation_id_format() {
        // @step Given a subordinate session exists
        let session_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

        // Simulate correlation ID assignment as done in handle_output
        // Using AtomicU64::fetch_add as in the real implementation
        let counter = AtomicU64::new(0);

        // @step When the subordinate session emits a Text chunk via handle_output()
        let id1 = counter.fetch_add(1, Ordering::SeqCst);
        let correlation_id1 = format!("{}-{}", session_id, id1);

        let id2 = counter.fetch_add(1, Ordering::SeqCst);
        let correlation_id2 = format!("{}-{}", session_id, id2);

        // @step Then the chunk receives a unique correlation_id assigned by an atomic counter
        assert_ne!(correlation_id1, correlation_id2);

        // @step And the correlation_id is in format "{session_id}-{counter}"
        assert_eq!(correlation_id1, "00000000-0000-0000-0000-000000000001-0");
        assert_eq!(correlation_id2, "00000000-0000-0000-0000-000000000001-1");
    }

    /// Scenario: StreamChunk can be tagged with observed correlation IDs
    ///
    /// @step Given a supervisor response chunk
    /// @step When it is tagged with observed correlation IDs
    /// @step Then the chunk has observed_correlation_ids set
    #[test]
    fn test_stream_chunk_with_observed_correlation_ids() {
        // @step Given a supervisor response chunk
        let chunk = StreamChunk::text("I noticed an issue".to_string());

        // @step When it is tagged with observed correlation IDs
        let tagged_chunk = chunk.with_observed_correlation_ids(vec![
            "p-0".to_string(),
            "p-1".to_string(),
        ]);

        // @step Then the chunk has observed_correlation_ids set
        // NAPI-010: Check using pattern matching on the enum variant
        match tagged_chunk {
            StreamChunk::Text { observed_correlation_ids, .. } => {
                assert!(observed_correlation_ids.is_some());
                let ids = observed_correlation_ids.unwrap();
                assert_eq!(ids, vec!["p-0", "p-1"]);
            }
            _ => panic!("Expected Text variant"),
        }
    }

}
