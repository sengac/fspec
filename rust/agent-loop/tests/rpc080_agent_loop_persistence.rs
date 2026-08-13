//! Feature: spec/features/agent-loop-persistence.feature
//!
//! RPC-080: Agent loop persists user, assistant, tool_result, and token state per turn.
//!
//! These tests are organised in two groups:
//!
//!   - Behavioural tests (#[serial] + tempfile + set_data_directory) that
//!     drive the persist_* helpers directly against a hermetic on-disk
//!     manifest and read back the persisted MessageEnvelope shape via
//!     codelet_core::persistence::get_session_messages_full /
//!     load_session.
//!
//!   - Source-shape tests that scan the canonical call-site files in
//!     rust/agent-loop/src/ to pin Rules [0]-[7] against future
//!     drift (assistant-flush before tool_result, persist_token_state
//!     on Done, persist_assistant_message on Error/Interrupted, no
//!     codelet_napi::persist references).
//!
//! All behavioural tests serialise via `serial_test::serial` because
//! they reach for the process-global data directory + MESSAGE_STORE /
//! SESSION_STORE singletons in codelet_core::persistence.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use codelet_agent_loop::persist::{
    persist_assistant_message_internal, persist_token_state, persist_tool_result_internal,
    persist_user_message,
};
use codelet_core::persistence::{
    create_session, get_session_messages_full, load_session, reset_stores_for_tests,
    AssistantContent, MessageEnvelope, MessagePayload, StoredMessage, UserContent,
};
use serial_test::serial;
use tempfile::TempDir;
use uuid::Uuid;

/// Configure a unique temp data dir for the test and return the guard.
fn setup_data_dir() -> TempDir {
    let tmp = tempfile::tempdir().expect("create temp data dir");
    codelet_common::set_data_directory(tmp.path().to_path_buf())
        .expect("set_data_directory must succeed");
    reset_stores_for_tests();
    tmp
}

/// Create a hermetic session and return (session_id, _data_dir_guard).
fn fresh_session(name: &str) -> (Uuid, TempDir) {
    let guard = setup_data_dir();
    let project = PathBuf::from("/test/project/rpc080");
    let session = create_session(name, &project).expect("create_session");
    (session.id, guard)
}

/// Read back the persisted stored messages for a session, then
/// reconstruct each MessageEnvelope by round-tripping the metadata
/// HashMap (which is exactly the envelope-as-JSON the persist_*
/// helpers wrote) through serde_json.
///
/// NOTE: `MessagePayload` is `#[serde(untagged)]` and `UserContent::Text`
/// / `AssistantContent::Text` are isomorphic on the wire, so
/// deserialising back into the typed enum can collapse an Assistant
/// envelope into the User variant. We therefore expose the raw
/// metadata JSON alongside the typed envelope so assistant-shape
/// scenarios can inspect `message.stop_reason` from the JSON directly.
fn read_envelopes(session_id: Uuid) -> Vec<MessageEnvelope> {
    let manifest = load_session(session_id).expect("load_session after persist");
    let stored: Vec<StoredMessage> =
        get_session_messages_full(&manifest).expect("get_session_messages_full");
    stored
        .into_iter()
        .map(|m| {
            let metadata_json = serde_json::to_value(&m.metadata)
                .expect("metadata HashMap must serialise back to JSON");
            serde_json::from_value::<MessageEnvelope>(metadata_json)
                .expect("metadata must round-trip back to a MessageEnvelope")
        })
        .collect()
}

/// Read the raw envelope JSON for each persisted message — preserves
/// the on-disk shape without being lossy on `#[serde(untagged)]`
/// variants.
fn read_envelope_jsons(session_id: Uuid) -> Vec<serde_json::Value> {
    let manifest = load_session(session_id).expect("load_session after persist");
    let stored: Vec<StoredMessage> =
        get_session_messages_full(&manifest).expect("get_session_messages_full");
    stored
        .into_iter()
        .map(|m| {
            serde_json::to_value(&m.metadata).expect("metadata HashMap must serialise back to JSON")
        })
        .collect()
}

// ============================================================================
// Scenario: persist_user_message writes a User MessageEnvelope before the LLM stream begins
// ============================================================================

#[test]
#[serial]
fn persist_user_message_writes_user_envelope() {
    // @step Given a hermetic session manifest exists on disk for session id S
    let (session_id, _guard) = fresh_session("user-msg-envelope");

    // @step When persist_user_message is invoked with session id S and text "hello"
    persist_user_message(&session_id, "hello").expect("persist_user_message must succeed");

    // @step Then the manifest gains exactly one MessageEnvelope with message_type "user"
    let envelopes = read_envelopes(session_id);
    assert_eq!(
        envelopes.len(),
        1,
        "exactly one envelope expected after persist_user_message"
    );
    assert_eq!(envelopes[0].message_type, "user");

    // @step And the envelope's payload is a UserMessage with content [Text("hello")]
    match &envelopes[0].message {
        MessagePayload::User(user_msg) => {
            assert_eq!(user_msg.content.len(), 1, "exactly one content block");
            match &user_msg.content[0] {
                UserContent::Text { text } => assert_eq!(text, "hello"),
                other => panic!("expected Text content, got {other:?}"),
            }
        }
        other => panic!("expected MessagePayload::User, got {other:?}"),
    }

    // @step And the envelope's provider is the literal string "user"
    assert_eq!(envelopes[0].provider, "user");
}

// ============================================================================
// Scenario: persist_assistant_message_internal writes an Assistant envelope with provider and stop_reason
// ============================================================================

#[test]
#[serial]
fn persist_assistant_message_writes_assistant_envelope_with_stop_reason() {
    // @step Given a hermetic session manifest exists on disk for session id S
    let (session_id, _guard) = fresh_session("assistant-stop-reason");

    // @step When persist_assistant_message_internal is invoked with provider "stub" and content [Text("hi back")] and stop_reason Some("end_turn")
    let content = vec![AssistantContent::Text {
        text: "hi back".to_string(),
    }];
    persist_assistant_message_internal(&session_id, "stub", content, Some("end_turn".to_string()))
        .expect("persist_assistant_message_internal must succeed");

    // @step Then the manifest gains exactly one MessageEnvelope with message_type "assistant"
    let jsons = read_envelope_jsons(session_id);
    assert_eq!(jsons.len(), 1, "exactly one assistant envelope");
    assert_eq!(
        jsons[0].get("type").and_then(|v| v.as_str()),
        Some("assistant"),
        "envelope's on-disk `type` field must equal \"assistant\""
    );

    // @step And the envelope's provider equals "stub"
    assert_eq!(
        jsons[0].get("provider").and_then(|v| v.as_str()),
        Some("stub")
    );

    // @step And the envelope's stop_reason equals "end_turn"
    assert_eq!(
        jsons[0]
            .pointer("/message/stop_reason")
            .and_then(|v| v.as_str()),
        Some("end_turn")
    );
}

// ============================================================================
// Scenario: PROV-039 — stop_reason=None becomes the literal "unknown" on disk
// ============================================================================

#[test]
#[serial]
fn persist_assistant_message_none_stop_reason_becomes_unknown() {
    // @step Given a hermetic session manifest exists on disk for session id S
    let (session_id, _guard) = fresh_session("assistant-stop-none");

    // @step When persist_assistant_message_internal is invoked with stop_reason None
    let content = vec![AssistantContent::Text {
        text: "no stop reason here".to_string(),
    }];
    persist_assistant_message_internal(&session_id, "stub", content, None)
        .expect("persist_assistant_message_internal None must succeed");

    // @step Then the on-disk Assistant envelope's stop_reason equals "unknown"
    let jsons = read_envelope_jsons(session_id);
    assert_eq!(jsons.len(), 1);
    let stop_reason = jsons[0]
        .pointer("/message/stop_reason")
        .and_then(|v| v.as_str())
        .map(String::from);
    assert_eq!(stop_reason.as_deref(), Some("unknown"));

    // @step And the stop_reason is NOT the legacy sentinel "end_turn"
    assert_ne!(stop_reason.as_deref(), Some("end_turn"));
}

// ============================================================================
// Scenario: persist_tool_result_internal writes a User envelope tagged with provider "tool"
// ============================================================================

#[test]
#[serial]
fn persist_tool_result_writes_user_envelope_tagged_tool() {
    // @step Given a hermetic session manifest exists on disk for session id S
    let (session_id, _guard) = fresh_session("tool-result");

    // @step When persist_tool_result_internal is invoked with tool_call_id "call_abc", content "contents", is_error false
    persist_tool_result_internal(&session_id, "call_abc", "contents", false)
        .expect("persist_tool_result_internal must succeed");

    // @step Then the manifest gains exactly one MessageEnvelope with message_type "user"
    let envelopes = read_envelopes(session_id);
    assert_eq!(envelopes.len(), 1, "exactly one tool_result envelope");
    assert_eq!(envelopes[0].message_type, "user");

    // @step And the envelope's provider equals the literal string "tool"
    assert_eq!(envelopes[0].provider, "tool");

    // @step And the envelope's payload is a UserMessage with a ToolResult content whose tool_use_id is "call_abc"
    // @step And the ToolResult content equals "contents" and is_error is false
    match &envelopes[0].message {
        MessagePayload::User(user_msg) => {
            assert_eq!(user_msg.content.len(), 1);
            match &user_msg.content[0] {
                UserContent::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                    ..
                } => {
                    assert_eq!(tool_use_id, "call_abc");
                    assert_eq!(content, "contents");
                    assert!(!*is_error);
                }
                other => panic!("expected ToolResult content, got {other:?}"),
            }
        }
        other => panic!("expected MessagePayload::User, got {other:?}"),
    }
}

// ============================================================================
// Scenario: persist_token_state updates the session manifest's cumulative token totals
// ============================================================================

#[test]
#[serial]
fn persist_token_state_updates_session_manifest() {
    // @step Given a hermetic session manifest exists on disk for session id S
    let (session_id, _guard) = fresh_session("token-state");

    // @step When persist_token_state is invoked with input_tokens 100, output_tokens 50
    persist_token_state(&session_id, 100, 50).expect("persist_token_state must succeed");

    // @step Then the manifest's persisted token state shows input_tokens 100 and output_tokens 50
    let manifest = load_session(session_id).expect("reload manifest");
    assert_eq!(
        manifest.token_usage.cumulative_billed_input, 100,
        "cumulative_billed_input must reflect persisted input tokens"
    );
    assert_eq!(
        manifest.token_usage.cumulative_billed_output, 50,
        "cumulative_billed_output must reflect persisted output tokens"
    );
}

// ============================================================================
// Scenario: persist_user_message returns Err on a missing manifest without panicking
// ============================================================================

#[test]
#[serial]
fn persist_user_message_returns_err_on_missing_manifest() {
    // @step Given no manifest exists on disk for an arbitrary session id S
    let _guard = setup_data_dir();
    let missing_id = Uuid::new_v4();

    // @step When persist_user_message is invoked with session id S and text "hello"
    let result = persist_user_message(&missing_id, "hello");

    // @step Then persist_user_message returns Err(String) referencing the load failure
    assert!(
        result.is_err(),
        "expected Err on missing manifest, got: {result:?}"
    );
    // @step And no thread panics
    // (Asserted implicitly: this line is only reached because no panic
    // occurred inside persist_user_message.)
}

// ============================================================================
// Source-shape helpers
// ============================================================================

fn read_agent_loop_src(filename: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(filename);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "must be able to read rust/agent-loop/src/{filename} at {}: {e}",
            path.display()
        )
    })
}

// ============================================================================
// Scenario: Source-shape — agent_loop body invokes persist_user_message before dispatching to the provider
// ============================================================================

#[test]
fn agent_loop_invokes_persist_user_message_before_provider_dispatch() {
    // @step Given the source of rust/agent-loop/src/agent_loop.rs
    let src = read_agent_loop_src("agent_loop.rs");

    // @step When the file is scanned

    // @step Then it imports persist_user_message from crate::persist
    assert!(
        src.contains("use crate::persist::") && src.contains("persist_user_message"),
        "agent_loop.rs must import persist_user_message from crate::persist"
    );

    // @step And the function body contains a call to persist_user_message(&session.id, input)
    assert!(
        src.contains("persist_user_message(&session.id, input)"),
        "agent_loop.rs must call persist_user_message(&session.id, input)"
    );

    // @step And that call is followed by the provider dispatch
    // (no provider dispatch precedes it within the same turn block)
    let persist_pos = src
        .find("persist_user_message(&session.id, input)")
        .expect("persist_user_message call site must exist");
    let set_running_pos = src[persist_pos..]
        .find("set_status(SessionStatus::Running)")
        .map(|i| i + persist_pos);
    assert!(
        set_running_pos.is_some(),
        "set_status(SessionStatus::Running) must come AFTER persist_user_message"
    );
}

// ============================================================================
// Scenario: Source-shape — BackgroundOutput's StreamEvent::ToolResult arm
// persists assistant before tool_result
// ============================================================================

#[test]
fn background_output_tool_result_arm_persists_assistant_first() {
    // @step Given the source of rust/agent-loop/src/background_output.rs
    let src = read_agent_loop_src("background_output.rs");

    // @step When the file is scanned
    let arm_start = src
        .find("StreamEvent::ToolResult(ref tr)")
        .expect("StreamEvent::ToolResult arm must exist");
    let arm_end = src[arm_start..]
        .find("StreamEvent::ToolProgress(tp)")
        .map(|i| i + arm_start)
        .expect("ToolResult arm must be followed by another arm");
    let arm = &src[arm_start..arm_end];

    // @step Then the StreamEvent::ToolResult arm calls self.persist_assistant_message()
    let assistant_flush_pos = arm
        .find("self.persist_assistant_message()")
        .expect("ToolResult arm must call self.persist_assistant_message()");

    // @step And the same arm subsequently calls persist_tool_result_internal(...)
    let tool_result_pos = arm
        .find("persist_tool_result_internal(")
        .expect("ToolResult arm must call persist_tool_result_internal(...)");

    // @step And the assistant-flush call precedes the tool-result persist call
    // textually within that arm
    assert!(
        assistant_flush_pos < tool_result_pos,
        "self.persist_assistant_message() must precede persist_tool_result_internal(...) in the ToolResult arm"
    );
}

// ============================================================================
// Scenario: Source-shape — BackgroundOutput's StreamEvent::Done arm
// persists assistant then token state
// ============================================================================

#[test]
fn background_output_done_arm_persists_assistant_then_token_state() {
    // @step Given the source of rust/agent-loop/src/background_output.rs
    let src = read_agent_loop_src("background_output.rs");

    // @step When the file is scanned
    let arm_start = src
        .find("StreamEvent::Done(stop_reason)")
        .expect("StreamEvent::Done arm must exist");
    let arm_end = src[arm_start..]
        .find("StreamEvent::CompactionStarted")
        .map(|i| i + arm_start)
        .expect("Done arm must be followed by CompactionStarted");
    let arm = &src[arm_start..arm_end];

    // @step Then the StreamEvent::Done arm calls
    // self.persist_assistant_message_with_stop_reason(stop_reason)
    let assistant_pos = arm
        .find("self.persist_assistant_message_with_stop_reason(stop_reason)")
        .expect("Done arm must call self.persist_assistant_message_with_stop_reason(stop_reason)");

    // @step And the same arm subsequently calls
    // persist_token_state(&self.session.id, input_tokens, output_tokens)
    let token_pos = arm
        .find("persist_token_state(&self.session.id, input_tokens, output_tokens)")
        .expect(
            "Done arm must call persist_token_state(&self.session.id, input_tokens, output_tokens)",
        );

    assert!(
        assistant_pos < token_pos,
        "persist_assistant_message_with_stop_reason must precede persist_token_state in the Done arm"
    );
}

// ============================================================================
// Scenario: Source-shape — BackgroundOutput's StreamEvent::Error arm
// flushes accumulated assistant content
// ============================================================================

#[test]
fn background_output_error_arm_flushes_assistant() {
    // @step Given the source of rust/agent-loop/src/background_output.rs
    let src = read_agent_loop_src("background_output.rs");

    // @step When the file is scanned
    let arm_start = src
        .find("StreamEvent::Error(error)")
        .expect("StreamEvent::Error arm must exist");
    let arm_end = src[arm_start..]
        .find("StreamEvent::Interrupted(queued)")
        .map(|i| i + arm_start)
        .expect("Error arm must be followed by Interrupted");
    let arm = &src[arm_start..arm_end];

    // @step Then the StreamEvent::Error arm calls self.persist_assistant_message()
    assert!(
        arm.contains("self.persist_assistant_message()"),
        "Error arm must call self.persist_assistant_message() to flush partial content"
    );
}

// ============================================================================
// Scenario: Source-shape — BackgroundOutput's StreamEvent::Interrupted arm
// flushes accumulated assistant content
// ============================================================================

#[test]
fn background_output_interrupted_arm_flushes_assistant() {
    // @step Given the source of rust/agent-loop/src/background_output.rs
    let src = read_agent_loop_src("background_output.rs");

    // @step When the file is scanned
    let arm_start = src
        .find("StreamEvent::Interrupted(queued)")
        .expect("StreamEvent::Interrupted arm must exist");
    let arm_end = src[arm_start..]
        .find("StreamEvent::Done(stop_reason)")
        .map(|i| i + arm_start)
        .expect("Interrupted arm must be followed by Done");
    let arm = &src[arm_start..arm_end];

    // @step Then the StreamEvent::Interrupted arm calls self.persist_assistant_message()
    assert!(
        arm.contains("self.persist_assistant_message()"),
        "Interrupted arm must call self.persist_assistant_message() to flush partial content"
    );
}

// ============================================================================
// Scenario: Boundary — persistence calls in codelet-agent-loop import from
// crate::persist (not codelet_napi)
// ============================================================================

#[test]
fn persistence_calls_in_agent_loop_use_crate_persist_not_napi() {
    // @step Given the codelet-agent-loop crate
    let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");

    // @step When its source tree is scanned
    let mut violations: Vec<String> = Vec::new();
    for entry in walk_rs_files(&src_dir) {
        let body = std::fs::read_to_string(&entry)
            .unwrap_or_else(|e| panic!("must read {}: {e}", entry.display()));

        // @step Then no .rs file under rust/agent-loop/src/ references codelet_napi::persist
        if body.contains("codelet_napi::persist") || body.contains("codelet_napi :: persist") {
            violations.push(format!(
                "{}: references codelet_napi::persist",
                entry.display()
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "codelet_napi::persist must not be referenced from rust/agent-loop/src/. \
         Violations:\n{}",
        violations.join("\n")
    );

    // @step And persist.rs lives at rust/agent-loop/src/persist.rs
    let persist_path = src_dir.join("persist.rs");
    assert!(
        persist_path.is_file(),
        "rust/agent-loop/src/persist.rs must exist"
    );

    // @step And it exports persist_user_message, persist_assistant_message_internal,
    // persist_tool_result_internal, and persist_token_state as pub
    let persist_src = std::fs::read_to_string(&persist_path)
        .unwrap_or_else(|e| panic!("must read persist.rs: {e}"));
    for name in [
        "pub fn persist_user_message",
        "pub fn persist_assistant_message_internal",
        "pub fn persist_tool_result_internal",
        "pub fn persist_token_state",
    ] {
        assert!(
            persist_src.contains(name),
            "persist.rs must export `{name}` as a pub fn"
        );
    }
}

/// Recursively collect all .rs files under `dir`.
fn walk_rs_files(dir: &PathBuf) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|e| panic!("must read_dir {}: {e}", dir.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk_rs_files(&path));
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    out
}
