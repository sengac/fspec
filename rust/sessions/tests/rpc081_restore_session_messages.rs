//! Feature: spec/features/agent-loop-conversation-history-session-inner-messages-round-trip-session-restore-messages-parity.feature
//!
//! RPC-081 — Behavioural tests for `SessionManagerHandle::restore_session_messages`.
//! Each `#[test]` maps 1:1 to a Gherkin scenario in the feature file.
//!
//! The tests construct a fresh `SessionManager`, create a session via
//! the trait's `create_session` bridge (which uses
//! `NoopSessionManagerHooks` so no agent loop is spawned), then drive
//! `restore_session_messages` with hand-crafted envelope JSON strings.
//! Inner messages are inspected via `session.inner.lock().await.messages`;
//! broadcasted StreamChunks are observed via `chunks_rx()`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "fixtures/offline_catalog.rs"]
mod offline_catalog;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{SessionId, StreamChunk};
use codelet_sessions::SessionManager;
use tokio::sync::{broadcast, Mutex};

/// PROV-132: Serializes the tests that swap the process-global data directory
/// (`codelet_common::set_data_directory`) so a parallel test in this binary
/// cannot swap the pointer out from under another test's `SessionManager::new()`
/// — which eagerly loads `<data_dir>/default-model.json`. Without this guard a
/// racing test observes a foreign persisted default model, making the
/// malformed-envelope / no-default-model assertions flake on the first
/// full-suite run. Mirrors RPC-385's async `DATA_DIR_GUARD` (a
/// `tokio::sync::Mutex`, held across the `.await`s that follow — the awaits only
/// drive this test's own already-constructed manager, never the global pointer,
/// so serialising the whole body is both correct and async-safe).
static DATA_DIR_GUARD: Mutex<()> = Mutex::const_new(());

/// Drain every chunk already broadcast for the given session, without
/// waiting (TEST-042: restore_session_messages has returned by the time
/// this runs, and any chunk it emits is broadcast synchronously, so it
/// would already be here).
fn drain_chunks_for(
    rx: &mut broadcast::Receiver<(SessionId, StreamChunk)>,
    sid: &str,
) -> Vec<StreamChunk> {
    let mut out = Vec::new();
    loop {
        match rx.try_recv() {
            Ok((session_id, chunk)) => {
                if session_id.value == sid {
                    out.push(chunk);
                }
            }
            Err(broadcast::error::TryRecvError::Lagged(_)) => continue,
            Err(_) => return out,
        }
    }
}

/// Create a fresh BackgroundSession via the SessionManagerHandle bridge.
/// The Noop hooks ensure no agent loop is spawned for the session.
async fn fresh_session(manager: &SessionManager) -> SessionId {
    offline_catalog::seed_offline_catalog().expect("seed the offline model catalog");
    assert_eq!(
        manager.set_default_model("anthropic/claude-opus-4-5"),
        Ok(())
    );
    let handle: &dyn SessionManagerHandle = manager;
    handle
        .create_session_checked(None)
        .expect("fresh session must be created")
}

fn user_envelope(content: serde_json::Value) -> String {
    serde_json::to_string(&serde_json::json!({
        "message": {
            "role": "user",
            "content": content
        }
    }))
    .expect("user envelope JSON serialises")
}

fn assistant_envelope(content: serde_json::Value) -> String {
    serde_json::to_string(&serde_json::json!({
        "message": {
            "role": "assistant",
            "content": content
        }
    }))
    .expect("assistant envelope JSON serialises")
}

// ============================================================================
// Scenario: restore_session_messages installs a one-user-one-assistant
// transcript into inner.messages
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restore_messages_replays_user_and_assistant_text_into_inner_and_output() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a SessionManager has created a fresh BackgroundSession via SessionManagerHandle
    let data_dir =
        codelet_test_helpers::shared_temp_subdirectory().expect("shared temp subdirectory");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(data_dir.clone()).expect("set the data directory");
    codelet_core::persistence::reset_stores_for_tests();
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    let mut chunks_rx = manager.chunks_tx().subscribe();
    let sid = fresh_session(&manager).await;

    // Capture baseline inner.messages.len() — a fresh session already
    // has context-reminder messages injected by inject_context_reminders().
    let baseline_len = {
        let session = manager.get_session(&sid.value).expect("session must exist");
        let inner = session.inner.lock().await;
        inner.messages.len()
    };

    // @step And a user MessageEnvelope JSON whose content is [{"type":"text","text":"hello"}]
    let user_env = user_envelope(serde_json::json!([
        { "type": "text", "text": "hello" }
    ]));

    // @step And an assistant MessageEnvelope JSON whose content is [{"type":"text","text":"hi back"}]
    let assistant_env = assistant_envelope(serde_json::json!([
        { "type": "text", "text": "hi back" }
    ]));

    // @step When restore_session_messages is invoked with those two envelopes via SessionManagerHandle
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.restore_session_messages(&sid, vec![user_env, assistant_env]);
    assert!(
        result.is_ok(),
        "restore_session_messages must succeed: {result:?}"
    );

    // @step Then session.inner.lock().await.messages.len() equals 2
    let session = manager.get_session(&sid.value).expect("session must exist");
    let inner = session.inner.lock().await;
    let added = inner.messages.len() - baseline_len;
    assert_eq!(
        added, 2,
        "expected 2 rig messages to be appended, got {added}"
    );
    let restored = &inner.messages[baseline_len..];

    // @step And the first inner message is a rig::message::Message::User whose joined text equals "hello"
    match &restored[0] {
        rig::message::Message::User { content } => {
            let joined: String = content
                .iter()
                .map(|uc| match uc {
                    rig::message::UserContent::Text(t) => t.text.clone(),
                    _ => String::new(),
                })
                .collect();
            assert_eq!(joined, "hello", "first restored message text mismatch");
        }
        other => panic!("expected User message at restored[0], got: {other:?}"),
    }

    // @step And the second inner message is a rig::message::Message::Assistant whose joined text equals "hi back"
    match &restored[1] {
        rig::message::Message::Assistant { content, .. } => {
            let joined: String = content
                .iter()
                .map(|ac| match ac {
                    rig::message::AssistantContent::Text(t) => t.text.clone(),
                    _ => String::new(),
                })
                .collect();
            assert_eq!(joined, "hi back", "second restored message text mismatch");
        }
        other => panic!("expected Assistant message at restored[1], got: {other:?}"),
    }
    drop(inner);

    // @step And no UserInput or Text StreamChunk is built from the envelopes, because the visible transcript is paged from the persisted archival transcript
    let chunks = drain_chunks_for(&mut chunks_rx, &sid.value);
    let replayed: Vec<&StreamChunk> = chunks
        .iter()
        .filter(|c| matches!(c, StreamChunk::UserInput { .. } | StreamChunk::Text { .. }))
        .collect();
    assert!(
        replayed.is_empty(),
        "envelopes must not become chunks: {replayed:?}"
    );
}

// ============================================================================
// Scenario: Assistant restoration installs thinking, text, and tool_use blocks
// as one assistant message
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restore_messages_replays_assistant_thinking_text_and_tool_use() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a SessionManager has created a fresh BackgroundSession via SessionManagerHandle
    let data_dir =
        codelet_test_helpers::shared_temp_subdirectory().expect("shared temp subdirectory");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(data_dir.clone()).expect("set the data directory");
    codelet_core::persistence::reset_stores_for_tests();
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    let sid = fresh_session(&manager).await;
    let baseline_len = {
        let session = manager.get_session(&sid.value).expect("session must exist");
        let inner = session.inner.lock().await;
        inner.messages.len()
    };

    // @step And an assistant MessageEnvelope JSON whose content is [{"type":"thinking","thinking":"hmm"},{"type":"text","text":"reading"},{"type":"tool_use","id":"t1","name":"Read","input":{"path":"/tmp/x"}}]
    let envelope = assistant_envelope(serde_json::json!([
        { "type": "thinking", "thinking": "hmm" },
        { "type": "text", "text": "reading" },
        {
            "type": "tool_use",
            "id": "t1",
            "name": "Read",
            "input": { "path": "/tmp/x" }
        }
    ]));

    // @step When restore_session_messages is invoked with that envelope via SessionManagerHandle
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.restore_session_messages(&sid, vec![envelope]);
    assert!(
        result.is_ok(),
        "restore_session_messages must succeed: {result:?}"
    );

    // @step Then session.inner.lock().await.messages contains exactly one rig::message::Message::Assistant
    let session = manager.get_session(&sid.value).expect("session must exist");
    let inner = session.inner.lock().await;
    let restored = &inner.messages[baseline_len..];
    assert_eq!(restored.len(), 1, "expected exactly 1 restored message");
    let rig::message::Message::Assistant { content, .. } = &restored[0] else {
        panic!("expected Assistant at restored[0], got {:?}", restored[0]);
    };

    // @step And its content is, in order, reasoning "hmm", text "reading", and a tool call with id "t1" and name "Read" and arguments {"path":"/tmp/x"}
    let blocks: Vec<_> = content.iter().collect();
    assert_eq!(blocks.len(), 3, "{blocks:?}");
    match blocks[0] {
        rig::message::AssistantContent::Reasoning(reasoning) => {
            assert_eq!(reasoning.reasoning.join(""), "hmm")
        }
        other => panic!("expected reasoning first, got {other:?}"),
    }
    match blocks[1] {
        rig::message::AssistantContent::Text(text) => assert_eq!(text.text, "reading"),
        other => panic!("expected text second, got {other:?}"),
    }
    match blocks[2] {
        rig::message::AssistantContent::ToolCall(call) => {
            assert_eq!(call.id, "t1");
            assert_eq!(call.function.name, "Read");
            assert_eq!(
                call.function.arguments,
                serde_json::json!({ "path": "/tmp/x" })
            );
        }
        other => panic!("expected tool call third, got {other:?}"),
    }
}

// ============================================================================
// Scenario: User restoration installs tool_result blocks as a user tool-result message
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restore_messages_replays_tool_result_without_appending_inner() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a SessionManager has created a fresh BackgroundSession via SessionManagerHandle
    let data_dir =
        codelet_test_helpers::shared_temp_subdirectory().expect("shared temp subdirectory");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(data_dir.clone()).expect("set the data directory");
    codelet_core::persistence::reset_stores_for_tests();
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    let sid = fresh_session(&manager).await;
    let baseline_len = {
        let session = manager.get_session(&sid.value).expect("session must exist");
        let inner = session.inner.lock().await;
        inner.messages.len()
    };

    // @step And a user MessageEnvelope JSON whose content is [{"type":"tool_result","tool_use_id":"t1","content":"contents","is_error":false}]
    let envelope = user_envelope(serde_json::json!([
        {
            "type": "tool_result",
            "tool_use_id": "t1",
            "content": "contents",
            "is_error": false
        }
    ]));

    // @step When restore_session_messages is invoked with that envelope via SessionManagerHandle
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.restore_session_messages(&sid, vec![envelope]);
    assert!(
        result.is_ok(),
        "restore_session_messages must succeed: {result:?}"
    );

    // @step Then session.inner.lock().await.messages contains exactly one rig::message::Message::User
    let session = manager.get_session(&sid.value).expect("session must exist");
    let inner = session.inner.lock().await;
    let restored = &inner.messages[baseline_len..];
    assert_eq!(restored.len(), 1, "expected exactly 1 restored message");
    let rig::message::Message::User { content } = &restored[0] else {
        panic!("expected User at restored[0], got {:?}", restored[0]);
    };

    // @step And its only content is a tool result for "t1" whose text is "contents"
    let blocks: Vec<_> = content.iter().collect();
    match blocks.as_slice() {
        [rig::message::UserContent::ToolResult(result)] => {
            assert_eq!(result.id, "t1");
            let text: String = result
                .content
                .iter()
                .map(|part| match part {
                    rig::message::ToolResultContent::Text(text) => text.text.clone(),
                    _ => String::new(),
                })
                .collect();
            assert_eq!(text, "contents");
        }
        other => panic!("expected one tool result, got {other:?}"),
    }
}

// ============================================================================
// Scenario: Replaceable runtime-reminder envelopes are silently skipped during restoration
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restore_messages_skips_system_reminder_envelopes_silently() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a SessionManager has created a fresh BackgroundSession via SessionManagerHandle
    let data_dir =
        codelet_test_helpers::shared_temp_subdirectory().expect("shared temp subdirectory");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(data_dir.clone()).expect("set the data directory");
    codelet_core::persistence::reset_stores_for_tests();
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    let mut chunks_rx = manager.chunks_tx().subscribe();
    let sid = fresh_session(&manager).await;
    let baseline_len = {
        let session = manager.get_session(&sid.value).expect("session must exist");
        let inner = session.inner.lock().await;
        inner.messages.len()
    };

    // @step And a user MessageEnvelope JSON marked "_runtimeReminder" {"version":1,"replaceable":true} whose content is a stale system-reminder
    let envelope = serde_json::to_string(&serde_json::json!({
        "_runtimeReminder": { "version": 1, "replaceable": true },
        "message": {
            "role": "user",
            "content": [{
                "type": "text",
                "text": "<system-reminder>\n<!-- type:fspecWorkflow -->\nstale\n</system-reminder>"
            }]
        }
    }))
    .expect("reminder envelope JSON serialises");

    // @step When restore_session_messages is invoked with that envelope via SessionManagerHandle
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.restore_session_messages(&sid, vec![envelope]);
    assert!(
        result.is_ok(),
        "restore_session_messages must succeed: {result:?}"
    );

    // @step Then session.inner.lock().await.messages.len() equals 0
    let session = manager.get_session(&sid.value).expect("session must exist");
    let inner = session.inner.lock().await;
    let added = inner.messages.len() - baseline_len;
    assert_eq!(
        added, 0,
        "system-reminder envelope must NOT append to inner.messages (added: {added})"
    );
    drop(inner);

    // @step And no StreamChunk is broadcasted to the session's output for that envelope
    let chunks = drain_chunks_for(&mut chunks_rx, &sid.value);
    // Filter out non-content chunks emitted by session creation/etc. that
    // are not part of restoration. The restoration path itself MUST emit
    // zero chunks for a system-reminder-only envelope.
    let restoration_emitted: Vec<&StreamChunk> = chunks
        .iter()
        .filter(|c| {
            matches!(
                c,
                StreamChunk::UserInput { .. }
                    | StreamChunk::Text { .. }
                    | StreamChunk::Thinking { .. }
                    | StreamChunk::ToolCall { .. }
                    | StreamChunk::ToolResult { .. }
                    | StreamChunk::Done
            )
        })
        .collect();
    assert!(
        restoration_emitted.is_empty(),
        "system-reminder envelope must NOT emit any content/Done StreamChunks; got: {restoration_emitted:?}"
    );
}

// ============================================================================
// Scenario: restore_session_messages returns Err on an unknown session id
// without panicking
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restore_messages_returns_err_on_unknown_session_id_without_panic() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a SessionManagerHandle whose underlying SessionManager has no session registered under the id "00000000-0000-0000-0000-000000000000"
    let data_dir =
        codelet_test_helpers::shared_temp_subdirectory().expect("shared temp subdirectory");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(data_dir).expect("set the data directory");
    codelet_core::persistence::reset_stores_for_tests();
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    let unknown = SessionId::from("00000000-0000-0000-0000-000000000000".to_string());

    // @step When restore_session_messages is invoked with that id and an empty envelope vector
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.restore_session_messages(&unknown, vec![]);

    // @step Then the call returns Err whose message contains "Session not found"
    let err = result.expect_err("expected Err on unknown session id");
    assert!(
        err.contains("Session not found"),
        "expected 'Session not found' in err message, got: {err}"
    );

    // @step And the process does not panic
    // (Reaching here proves no panic occurred.)
}

// ============================================================================
// Scenario: restore_session_messages returns Err on malformed envelope JSON
// without mutating inner.messages
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restore_messages_returns_err_on_malformed_envelope_json() {
    let _guard = DATA_DIR_GUARD.lock().await;
    // @step Given a SessionManager has created a fresh BackgroundSession with messages.len() == 0
    let data_dir =
        codelet_test_helpers::shared_temp_subdirectory().expect("shared temp subdirectory");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(data_dir.clone()).expect("set the data directory");
    codelet_core::persistence::reset_stores_for_tests();
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );
    let sid = fresh_session(&manager).await;
    let baseline_len = {
        let session = manager.get_session(&sid.value).expect("session must exist");
        let inner = session.inner.lock().await;
        inner.messages.len()
    };

    // @step When restore_session_messages is invoked with the single envelope string "{ not json"
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.restore_session_messages(&sid, vec!["{ not json".to_string()]);

    // @step Then the call returns Err whose message contains "invalid replay JSON"
    let err = result.expect_err("expected Err on malformed JSON");
    assert!(
        err.contains("invalid replay JSON"),
        "expected 'invalid replay JSON' in err message, got: {err}"
    );

    // @step And session.inner.lock().await.messages.len() still equals 0
    let session = manager.get_session(&sid.value).expect("session must exist");
    let inner = session.inner.lock().await;
    assert_eq!(
        inner.messages.len(),
        baseline_len,
        "malformed envelope must NOT mutate inner.messages"
    );
}

// ============================================================================
// Scenario: Boundary — codelet-sessions still has zero dependency on
// codelet-napi after the restoration port
// ============================================================================

#[test]
fn restoration_port_keeps_codelet_sessions_napi_free() {
    // @step Given the restore_session_messages port has landed in rust/sessions/src/handle_impl.rs
    // @step When cargo metadata is invoked for the codelet-sessions package
    // @step Then the resulting transitive package set does not contain "codelet-napi"
    codelet_test_helpers::assert_no_transitive_dependency!("codelet-sessions", "codelet-napi");

    // @step And no .rs file under rust/sessions/src/ contains the substring "codelet_napi"
    codelet_test_helpers::assert_no_import_in_sources!("sessions", "codelet_napi");
}
