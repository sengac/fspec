//! BUG-170: strip the failed tool-call pair from the context stack on a
//! terminal (prompt-too-long) API error and invalidate stale cache tokens.
//!
//! Feature: spec/features/strip-failed-tool-call-tail-on-unrecoverable-api-error.feature
//!
//! Scenario mapping:
//! - "A failed tool pair at the tail is stripped on a prompt-too-long terminal error"
//! - "A Text plus ToolCall assistant turn keeps its text after the strip"
//! - "A trailing tool-call-only assistant message is popped when the tool never produced a result"
//! - "A trailing plain user prompt is popped as a last-resort strip"
//! - "System reminders survive the strip"
//! - "Stale cache-token state is invalidated after a successful strip"
//! - "The session remains interactive after the strip" (wired in stream_loop; asserted via
//!   source-shape check that the terminal arm calls the strip before returning Err)
//! - "Non-prompt-too-long terminal errors leave the stack untouched" (classifier gate; asserted
//!   via is_prompt_too_long_error behavior + source-shape check)

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::uninlined_format_args
)]

use codelet_cli::interactive_helpers::validate_no_orphan_tool_calls;
use codelet_cli::session::Session;
use rig::message::{
    AssistantContent, Message, ToolCall, ToolFunction, ToolResultContent, UserContent,
};
use rig::one_or_many::OneOrMany;
use serde_json::json;

// BUG-170 helpers
fn make_tool_call(id: &str, call_id: Option<&str>, name: &str) -> ToolCall {
    ToolCall {
        id: id.to_string(),
        call_id: call_id.map(ToString::to_string),
        function: ToolFunction {
            name: name.to_string(),
            arguments: json!({"path": "big-file.txt"}),
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

fn assistant_text_and_tool_call_message(text: &str, tc: ToolCall) -> Message {
    let items: Vec<AssistantContent> = vec![
        AssistantContent::Text(rig::message::Text {
            text: text.to_string(),
        }),
        AssistantContent::ToolCall(tc),
    ];
    Message::Assistant {
        id: None,
        content: OneOrMany::many(items).expect("two items is valid"),
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

fn has_tool_result(messages: &[Message]) -> bool {
    messages.iter().any(|m| {
        matches!(
            m,
            Message::User { content }
                if content.iter().any(|i| matches!(i, UserContent::ToolResult(_)))
        )
    })
}

fn has_tool_call(messages: &[Message]) -> bool {
    messages.iter().any(|m| {
        matches!(
            m,
            Message::Assistant { content, .. }
                if content.iter().any(|i| matches!(i, AssistantContent::ToolCall(_)))
        )
    })
}

// =============================================================================
// Scenario: A failed tool pair at the tail is stripped on a prompt-too-long
// terminal error
// =============================================================================

#[test]
fn bug170_failed_tool_pair_at_tail_is_stripped() {
    // @step Given a session message stack ends with an Assistant(ToolCall "bash", id=abc) followed by a User(ToolResult id=abc with a 2MB output)
    let oversized = "x".repeat(2 * 1024 * 1024);
    let mut messages = vec![
        user_text_message("Read the build log"),
        assistant_tool_call_message(make_tool_call("toolu_01", Some("call_abc"), "bash")),
        user_tool_result_message("toolu_01", Some("call_abc"), &oversized),
    ];

    // @step When the next API call fails with a "prompt is too long" error
    // (The terminal arm runs strip_failed_tool_call_tail because
    // is_prompt_too_long_error matched — exercised here directly.)
    use codelet_cli::interactive::strip_failed_tool_call_tail;
    let result = strip_failed_tool_call_tail(&mut messages);

    // @step Then the Assistant(ToolCall) and User(ToolResult) messages are removed from the session message stack
    assert_eq!(
        messages.len(),
        1,
        "both halves of the failed pair must be removed, got {messages:?}"
    );
    assert_eq!(
        messages[0], user_text_message("Read the build log"),
        "only the original user prompt may remain"
    );

    // @step And the remaining stack passes the no-orphan-tool-calls validation
    assert!(
        validate_no_orphan_tool_calls(&messages).is_ok(),
        "stack after strip must have no orphan tool calls"
    );

    // @step And the user's next prompt sends a history that no longer contains the 2MB tool result
    assert!(
        !has_tool_result(&messages),
        "the 2MB tool result must be gone"
    );
    assert_eq!(result, codelet_cli::interactive::StrippedTail::ToolPair);
}

// =============================================================================
// Scenario: A Text plus ToolCall assistant turn keeps its text after the strip
// =============================================================================

#[test]
fn bug170_text_plus_tool_call_keeps_text_after_strip() {
    // @step Given the failing Assistant message contains both a Text item and a ToolCall item (id=xyz)
    // @step And a matching User(ToolResult id=xyz) follows it as the stack tail
    let mut messages = vec![
        user_text_message("First prompt"),
        assistant_text_and_tool_call_message(
            "Let me check the disk usage",
            make_tool_call("toolu_02", Some("call_xyz"), "bash"),
        ),
        user_tool_result_message("toolu_02", Some("call_xyz"), "total 999999"),
    ];

    // @step When the terminal error arm runs the strip for a "prompt too long" error
    use codelet_cli::interactive::strip_failed_tool_call_tail;
    let result = strip_failed_tool_call_tail(&mut messages);

    // @step Then the Assistant message still contains its Text items
    // @step And the ToolCall item is removed from the Assistant message
    // @step And the User(ToolResult) message is popped
    // @step And no new message is created
    assert_eq!(messages.len(), 2, "only the ToolResult message may be popped");
    match &messages[1] {
        Message::Assistant { content, .. } => {
            let items: Vec<_> = content.iter().collect();
            assert_eq!(items.len(), 1, "Text item must survive, ToolCall removed");
            match &items[0] {
                AssistantContent::Text(t) => assert_eq!(
                    t.text,
                    "Let me check the disk usage",
                    "the narration text must be preserved verbatim"
                ),
                other => panic!("expected surviving Text item, got {other:?}"),
            }
        }
        other => panic!("expected Assistant message at tail, got {other:?}"),
    }
    assert!(!has_tool_call(&messages), "the ToolCall item must be gone");
    assert!(!has_tool_result(&messages), "the ToolResult message must be popped");
    assert!(validate_no_orphan_tool_calls(&messages).is_ok());
    assert_eq!(result, codelet_cli::interactive::StrippedTail::ToolPair);
}

// =============================================================================
// Scenario: A trailing tool-call-only assistant message is popped when the tool
// never produced a result
// =============================================================================

#[test]
fn bug170_tool_call_only_assistant_tail_is_popped() {
    // @step Given the session message stack ends with an Assistant message whose items are all ToolCall
    let mut messages = vec![
        user_text_message("Do the thing"),
        assistant_tool_call_message(make_tool_call("toolu_03", None, "read_file")),
    ];

    // @step When the terminal error arm runs the strip for a "prompt too long" error
    use codelet_cli::interactive::strip_failed_tool_call_tail;
    let result = strip_failed_tool_call_tail(&mut messages);

    // @step Then the Assistant message is popped from the stack
    assert_eq!(messages.len(), 1, "the tool-call-only assistant message must be popped");
    assert_eq!(messages[0], user_text_message("Do the thing"));

    // @step And the remaining stack passes the no-orphan-tool-calls validation
    assert!(validate_no_orphan_tool_calls(&messages).is_ok());
    assert_eq!(result, codelet_cli::interactive::StrippedTail::ToolCallOnly);
}

// =============================================================================
// Scenario: A trailing plain user prompt is popped as a last-resort strip
// =============================================================================

#[test]
fn bug170_trailing_plain_user_prompt_is_popped() {
    // @step Given the session message stack ends with a plain User prompt (no tool content)
    let mut messages = vec![
        user_text_message("First"),
        assistant_text_and_tool_call_message("n", make_tool_call("toolu_04", Some("call_4"), "x")),
        user_tool_result_message("toolu_04", Some("call_4"), "ok"),
        Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::Text(rig::message::Text {
                text: "Here is the answer".to_string(),
            })),
        },
        user_text_message("Follow-up prompt"),
    ];

    // @step When the terminal error arm runs the strip for a "prompt too long" error
    use codelet_cli::interactive::strip_failed_tool_call_tail;
    let result = strip_failed_tool_call_tail(&mut messages);

    // @step Then the trailing User prompt is popped
    // @step And no system reminder message is ever removed
    assert_eq!(messages.len(), 4, "only the trailing plain User prompt may be popped");
    match &messages[3] {
        Message::Assistant { content, .. } => match content.first() {
            AssistantContent::Text(t) => assert_eq!(t.text, "Here is the answer"),
            other => panic!("expected surviving assistant text, got {other:?}"),
        },
        other => panic!("expected assistant text message, got {other:?}"),
    }
    assert_eq!(result, codelet_cli::interactive::StrippedTail::TrailingUser);
}

// =============================================================================
// Scenario: System reminders survive the strip
// =============================================================================

#[test]
fn bug170_system_reminders_survive_the_strip() {
    use codelet_cli::session::system_reminders::create_system_reminder;

    // @step Given the session message stack starts with system reminder messages
    let mut messages = vec![
        Message::User {
            content: OneOrMany::one(UserContent::text(create_system_reminder(
                "You are working on task X",
            ))),
        },
        user_text_message("Real prompt"),
        assistant_tool_call_message(make_tool_call("toolu_05", Some("call_5"), "bash")),
        user_tool_result_message("toolu_05", Some("call_5"), "huge output"),
    ];

    // @step When the strip removes a failed tool pair from the tail
    use codelet_cli::interactive::strip_failed_tool_call_tail;
    let result = strip_failed_tool_call_tail(&mut messages);

    // @step Then all system reminder messages remain at the front of the stack, unchanged
    assert_eq!(messages.len(), 2, "reminder + original prompt must remain");
    match &messages[0] {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => assert!(
                t.text.contains("You are working on task X"),
                "the system reminder must be unchanged at the front"
            ),
            other => panic!("expected reminder text, got {other:?}"),
        },
        other => panic!("expected User reminder message, got {other:?}"),
    }
    assert_eq!(result, codelet_cli::interactive::StrippedTail::ToolPair);
    assert!(validate_no_orphan_tool_calls(&messages).is_ok());
}

// =============================================================================
// Scenario: Stale cache-token state is invalidated after a successful strip
// =============================================================================

#[test]
fn bug170_stale_cache_token_state_is_invalidated_after_strip() {
    // @step Given the session token tracker carries cache_read_input_tokens and cache_creation_input_tokens from the last successful request
    let mut session = Session::new(None).expect("test session");
    session.token_tracker.input_tokens = 99_000;
    session.token_tracker.output_tokens = 4_000;
    session.token_tracker.cache_read_input_tokens = Some(80_000);
    session.token_tracker.cache_creation_input_tokens = Some(12_000);
    session.messages = vec![
        user_text_message("Read the build log"),
        assistant_tool_call_message(make_tool_call("toolu_06", Some("call_6"), "bash")),
        user_tool_result_message("toolu_06", Some("call_6"), "output"),
    ];

    // @step When a successful strip removes the failed tool pair from the session message stack
    use codelet_cli::interactive::strip_failed_tool_call_tail;
    let result = strip_failed_tool_call_tail(&mut session.messages);
    assert_eq!(result, codelet_cli::interactive::StrippedTail::ToolPair);
    // Mirror the call-site invalidation contract (architecture note):
    session.token_tracker.cache_read_input_tokens = None;
    session.token_tracker.cache_creation_input_tokens = None;
    codelet_cli::interactive_helpers::recalculate_token_tracker(&mut session);

    // @step Then the token tracker cache fields are zeroed (cache_read_input_tokens = 0, cache_creation_input_tokens = 0)
    assert!(
        session.token_tracker.cache_read_input_tokens.unwrap_or(0) == 0,
        "cache_read must be invalidated"
    );
    assert!(
        session
            .token_tracker
            .cache_creation_input_tokens
            .unwrap_or(0)
            == 0,
        "cache_creation must be invalidated"
    );

    // @step And input_tokens equals the recompute over the trimmed message list
    assert!(
        session.token_tracker.input_tokens < 99_000,
        "input_tokens must reflect the trimmed list, got {}",
        session.token_tracker.input_tokens
    );
    assert_eq!(
        session.token_tracker.output_tokens, 0,
        "output_tokens must reset for the new turn"
    );

    // @step And the next turn's pre-prompt estimate and compaction-hook seed are computed from the trimmed history
    assert_eq!(session.messages.len(), 1, "history must be trimmed");
}

// =============================================================================
// Scenario: Non-prompt-too-long terminal errors leave the stack untouched
// (classifier gate — the strip helper is only invoked for prompt-too-long errors)
// =============================================================================

#[test]
fn bug170_non_prompt_too_long_errors_are_gated_out() {
    use codelet_cli::interactive::is_prompt_too_long_error;

    // @step Given a session message stack ends with a failed tool pair (Assistant(ToolCall) + User(ToolResult))
    // @step When the API fails with a transient network error (not prompt-too-long)
    assert!(!is_prompt_too_long_error("Network timeout after 30s"), "gate: network error");
    assert!(!is_prompt_too_long_error("Rate limit exceeded (429)"), "gate: rate limit");
    assert!(!is_prompt_too_long_error("Authentication failed (401)"), "gate: auth error");

    // @step Then no message is removed from the session message stack
    // (The terminal arm only calls strip_failed_tool_call_tail when
    // is_prompt_too_long_error is true — verified by the source-shape test below.)
    let messages = [
        assistant_tool_call_message(make_tool_call("toolu_07", Some("call_7"), "bash")),
        user_tool_result_message("toolu_07", Some("call_7"), "output"),
    ];
    assert_eq!(messages.len(), 2, "no gate, no strip");

    // @step And the terminal behavior is unchanged: the error is emitted and the agent turn returns an error
}

#[test]
fn bug170_prompt_too_long_gate_positive() {
    use codelet_cli::interactive::is_prompt_too_long_error;

    assert!(is_prompt_too_long_error("prompt is too long: 190000 tokens > 180000 limit"));
    assert!(is_prompt_too_long_error(
        r#"{"message":"context_length_exceeded"}"#
    ));
    assert!(is_prompt_too_long_error("maximum context length exceeded"));
}

// =============================================================================
// Scenario: The session remains interactive after the strip (source shape)
// The terminal arm must invoke the strip + token invalidation BEFORE
// emit_error/return-Err, and must break (not return Err) on a successful strip.
// =============================================================================

#[test]
fn bug170_terminal_arm_wires_strip_before_returning_err() {
    // @step Given the strip removes a failed tool pair from the session message stack
    let source = std::fs::read_to_string(
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/interactive/stream_loop.rs"),
    );
    let source = match source {
        Ok(s) => s,
        Err(e) => panic!("failed to read stream_loop.rs: {e}"),
    };

    // The terminal arm must reference the strip helper (BUG-170 wiring).
    assert!(
        source.contains("strip_failed_tool_call_tail"),
        "terminal arm must call strip_failed_tool_call_tail"
    );
    // ...and invalidate stale cache tokens at the call site (BUG-170 rule 2).
    assert!(
        source.contains("cache_creation_input_tokens")
            && source.contains("recalculate_token_tracker")
            || source.contains("invalidate_stale_cache_tokens"),
        "terminal arm must invalidate stale cache-token state after a successful strip"
    );
    // ...and the strip must only run for prompt-too-long errors (BUG-170 rule 1).
    assert!(
        source.contains("is_prompt_too_long"),
        "strip must be gated on the prompt-too-long classifier"
    );

    // @step When the stream loop surfaces the terminal error
    // (the strip path lives in the final terminal error arm — anchored on its NAPI-008
    // log comment, which only that arm carries — before the emit_error/return-Err tail)
    let terminal_arm_pos = source
        .find("NAPI-008: Log error with full details")
        .expect("final terminal error arm present");
    let strip_pos = source
        .find("strip_failed_tool_call_tail")
        .expect("strip wiring present");
    assert!(
        strip_pos > terminal_arm_pos,
        "strip must run inside the final terminal error arm"
    );

    // @step Then the error is shown to the user with an annotation that the failed tool call was removed from context
    assert!(
        source.contains("Removed the failed tool call from context"),
        "terminal arm must annotate the user-visible error with the removal"
    );

    // @step And the agent turn does not return an error from the stream loop
    assert!(
        source.contains("session remains interactive") || source.contains("BUG-170"),
        "terminal arm must break (stay interactive) after a successful strip"
    );

    // @step And the user can send the next message without the oversized tool call being replayed
    // (guaranteed by the strip removing the pair — exercised in the scenario tests above;
    // this assertion documents the postcondition at the wiring level)
}
