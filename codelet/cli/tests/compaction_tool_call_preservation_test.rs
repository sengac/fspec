#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/preserve-mid-tool-call-state-when-promptcancelled-fires.feature
//!
//! CMPCT-029: Preserve mid-tool-call state when PromptCancelled fires.
//!
//! These tests cover the three fspec-side recovery helpers:
//! - `validate_no_orphan_tool_calls` (defensive guard at execute_compaction entry)
//! - `reconcile_session_messages` (merges rig-side tool pairs into fspec's session)
//! - `inject_synthetic_tool_results_for_orphans` (closes dangling tool_calls)
//!
//! Plus a structural check that the rig patch at site 508 flushes the pending
//! tool_call/tool_result pair into `chat_history` before yielding
//! `PromptCancelled`. See the attached AST research for the cancel-site map.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, OnceLock};

use codelet_cli::interactive_helpers::{
    execute_compaction, inject_synthetic_tool_results_for_orphans, reconcile_session_messages,
    validate_no_orphan_tool_calls,
};
use codelet_cli::session::Session;
use rig::message::{
    AssistantContent, Message, ToolCall, ToolFunction, ToolResultContent, UserContent,
};
use rig::OneOrMany;

/// Process-wide tempdir kept alive for the lifetime of the test binary.
/// Mirrors the helper from `compaction_error_cascade_test.rs` — `execute_compaction`
/// fans out into debug-capture which requires `set_data_directory`.
static TEST_DATA_DIR: OnceLock<tempfile::TempDir> = OnceLock::new();

fn ensure_test_data_dir() {
    TEST_DATA_DIR.get_or_init(|| {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
        codelet_common::set_data_directory(temp_dir.path().to_path_buf())
            .expect("Failed to set data directory");
        temp_dir
    });
}

fn fresh_session() -> Session {
    Session::new(None).expect("failed to create test session")
}

fn make_tool_call(
    id: &str,
    call_id: Option<&str>,
    name: &str,
    args: serde_json::Value,
) -> ToolCall {
    ToolCall {
        id: id.to_string(),
        call_id: call_id.map(ToString::to_string),
        function: ToolFunction {
            name: name.to_string(),
            arguments: args,
        },
        signature: None,
        additional_params: None,
    }
}

fn assistant_tool_call_message(tc: ToolCall) -> Message {
    Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::ToolCall(tc)),
    }
}

fn user_tool_result_message(id: &str, call_id: Option<&str>, body: &str) -> Message {
    let content = OneOrMany::one(ToolResultContent::Text(rig::message::Text {
        text: body.to_string(),
    }));
    let user_content = match call_id {
        Some(cid) => UserContent::tool_result_with_call_id(id, cid.to_string(), content),
        None => UserContent::tool_result(id, content),
    };
    Message::User {
        content: OneOrMany::one(user_content),
    }
}

// ============================================================================
// Scenario: PromptCancelled at site 509 preserves the complete tool_call and
// tool_result pair in session.messages
// ============================================================================

#[test]
fn reconcile_appends_missing_tool_pair_from_rig_chat_history() {
    // @step Given the streaming hook fires cancel immediately after on_tool_result in rig
    // (Expressed as: rig's chat_history payload already carries the pair)

    // @step And rig's local tool_calls vec contains one tool_call and tool_results vec contains its matching result
    // @step And the rig patch has flushed that pair into chat_history before yielding PromptCancelled
    let call = make_tool_call(
        "toolu_01",
        Some("call_abc"),
        "read_file",
        serde_json::json!({"path": "README.md"}),
    );
    let rig_chat_history = vec![
        assistant_tool_call_message(call),
        user_tool_result_message("toolu_01", Some("call_abc"), "file contents"),
    ];

    // fspec's session knows nothing about this pair yet.
    let mut session_messages: Vec<Message> = Vec::new();

    // @step When fspec's compaction-cancel branch consumes the PromptCancelled error
    reconcile_session_messages(&mut session_messages, &rig_chat_history);

    // @step Then reconciliation appends the missing Assistant(ToolCall) message to session.messages
    assert!(
        matches!(
            session_messages.first(),
            Some(Message::Assistant { content, .. })
                if content.iter().any(|c| matches!(c, AssistantContent::ToolCall(tc) if tc.call_id.as_deref() == Some("call_abc")))
        ),
        "reconcile must append the missing Assistant(ToolCall) first"
    );

    // @step And reconciliation appends the matching User(ToolResult) message to session.messages
    assert!(
        matches!(
            session_messages.get(1),
            Some(Message::User { content })
                if content.iter().any(|c| matches!(c, UserContent::ToolResult(tr) if tr.call_id.as_deref() == Some("call_abc")))
        ),
        "reconcile must append the matching User(ToolResult) immediately after"
    );

    // @step And the orphan detector reports zero orphan call_ids for the reconciled session
    assert!(
        validate_no_orphan_tool_calls(&session_messages).is_ok(),
        "orphan detector must pass on reconciled history"
    );
}

// ============================================================================
// Scenario: PromptCancelled at site 486 closes the dangling tool_call with a
// synthetic cancelled tool_result
// ============================================================================

#[test]
fn inject_synthetic_result_closes_dangling_tool_call_at_site_486() {
    // @step Given fspec's tool_calls_buffer already contains one Assistant(ToolCall) for an in-flight tool
    let call = make_tool_call(
        "toolu_02",
        Some("call_xyz"),
        "bash",
        serde_json::json!({"cmd": "ls"}),
    );

    // @step And the streaming hook fires cancel during on_tool_call before the tool has executed
    // (Expressed as: no User(ToolResult) exists for this call yet)

    // @step And the session.messages tail already holds the Assistant(ToolCall) from the tool_calls_buffer flush
    let mut session_messages: Vec<Message> = vec![assistant_tool_call_message(call)];

    let orphans_before = validate_no_orphan_tool_calls(&session_messages)
        .err()
        .unwrap_or_default();
    assert_eq!(
        orphans_before.len(),
        1,
        "precondition: exactly one orphan tool_call must be present"
    );
    assert_eq!(orphans_before[0], "call_xyz");

    // @step When fspec's compaction-cancel branch runs inject_synthetic_tool_results_for_orphans
    let injected = inject_synthetic_tool_results_for_orphans(&mut session_messages);

    // Exactly one synthetic tool_result must have been injected.
    assert_eq!(injected, 1, "expected exactly one synthetic injection");

    // @step Then the dangling tool_call receives a matching User(ToolResult) carrying status "cancelled_by_context_limit"
    let tail = session_messages.last().expect("tail must exist");
    let tool_result = match tail {
        Message::User { content } => content
            .iter()
            .find_map(|c| match c {
                UserContent::ToolResult(tr) => Some(tr.clone()),
                _ => None,
            })
            .expect("tail must be a ToolResult"),
        other => panic!("tail must be a User(ToolResult), got {other:?}"),
    };

    let body_text: String = tool_result
        .content
        .iter()
        .map(|c| match c {
            ToolResultContent::Text(t) => t.text.clone(),
            ToolResultContent::Image(_) => String::new(),
        })
        .collect();
    assert!(
        body_text.contains("cancelled_by_context_limit"),
        "synthetic result body must carry the cancelled_by_context_limit status, got: {body_text:?}"
    );

    // @step And the synthetic tool_result uses the original call_id so the pair is structurally complete
    assert_eq!(
        tool_result.call_id.as_deref(),
        Some("call_xyz"),
        "synthetic result must reuse the original call_id"
    );
    assert_eq!(
        tool_result.id, "toolu_02",
        "synthetic result must reuse the original tool id"
    );

    // @step And the orphan detector now reports zero orphan call_ids
    assert!(
        validate_no_orphan_tool_calls(&session_messages).is_ok(),
        "orphan detector must pass after synthetic injection"
    );
}

// ============================================================================
// Scenario: Clean conversation passes the orphan detector with no changes
// ============================================================================

#[test]
fn clean_conversation_passes_orphan_detector_unchanged() {
    // @step Given a session whose Assistant tool_calls each have a matching User tool_result
    let call_1 = make_tool_call(
        "tool_1",
        Some("call_1"),
        "read",
        serde_json::json!({"path": "a"}),
    );
    let call_2 = make_tool_call(
        "tool_2",
        Some("call_2"),
        "bash",
        serde_json::json!({"cmd": "echo hi"}),
    );

    let mut session_messages: Vec<Message> = vec![
        assistant_tool_call_message(call_1),
        user_tool_result_message("tool_1", Some("call_1"), "A contents"),
        assistant_tool_call_message(call_2),
        user_tool_result_message("tool_2", Some("call_2"), "hi"),
    ];
    let len_before = session_messages.len();

    // @step When validate_no_orphan_tool_calls inspects session.messages
    let result = validate_no_orphan_tool_calls(&session_messages);

    // @step Then the detector returns Ok with zero orphan call_ids
    assert!(
        result.is_ok(),
        "clean conversation must pass the orphan detector"
    );

    // @step And inject_synthetic_tool_results_for_orphans reports zero synthetic injections
    let injected = inject_synthetic_tool_results_for_orphans(&mut session_messages);
    assert_eq!(injected, 0, "no orphan → no synthetic injections");
    assert_eq!(
        session_messages.len(),
        len_before,
        "message count must be unchanged when there are no orphans"
    );
    // @step And execute_compaction is allowed to proceed
    // (Exercised by a separate scenario — see execute_compaction_refuses_on_orphans_remaining;
    // this test's purpose is to verify the pre-condition holds on a clean history.)
}

// ============================================================================
// Scenario: Reconciliation does not duplicate tool pairs that fspec already
// holds
// ============================================================================

#[test]
fn reconcile_does_not_duplicate_pairs_fspec_already_has() {
    // @step Given rig's chat_history payload contains an Assistant(ToolCall) and matching User(ToolResult)
    let call = make_tool_call(
        "tool_dup",
        Some("call_dup"),
        "read",
        serde_json::json!({"path": "dup"}),
    );
    let rig_chat_history = vec![
        assistant_tool_call_message(call.clone()),
        user_tool_result_message("tool_dup", Some("call_dup"), "dup contents"),
    ];

    // @step And fspec's session.messages already contains the same Assistant(ToolCall) and User(ToolResult) pair
    let mut session_messages: Vec<Message> = vec![
        assistant_tool_call_message(call),
        user_tool_result_message("tool_dup", Some("call_dup"), "dup contents"),
    ];
    let len_before = session_messages.len();

    // @step When reconcile_session_messages runs with the rig_chat_history
    reconcile_session_messages(&mut session_messages, &rig_chat_history);

    // @step Then no new messages are appended to session.messages
    assert_eq!(
        session_messages.len(),
        len_before,
        "reconcile must not duplicate pairs fspec already has"
    );

    // @step And the deduplication key is the tool correlation id (call_id if present, otherwise id)
    // Exercise the fallback: rig history uses the id alone (call_id=None), fspec already has it.
    let call_no_cid = make_tool_call(
        "call_only_id",
        None,
        "bash",
        serde_json::json!({"cmd": "pwd"}),
    );
    let mut session_only_id: Vec<Message> = vec![
        assistant_tool_call_message(call_no_cid.clone()),
        user_tool_result_message("call_only_id", None, "/home/test"),
    ];
    let rig_only_id = vec![
        assistant_tool_call_message(call_no_cid),
        user_tool_result_message("call_only_id", None, "/home/test"),
    ];
    let before = session_only_id.len();
    reconcile_session_messages(&mut session_only_id, &rig_only_id);
    assert_eq!(
        session_only_id.len(),
        before,
        "dedupe must use id when call_id is None"
    );

    // @step And the orphan detector still reports zero orphan call_ids
    assert!(
        validate_no_orphan_tool_calls(&session_messages).is_ok(),
        "orphan detector must pass on deduplicated history"
    );
    assert!(
        validate_no_orphan_tool_calls(&session_only_id).is_ok(),
        "orphan detector must pass on id-based dedupe path"
    );
}

// ============================================================================
// Scenario: execute_compaction refuses to run when orphan tool_calls remain
// and reports the offending call_ids
// ============================================================================

#[tokio::test]
async fn execute_compaction_refuses_on_orphans_remaining() {
    ensure_test_data_dir();

    // @step Given a session whose tail contains an Assistant(ToolCall) with no matching User(ToolResult)
    let mut session = fresh_session();
    // Seed a full user/assistant pair so the session has compactable turns,
    // then append the dangling assistant tool_call.
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("first user prompt")),
    });
    session.messages.push(Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::Text(rig::message::Text {
            text: "ok".to_string(),
        })),
    });
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("second user prompt")),
    });
    session
        .messages
        .push(assistant_tool_call_message(make_tool_call(
            "dangling_1",
            Some("call_dangling"),
            "read",
            serde_json::json!({"path": "missing.txt"}),
        )));

    let messages_len_before = session.messages.len();
    session.token_tracker.input_tokens = 42;

    // @step And no reconciliation or synthetic injection has been performed
    // (We intentionally skip the recovery helpers so execute_compaction sees orphans.)

    let compaction_flag = Arc::new(AtomicBool::new(false));

    // @step When execute_compaction is invoked
    let result = execute_compaction(
        &mut session,
        compaction_flag.clone(),
        Some("original prompt"),
    )
    .await;

    // @step Then it returns an error describing the orphan call_ids
    let err = result.expect_err("execute_compaction must refuse when orphans remain");
    let err_text = format!("{err:#}");
    assert!(
        err_text.to_lowercase().contains("orphan") && err_text.contains("call_dangling"),
        "error must describe the orphan call_ids, got: {err_text:?}"
    );

    // @step And no compaction instruction is pushed onto session.messages
    assert_eq!(
        session.messages.len(),
        messages_len_before,
        "messages must not be mutated when the orphan guard fires"
    );

    // @step And the token tracker is NOT reset
    assert_eq!(
        session.token_tracker.input_tokens, 42,
        "token tracker must not be reset when the orphan guard fires"
    );
}

// ============================================================================
// Structural check — rig patch at site 508 flushes pending tool pair
// ============================================================================

#[test]
fn rig_streaming_patch_flushes_pending_tool_pair_before_prompt_cancelled_at_site_508() {
    // Locate codelet/patches/rig-core/src/agent/prompt_request/streaming.rs by
    // walking up from the crate's manifest dir.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest
        .parent()
        .expect("cli/ has a parent")
        .parent()
        .expect("codelet/ has a parent");
    let streaming_path = workspace_root
        .join("codelet")
        .join("patches")
        .join("rig-core")
        .join("src")
        .join("agent")
        .join("prompt_request")
        .join("streaming.rs");

    let source = std::fs::read_to_string(&streaming_path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", streaming_path.display()));

    // CMPCT-029: before yielding PromptCancelled AFTER on_tool_result, the
    // patch pushes the current (tool_call, tool_result) pair into the local
    // vecs and then flushes both vecs into chat_history so the error
    // payload carries the complete pair.
    //
    // We look for the tell-tale marker comment so the intent is observable
    // from the test (not just matching generic code shapes).
    assert!(
        source.contains("CMPCT-029"),
        "streaming.rs must carry a CMPCT-029 marker comment at the site 508 flush"
    );

    // Must append an Assistant message carrying tool_calls into chat_history
    // at the cancel site.
    assert!(
        source.contains("chat_history.write().await.push(Message::Assistant"),
        "streaming.rs must push an Assistant(tool_calls) into chat_history at the cancel flush"
    );

    // Must push the User(ToolResult) sequence — rig's natural-exit flush uses
    // `UserContent::tool_result_with_call_id` and `UserContent::tool_result`.
    // The CMPCT-029 flush must use the SAME pattern.
    assert!(
        source.contains("UserContent::tool_result_with_call_id")
            && source.contains("UserContent::tool_result"),
        "streaming.rs must push User(ToolResult) messages into chat_history at the cancel flush"
    );
}
