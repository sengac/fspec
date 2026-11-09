//! Behavioural scenarios: drive the persist_* helpers against a hermetic
//! on-disk manifest and read the persisted envelopes back.

use super::*;
use codelet_agent_loop::persist::{
    persist_assistant_message_internal, persist_token_state, persist_tool_result_internal,
    persist_user_message,
};
use codelet_core::persistence::{AssistantContent, MessagePayload, UserContent};
use serial_test::serial;

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
    persist_token_state(&session_id, 100, 50, 0).expect("persist_token_state must succeed");

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
