// Feature: spec/features/napi-re-export-shim-for-message-envelope.feature
//
// This integration test validates RPC-031 scenarios that are NAPI-side
// observable: the re-export shim and "all NAPI persistence tests pass"
// invariants. Living in `codelet/napi/tests/` lets us consume codelet-napi
// the same way TS callers do, while also reaching the codelet-core types
// directly to assert they ARE the same types.

// Scenario coverage uses the NAPI test crate which has access to both the
// flat re-export (`codelet_napi::persistence::MessageEnvelope`) and the
// underlying lifted module (`codelet_core::persistence::message_envelope::*`).

use chrono::Utc;
use uuid::Uuid;

// @step Given the NAPI persistence module has a re-export shim file that does `pub use codelet_core::persistence::message_envelope::*;`
//
// The shim is in `codelet/napi/src/persistence/message_envelope.rs` and is
// validated at compile time by these imports. The fact that BOTH paths
// below refer to the same type is the assertion.
use codelet_napi::persistence::MessageEnvelope as NapiReexportedEnvelope;
use codelet_napi::persistence::MessagePayload as NapiReexportedPayload;
use codelet_napi::persistence::UserMessage as NapiReexportedUserMessage;
use codelet_napi::persistence::UserContent as NapiReexportedUserContent;
use codelet_napi::persistence::AssistantMessage as NapiReexportedAssistantMessage;
use codelet_napi::persistence::AssistantContent as NapiReexportedAssistantContent;

use codelet_core::persistence::message_envelope::{
    MessageEnvelope as CoreEnvelope, MessagePayload as CorePayload, UserContent as CoreUserContent,
    UserMessage as CoreUserMessage,
};

// Compile-time proof that the two paths resolve to identical types.
fn _shim_alignment() {
    // @step When an internal NAPI module writes `use crate::persistence::{MessageEnvelope, MessagePayload, UserMessage, UserContent, AssistantMessage, AssistantContent}`
    fn assert_same<T>(_: &T, _: &T) {}
    let core_env: CoreEnvelope = CoreEnvelope {
        uuid: Uuid::nil(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: CorePayload::User(CoreUserMessage {
            role: "user".to_string(),
            content: vec![CoreUserContent::Text { text: String::new() }],
        }),
        request_id: None,
    };
    let napi_env: NapiReexportedEnvelope = NapiReexportedEnvelope {
        uuid: Uuid::nil(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: NapiReexportedPayload::User(NapiReexportedUserMessage {
            role: "user".to_string(),
            content: vec![NapiReexportedUserContent::Text { text: String::new() }],
        }),
        request_id: None,
    };
    // @step Then the import resolves to the codelet-core types
    // The compiler accepts this only if the two paths name the same type.
    assert_same(&core_env, &napi_env);
}

#[test]
fn napi_shim_round_trips_envelope_via_flat_reexport_path() {
    // @step Given the NAPI persistence module has a re-export shim file that does `pub use codelet_core::persistence::message_envelope::*;`
    // (verified by the use statements above)

    // @step When an internal NAPI module writes `use crate::persistence::{MessageEnvelope, MessagePayload, UserMessage, UserContent, AssistantMessage, AssistantContent}`
    //
    // NOTE: We include an AssistantContent::ToolUse alongside the Text so the
    // serde(untagged) MessagePayload discriminator can pick the Assistant
    // variant on round-trip — UserContent has no ToolUse counterpart. This
    // mirrors how production session JSONL files look (assistant turns
    // always include tool_use events when the agent calls a tool) and is
    // the same disambiguation strategy used by the lifted core test.
    let envelope = NapiReexportedEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "assistant".to_string(),
        provider: "claude".to_string(),
        message: NapiReexportedPayload::Assistant(NapiReexportedAssistantMessage {
            role: "assistant".to_string(),
            id: Some("msg_shim_test".to_string()),
            model: Some("claude-opus-4-5-20251101".to_string()),
            content: vec![
                NapiReexportedAssistantContent::Text {
                    text: "via shim".to_string(),
                },
                NapiReexportedAssistantContent::ToolUse {
                    id: "toolu_shim_disc".to_string(),
                    name: "read_file".to_string(),
                    input: serde_json::json!({"path": "/foo.ts"}),
                },
            ],
            stop_reason: Some("end_turn".to_string()),
            usage: None,
        }),
        request_id: None,
    };

    // @step Then the import resolves to the codelet-core types
    // Round-trip through the core deserializer proves type equivalence.
    let json = serde_json::to_string(&envelope).unwrap();
    let core_round_trip: CoreEnvelope =
        serde_json::from_str(&json).expect("core deserializer must accept NAPI-emitted JSON");
    match core_round_trip.message {
        CorePayload::Assistant(msg) => assert_eq!(
            msg.id,
            Some("msg_shim_test".to_string()),
            "type identity must survive the shim"
        ),
        _ => panic!("expected Assistant payload"),
    }

    // @step And `cargo build -p codelet-napi` succeeds without modification of the importing modules
    // (verified by CI running `cargo build -p codelet-napi` after the lift; this test merely
    //  validates the runtime equivalence of the two import paths.)
}

#[test]
fn napi_persistence_tests_continue_to_pass_after_the_lift() {
    // @step Given MessageEnvelope and supporting types live in codelet-core and are re-exported by NAPI
    use codelet_napi::persistence::{
        AssistantContent, AssistantMessage, MessageEnvelope, MessagePayload, ToolUseResultMetadata,
        UserContent, UserMessage,
    };

    // @step When the existing test suites are run with `cargo test -p codelet-core` and `cargo test -p codelet-napi`
    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::ToolResult {
                tool_use_id: "toolu_abc123".to_string(),
                content: "raw".to_string(),
                is_error: false,
                tool_use_result: Some(ToolUseResultMetadata::with_output("stdout", "stderr")),
            }],
        }),
        request_id: None,
    };
    let json = serde_json::to_string(&envelope).unwrap();
    let restored: MessageEnvelope = serde_json::from_str(&json).unwrap();

    // @step Then all pre-existing message_envelope serialization tests pass against the codelet-core types
    assert_eq!(restored.message_type, "user");
    match restored.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::ToolResult { tool_use_result, .. } => {
                let meta = tool_use_result.as_ref().expect("metadata preserved");
                assert_eq!(meta.stdout, Some("stdout".to_string()));
            }
            _ => panic!("expected ToolResult"),
        },
        _ => panic!("expected User payload"),
    }

    // Force a no-op reference to AssistantContent + AssistantMessage so the
    // import is exercised — proves the full envelope surface re-exports.
    let _ = AssistantContent::Text { text: String::new() };
    let _ = AssistantMessage {
        role: "assistant".to_string(),
        id: None,
        model: None,
        content: Vec::new(),
        stop_reason: None,
        usage: None,
    };

    // @step And the `test_blob_threshold` test referencing `crate::persistence::should_use_blob_storage` still passes from the NAPI shim
    // The test_blob_threshold test lives in the NAPI shim's #[cfg(test)] block
    // and is exercised by `cargo test -p codelet-napi persistence::message_envelope::tests`.
    // Asserting here that `should_use_blob_storage` is still reachable via the
    // NAPI persistence flat re-export validates the shim hasn't broken its
    // module structure.
    use codelet_napi::persistence::should_use_blob_storage;
    let small = vec![0u8; 100];
    let large = vec![0u8; 20_000];
    assert!(!should_use_blob_storage(&small));
    assert!(should_use_blob_storage(&large));
}
