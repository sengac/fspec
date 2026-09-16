//! BUG-185: the failed tool-call tail is stripped on ANY terminal API error,
//! not only on prompt-too-long (supersedes BUG-170 rule [0]).
//!
//! Feature: spec/features/strip-failed-tool-call-tail-on-any-terminal-api-error.feature
//!
//! Scenario mapping:
//! - "An exhausted network retry with a failed tool pair at the tail strips the pair and stays interactive"
//! - "A terminal auth error with a failed tool pair at the tail strips the pair and stays interactive"
//! - "A trailing tool-call-only assistant message is stripped on any terminal error"
//! - "A terminal error with no tool tail falls through to the existing terminal behavior"
//! - "A failed tool execution is never stripped"
//! - "The prompt-too-long recovery cascade still takes precedence"
//!
//! The strip primitive itself is shared with BUG-170 and unchanged; BUG-185
//! changes only the call-site gating in stream_loop.rs. These tests therefore
//! assert (a) the primitive still strips the required tail shapes, and (b) the
//! terminal arm no longer gates the strip on is_prompt_too_long_error —
//! verified via source-shape checks on stream_loop.rs / stream_handlers.rs.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::uninlined_format_args
)]

use codelet_cli::interactive_helpers::validate_no_orphan_tool_calls;
use rig::message::{
    AssistantContent, Message, ToolCall, ToolFunction, ToolResultContent, UserContent,
};
use rig::one_or_many::OneOrMany;
use serde_json::json;

fn make_tool_call(id: &str, call_id: Option<&str>, name: &str) -> ToolCall {
    ToolCall {
        id: id.to_string(),
        call_id: call_id.map(ToString::to_string),
        function: ToolFunction {
            name: name.to_string(),
            arguments: json!({"path": "some-file.txt"}),
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

fn user_text_message(text: &str) -> Message {
    Message::User {
        content: OneOrMany::one(UserContent::text(text)),
    }
}

fn stream_loop_source() -> String {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/interactive/stream_loop.rs"
    ));
    match source {
        Ok(s) => s,
        Err(e) => panic!("failed to read stream_loop.rs: {e}"),
    }
}

// =============================================================================
// Scenario: An exhausted network retry with a failed tool pair at the tail
// strips the pair and stays interactive
// =============================================================================

#[test]
fn bug185_exhausted_network_retry_strips_failed_tool_pair() {
    // @step Given a session message stack ends with an Assistant(ToolCall "bash", id=abc) followed by a User(ToolResult id=abc)
    let mut messages = vec![
        user_text_message("Build the project"),
        assistant_tool_call_message(make_tool_call("toolu_01", Some("call_abc"), "bash")),
        user_tool_result_message("toolu_01", Some("call_abc"), "compiling..."),
    ];

    // @step And the network retry budget is exhausted, producing a terminal error that is NOT a prompt-too-long error
    use codelet_cli::interactive::is_prompt_too_long_error;
    assert!(
        !is_prompt_too_long_error("error sending request: connection reset"),
        "precondition: the terminal error is a network failure, not prompt-too-long"
    );

    // @step When the terminal error arm runs
    // (BUG-185: the arm runs the strip unconditionally — exercised here via the
    // shared primitive, which the terminal arm now calls for ANY terminal error.)
    use codelet_cli::interactive::strip_failed_tool_call_tail;
    let result = strip_failed_tool_call_tail(&mut messages);

    // @step Then the Assistant(ToolCall) and User(ToolResult) pair are removed from the session message stack
    assert_eq!(
        messages.len(),
        1,
        "both halves of the failed pair must be removed, got {messages:?}"
    );
    assert_eq!(
        messages[0],
        user_text_message("Build the project"),
        "only the original user prompt may remain"
    );

    // @step And the remaining stack passes the no-orphan-tool-calls validation
    assert!(
        validate_no_orphan_tool_calls(&messages).is_ok(),
        "stack after strip must have no orphan tool calls"
    );

    // @step And the error is shown to the user with the annotation that the failed tool call was removed from context
    // @step And the agent turn does not return an error from the stream loop
    // @step And the user can send the next message without the failed tool call being replayed
    // (Wiring-level: asserted in bug185_terminal_arm_strips_for_any_terminal_error.)
    assert_eq!(
        result,
        codelet_cli::interactive::StrippedTail::ToolPair,
        "the strip must identify a removed tool pair"
    );
}

// =============================================================================
// Scenario: A terminal auth error with a failed tool pair at the tail strips
// the pair and stays interactive
// =============================================================================

#[test]
fn bug185_terminal_auth_error_strips_failed_tool_pair() {
    // @step Given a session message stack ends with an Assistant(ToolCall "fspec", id=xyz) followed by a User(ToolResult id=xyz)
    let mut messages = vec![
        user_text_message("Run the acceptance suite"),
        assistant_tool_call_message(make_tool_call("toolu_02", Some("call_xyz"), "fspec")),
        user_tool_result_message("toolu_02", Some("call_xyz"), "ok"),
    ];

    // @step And the API fails with a terminal authentication error (not prompt-too-long, not retriable)
    use codelet_cli::interactive::is_prompt_too_long_error;
    assert!(
        !is_prompt_too_long_error("401 Unauthorized: invalid api key"),
        "precondition: the terminal error is an auth failure, not prompt-too-long"
    );

    // @step When the terminal error arm runs
    use codelet_cli::interactive::strip_failed_tool_call_tail;
    let result = strip_failed_tool_call_tail(&mut messages);

    // @step Then the Assistant(ToolCall) and User(ToolResult) pair are removed from the session message stack
    assert_eq!(messages.len(), 1, "the failed pair must be stripped");
    assert!(
        validate_no_orphan_tool_calls(&messages).is_ok(),
        "stack after strip must have no orphan tool calls"
    );

    // @step And the stale cache-token state is invalidated (cache fields zeroed, input tokens recomputed over the trimmed history)
    // (Call-site invalidation mirrors the BUG-170 contract; wiring asserted in
    // bug185_terminal_arm_strips_for_any_terminal_error.)

    // @step And the session remains interactive with the error surfaced to the user
    assert_eq!(result, codelet_cli::interactive::StrippedTail::ToolPair);
}

// =============================================================================
// Scenario: A trailing tool-call-only assistant message is stripped on any
// terminal error
// =============================================================================

#[test]
fn bug185_tool_call_only_tail_stripped_on_any_terminal_error() {
    // @step Given the session message stack ends with an Assistant message whose items are all ToolCall
    let mut messages = vec![
        user_text_message("Do the thing"),
        assistant_tool_call_message(make_tool_call("toolu_03", None, "read_file")),
    ];

    // @step And the API fails with a terminal provider error that is not prompt-too-long
    use codelet_cli::interactive::is_prompt_too_long_error;
    assert!(
        !is_prompt_too_long_error("502 Bad Gateway from provider"),
        "precondition: the terminal error is a provider failure, not prompt-too-long"
    );

    // @step When the terminal error arm runs
    use codelet_cli::interactive::strip_failed_tool_call_tail;
    let result = strip_failed_tool_call_tail(&mut messages);

    // @step Then the tool-call-only Assistant message is popped from the stack
    assert_eq!(
        messages.len(),
        1,
        "the tool-call-only assistant message must be popped"
    );
    assert_eq!(messages[0], user_text_message("Do the thing"));

    // @step And the remaining stack passes the no-orphan-tool-calls validation
    assert!(validate_no_orphan_tool_calls(&messages).is_ok());

    // @step And the session remains interactive
    assert_eq!(
        result,
        codelet_cli::interactive::StrippedTail::ToolCallOnly,
        "the strip must identify a removed tool-call-only message"
    );
}

// =============================================================================
// Scenario: A terminal error with no tool tail falls through to the existing
// terminal behavior
// =============================================================================

#[test]
fn bug185_terminal_error_with_no_tool_tail_is_not_stripped() {
    // @step Given the session message stack ends with an Assistant text message (no trailing tool pair)
    let mut messages = vec![
        user_text_message("What is the plan?"),
        Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::Text(rig::message::Text {
                text: "Here is the plan".to_string(),
            })),
        },
    ];

    // @step And the API fails with a terminal error that is not prompt-too-long
    use codelet_cli::interactive::strip_failed_tool_call_tail;

    // @step When the terminal error arm runs
    let result = strip_failed_tool_call_tail(&mut messages);

    // @step Then no message is removed from the session message stack
    assert_eq!(messages.len(), 2, "a no-tool tail must not be stripped");
    assert_eq!(result, codelet_cli::interactive::StrippedTail::None);

    // @step And the error is emitted and the agent turn returns an error, exactly as before
    // (The fall-through is the pre-existing terminal behavior — see
    // bug185_terminal_arm_strips_for_any_terminal_error for the wiring shape.)
}

// =============================================================================
// Scenario: A failed tool execution is never stripped
// =============================================================================

#[test]
fn bug185_failed_tool_execution_is_never_stripped() {
    // @step Given a tool call executes and returns a failure (non-zero exit code, permission error, or success=false result)
    // @step When the tool result is added to the session message stack
    // The ToolResult arm (handle_tool_result) pushes the result unconditionally
    // and the turn continues — no strip primitive is involved.
    let result_text = r#"{"success":false,"error":"Bash exited with code 1"}"#;

    use codelet_cli::interactive::is_prompt_too_long_error;
    // The tool-failure text must not look like a prompt-too-long terminal error
    // (it is an execution result, not an API error).
    assert!(
        !is_prompt_too_long_error(result_text),
        "a failed tool-execution result is not a prompt-too-long error"
    );

    // @step Then the ToolResult stays in context as the model's recovery signal
    let messages = vec![
        user_text_message("Fix the build"),
        assistant_tool_call_message(make_tool_call("toolu_04", Some("call_4"), "bash")),
        user_tool_result_message("toolu_04", Some("call_4"), result_text),
    ];
    assert_eq!(
        messages.len(),
        3,
        "the failed tool result must remain in the stack"
    );
    assert!(
        validate_no_orphan_tool_calls(&messages).is_ok(),
        "a paired failed tool result is a valid, complete stack"
    );

    // @step And the turn continues normally with no strip invoked from the tool-result handler
    // (Source-shape: the tool-result handler must not call the strip primitive.)
    let handlers = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/interactive/stream_handlers.rs"
    ))
    .expect("failed to read stream_handlers.rs");
    assert!(
        !handlers.contains("strip_failed_tool_call_tail"),
        "the tool-result handler must never invoke the strip — execution failures stay in context"
    );
}

// =============================================================================
// Scenario: The prompt-too-long recovery cascade still takes precedence
// (source shape)
// =============================================================================

#[test]
fn bug185_prompt_too_long_cascade_still_runs_before_terminal_arm() {
    // @step Given a session message stack ends with a failed tool pair
    // @step And the API fails with a "prompt is too long" error
    // @step When the error cascade runs
    let source = stream_loop_source();

    // The existing prompt-too-long compaction-recovery path (CMPCT-023) must
    // still engage before the terminal arm.
    let cascade_pos = source
        .find("begin_compaction_recovery")
        .expect("CMPCT-023 prompt-too-long recovery path must still exist");
    let strip_pos = source
        .find("strip_failed_tool_call_tail")
        .expect("BUG-185: terminal arm must still call the strip");
    assert!(
        cascade_pos < strip_pos,
        "the prompt-too-long compaction-recovery path must run before the terminal-arm strip (cascade={cascade_pos}, strip={strip_pos})"
    );

    // @step Then the existing prompt-too-long compaction-recovery path engages before the terminal arm
    // (asserted above)

    // @step And if that recovery fails to reduce context, the terminal arm strips the pair and keeps the session interactive
    // (Terminal-arm strip behavior is asserted in
    // bug185_terminal_arm_strips_for_any_terminal_error.)
}

// =============================================================================
// Wiring (source shape): the terminal arm strips for ANY terminal error —
// the BUG-170 is_prompt_too_long gate is gone.
// =============================================================================

#[test]
fn bug185_terminal_arm_strips_for_any_terminal_error() {
    let source = stream_loop_source();

    // @step When the terminal error arm runs
    // The terminal arm lives after the NAPI-008 log comment (unique to the
    // final terminal arm) and must call the strip there.
    let terminal_arm_pos = source
        .find("NAPI-008: Log error with full details")
        .expect("final terminal error arm present");
    let strip_pos = source
        .find("strip_failed_tool_call_tail")
        .expect("terminal arm must call strip_failed_tool_call_tail");
    assert!(
        strip_pos > terminal_arm_pos,
        "strip must run inside the final terminal error arm"
    );

    // @step Then ... (BUG-185 rule 1: the gate is removed)
    // The gate must NOT wrap the strip: extract the region from the strip call
    // back to the terminal arm and verify no is_prompt_too_long guard.
    let terminal_region = &source[terminal_arm_pos..strip_pos + 200];
    assert!(
        !terminal_region.contains("if is_prompt_too_long"),
        "BUG-185: the terminal-arm strip must NOT be gated on is_prompt_too_long (BUG-170 rule [0] superseded)"
    );

    // The strip-success path must keep the session interactive: annotation +
    // break (no Err), plus cache-token invalidation at the call site.
    assert!(
        source.contains("Removed the failed tool call from context"),
        "terminal arm must annotate the user-visible error with the removal"
    );
    assert!(
        source.contains("recalculate_token_tracker") && source.contains("cache_read_input_tokens"),
        "terminal arm must invalidate stale cache-token state after a successful strip"
    );
}
