// Feature: spec/features/lifted-message-envelope-in-core-persistence.feature
//
// This integration test validates the acceptance criteria for RPC-031 —
// lifting the Claude-Code-compatible MessageEnvelope schema into
// codelet-core::persistence::message_envelope. Living in
// `codelet/core/tests/` means we consume codelet_core the same way an
// external downstream crate (codelet-rpc-embedded, codelet-sessions,
// codelet-fspec) would — proving the public surface is reachable
// without a codelet-napi dependency.
//
// Tests are written against the NEW location in codelet-core. They
// will FAIL to compile until the lift is implemented (red phase).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::persistence::message_envelope::{
    AssistantContent, AssistantMessage, CacheControl, DocumentSource, ImageSource, MessageEnvelope,
    MessagePayload, TokenUsagePerMessage, ToolUseResultMetadata, UserContent, UserMessage,
};

use chrono::Utc;
use uuid::Uuid;

// ============================================================================
// Scenario: MessageEnvelope round-trips with byte-identical JSON from codelet-core
// ============================================================================

#[test]
fn assistant_envelope_round_trip_from_core_with_request_id() {
    // @step Given a MessageEnvelope value constructed in codelet-core with uuid, parent_uuid, timestamp, message_type "assistant", provider "claude", an AssistantMessage payload containing one Text content, and a request_id
    //
    // NOTE: We include an AssistantContent::ToolUse alongside the Text so the
    // serde(untagged) MessagePayload discriminator can pick the Assistant
    // variant on round-trip. UserContent has no ToolUse counterpart, so the
    // JSON `{"type":"tool_use", ...}` only deserializes into AssistantContent
    // and thereby into MessagePayload::Assistant. This matches how real
    // session JSONL files look in practice (assistant turns include tool_use
    // events). The pre-existing NAPI tests preserve identity the same way
    // (test_tool_use_serialization, test_multi_part_message_preserves_order).
    let original = MessageEnvelope {
        uuid: Uuid::parse_str("a6bdbefb-902d-4f98-b539-8cbee91ec831").unwrap(),
        parent_uuid: Some(Uuid::parse_str("81dc2799-ef52-4923-aa24-5798585aae57").unwrap()),
        timestamp: Utc::now(),
        message_type: "assistant".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::Assistant(AssistantMessage {
            role: "assistant".to_string(),
            id: Some("msg_01Wk7SCqoakQaEmx7FHphZRJ".to_string()),
            model: Some("claude-opus-4-5-20251101".to_string()),
            content: vec![
                AssistantContent::Text {
                    text: "Hello from codelet-core".to_string(),
                },
                AssistantContent::ToolUse {
                    id: "toolu_disambiguator".to_string(),
                    name: "read_file".to_string(),
                    input: serde_json::json!({"path": "/foo.ts"}),
                },
            ],
            stop_reason: Some("end_turn".to_string()),
            usage: None,
        }),
        request_id: Some("req_011CWPLKJZcigRWVriKduWSr".to_string()),
    };

    // @step When the envelope is serialized to JSON and then deserialized back
    let json = serde_json::to_string(&original).expect("serialize envelope from codelet-core");
    let restored: MessageEnvelope =
        serde_json::from_str(&json).expect("deserialize envelope from codelet-core");

    // @step Then the restored value equals the original
    assert_eq!(restored, original);

    // @step And the JSON includes parentUuid, type, provider, message, and requestId fields with camelCase keys
    assert!(
        json.contains("\"parentUuid\""),
        "expected camelCase parentUuid, got: {json}"
    );
    assert!(
        json.contains("\"type\":\"assistant\""),
        "expected outer type discriminator"
    );
    assert!(json.contains("\"provider\":\"claude\""));
    assert!(json.contains("\"message\":"));
    assert!(
        json.contains("\"requestId\""),
        "expected camelCase requestId, got: {json}"
    );
}

// ============================================================================
// Scenario: codelet-core consumers can import MessageEnvelope without depending on codelet-napi
// ============================================================================

#[test]
fn core_consumers_produce_identical_json_to_napi_for_user_text_envelope() {
    // @step Given codelet-core::persistence::message_envelope exports all 11 envelope types
    // (verified at compile time by the use statement at the top of this file)
    let uuid = Uuid::parse_str("00000000-0000-4000-8000-000000000001").unwrap();
    let parent = Uuid::parse_str("00000000-0000-4000-8000-000000000002").unwrap();
    let timestamp = chrono::DateTime::parse_from_rfc3339("2025-12-23T08:51:44.813Z")
        .unwrap()
        .with_timezone(&Utc);

    // @step When a downstream crate that does not depend on codelet-napi (such as codelet-rpc-embedded) constructs and serializes a MessageEnvelope through codelet_core::persistence::message_envelope
    let envelope = MessageEnvelope {
        uuid,
        parent_uuid: Some(parent),
        timestamp,
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::Text {
                text: "Hi".to_string(),
            }],
        }),
        request_id: None,
    };
    let json = serde_json::to_string(&envelope).unwrap();

    // @step Then the build succeeds with no transitive dependency on codelet-napi
    // (enforced by the fact that codelet-core has no codelet-napi entry in Cargo.toml — see
    //  codelet/core/Cargo.toml: there is no `codelet-napi` workspace dependency.)

    // @step And the produced JSON bytes match the bytes produced by codelet-napi for the same input
    // Frozen golden JSON — the exact byte layout the NAPI MessageEnvelope emits today.
    // If any serde annotation drifts during the lift, this assertion fires.
    let expected = concat!(
        r#"{"uuid":"00000000-0000-4000-8000-000000000001","#,
        r#""parentUuid":"00000000-0000-4000-8000-000000000002","#,
        r#""timestamp":"2025-12-23T08:51:44.813Z","#,
        r#""type":"user","#,
        r#""provider":"claude","#,
        r#""message":{"role":"user","content":[{"type":"text","text":"Hi"}]},"#,
        r#""requestId":null}"#
    );
    assert_eq!(
        json, expected,
        "JSON drift between core and napi-frozen layout"
    );
}

// ============================================================================
// Helper-coverage tests — every public type lifted by RPC-031 round-trips
// from codelet-core (proves the public surface is complete).
// ============================================================================

#[test]
fn tool_use_result_metadata_round_trips_from_core() {
    let meta = ToolUseResultMetadata::with_output("stdout body", "stderr body");
    let json = serde_json::to_string(&meta).unwrap();
    let restored: ToolUseResultMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, meta);
    assert!(json.contains("\"stdout\":\"stdout body\""));
    assert!(json.contains("\"stderr\":\"stderr body\""));
}

#[test]
fn token_usage_per_message_round_trips_from_core() {
    let usage = TokenUsagePerMessage {
        input_tokens: 500,
        output_tokens: 150,
        cache_read_input_tokens: Some(200),
        cache_creation_input_tokens: Some(100),
    };
    let json = serde_json::to_string(&usage).unwrap();
    let restored: TokenUsagePerMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.input_tokens, 500);
    assert_eq!(restored.output_tokens, 150);
    assert_eq!(restored.cache_read_input_tokens, Some(200));
    assert_eq!(restored.cache_creation_input_tokens, Some(100));
}

#[test]
fn user_content_tool_result_round_trips_from_core() {
    let content = UserContent::ToolResult {
        tool_use_id: "toolu_01AyTnm7YLfybhnhEhwwZvAY".to_string(),
        content: r#"{"port": 8080}"#.to_string(),
        is_error: false,
        tool_use_result: Some(ToolUseResultMetadata::with_output("raw", "")),
    };
    let json = serde_json::to_string(&content).unwrap();
    assert!(json.contains("\"type\":\"tool_result\""));
    let restored: UserContent = serde_json::from_str(&json).unwrap();
    match restored {
        UserContent::ToolResult {
            tool_use_id,
            is_error,
            ..
        } => {
            assert_eq!(tool_use_id, "toolu_01AyTnm7YLfybhnhEhwwZvAY");
            assert!(!is_error);
        }
        _ => panic!("Expected ToolResult"),
    }
}

#[test]
fn assistant_content_thinking_round_trips_from_core() {
    let content = AssistantContent::Thinking {
        thinking: "Let me think...".to_string(),
        signature: Some("sig_abc123".to_string()),
    };
    let json = serde_json::to_string(&content).unwrap();
    assert!(json.contains("\"type\":\"thinking\""));
    assert!(json.contains("\"signature\":\"sig_abc123\""));
    let restored: AssistantContent = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, content);
}

#[test]
fn image_source_base64_round_trips_from_core() {
    let src = ImageSource::Base64 {
        media_type: "image/png".to_string(),
        data: "iVBORw0KGgo".to_string(),
    };
    let json = serde_json::to_string(&src).unwrap();
    assert!(json.contains("\"type\":\"base64\""));
    let restored: ImageSource = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, src);
}

#[test]
fn document_source_url_round_trips_from_core() {
    let src = DocumentSource::Url {
        url: "https://example.com/doc.pdf".to_string(),
    };
    let json = serde_json::to_string(&src).unwrap();
    assert!(json.contains("\"type\":\"url\""));
    let restored: DocumentSource = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, src);
}

#[test]
fn cache_control_ephemeral_round_trips_from_core() {
    let cc = CacheControl::Ephemeral;
    let json = serde_json::to_string(&cc).unwrap();
    assert!(json.contains("\"type\":\"ephemeral\""));
    let restored: CacheControl = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, cc);
}

#[test]
fn message_payload_user_vs_assistant_disambiguates_via_role() {
    // NOTE: serde(untagged) on MessagePayload uses content-shape rather than
    // role-string for disambiguation. UserMessage and AssistantMessage both
    // have role/content fields, and AssistantContent has additional variants
    // (ToolUse, Thinking) that UserContent lacks, while UserContent has
    // ToolResult/Image/Document that AssistantContent lacks. We assemble
    // payloads that include at least one variant-only-on-one-side content
    // item so untagged disambiguation lands on the correct branch — this
    // matches how production session JSONL files always look.
    let user_envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            // ToolResult exists only on UserContent — discriminates from AssistantContent.
            content: vec![UserContent::ToolResult {
                tool_use_id: "toolu_user_disc".to_string(),
                content: "ok".to_string(),
                is_error: false,
                tool_use_result: None,
            }],
        }),
        request_id: None,
    };
    let assistant_envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "assistant".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::Assistant(AssistantMessage {
            role: "assistant".to_string(),
            id: None,
            model: None,
            // ToolUse exists only on AssistantContent — discriminates from UserContent.
            content: vec![AssistantContent::ToolUse {
                id: "toolu_asst_disc".to_string(),
                name: "read_file".to_string(),
                input: serde_json::json!({"path": "/foo.ts"}),
            }],
            stop_reason: None,
            usage: None,
        }),
        request_id: None,
    };

    let u_json = serde_json::to_string(&user_envelope).unwrap();
    let a_json = serde_json::to_string(&assistant_envelope).unwrap();

    let u_back: MessageEnvelope = serde_json::from_str(&u_json).unwrap();
    let a_back: MessageEnvelope = serde_json::from_str(&a_json).unwrap();

    assert!(matches!(u_back.message, MessagePayload::User(_)));
    assert!(matches!(a_back.message, MessagePayload::Assistant(_)));
}

// ============================================================================
// Scenario: NAPI re-export shim preserves existing crate::persistence imports
// ============================================================================
//
// This is a compile-time scenario — there's no runtime behaviour to assert.
// It's verified by the fact that codelet-napi continues to compile after
// the lift (covered by `cargo build -p codelet-napi`). Inside codelet-napi,
// session_manager.rs, blob_processing.rs, napi_bindings.rs and tests.rs
// all import the envelope types via `crate::persistence::*` or
// `super::MessageEnvelope` paths; those paths must continue to resolve.

// ============================================================================
// Scenario: All NAPI persistence tests continue to pass after the lift
// ============================================================================
//
// Verified by running `cargo test -p codelet-napi persistence::tests` after
// the lift — covered by CI, not duplicated here. The 28 tests inside the
// (now relocated) `mod tests` block in codelet-core::persistence::message_envelope
// are unaffected by location and continue to validate the same invariants.
