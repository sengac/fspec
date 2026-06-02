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

use std::sync::Arc;
use std::time::Duration;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{SessionId, StreamChunk};
use codelet_sessions::SessionManager;
use tokio::sync::broadcast;

/// Drain all currently-broadcasted chunks for the given session,
/// waiting up to `total_timeout` and collecting everything that
/// arrives until either (a) a `Done` is observed, OR (b) the timeout
/// elapses, OR (c) `recv` errors. The Done chunk is included in the
/// returned vector when present.
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
                if session_id.value != sid {
                    continue;
                }
                let is_done = matches!(chunk, StreamChunk::Done);
                out.push(chunk);
                if is_done {
                    return out;
                }
            }
            Ok(Err(_)) => return out,
            Err(_) => {
                // Tick timed out — if we already have something, return.
                if !out.is_empty() {
                    return out;
                }
            }
        }
    }
    out
}

/// Create a fresh BackgroundSession via the SessionManagerHandle bridge.
/// The Noop hooks ensure no agent loop is spawned for the session.
async fn fresh_session(manager: &SessionManager) -> SessionId {
    manager.set_default_model("anthropic/claude-opus-4-5");
    let handle: &dyn SessionManagerHandle = manager;
    handle.create_session(None)
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
// Scenario: restore_session_messages replays a one-user-one-assistant
// transcript into inner.messages and the output stream
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restore_messages_replays_user_and_assistant_text_into_inner_and_output() {
    // @step Given a SessionManager has created a fresh BackgroundSession via SessionManagerHandle
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
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
    assert!(result.is_ok(), "restore_session_messages must succeed: {result:?}");

    // @step Then session.inner.lock().await.messages.len() equals 2 (post-restoration delta)
    let session = manager.get_session(&sid.value).expect("session must exist");
    let inner = session.inner.lock().await;
    let added = inner.messages.len() - baseline_len;
    assert_eq!(added, 2, "expected 2 rig messages to be appended, got {added}");
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

    // @step And the broadcasted StreamChunks for that session are, in order, UserInput("hello"), Text("hi back"), Done
    let chunks = drain_chunks_for(&mut chunks_rx, &sid.value, Duration::from_secs(2)).await;
    let user_inputs: Vec<&str> = chunks
        .iter()
        .filter_map(|c| match c {
            StreamChunk::UserInput { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    let texts: Vec<&str> = chunks
        .iter()
        .filter_map(|c| match c {
            StreamChunk::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    let done_count = chunks.iter().filter(|c| matches!(c, StreamChunk::Done)).count();
    assert_eq!(user_inputs, vec!["hello"], "UserInput chunks mismatch");
    assert_eq!(texts, vec!["hi back"], "Text chunks mismatch");
    assert!(done_count >= 1, "expected at least one Done chunk, got {done_count}");
}

// ============================================================================
// Scenario: Assistant restoration replays thinking, text, and tool_use blocks
// then a terminating Done
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restore_messages_replays_assistant_thinking_text_and_tool_use() {
    // @step Given a SessionManager has created a fresh BackgroundSession via SessionManagerHandle
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
    let mut chunks_rx = manager.chunks_tx().subscribe();
    let sid = fresh_session(&manager).await;
    let baseline_len = {
        let session = manager.get_session(&sid.value).expect("session must exist");
        let inner = session.inner.lock().await;
        inner.messages.len()
    };

    // @step And an assistant MessageEnvelope JSON whose content is [{"type":"thinking", ...}, {"type":"text", ...}, {"type":"tool_use", ...}]
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
    assert!(result.is_ok(), "restore_session_messages must succeed: {result:?}");

    // @step Then the broadcasted StreamChunks for that session are, in order, Thinking("hmm"), Text("reading"), ToolCall{...}, Done
    let chunks = drain_chunks_for(&mut chunks_rx, &sid.value, Duration::from_secs(2)).await;
    // Filter the relevant content chunks (strip session_state_change/etc. noise).
    let relevant: Vec<&StreamChunk> = chunks
        .iter()
        .filter(|c| {
            matches!(
                c,
                StreamChunk::Thinking { .. }
                    | StreamChunk::Text { .. }
                    | StreamChunk::ToolCall { .. }
                    | StreamChunk::Done
            )
        })
        .collect();
    assert_eq!(
        relevant.len(),
        4,
        "expected 4 relevant chunks (Thinking, Text, ToolCall, Done); got {} — full chunks: {:?}",
        relevant.len(),
        chunks
    );
    match relevant[0] {
        StreamChunk::Thinking { thinking, .. } => assert_eq!(thinking, "hmm"),
        other => panic!("expected Thinking at [0], got {other:?}"),
    }
    match relevant[1] {
        StreamChunk::Text { text, .. } => assert_eq!(text, "reading"),
        other => panic!("expected Text at [1], got {other:?}"),
    }
    match relevant[2] {
        StreamChunk::ToolCall { tool_call, .. } => {
            assert_eq!(tool_call.id, "t1");
            assert_eq!(tool_call.name, "Read");
            let parsed: serde_json::Value =
                serde_json::from_str(&tool_call.input).expect("ToolCall.input must be valid JSON");
            assert_eq!(parsed, serde_json::json!({ "path": "/tmp/x" }));
        }
        other => panic!("expected ToolCall at [2], got {other:?}"),
    }
    assert!(matches!(relevant[3], StreamChunk::Done));

    // @step And session.inner.lock().await.messages contains exactly one rig::message::Message::Assistant whose joined text equals "reading"
    let session = manager.get_session(&sid.value).expect("session must exist");
    let inner = session.inner.lock().await;
    let added = inner.messages.len() - baseline_len;
    assert_eq!(added, 1, "expected exactly 1 restored message, got {added}");
    let restored = &inner.messages[baseline_len..];
    match &restored[0] {
        rig::message::Message::Assistant { content, .. } => {
            let joined: String = content
                .iter()
                .map(|ac| match ac {
                    rig::message::AssistantContent::Text(t) => t.text.clone(),
                    _ => String::new(),
                })
                .collect();
            assert_eq!(joined, "reading");
        }
        other => panic!("expected Assistant at restored[0], got {other:?}"),
    }
}

// ============================================================================
// Scenario: User restoration replays tool_result blocks as StreamChunk::ToolResult
// and does not append to inner messages
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restore_messages_replays_tool_result_without_appending_inner() {
    // @step Given a SessionManager has created a fresh BackgroundSession via SessionManagerHandle
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
    let mut chunks_rx = manager.chunks_tx().subscribe();
    let sid = fresh_session(&manager).await;
    let baseline_len = {
        let session = manager.get_session(&sid.value).expect("session must exist");
        let inner = session.inner.lock().await;
        inner.messages.len()
    };

    // @step And a user MessageEnvelope JSON whose content is [{"type":"tool_result", ...}]
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
    assert!(result.is_ok(), "restore_session_messages must succeed: {result:?}");

    // @step Then the broadcasted StreamChunks for that session contain a ToolResult with tool_call_id "t1" and content "contents" and is_error false
    let chunks = drain_chunks_for(&mut chunks_rx, &sid.value, Duration::from_secs(1)).await;
    let tool_results: Vec<_> = chunks
        .iter()
        .filter_map(|c| match c {
            StreamChunk::ToolResult { tool_result, .. } => Some(tool_result),
            _ => None,
        })
        .collect();
    assert_eq!(tool_results.len(), 1, "expected exactly one ToolResult chunk");
    let tr = tool_results[0];
    assert_eq!(tr.tool_call_id, "t1");
    assert_eq!(tr.content, "contents");
    assert!(!tr.is_error);

    // @step And session.inner.lock().await.messages.len() equals 0 (no new messages from tool_result-only envelope)
    let session = manager.get_session(&sid.value).expect("session must exist");
    let inner = session.inner.lock().await;
    let added = inner.messages.len() - baseline_len;
    assert_eq!(
        added, 0,
        "tool_result-only envelope must NOT append to inner.messages (added: {added})"
    );
}

// ============================================================================
// Scenario: System-reminder envelopes are silently skipped during restoration
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restore_messages_skips_system_reminder_envelopes_silently() {
    // @step Given a SessionManager has created a fresh BackgroundSession via SessionManagerHandle
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
    let mut chunks_rx = manager.chunks_tx().subscribe();
    let sid = fresh_session(&manager).await;
    let baseline_len = {
        let session = manager.get_session(&sid.value).expect("session must exist");
        let inner = session.inner.lock().await;
        inner.messages.len()
    };

    // @step And a user MessageEnvelope JSON whose content contains a stale system-reminder
    let envelope = user_envelope(serde_json::json!([
        {
            "type": "text",
            "text": "<system-reminder>\n<!-- type:fspecWorkflow -->\nstale\n</system-reminder>"
        }
    ]));

    // @step When restore_session_messages is invoked with that envelope via SessionManagerHandle
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.restore_session_messages(&sid, vec![envelope]);
    assert!(result.is_ok(), "restore_session_messages must succeed: {result:?}");

    // @step Then session.inner.lock().await.messages.len() equals 0 (delta from baseline)
    let session = manager.get_session(&sid.value).expect("session must exist");
    let inner = session.inner.lock().await;
    let added = inner.messages.len() - baseline_len;
    assert_eq!(
        added, 0,
        "system-reminder envelope must NOT append to inner.messages (added: {added})"
    );
    drop(inner);

    // @step And no StreamChunk is broadcasted to the session's output for that envelope
    let chunks = drain_chunks_for(&mut chunks_rx, &sid.value, Duration::from_millis(500)).await;
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
    // @step Given a SessionManagerHandle whose underlying SessionManager has no session registered under the id "00000000-0000-0000-0000-000000000000"
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
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
    // @step Given a SessionManager has created a fresh BackgroundSession with baseline inner.messages.len() == N
    let data_dir = tempfile::tempdir().expect("tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());
    let manager = Arc::new(SessionManager::new());
    let sid = fresh_session(&manager).await;
    let baseline_len = {
        let session = manager.get_session(&sid.value).expect("session must exist");
        let inner = session.inner.lock().await;
        inner.messages.len()
    };

    // @step When restore_session_messages is invoked with the single envelope string "{ not json"
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.restore_session_messages(&sid, vec!["{ not json".to_string()]);

    // @step Then the call returns Err whose message contains "Failed to parse envelope"
    let err = result.expect_err("expected Err on malformed JSON");
    assert!(
        err.contains("Failed to parse envelope"),
        "expected 'Failed to parse envelope' in err message, got: {err}"
    );

    // @step And session.inner.lock().await.messages.len() still equals N (no mutation on parse failure)
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
    // @step Given the restore_session_messages port has landed in codelet/sessions/src/handle_impl.rs
    // @step When cargo metadata is invoked for the codelet-sessions package
    // @step Then the resulting transitive package set does not contain "codelet-napi"
    codelet_test_helpers::assert_no_transitive_dependency!("codelet-sessions", "codelet-napi");

    // @step And no .rs file under codelet/sessions/src/ contains the substring "codelet_napi"
    codelet_test_helpers::assert_no_import_in_sources!("sessions", "codelet_napi");
}
