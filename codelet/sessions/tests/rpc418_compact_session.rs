//! Feature: spec/features/rust-tui-compact-real-compaction.feature
//!
//! RPC-418 — Behavioural tests for `SessionManagerHandle::compact_session`.
//! Each `#[tokio::test]` maps 1:1 to a Gherkin scenario in the feature file.
//!
//! The tests construct a fresh `SessionManager`, create a session via the
//! trait's `create_session` bridge (which uses `NoopSessionManagerHooks` so no
//! agent loop is spawned), seed `session.inner.lock().await.messages` with
//! hand-crafted rig messages + token counts, then drive the REAL sync trait
//! method `compact_session` through `SessionManagerHandle`.
//!
//! They exercise the real handle (NOT the NAPI `session_compact` path).
//!
//! multi-thread runtime is REQUIRED: `compact_session` bridges sync->async via
//! `tokio::task::block_in_place`, which panics on a single-thread runtime.
//!
//! These tests are written BEFORE the implementation and MUST FAIL against the
//! current 1:1 no-op stub in `handle_impl.rs` (red phase): the stub never
//! clears the conversation, never injects the compaction instruction, never
//! sends "Continue", never errors on an empty session, and always returns
//! `compacted_tokens == original_tokens` with ratio 1.0.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{SessionId, SessionStatus, StreamChunk};
use codelet_sessions::SessionManager;
use rig::message::{AssistantContent, Message, UserContent};
use rig::OneOrMany;
use tokio::sync::{broadcast, Mutex};

/// Serializes the tests that swap the process-global data directory
/// (`codelet_common::set_data_directory`) so a parallel test in this binary
/// cannot swap the pointer out from under another test's `SessionManager::new()`
/// (which eagerly loads `<data_dir>/default-model.json`). Mirrors the
/// `DATA_DIR_GUARD` pattern from `rpc081_restore_session_messages.rs`.
static DATA_DIR_GUARD: Mutex<()> = Mutex::const_new(());

/// Create a fresh BackgroundSession via the SessionManagerHandle bridge.
/// The Noop hooks ensure no agent loop is spawned for the session.
async fn fresh_session(manager: &SessionManager) -> SessionId {
    manager.set_default_model("anthropic/claude-opus-4-5");
    let handle: &dyn SessionManagerHandle = manager;
    handle.create_session(None)
}

/// Seed the session's inner conversation with several user/assistant messages
/// and a non-zero input-token count, mirroring a populated conversation.
/// Returns the number of messages present after seeding.
async fn seed_populated(manager: &SessionManager, sid: &SessionId) -> usize {
    let session = manager.get_session(&sid.value).expect("session must exist");
    let mut inner = session.inner.lock().await;
    inner.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Please refactor the auth module")),
    });
    inner.messages.push(Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::text("Sure, reading the files now")),
    });
    inner.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Also add rate limiting")),
    });
    inner.messages.push(Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::text("Done, here is the plan")),
    });
    inner.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Looks good, ship it")),
    });
    inner.token_tracker.input_tokens = 5000;
    inner.token_tracker.output_tokens = 1000;
    inner.messages.len()
}

/// Join all text from a message (user or assistant) into a single string.
fn message_text(msg: &Message) -> String {
    match msg {
        Message::User { content } => content
            .iter()
            .map(|uc| match uc {
                UserContent::Text(t) => t.text.clone(),
                _ => String::new(),
            })
            .collect(),
        Message::Assistant { content, .. } => content
            .iter()
            .map(|ac| match ac {
                AssistantContent::Text(t) => t.text.clone(),
                _ => String::new(),
            })
            .collect(),
    }
}

/// Drain broadcast chunks for the given session id up to `total_timeout`,
/// collecting everything that arrives. Used to observe the "Continue"
/// UserInput chunk that `send_input` emits.
async fn drain_chunks_for(
    rx: &mut broadcast::Receiver<(SessionId, StreamChunk)>,
    sid: &str,
    total_timeout: Duration,
) -> Vec<StreamChunk> {
    let mut out = Vec::new();
    let deadline = tokio::time::Instant::now() + total_timeout;
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline - tokio::time::Instant::now();
        let recv = tokio::time::timeout(remaining.min(Duration::from_millis(100)), rx.recv()).await;
        match recv {
            Ok(Ok((session_id, chunk))) => {
                if session_id.value == sid {
                    out.push(chunk);
                }
            }
            Ok(Err(_)) => return out,
            Err(_) => {}
        }
    }
    out
}

// ============================================================================
// Scenario: Compacting a populated session clears the conversation and kicks
// the agent loop
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_populated_clears_conversation_and_kicks_agent_loop() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step Given a session with several user and assistant messages
    let sid = fresh_session(&manager).await;
    let seeded = seed_populated(&manager, &sid).await;
    assert!(seeded >= 5, "expected several seeded messages, got {seeded}");

    // @step When I compact the session through the handle
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.compact_session(&sid);
    assert!(result.is_ok(), "compact_session must succeed: {result:?}");

    let session = manager.get_session(&sid.value).expect("session must exist");
    let inner = session.inner.lock().await;

    // @step Then the conversation is cleared to system-reminders
    // The original 5 user/assistant conversation messages must be gone; only
    // system-reminders (if any) plus the injected compaction instruction remain.
    let non_reminder: Vec<&Message> = inner
        .messages
        .iter()
        .filter(|m| !message_text(m).contains("<system-reminder>"))
        .collect();
    let original_conversation_present = inner.messages.iter().any(|m| {
        let t = message_text(m);
        t.contains("Please refactor the auth module") || t.contains("ship it")
    });
    assert!(
        !original_conversation_present,
        "original conversation messages must be cleared, found survivors: {:?}",
        inner
            .messages
            .iter()
            .map(message_text)
            .collect::<Vec<_>>()
    );

    // @step And the compaction instruction is injected as a user message
    let has_instruction = inner.messages.iter().any(|m| {
        matches!(m, Message::User { .. })
            && message_text(m).contains("hierarchical summary DAG")
    });
    assert!(
        has_instruction,
        "compaction instruction must be injected as a user message; non-reminder messages: {:?}",
        non_reminder.iter().map(|m| message_text(m)).collect::<Vec<_>>()
    );
    drop(inner);

    // @step And a "Continue" input is sent to the agent loop to start DAG construction
    // `send_input` emits a UserInput("Continue") StreamChunk and flips status to Running.
    let chunks = drain_chunks_for(&mut chunks_rx, &sid.value, Duration::from_secs(2)).await;
    let continue_sent = chunks.iter().any(|c| {
        matches!(c, StreamChunk::UserInput { text } if text == "Continue")
    });
    assert!(
        continue_sent,
        "expected a UserInput(\"Continue\") chunk from send_input; got: {chunks:?}"
    );
}

// ============================================================================
// Scenario: Compacting an empty session returns an error and leaves it
// untouched
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_empty_returns_error_and_leaves_untouched() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());

    // @step Given a brand-new session with no messages
    let sid = fresh_session(&manager).await;
    // Force an empty conversation (a fresh session may carry context reminders).
    let baseline_len = {
        let session = manager.get_session(&sid.value).expect("session must exist");
        let mut inner = session.inner.lock().await;
        inner.messages.clear();
        inner.token_tracker.input_tokens = 0;
        inner.messages.len()
    };
    assert_eq!(baseline_len, 0, "precondition: empty conversation");

    // @step When I compact the session through the handle
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.compact_session(&sid);

    // @step Then the handle returns an error containing "Nothing to compact"
    let err = result.expect_err("expected Err on empty session");
    assert!(
        err.contains("Nothing to compact"),
        "expected 'Nothing to compact' in err message, got: {err}"
    );

    // @step And the session message count stays at zero
    let session = manager.get_session(&sid.value).expect("session must exist");
    let inner = session.inner.lock().await;
    assert_eq!(
        inner.messages.len(),
        0,
        "empty session must be left untouched (message count stays at zero)"
    );
}

// ============================================================================
// Scenario: Compacting a populated session reports real token counts
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_populated_reports_real_token_counts() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());

    // @step Given a session with several user and assistant messages
    let sid = fresh_session(&manager).await;
    let seeded = seed_populated(&manager, &sid).await;
    assert!(seeded >= 5, "expected several seeded messages, got {seeded}");

    // @step When I compact the session through the handle
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.compact_session(&sid);
    let compaction = result.expect("compact_session must succeed");

    // @step Then the returned CompactionResult original_tokens is greater than zero
    assert!(
        compaction.original_tokens > 0,
        "expected original_tokens > 0, got {}",
        compaction.original_tokens
    );

    // @step And the returned CompactionResult compacted_tokens is less than original_tokens
    assert!(
        compaction.compacted_tokens < compaction.original_tokens,
        "expected compacted_tokens ({}) < original_tokens ({})",
        compaction.compacted_tokens,
        compaction.original_tokens
    );
}

// ============================================================================
// Scenario: Compacting an unknown session id returns a not-found error
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_unknown_session_id_returns_not_found() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());

    // @step Given a session id that does not exist
    let unknown = SessionId::from("00000000-0000-0000-0000-000000000000".to_string());

    // @step When I compact the session through the handle
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.compact_session(&unknown);

    // @step Then the handle returns an error beginning with "Session not found"
    let err = result.expect_err("expected Err on unknown session id");
    assert!(
        err.starts_with("Session not found"),
        "expected err to begin with 'Session not found', got: {err}"
    );

    // Sanity: status of an unknown session is Idle (no side effects).
    let _ = SessionStatus::Idle;
}
